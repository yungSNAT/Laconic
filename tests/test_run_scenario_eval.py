"""Offline checks for session capture and compatibility with paired judging."""

import argparse
import contextlib
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import judge
import run_evals
import run_scenario_eval

SCENARIO = ROOT / "evals/scenarios/persistence-topic-switch-stop"


class ScenarioEvaluationTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name)
        self.args = argparse.Namespace(
            scenario=SCENARIO, condition="candidate",
            condition_skill=ROOT / "skills/laconic/SKILL.md",
            model="fixture", budget_usd=1.0, trial=1,
            output=self.root / "response.jsonl",
        )

    def run_capture(self):
        with contextlib.redirect_stdout(io.StringIO()):
            run_scenario_eval.run_scenario(self.args)
        return run_evals.read_jsonl(self.args.output)[0]

    def test_invalid_turns_are_rejected(self):
        original = run_scenario_eval.load_scenario(SCENARIO)
        self.assertEqual(6, len(original["turns"]))
        for turns in ([], [None, None], [{"id": "a", "prompt": ""}] * 2,
                      [{"id": "a", "prompt": "task"}] * 2):
            with self.subTest(turns=turns):
                case = dict(original, turns=turns)
                (self.root / "case.jsonl").write_text(json.dumps(case) + "\n")
                with self.assertRaises(ValueError):
                    run_scenario_eval.load_scenario(self.root)

    def test_validate_command_reports_valid_and_invalid_scenarios_without_a_provider(self):
        with mock.patch.object(sys, "argv", ["run_scenario_eval.py", "validate", str(SCENARIO)]), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(0, run_scenario_eval.main())
        self.assertIn(f"Valid scenario: {SCENARIO.name}", output.getvalue())

        (self.root / "case.jsonl").write_text("{}\n", encoding="utf-8")
        with mock.patch.object(sys, "argv", ["run_scenario_eval.py", "validate", str(self.root)]), \
                contextlib.redirect_stderr(io.StringIO()) as error:
            self.assertEqual(1, run_scenario_eval.main())
        self.assertTrue(error.getvalue().startswith("error:"))

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_capture_is_compatible_with_existing_blind_judge(self, run):
        run.return_value = subprocess.CompletedProcess(
            [], 0, json.dumps({"result": "fixture response", "total_cost_usd": 0.01}), ""
        )
        candidate = self.run_capture()
        calls = run.call_args_list
        commands = [call.args[0] for call in calls]
        session_id = commands[0][commands[0].index("--session-id") + 1]
        self.assertEqual(6, len(commands))
        self.assertIn(b"<response_style>", calls[0].kwargs["input"])
        self.assertNotIn(b"disable-model-invocation:", calls[0].kwargs["input"])
        for command, call in zip(commands[1:], calls[1:]):
            self.assertEqual(session_id, command[command.index("--resume") + 1])
            self.assertNotIn(b"<response_style>", call.kwargs["input"])
        self.assertEqual(1, len({call.kwargs["cwd"] for call in calls}))
        for call in calls:
            self.assertNotEqual(str(ROOT), call.kwargs["cwd"])
            self.assertIsNone(call.kwargs["stdin"])
            self.assertIn("--safe-mode", call.args[0])
            self.assertIn("--strict-mcp-config", call.args[0])
            command = call.args[0]
            self.assertEqual("", command[command.index("--tools") + 1])
        self.assertEqual("0.950000", commands[-1][-1])
        self.assertAlmostEqual(0.06, candidate["cost_usd"])

        self.args.condition = "baseline"
        self.args.output = self.root / "baseline.jsonl"
        run.reset_mock()
        baseline = self.run_capture()
        self.assertNotIn(b"response_style", run.call_args_list[0].kwargs["input"])
        self.assertEqual(baseline["response"], candidate["response"])
        groups = judge.group_responses([baseline, candidate])
        key = (SCENARIO.name, 1)
        labels = judge.assign_labels(key, list(groups[key]))
        prompt = judge.build_judge_prompt(
            run_scenario_eval.load_scenario(SCENARIO), groups[key], labels,
            judge.grader_rubric((ROOT / "evals/rubric.md").read_text()),
        )
        for hidden in ("baseline", "candidate", session_id, "response_style"):
            self.assertNotIn(hidden, prompt)
        verdict = {label: dict.fromkeys(judge.DIMENSIONS, 4) | {
            "blocker": False, "notes": "fixture only"
        } for label in labels.values()}
        scores = judge.parse_judge_scores(json.dumps(verdict), key, labels)
        self.assertEqual(2, len(scores))
        self.assertFalse(run_evals.summarize_scores(scores)["release_gate"]["passed"])

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_failed_capture_cannot_be_judged_as_complete(self, run):
        for payload, returncode, message in (
            ({"total_cost_usd": 0.12}, 0, "0.130000"),
            ({"result": "error", "total_cost_usd": 0.12}, 1, "0.130000"),
            ({"is_error": True, "result": "error", "total_cost_usd": 0.12}, 0, "0.130000"),
            ({"result": "ok", "total_cost_usd": 2}, 0, "Budget exceeded"),
        ):
            with self.subTest(payload=payload, returncode=returncode):
                self.args.output = self.root / f"failed-{returncode}-{message}-{len(payload)}.jsonl"
                run.side_effect = [
                    subprocess.CompletedProcess([], 0, '{"result":"ok","total_cost_usd":0.01}', ""),
                    subprocess.CompletedProcess([], returncode, json.dumps(payload), ""),
                ]
                with self.assertRaisesRegex(RuntimeError, message):
                    self.run_capture()
                self.assertEqual([], run_evals.read_jsonl(self.args.output))

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_invalid_costs_and_payloads_fail_closed(self, run):
        payloads = [[], None, "error"] + [
            {"result": "ok", "total_cost_usd": cost}
            for cost in (True, False, float("nan"), float("inf"), -1, None, "0.1")
        ]
        for payload in payloads:
            with self.subTest(payload=payload):
                run.return_value = subprocess.CompletedProcess([], 0, json.dumps(payload), "")
                with self.assertRaisesRegex(RuntimeError, "call cost unavailable"):
                    run_scenario_eval.call_claude([], str(self.root), 0.1)

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_invalid_inputs_and_existing_output_do_not_start_provider(self, run):
        for budget in (0, -1, 26, float("nan"), float("inf")):
            self.args.budget_usd = budget
            with self.assertRaises(ValueError):
                self.run_capture()
        self.args.budget_usd = 1
        self.args.output.write_text("existing results")
        with self.assertRaises(FileExistsError):
            self.run_capture()
        self.assertEqual("existing results", self.args.output.read_text())
        run.assert_not_called()

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_model_cannot_become_another_cli_option(self, run):
        run.return_value = subprocess.CompletedProcess(
            [], 0, '{"result":"ok","total_cost_usd":0.01}', ""
        )
        for index, model in enumerate(("", "-", "--help", "--tools=Bash", "-pwned",
                                       "opus --tools Bash", "opus\n", None)):
            with self.subTest(model=model):
                run.reset_mock()
                self.args.model = model
                self.args.output = self.root / f"invalid-{index}.jsonl"
                with self.assertRaisesRegex(ValueError, "Model must"):
                    self.run_capture()
                run.assert_not_called()
                self.assertFalse(self.args.output.exists())

        for index, model in enumerate(("sonnet", "claude-opus-4-1[1m]",
                                       "claude-opus-4-5@20251101",
                                       "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
                                       "vendor/model_2")):
            with self.subTest(model=model):
                run.reset_mock()
                self.args.model = model
                self.args.output = self.root / f"valid-{index}.jsonl"
                self.assertEqual(f"claude:{model}", self.run_capture()["runner"])
                for call in run.call_args_list:
                    self.assertIn(f"--model={model}", call.args[0])
                    self.assertNotIn("--model", call.args[0])

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_remaining_budget_is_rounded_down_and_stops_further_calls(self, run):
        self.args.budget_usd = 0.1234569
        run.return_value = subprocess.CompletedProcess(
            [], 0, '{"result":"ok","total_cost_usd":0.123456}', ""
        )
        with self.assertRaisesRegex(RuntimeError, "Budget exhausted"):
            self.run_capture()
        run.assert_called_once()
        self.assertEqual("0.123456", run.call_args.args[0][-1])
        self.assertEqual([], run_evals.read_jsonl(self.args.output))

    def test_actual_child_receives_scenario_prompts_as_data(self):
        # A parser exit must never substitute for a completed evaluation turn.
        fixture = self.root / "claude_fixture.py"
        fixture.write_text(
            "import argparse, json, sys\n"
            "parser = argparse.ArgumentParser()\n"
            "parser.add_argument('--version', action='version', version='fixture version')\n"
            "for flag in ('--safe-mode', '--strict-mcp-config', '--print'):\n"
            "    parser.add_argument(flag, action='store_true')\n"
            "for flag in ('--output-format', '--model', '--tools', '--session-id', "
            "'--resume', '--max-budget-usd'):\n"
            "    parser.add_argument(flag)\n"
            "parser.add_argument('prompt', nargs='?')\n"
            "args = parser.parse_args()\n"
            "prompt = args.prompt if args.prompt is not None else "
            "sys.stdin.buffer.read().decode('utf-8')\n"
            "print(json.dumps({'result': json.dumps({'prompt': prompt, "
            "'tools': args.tools, 'model': args.model}), 'total_cost_usd': 0.01}))\n",
            encoding="utf-8",
        )
        original = run_scenario_eval.load_scenario(SCENARIO)
        real_run = subprocess.run
        for condition in ("baseline", "candidate"):
            for first_prompt in ("--version", "--help"):
                with self.subTest(condition=condition, first_prompt=first_prompt):
                    prompts = [
                        first_prompt, "--version", "--help", "-h", "--tools=Bash",
                        "mcp", "auth status", "$(echo literal); & echo literal",
                        "  Croft Manor: caf\u00e9 \u6f22\u5b57 \U0001f3f9\nLF\r\nCRLF\rCR\n  ",
                    ]
                    turns = [{"id": str(i), "prompt": prompt}
                             for i, prompt in enumerate(prompts)]
                    (self.root / "case.jsonl").write_text(
                        json.dumps(dict(original, turns=turns)) + "\n", encoding="utf-8"
                    )
                    self.args.scenario = self.root
                    self.args.condition = condition
                    self.args.output = self.root / f"{condition}-{first_prompt}.jsonl"

                    def invoke_fixture(command, **kwargs):
                        return real_run(
                            [sys.executable, str(fixture), *command[1:]], **kwargs
                        )

                    with mock.patch("run_scenario_eval.subprocess.run",
                                    side_effect=invoke_fixture) as run:
                        row = self.run_capture()
                    transcript = json.loads(row["response"])
                    for index, (turn, call) in enumerate(zip(transcript, run.call_args_list)):
                        expected = run_evals._condition_prompt(
                            prompts[index], condition, self.args.condition_skill
                        ) if index == 0 else prompts[index]
                        response = json.loads(turn["response"])
                        self.assertEqual(expected, response["prompt"])
                        self.assertEqual("", response["tools"])
                        self.assertEqual("fixture", response["model"])
                        self.assertNotIn(expected, call.args[0])
                    self.assertEqual(len(prompts), len(transcript))
                    self.assertAlmostEqual(0.09, row["cost_usd"])

    @mock.patch("run_scenario_eval.subprocess.run")
    def test_utf8_stderr_is_readable_when_a_child_fails(self, run):
        for output, returncode, cost_message in (
            (b"not json", 0, "call cost unavailable"),
            (b'{"result":"error","total_cost_usd":0.02}', 1,
             "cumulative reported cost: $0.120000"),
        ):
            with self.subTest(output=output):
                run.return_value = subprocess.CompletedProcess(
                    [], returncode, output, "caf\u00e9 failed".encode("utf-8")
                )
                with self.assertRaises(RuntimeError) as failure:
                    run_scenario_eval.call_claude([], str(self.root), 0.1)
                self.assertIn("caf\u00e9 failed", str(failure.exception))
                self.assertIn(cost_message, str(failure.exception))

    def test_actual_child_gets_no_inherited_input_and_times_out(self):
        cli = "import json, sys; print(json.dumps({'result': repr(sys.stdin.read()), 'total_cost_usd': 0}))"
        wrapper = (
            "import sys; sys.path.insert(0, sys.argv[1]); import run_scenario_eval as r; "
            "print(r.call_claude([sys.executable, '-c', sys.argv[2]], sys.argv[3], 0))"
        )
        result = subprocess.run(
            [sys.executable, "-c", wrapper, str(ROOT / "scripts"), cli, str(self.root)],
            input="unrelated parent input", text=True, capture_output=True,
            timeout=5, check=True,
        )
        self.assertIn("''", result.stdout)
        self.assertNotIn("unrelated parent input", result.stdout)
        with mock.patch.object(run_scenario_eval, "CALL_TIMEOUT_SECONDS", 0.1):
            with self.assertRaisesRegex(RuntimeError, "timed out.*0.100000"):
                run_scenario_eval.call_claude(
                    [sys.executable, "-c", "import time; time.sleep(5)"], str(self.root), 0.1
                )


if __name__ == "__main__":
    unittest.main()
