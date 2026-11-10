import contextlib
import io
import json
import runpy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import judge  # noqa: E402
import run_evals  # noqa: E402


class LabelAssignmentTest(unittest.TestCase):
    def test_labels_are_a_stable_bijection_over_conditions(self):
        conditions = ["baseline", "candidate"]

        first = judge.assign_labels(("direct-answer", 1), conditions)
        second = judge.assign_labels(("direct-answer", 1), conditions)

        self.assertEqual(first, second)
        self.assertEqual(sorted(conditions), sorted(first))
        self.assertEqual({"A", "B"}, set(first.values()))

    def test_labels_vary_across_groups_so_position_never_leaks_condition(self):
        conditions = ["baseline", "candidate"]

        seen = {
            judge.assign_labels(("direct-answer", trial), conditions)["baseline"]
            for trial in range(20)
        }

        self.assertEqual({"A", "B"}, seen)


class GroupingTest(unittest.TestCase):
    def test_responses_group_by_case_and_trial(self):
        rows = [
            {"case_id": "direct-answer", "trial": 1, "condition": "baseline", "response": "51"},
            {"case_id": "direct-answer", "trial": 1, "condition": "candidate", "response": "102"},
            {"case_id": "direct-answer", "trial": 2, "condition": "baseline", "response": "again"},
        ]

        groups = judge.group_responses(rows)

        self.assertEqual({"baseline": "51", "candidate": "102"}, groups[("direct-answer", 1)])
        self.assertEqual({"baseline": "again"}, groups[("direct-answer", 2)])

    def test_groups_missing_a_condition_are_partitioned_out_not_dropped(self):
        groups = {
            ("direct-answer", 1): {"baseline": "x", "candidate": "y"},
            ("casual-message", 1): {"baseline": "only one condition ran"},
        }

        complete, incomplete = judge.partition_groups(groups, {"baseline", "candidate"})

        self.assertEqual([("direct-answer", 1)], sorted(complete))
        self.assertEqual([("casual-message", 1)], sorted(incomplete))


class ParseJudgeScoresTest(unittest.TestCase):
    @staticmethod
    def _verdict(value, blocker=False, notes="fixture"):
        return {
            "correctness": value,
            "autonomy": value,
            "actionability": value,
            "safety": value,
            "concision": value,
            "blocker": blocker,
            "notes": notes,
        }

    def test_labels_are_mapped_back_to_their_conditions(self):
        payload = json.dumps({"A": self._verdict(5), "B": self._verdict(2, blocker=True)})

        rows = judge.parse_judge_scores(
            payload, ("direct-answer", 1), {"baseline": "B", "candidate": "A"}
        )

        by_condition = {row["condition"]: row for row in rows}
        self.assertEqual(5, by_condition["candidate"]["correctness"])
        self.assertEqual(2, by_condition["baseline"]["correctness"])
        self.assertTrue(by_condition["baseline"]["blocker"])
        self.assertEqual("direct-answer", by_condition["candidate"]["case_id"])
        self.assertEqual(1, by_condition["candidate"]["trial"])

    def test_out_of_range_score_names_the_case_it_came_from(self):
        payload = json.dumps({"A": self._verdict(9), "B": self._verdict(3)})

        with self.assertRaisesRegex(ValueError, "direct-answer"):
            judge.parse_judge_scores(
                payload, ("direct-answer", 1), {"baseline": "B", "candidate": "A"}
            )

    def test_blocker_must_be_a_boolean_in_the_judges_verdict(self):
        payload = json.dumps({"A": self._verdict(4, blocker="false")})
        with self.assertRaisesRegex(ValueError, "blocker must be boolean"):
            judge.parse_judge_scores(payload, ("direct-answer", 1), {"candidate": "A"})

    def test_json_wrapped_in_code_fences_is_still_parsed(self):
        payload = "```json\n" + json.dumps({"A": self._verdict(4), "B": self._verdict(4)}) + "\n```"

        rows = judge.parse_judge_scores(
            payload, ("direct-answer", 1), {"baseline": "B", "candidate": "A"}
        )

        self.assertEqual({"baseline", "candidate"}, {row["condition"] for row in rows})

    def test_missing_label_names_the_label_the_judge_skipped(self):
        payload = json.dumps({"A": self._verdict(4)})

        with self.assertRaisesRegex(ValueError, "B"):
            judge.parse_judge_scores(
                payload, ("direct-answer", 1), {"baseline": "B", "candidate": "A"}
            )


class GraderRubricTest(unittest.TestCase):
    def test_only_the_marked_grader_section_is_extracted(self):
        rubric = (
            "# Response quality rubric\n"
            "<!-- judge:begin -->\n"
            "Score each dimension 1-5.\n"
            "<!-- judge:end -->\n"
            "Release the candidate only when it beats baseline.\n"
        )

        self.assertEqual("Score each dimension 1-5.", judge.grader_rubric(rubric))

    def test_rubric_without_markers_is_used_whole(self):
        self.assertEqual("everything", judge.grader_rubric("everything\n"))

    def test_shipped_rubric_grader_section_never_names_a_condition(self):
        shipped = (ROOT / "evals" / "rubric.md").read_text(encoding="utf-8")

        section = judge.grader_rubric(shipped).lower()

        self.assertNotIn("baseline", section)
        self.assertNotIn("candidate", section)
        self.assertIn("correctness", section)


class PromptTest(unittest.TestCase):
    CASE = {
        "id": "direct-answer",
        "prompt": "What is 17 multiplied by 6?",
        "criteria": ["States 102."],
        "risk": "low",
    }

    def test_prompt_carries_responses_under_labels_and_never_names_conditions(self):
        responses = {"baseline": "Great question! The answer is 102.", "candidate": "102"}
        labels = {"baseline": "B", "candidate": "A"}

        prompt = judge.build_judge_prompt(self.CASE, responses, labels, "SCORE 1-5 PER DIMENSION")

        self.assertIn("Great question! The answer is 102.", prompt)
        self.assertIn("SCORE 1-5 PER DIMENSION", prompt)
        self.assertIn("States 102.", prompt)
        self.assertNotIn("baseline", prompt.lower())
        self.assertNotIn("candidate", prompt.lower())
        self.assertNotIn("written on", prompt)

    def test_prompt_dates_each_response_so_today_is_judged_against_it(self):
        # A run that crosses midnight leaves conditions with different 'today's.
        responses = {"baseline": "Yesterday was 18 October.", "candidate": "Yesterday was 19 October."}
        labels = {"baseline": "B", "candidate": "A"}
        dates = {"baseline": "2026-10-19", "candidate": "2026-10-20"}

        prompt = judge.build_judge_prompt(self.CASE, responses, labels, "RUBRIC", dates)

        self.assertIn("## Response A (written on 20 October 2026)", prompt)
        self.assertIn("## Response B (written on 19 October 2026)", prompt)
        self.assertIn("not your own", prompt)
        self.assertEqual(
            {("direct-answer", 1): {"baseline": "2026-10-19"}},
            judge.group_dates(
                [
                    {"case_id": "direct-answer", "trial": 1, "condition": "baseline", "date": "2026-10-19"},
                    {"case_id": "direct-answer", "trial": 1, "condition": "candidate"},
                ]
            ),
        )


class NeutralWorkingDirectoryTest(unittest.TestCase):
    def test_runner_cwd_is_fresh_and_outside_the_repository(self):
        # An agent CLI adopts its working directory as project context. Run it
        # in the repo and it starts inspecting the eval harness instead of
        # answering the prompt, which contaminates the responses being graded.
        with run_evals._neutral_cwd() as first:
            cwd = Path(first).resolve()
            self.assertNotEqual(judge.ROOT.resolve(), cwd)
            self.assertFalse(str(cwd).startswith(str(judge.ROOT.resolve())))
            self.assertEqual([], list(cwd.iterdir()))
            (cwd / "state-from-prior-run").write_text("not reusable")

        with run_evals._neutral_cwd() as second:
            next_cwd = Path(second).resolve()
            self.assertNotEqual(cwd, next_cwd)
            self.assertEqual([], list(next_cwd.iterdir()))


class EndToEndTest(unittest.TestCase):
    VERDICT = {
        "correctness": 4,
        "autonomy": 4,
        "actionability": 4,
        "safety": 5,
        "concision": 3,
        "blocker": False,
        "notes": "fixture",
    }

    def test_judging_produces_paired_rows_the_scorer_accepts(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            responses = tmp_path / "responses.jsonl"
            responses.write_text(
                "".join(
                    json.dumps(row) + "\n"
                    for row in (
                        {
                            "case_id": "direct-answer",
                            "trial": 1,
                            "condition": "baseline",
                            "runner": "claude",
                            "response": "Great question! The answer is 102.",
                        },
                        {
                            "case_id": "direct-answer",
                            "trial": 1,
                            "condition": "candidate",
                            "runner": "claude",
                            "response": "102",
                        },
                    )
                )
            )
            payload = tmp_path / "verdict.json"
            payload.write_text(json.dumps({"A": self.VERDICT, "B": self.VERDICT}))
            captured = tmp_path / "prompt.txt"
            runner_config = tmp_path / "runners.json"
            runner_config.write_text(
                json.dumps(
                    {
                        "stub": {
                            # Reads the prompt from stdin, not argv: a trailing
                            # option such as `--tools ""` otherwise swallows a
                            # prompt appended to the command line.
                            "command": ["sh", "-c", f"cat > {captured.as_posix()}; cat {payload.as_posix()}"],
                            "response_format": "text",
                        }
                    }
                )
            )
            output = tmp_path / "scores.jsonl"

            exit_code = judge.main(
                [
                    "--responses", str(responses),
                    "--cases", str(ROOT / "evals" / "cases.jsonl"),
                    "--rubric", str(ROOT / "evals" / "rubric.md"),
                    "--runner-config", str(runner_config),
                    "--runner", "stub",
                    "--output", str(output),
                ]
            )

            self.assertEqual(0, exit_code)
            rows = run_evals.read_jsonl(output)
            self.assertEqual({"baseline", "candidate"}, {row["condition"] for row in rows})

            # The scorer is the real consumer: it must accept what the judge writes.
            summary = run_evals.summarize_scores(rows)
            self.assertEqual(2, summary["conditions"]["baseline"]["rows"] + summary["conditions"]["candidate"]["rows"])

            # The prompt actually sent must carry the responses but never the conditions.
            prompt = captured.read_text()
            self.assertIn("Great question! The answer is 102.", prompt)
            self.assertNotIn("baseline", prompt.lower())
            self.assertNotIn("candidate", prompt.lower())

    def test_a_malformed_verdict_skips_its_group_instead_of_killing_the_run(self):
        # One bad grader response must not discard the groups already judged
        # nor the ones still queued behind it.
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            responses = tmp_path / "responses.jsonl"
            responses.write_text(
                "".join(
                    json.dumps(
                        {
                            "case_id": case_id,
                            "trial": 1,
                            "condition": condition,
                            "runner": "claude",
                            "response": f"{case_id} {condition} text",
                        }
                    )
                    + "\n"
                    for case_id in ("direct-answer", "casual-message")
                    for condition in ("baseline", "candidate")
                )
            )
            good = tmp_path / "good.json"
            good.write_text(json.dumps({"A": self.VERDICT, "B": self.VERDICT}))
            runner_config = tmp_path / "runners.json"
            runner_config.write_text(
                json.dumps(
                    {
                        "stub": {
                            "command": [
                                "sh",
                                "-c",
                                # `blocker` omitted for the casual-message group.
                                f'p=$(cat); case "$p" in *casual-message*)'
                                f' echo \'{{"A":{{"correctness":3}},"B":{{"correctness":3}}}}\';;'
                                f" *) cat {good.as_posix()};; esac",
                            ],
                            "response_format": "text",
                        }
                    }
                )
            )
            output = tmp_path / "scores.jsonl"

            exit_code = judge.main(
                [
                    "--responses", str(responses),
                    "--cases", str(ROOT / "evals" / "cases.jsonl"),
                    "--rubric", str(ROOT / "evals" / "rubric.md"),
                    "--runner-config", str(runner_config),
                    "--runner", "stub",
                    "--retries", "0",
                    "--output", str(output),
                ]
            )

            rows = run_evals.read_jsonl(output)
            self.assertEqual({"direct-answer"}, {row["case_id"] for row in rows})
            self.assertEqual(2, len(rows))
            self.assertNotEqual(0, exit_code, "skipped groups must not report success")

    def test_missing_entire_condition_fails_before_judging(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            responses = tmp_path / "responses.jsonl"
            responses.write_text(
                json.dumps(
                    {
                        "case_id": "direct-answer",
                        "trial": 1,
                        "condition": "baseline",
                        "runner": "stub",
                        "response": "102",
                    }
                )
                + "\n"
            )
            output = tmp_path / "scores.jsonl"
            runner_config = tmp_path / "runners.json"
            runner_config.write_text(
                json.dumps(
                    {
                        "stub": {
                            "command": ["sh", "-c", "exit 99"],
                            "response_format": "text",
                        }
                    }
                )
            )

            with self.assertRaisesRegex(ValueError, "missing required condition"):
                judge.main(
                    [
                        "--responses", str(responses),
                        "--runner-config", str(runner_config),
                        "--runner", "stub",
                        "--output", str(output),
                    ]
                )
            self.assertFalse(output.exists())

    def test_runner_failure_skips_its_group_and_continues(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            responses = tmp_path / "responses.jsonl"
            responses.write_text(
                "".join(
                    json.dumps(
                        {
                            "case_id": case_id,
                            "trial": 1,
                            "condition": condition,
                            "runner": "stub",
                            "response": f"{case_id} {condition} text",
                        }
                    )
                    + "\n"
                    for case_id in ("direct-answer", "casual-message")
                    for condition in ("baseline", "candidate")
                )
            )
            verdict = tmp_path / "verdict.json"
            verdict.write_text(json.dumps({"A": self.VERDICT, "B": self.VERDICT}))
            runner_config = tmp_path / "runners.json"
            runner_config.write_text(
                json.dumps(
                    {
                        "stub": {
                            "command": [
                                "sh",
                                "-c",
                                f'p=$(cat); case "$p" in *direct-answer*) exit 7;; *) cat {verdict.as_posix()};; esac',
                            ],
                            "response_format": "text",
                        }
                    }
                )
            )
            output = tmp_path / "scores.jsonl"

            exit_code = judge.main(
                [
                    "--responses", str(responses),
                    "--runner-config", str(runner_config),
                    "--runner", "stub",
                    "--retries", "0",
                    "--output", str(output),
                ]
            )

            rows = run_evals.read_jsonl(output)
            self.assertEqual({"casual-message"}, {row["case_id"] for row in rows})
            self.assertEqual(2, len(rows))
            self.assertNotEqual(0, exit_code, "skipped groups must not report success")


class JudgeCommandTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        self.responses = root / "responses.jsonl"
        self.output = root / "scores.jsonl"
        config = root / "runners.json"
        config.write_text(json.dumps({
            "stub": {"command": ["stub-runner"], "response_format": "claude-json"},
        }), encoding="utf-8")
        self.options = ["--runner-config", str(config), "--runner", "stub", "--retries", "1"]

    def write_responses(self, entries):
        rows = [
            {"case_id": case, "condition": condition, "trial": trial, "response": "102"}
            for case, condition, trial in entries
        ]
        self.responses.write_text(
            "".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8"
        )

    @staticmethod
    def verdict(*labels):
        result = json.dumps({label: EndToEndTest.VERDICT for label in labels})
        return subprocess.CompletedProcess(
            [], 0, json.dumps({"result": result, "total_cost_usd": 0.25}), ""
        )

    def test_script_calibrates_using_only_the_labelled_fixture(self):
        self.responses.write_text(json.dumps({
            "case_id": "direct-answer", "name": "right", "response": "102", "blocker": False,
        }) + "\n", encoding="utf-8")
        script = ROOT / "scripts" / "judge.py"
        argv = [str(script), *self.options, "--calibrate", str(self.responses)]
        with mock.patch.object(sys, "argv", argv), \
                mock.patch.object(run_evals, "_run_with_timeout", return_value=self.verdict("A")) as invoke, \
                contextlib.redirect_stdout(io.StringIO()) as output, \
                self.assertRaises(SystemExit) as exit_status:
            runpy.run_path(str(script), run_name="__main__")
        self.assertEqual(0, exit_status.exception.code)
        self.assertEqual({"compared": 1, "agreement": 1.0, "errors": [], "cost_usd": 0.25},
                         json.loads(output.getvalue()))
        self.assertIn("## Response A\n\n102", invoke.call_args.kwargs["input"])
        self.assertFalse(self.output.exists())

    def test_judge_reports_empty_unexpected_and_unknown_responses(self):
        argv = [*self.options, "--responses", str(self.responses), "--output", str(self.output)]
        cases = (
            ([], "no responses to judge"),
            ([("direct-answer", name, 1) for name in ("baseline", "candidate", "comparator")],
             r"unexpected condition\(s\): comparator"),
            ([("unknown", name, 1) for name in ("baseline", "candidate")], "no matching case"),
        )
        with mock.patch.object(run_evals, "_run_with_timeout") as invoke:
            for entries, expected in cases:
                with self.subTest(expected=expected), self.assertRaisesRegex(ValueError, expected):
                    self.write_responses(entries)
                    judge.main(argv)
        invoke.assert_not_called()

    def test_resume_skips_judged_and_incomplete_trials_and_retries_failed_calls(self):
        self.write_responses(
            [("direct-answer", name, trial) for trial in (1, 3) for name in ("baseline", "candidate")]
            + [("direct-answer", "baseline", 2)]
        )
        prior = {"case_id": "direct-answer", "trial": 1, "condition": "baseline"}
        self.output.write_text(json.dumps(prior) + "\n", encoding="utf-8")
        results = [
            subprocess.CompletedProcess([], 1, "", "unavailable"),
            subprocess.CompletedProcess([], 0, '{"result": "bad verdict"}', ""),
            self.verdict("A", "B"),
        ]
        with mock.patch.object(run_evals, "_run_with_timeout", side_effect=results) as invoke, \
                mock.patch.object(judge.time, "sleep") as backoff, \
                contextlib.redirect_stdout(io.StringIO()) as output, \
                contextlib.redirect_stderr(io.StringIO()) as error:
            self.assertEqual(0, judge.main([
                *self.options, "--responses", str(self.responses), "--output", str(self.output),
            ]))
        rows = run_evals.read_jsonl(self.output)
        self.assertEqual(prior, rows[0])
        self.assertEqual({(3, "baseline"), (3, "candidate")},
                         {(row["trial"], row["condition"]) for row in rows[1:]})
        self.assertEqual(3, invoke.call_count)
        self.assertEqual([mock.call(1), mock.call(1)], backoff.call_args_list)
        self.assertIn("skip judged direct-answer/trial 1", output.getvalue())
        self.assertIn("Reported judge cost: $0.2500", output.getvalue())
        self.assertIn("skip direct-answer/trial 2: missing candidate", error.getvalue())


class CalibrateTest(unittest.TestCase):
    def test_each_labelled_response_is_judged_alone_and_disagreements_are_counted(self):
        fixtures = [
            {"name": "right", "case_id": "c", "response": "102.", "blocker": False},
            {"name": "wrong", "case_id": "c", "response": "112.", "blocker": True},
        ]
        prompts = []

        def call(prompt, key, labels):
            prompts.append(prompt)
            return [{"blocker": False, "notes": "fine"}], 0.25

        result = judge.calibrate(fixtures, {"c": {"prompt": "17 times 6?"}}, "rubric", call)
        self.assertIn("## Response A\n\n112.", prompts[1])
        self.assertNotIn("102.", prompts[1])
        self.assertEqual((2, 0.5, 0.5), (result["compared"], result["agreement"], result["cost_usd"]))
        self.assertEqual([("wrong", True, False)], [(e["fixture"], e["expected"], e["got"]) for e in result["errors"]])

    def test_calibration_rejects_an_empty_fixture_file(self):
        with self.assertRaisesRegex(ValueError, "no calibration fixtures"):
            judge.calibrate([], {}, "rubric", lambda *_: ([], None))

    def test_calibration_fixtures_name_real_cases_and_cover_both_labels(self):
        cases = {case["id"] for case in run_evals.load_cases(ROOT / "evals" / "cases.jsonl")}
        fixtures = run_evals.read_jsonl(ROOT / "evals" / "judge-calibration.jsonl")
        self.assertTrue(8 <= len(fixtures) <= 10)
        self.assertEqual(len(fixtures), len({fixture["name"] for fixture in fixtures}))
        self.assertLessEqual({fixture["case_id"] for fixture in fixtures}, cases)
        self.assertEqual({True, False}, {fixture["blocker"] for fixture in fixtures})

    def test_main_requires_responses_unless_calibrating(self):
        with self.assertRaises(SystemExit):
            judge.main(["--runner", "claude", "--output", "scores.jsonl"])


if __name__ == "__main__":
    unittest.main()
