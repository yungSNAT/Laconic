import argparse
import contextlib
import datetime
import hashlib
import io
import json
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import run_evals  # noqa: E402


class EvaluationHarnessTest(unittest.TestCase):
    def test_plan_emits_one_row_per_case_trial_and_condition(self):
        cases_path = ROOT / "evals" / "holdout-cases.jsonl"
        case_ids = {case["id"] for case in run_evals.load_cases(cases_path)}
        with contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(0, run_evals.main([
                "plan", "--cases", str(cases_path), "--trials", "2", "--include-comparator",
            ]))

        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        expected = {
            (case_id, trial, condition)
            for case_id in case_ids
            for trial in (1, 2)
            for condition in ("baseline", "candidate", "comparator")
        }
        self.assertEqual(len(expected), len(rows))
        self.assertEqual(expected, {
            (row["case_id"], row["trial"], row["condition"]) for row in rows
        })

    def test_validate_and_plan_reject_an_invalid_catalog(self):
        with tempfile.TemporaryDirectory() as temporary:
            cases_path = Path(temporary) / "cases.jsonl"
            cases_path.write_text("{}\n", encoding="utf-8")
            with contextlib.redirect_stderr(io.StringIO()) as error:
                self.assertEqual(1, run_evals.main(["validate", "--cases", str(cases_path)]))
            self.assertIn("Case 1: missing fields", error.getvalue())
            with self.assertRaisesRegex(ValueError, "Case 1: missing fields"):
                run_evals.main(["plan", "--cases", str(cases_path)])

    def test_holdout_cases_are_valid_and_frozen(self):
        # Changing a held-out case after seeing results would make it a tuning case;
        # replace the whole set for a new comparison instead.
        cases = run_evals.load_cases(ROOT / "evals" / "holdout-cases.jsonl")
        self.assertEqual([], run_evals.validate_cases(cases))
        digest = hashlib.sha256(json.dumps(cases, sort_keys=True).encode()).hexdigest()
        self.assertEqual("328d719abc93df748ba1e5a399eb594ae728d1745141276dd21f384b6943d56d", digest)

    def test_case_catalog_is_valid_and_balanced(self):
        cases = run_evals.load_cases(ROOT / "evals" / "cases.jsonl")
        errors = run_evals.validate_cases(cases)

        self.assertEqual([], errors)
        self.assertGreaterEqual(len(cases), 12)
        self.assertGreaterEqual(len({case["category"] for case in cases}), 8)

    def test_regex_checks_must_accept_good_and_reject_bad_examples(self):
        good = {"must_include": [r"\b102\b"], "pass": ["102."], "fail": ["112."]}
        self.assertEqual([], run_evals._validate_checks(good))

        wrong = {**good, "pass": ["112."]}
        self.assertTrue(any("pass example" in e for e in run_evals._validate_checks(wrong)))
        lax = {**good, "fail": ["It is 102."]}
        self.assertTrue(any("passes every check" in e for e in run_evals._validate_checks(lax)))
        self.assertTrue(run_evals._validate_checks({"must_include": ["("], "pass": ["x"], "fail": ["y"]}))
        self.assertTrue(run_evals._validate_checks({"pass": ["x"], "fail": ["y"]}))

    def test_malformed_check_fields_are_errors_not_reinterpretations(self):
        for checks in (
            {"must_include": "102", "pass": ["120"], "fail": ["112"]},
            {"must_include": None, "pass": ["102"], "fail": ["112"]},
            {"must_include": ["102"], "pass": "102", "fail": ["112"]},
            {"must_include": ["102"], "pass": [None], "fail": ["112"]},
        ):
            with self.subTest(checks=checks):
                errors = run_evals._validate_checks(checks)
                self.assertTrue(any("list of strings" in e for e in errors), errors)

    def test_check_summary_scores_only_cases_with_checks(self):
        cases = [
            {"id": "sum", "checks": {"must_include": [r"\b102\b"], "must_not_include": ["percent"]}},
            {"id": "free"},
        ]
        responses = [
            {"case_id": "sum", "trial": 1, "condition": "candidate", "response": "102"},
            {"case_id": "sum", "trial": 2, "condition": "candidate", "response": "102 percent"},
            {"case_id": "free", "trial": 1, "condition": "candidate", "response": "anything"},
        ]

        summary = run_evals.summarize_checks(cases, responses)["candidate"]

        self.assertEqual((2, 1, 0.5), (summary["checked"], summary["passed"], summary["pass_rate"]))
        self.assertEqual(["contains 'percent'"], summary["failures"][0]["failed"])

    def test_summarize_checks_reports_length_as_a_share_of_the_target(self):
        cases = [{"id": "draft", "target_words": 4}, {"id": "free"}]
        responses = [
            {"case_id": "draft", "trial": 1, "condition": "candidate", "response": "one two three"},
            {"case_id": "draft", "trial": 2, "condition": "candidate", "response": "one two three four five"},
            {"case_id": "free", "trial": 1, "condition": "candidate", "response": "anything"},
        ]

        summary = run_evals.summarize_checks(cases, responses)["candidate"]

        self.assertEqual([3, 5], [item["words"] for item in summary["target_length"]])
        self.assertEqual([0.75, 1.25], [item["share"] for item in summary["target_length"]])
        self.assertEqual(1.0, summary["mean_target_share"])
        self.assertEqual((0, None), (summary["checked"], summary["pass_rate"]))

    def test_validate_cases_rejects_a_target_that_is_not_a_positive_integer(self):
        base = {"id": "draft", "category": "prose-draft", "prompt": "Write.", "risk": "low", "criteria": ["Lands near the target."]}
        self.assertEqual([], run_evals.validate_cases([{**base, "target_words": 400}]))
        for bad in (0, -5, 4.5, "400", True):
            self.assertTrue(run_evals.validate_cases([{**base, "target_words": bad}]), bad)


    def test_parse_response_tolerates_output_after_the_json_document(self):
        """The CLI can emit a notice after its JSON result; the first document still wins."""
        payload = json.dumps(
            {"result": "102", "usage": {"input_tokens": 2}, "total_cost_usd": 0.03}
        )
        noisy = payload + "\nWarning: no stdin data received in 3s, proceeding without it.\n"

        text, usage, cost = run_evals._parse_response(noisy, "claude-json")

        self.assertEqual("102", text)
        self.assertEqual({"input_tokens": 2}, usage)
        self.assertAlmostEqual(0.03, cost)

    def test_runner_invocation_closes_child_stdin(self):
        """A runner that inherits stdin can read unrelated bytes into the prompt."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cases = root / "cases.jsonl"
            cases.write_text(
                json.dumps(
                    {
                        "id": "probe",
                        "category": "direct-answer",
                        "prompt": "What is 17 multiplied by 6?",
                        "risk": "low",
                        "criteria": ["Answers 102."],
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            runners = root / "runners.json"
            runners.write_text(
                json.dumps(
                    {
                        "stub": {
                            "command": ["stub-runner"],
                            "response_format": "claude-json",
                        }
                    }
                ),
                encoding="utf-8",
            )
            output = root / "responses.jsonl"
            args = argparse.Namespace(
                cases=cases,
                runner_config=runners,
                runner="stub",
                condition="baseline",
                condition_skill=None,
                case=None,
                trials=1,
                retries=0,
                budget_usd=1.0,
                allow_unmetered=False,
                output=output,
            )
            completed = subprocess.CompletedProcess(
                args=["stub-runner"],
                returncode=0,
                stdout=json.dumps({"result": "102", "usage": {}, "total_cost_usd": 0.01}),
                stderr="",
            )

            with mock.patch.object(
                run_evals.subprocess, "run", return_value=completed
            ) as runner:
                run_evals.run_evaluations(args)

            self.assertEqual(1, runner.call_count)
            self.assertIs(subprocess.DEVNULL, runner.call_args.kwargs.get("stdin"))

    def test_parallel_jobs_record_every_case_and_stop_at_the_budget(self):
        completed = subprocess.CompletedProcess(
            args=["stub"], returncode=0, stdout=json.dumps({"result": "ok", "usage": {}, "total_cost_usd": 0.4}), stderr=""
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            runners = root / "runners.json"
            runners.write_text(json.dumps({"stub": {"command": ["stub"], "response_format": "claude-json"}}), encoding="utf-8")
            ids = ["direct-answer", "debugging-cause", "concept-explanation", "error-report"]
            args = argparse.Namespace(
                cases=ROOT / "evals" / "cases.jsonl", runner_config=runners, runner="stub", condition="baseline",
                condition_skill=None, case=ids, trials=1, retries=0, budget_usd=10.0,
                allow_unmetered=False, output=root / "out.jsonl", jobs=3,
            )
            with mock.patch.object(run_evals.subprocess, "run", return_value=completed):
                self.assertEqual(0, run_evals.run_evaluations(args))
            self.assertEqual(sorted(ids), sorted(row["case_id"] for row in run_evals.read_jsonl(args.output)))

            args.output, args.budget_usd, args.jobs = root / "capped.jsonl", 0.5, 2
            with mock.patch.object(run_evals.subprocess, "run", return_value=completed):
                self.assertEqual(2, run_evals.run_evaluations(args))
            # Calls already running finish after the budget runs out: 2 or 3 of the 4
            # run, depending on whether the first pair completes together.
            self.assertIn(len(run_evals.read_jsonl(args.output)), {2, 3})

    def test_a_failed_parallel_call_keeps_the_others_results(self):
        ok = {"result": "ok", "usage": {}, "total_cost_usd": 0.25}

        def run(cmd, **kwargs):
            if "17 multiplied" in cmd[-1]:
                return subprocess.CompletedProcess(args=cmd, returncode=1, stdout="", stderr="boom")
            time.sleep(delay)
            return subprocess.CompletedProcess(args=cmd, returncode=0, stdout=json.dumps(ok), stderr="")

        # The success may finish alongside the failure (delay 0) or after it is seen (delay 0.3).
        for delay in (0.0, 0.3):
            with self.subTest(delay=delay), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                runners = root / "runners.json"
                runners.write_text(json.dumps({"stub": {"command": ["stub"], "response_format": "claude-json"}}), encoding="utf-8")
                args = argparse.Namespace(
                    cases=ROOT / "evals" / "cases.jsonl", runner_config=runners, runner="stub", condition="baseline",
                    condition_skill=None, case=["direct-answer", "debugging-cause"], trials=1, retries=0,
                    budget_usd=10.0, allow_unmetered=False, output=root / "out.jsonl", jobs=2,
                )
                with mock.patch.object(run_evals.subprocess, "run", side_effect=run):
                    with self.assertRaisesRegex(RuntimeError, "boom"):
                        run_evals.run_evaluations(args)
                self.assertEqual(["debugging-cause"], [row["case_id"] for row in run_evals.read_jsonl(args.output)])

                completed = subprocess.CompletedProcess(args=["stub"], returncode=0, stdout=json.dumps(ok), stderr="")
                with mock.patch.object(run_evals.subprocess, "run", return_value=completed) as resumed:
                    self.assertEqual(0, run_evals.run_evaluations(args))
                self.assertEqual(1, resumed.call_count, "the saved response is not repeated")
                self.assertEqual(
                    ["debugging-cause", "direct-answer"], [row["case_id"] for row in run_evals.read_jsonl(args.output)]
                )

    def test_score_summary_applies_weights_and_release_gates(self):
        scores = []
        for condition, value in (("baseline", 3), ("candidate", 4)):
            scores.append(
                {
                    "case_id": "direct-answer",
                    "trial": 1,
                    "condition": condition,
                    "correctness": value,
                    "autonomy": value,
                    "actionability": value,
                    "safety": value,
                    "concision": value,
                    "blocker": False,
                    "notes": "fixture",
                }
            )

        summary = run_evals.summarize_scores(scores)

        self.assertAlmostEqual(3.0, summary["conditions"]["baseline"]["weighted_score"])
        self.assertAlmostEqual(4.0, summary["conditions"]["candidate"]["weighted_score"])
        self.assertTrue(summary["release_gate"]["passed"])

    def test_a_reference_condition_can_replace_the_baseline(self):
        scores = [self._score_row("direct-answer", "comparator", 3), self._score_row("direct-answer", "candidate", 4)]
        with self.assertRaisesRegex(ValueError, "baseline and candidate"):
            run_evals.summarize_scores(scores)
        summary = run_evals.summarize_scores(scores, "comparator")
        self.assertEqual("comparator", summary["reference"])
        self.assertTrue(summary["release_gate"]["passed"])
        with self.assertRaisesRegex(ValueError, "other than candidate"):
            run_evals.summarize_scores(scores, "candidate")

        usage = {"input_tokens": 10, "output_tokens": 5}
        rows = [
            self._usage_row("direct-answer", "comparator", "abcd", 1.0, usage),
            self._usage_row("direct-answer", "candidate", "ab", 0.5, usage),
        ]
        measured = run_evals.summarize_usage(rows, "comparator")
        self.assertAlmostEqual(-50, measured["delta"]["candidate"]["cost_usd_pct"])
        self.assertNotIn("comparator", measured["delta"])

        with tempfile.TemporaryDirectory() as temporary:
            for command, records, expected in (("score", scores, summary), ("measure", rows, measured)):
                with self.subTest(command=command):
                    path = Path(temporary) / f"{command}.jsonl"
                    path.write_text("".join(json.dumps(row) + "\n" for row in records), encoding="utf-8")
                    with contextlib.redirect_stdout(io.StringIO()) as output:
                        self.assertEqual(0, run_evals.main([command, str(path), "--reference", "comparator"]))
                    self.assertEqual(expected, json.loads(output.getvalue()))

    def test_candidate_blocker_fails_release_gate(self):
        rows = []
        for condition in ("baseline", "candidate"):
            rows.append(
                {
                    "case_id": "dangerous-action",
                    "trial": 1,
                    "condition": condition,
                    "correctness": 5,
                    "autonomy": 5,
                    "actionability": 5,
                    "safety": 5,
                    "concision": 5,
                    "blocker": condition == "candidate",
                    "notes": "fixture",
                }
            )

        summary = run_evals.summarize_scores(rows)

        self.assertFalse(summary["release_gate"]["passed"])
        self.assertIn("blocking", " ".join(summary["release_gate"]["reasons"]))

    def test_conditions_judged_on_different_cases_are_rejected(self):
        rows = [
            self._score_row("destructive-action", "baseline", 2),
            self._score_row("medical-boundary", "baseline", 2),
            self._score_row("direct-answer", "candidate", 5),
        ]

        with self.assertRaisesRegex(ValueError, "not judged on the same rows"):
            run_evals.summarize_scores(rows)

    def test_duplicate_score_rows_are_rejected(self):
        rows = [
            self._score_row("direct-answer", "baseline", 3),
            self._score_row("direct-answer", "candidate", 4),
            self._score_row("direct-answer", "candidate", 5),
        ]

        with self.assertRaisesRegex(ValueError, "duplicate score rows"):
            run_evals.summarize_scores(rows)

    def test_usage_summary_reports_token_and_cost_deltas(self):
        rows = [
            self._usage_row(
                "direct-answer",
                "baseline",
                "abcdefghij",
                1.0,
                {
                    "input_tokens": 100,
                    "cache_creation_input_tokens": 20,
                    "cache_read_input_tokens": 10,
                    "output_tokens": 80,
                },
            ),
            self._usage_row(
                "medical-boundary",
                "baseline",
                "abcdef",
                0.5,
                {"input_tokens": 50, "output_tokens": 20},
            ),
            self._usage_row(
                "direct-answer",
                "candidate",
                "abcd",
                0.6,
                {"input_tokens": 110, "cached_input_tokens": 10, "output_tokens": 40},
            ),
            self._usage_row(
                "medical-boundary",
                "candidate",
                "wxyz",
                0.3,
                {"input_tokens": 40, "output_tokens": 10},
            ),
        ]

        summary = run_evals.summarize_usage(rows)

        self.assertEqual("claude", summary["runner"])
        self.assertEqual(180, summary["conditions"]["baseline"]["input_tokens"])
        self.assertEqual(50, summary["conditions"]["candidate"]["output_tokens"])
        self.assertAlmostEqual(25.0, summary["conditions"]["candidate"]["mean_output_tokens"])
        self.assertEqual(-50, summary["delta"]["candidate"]["output_tokens"])
        self.assertAlmostEqual(-50.0, summary["delta"]["candidate"]["output_tokens_pct"])
        self.assertAlmostEqual(-0.6, summary["delta"]["candidate"]["cost_usd"])
        self.assertAlmostEqual(-40.0, summary["delta"]["candidate"]["cost_usd_pct"])
        self.assertAlmostEqual(
            -50.0, summary["delta"]["candidate"]["mean_response_chars_pct"]
        )

    def test_usage_summary_rejects_mixed_runners(self):
        rows = [
            self._usage_row("direct-answer", "baseline", "a", 0.1, {}, runner="claude"),
            self._usage_row("direct-answer", "candidate", "b", None, {}, runner="codex"),
        ]

        with self.assertRaisesRegex(ValueError, "claude.*codex"):
            run_evals.summarize_usage(rows)

    def test_usage_summary_rejects_unpaired_conditions(self):
        rows = [
            self._usage_row("direct-answer", "baseline", "a", 0.1, {}),
            self._usage_row("medical-boundary", "baseline", "b", 0.1, {}),
            self._usage_row("direct-answer", "candidate", "c", 0.1, {}),
        ]

        with self.assertRaisesRegex(ValueError, "not judged on the same rows"):
            run_evals.summarize_usage(rows)

    def test_usage_summary_marks_unreported_cost(self):
        rows = [
            self._usage_row("direct-answer", "baseline", "a", 0.1, {}),
            self._usage_row("direct-answer", "candidate", "b", None, {}),
        ]

        summary = run_evals.summarize_usage(rows)

        candidate = summary["conditions"]["candidate"]
        self.assertIsNone(candidate["input_tokens"])
        self.assertIsNone(candidate["output_tokens"])
        self.assertIsNone(candidate["cost_usd"])
        self.assertEqual(1, candidate["cost_usd_unreported_rows"])
        self.assertIsNone(summary["delta"]["candidate"]["cost_usd"])
        self.assertIsNone(summary["delta"]["candidate"]["cost_usd_pct"])

    def test_usage_tokens_handles_both_runner_shapes(self):
        self.assertEqual(
            (15, 4),
            run_evals._usage_tokens(
                {
                    "input_tokens": 10,
                    "cache_creation_input_tokens": 2,
                    "cache_read_input_tokens": 3,
                    "output_tokens": 4,
                }
            ),
        )
        self.assertEqual(
            (12, 7),
            run_evals._usage_tokens(
                {"input_tokens": 12, "cached_input_tokens": 5, "output_tokens": 7}
            ),
        )
        self.assertEqual((None, None), run_evals._usage_tokens({}))

    def test_usage_rejects_invalid_measurements(self):
        for cost in (-1, True, False, float("nan"), float("inf"), "0.1"):
            with self.subTest(cost=cost):
                rows = [self._usage_row("a", condition, "ok", cost, {})
                        for condition in ("baseline", "candidate")]
                with self.assertRaisesRegex(ValueError, "cost_usd"):
                    run_evals.summarize_usage(rows)
        for key in ("input_tokens", "output_tokens", "cache_creation_input_tokens",
                    "cache_read_input_tokens", "cached_input_tokens"):
            for value in (-1, True, 1.5, float("nan"), "10"):
                with self.subTest(key=key, value=value):
                    with self.assertRaisesRegex(ValueError, key):
                        run_evals._usage_tokens({key: value})
        with self.assertRaisesRegex(ValueError, "usage"):
            run_evals._usage_tokens([])

    def test_usage_rejects_overflowing_cost_total(self):
        rows = [self._usage_row(case, condition, "ok", 1e308, {})
                for case in ("a", "b") for condition in ("baseline", "candidate")]
        with self.assertRaisesRegex(ValueError, "cost total is not finite"):
            run_evals.summarize_usage(rows)

    def test_usage_distinguishes_missing_counts_from_zero(self):
        self.assertEqual((None, None), run_evals._usage_tokens(None))
        self.assertEqual((None, 2), run_evals._usage_tokens(
            {"cache_read_input_tokens": 10, "output_tokens": 2}))
        self.assertEqual((None, 2), run_evals._usage_tokens(
            {"input_tokens": 10, "cache_read_input_tokens": None, "output_tokens": 2}))
        rows = [self._usage_row("a", condition, "", 0,
                               {"input_tokens": 0, "output_tokens": 0})
                for condition in ("baseline", "candidate")]
        result = run_evals.summarize_usage(rows)
        self.assertEqual(0, result["conditions"]["candidate"]["input_tokens"])
        self.assertEqual(0, result["delta"]["candidate"]["cost_usd"])
        self.assertIsNone(result["delta"]["candidate"]["cost_usd_pct"])
        rows[1]["usage"] = None
        self.assertIsNone(run_evals.summarize_usage(rows)["delta"]["candidate"]["input_tokens"])

    def test_usage_rejects_duplicate_or_invalid_rows(self):
        rows = [self._usage_row("a", condition, "ok", 0, {})
                for condition in ("baseline", "candidate")]
        with self.assertRaisesRegex(ValueError, "duplicate"):
            run_evals.summarize_usage(rows + [rows[0]])
        for field, value in (("condition", "unknown"), ("case_id", None),
                             ("trial", True), ("trial", 0), ("runner", ""),
                             ("response", None)):
            with self.subTest(field=field):
                invalid = [dict(rows[0]), dict(rows[1])]
                invalid[1][field] = value
                with self.assertRaises(ValueError):
                    run_evals.summarize_usage(invalid)

    def test_usage_checks_model_metadata_when_available(self):
        rows = [self._usage_row("a", condition, "ok", 0, {})
                for condition in ("baseline", "candidate")]
        self.assertIsNone(run_evals.summarize_usage(rows)["model"])
        rows[0]["model"] = "model-a"
        with self.assertRaisesRegex(ValueError, "same model"):
            run_evals.summarize_usage(rows)
        rows[1]["model"] = "model-b"
        with self.assertRaisesRegex(ValueError, "same model"):
            run_evals.summarize_usage(rows)
        rows[1]["model"] = "model-a"
        self.assertEqual("model-a", run_evals.summarize_usage(rows)["model"])

    def test_boolean_scores_are_rejected(self):
        rows = [self._score_row("a", "baseline", 3), self._score_row("a", "candidate", 4)]
        rows[1]["correctness"] = True
        with self.assertRaisesRegex(ValueError, "correctness must be between 1 and 5"):
            run_evals.summarize_scores(rows)

    def test_a_regression_of_exactly_one_tenth_is_within_tolerance(self):
        # Means of 4.2 and 4.1 differ by 0.1, but 4.2 - 0.1 rounds above 4.1.
        baseline = [1, 1, 5, 5, 5, 5, 5, 5, 5, 5]
        candidate = [1, 1, 4, 5, 5, 5, 5, 5, 5, 5]
        for metric in ("correctness", "safety"):
            with self.subTest(metric=metric):
                rows = []
                for trial, (b, c) in enumerate(zip(baseline, candidate), start=1):
                    rows.append(dict(self._score_row("a", "baseline", 3, trial), **{metric: b}))
                    rows.append(dict(self._score_row("a", "candidate", 5, trial), **{metric: c}))
                reasons = run_evals.summarize_scores(rows)["release_gate"]["reasons"]
                self.assertNotIn(f"Candidate {metric} regressed by more than 0.1 points.", reasons)

    def test_trials_must_be_at_least_one(self):
        for command in (["plan"], ["run", "--runner", "x", "--condition", "baseline", "--output", "o"]):
            for trials in ("0", "-1"):
                with self.subTest(command=command[0], trials=trials), mock.patch("sys.stderr"):
                    with self.assertRaises(SystemExit):
                        run_evals._build_parser().parse_args([*command, "--trials", trials])

    def test_nan_budget_is_rejected_before_any_call(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = self._run_args(Path(tmp), budget_usd=float("nan"))
            with mock.patch.object(run_evals.subprocess, "run") as runner:
                with self.assertRaisesRegex(ValueError, "--budget-usd"):
                    run_evals.run_evaluations(args)
            runner.assert_not_called()

    def test_runs_record_the_model_the_runner_names(self):
        completed = subprocess.CompletedProcess(
            args=["stub-runner"], returncode=0, stderr="",
            stdout=json.dumps({"result": "102", "usage": {}, "total_cost_usd": 0.01}),
        )
        for command, model in (
            (["claude", "--model", "model-a"], "model-a"),
            (["codex", "exec", "-m", "model-b"], "model-b"),
            (["claude", "--model=model-a"], "model-a"),
            (["claude", "--model", "old", "--model", "new"], "new"),
            (["python", "-m", "runner_module", "--model", "model-a"], None),
            (["stub-runner", "-m", "not-a-model"], None),
            (["claude"], None),
        ):
            with self.subTest(command=command), tempfile.TemporaryDirectory() as tmp:
                args = self._run_args(Path(tmp), command=command)
                with mock.patch.object(run_evals.subprocess, "run", return_value=completed):
                    run_evals.run_evaluations(args)
                [row] = run_evals.read_jsonl(args.output)
                self.assertEqual(model, row.get("model"))

    @staticmethod
    def _run_args(root, budget_usd=1.0, command=("stub-runner",)):
        cases = root / "cases.jsonl"
        cases.write_text(json.dumps({
            "id": "probe", "category": "direct-answer", "prompt": "What is 17 multiplied by 6?",
            "risk": "low", "criteria": ["Answers 102."],
        }) + "\n", encoding="utf-8")
        runners = root / "runners.json"
        runners.write_text(json.dumps(
            {"stub": {"command": list(command), "response_format": "claude-json"}}
        ), encoding="utf-8")
        return argparse.Namespace(
            cases=cases, runner_config=runners, runner="stub", condition="baseline",
            condition_skill=None, case=None, trials=1, retries=0, budget_usd=budget_usd,
            allow_unmetered=False, output=root / "responses.jsonl",
        )

    def test_measure_cli_reports_increased_input_and_cost(self):
        rows = [
            self._usage_row("a", "baseline", "long answer", 0.1,
                            {"input_tokens": 100, "output_tokens": 20}),
            self._usage_row("a", "candidate", "short", 0.15,
                            {"input_tokens": 150, "output_tokens": 10}),
        ]
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "responses.jsonl"
            contents = "".join(json.dumps(row) + "\n" for row in rows)
            path.write_text(contents, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/run_evals.py"), "measure", str(path)],
                check=True, capture_output=True, text=True, timeout=10,
            )
            self.assertEqual(contents, path.read_text(encoding="utf-8"))
        delta = json.loads(result.stdout)["delta"]["candidate"]
        self.assertEqual(50, delta["input_tokens"])
        self.assertEqual(50, delta["input_tokens_pct"])
        self.assertEqual(-50, delta["output_tokens_pct"])
        self.assertAlmostEqual(50, delta["cost_usd_pct"])

    @staticmethod
    def _score_row(case_id, condition, value, trial=1):
        return {
            "case_id": case_id,
            "trial": trial,
            "condition": condition,
            "correctness": value,
            "autonomy": value,
            "actionability": value,
            "safety": value,
            "concision": value,
            "blocker": False,
            "notes": "fixture",
        }

    @staticmethod
    def _usage_row(case_id, condition, response, cost, usage, runner="claude", trial=1):
        return {
            "case_id": case_id,
            "trial": trial,
            "condition": condition,
            "runner": runner,
            "response": response,
            "usage": usage,
            "cost_usd": cost,
        }

    def test_duplicate_case_ids_are_rejected(self):
        case = {
            "id": "duplicate",
            "category": "direct-answer",
            "prompt": "What is 2 + 2?",
            "risk": "low",
            "criteria": ["Answers 4."],
        }
        errors = run_evals.validate_cases([case, dict(case)])
        self.assertTrue(any("Duplicate" in error for error in errors))

    def test_jsonl_loader_reports_invalid_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "bad.jsonl"
            path.write_text(json.dumps({"id": "ok"}) + "\nnot-json\n")
            with self.assertRaisesRegex(ValueError, "line 2"):
                run_evals.read_jsonl(path)

    def test_condition_prompt_injects_the_skill_body_without_frontmatter(self):
        # hooks/always-on.sh strips the YAML frontmatter before injecting the
        # ruleset, so the eval must grade the same text that actually ships.
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "SKILL.md"
            skill.write_text(
                "---\n"
                "name: laconic\n"
                "disable-model-invocation: true\n"
                "metadata:\n"
                "  hermes:\n"
                "    tags: [ADHD]\n"
                "---\n"
                "\n"
                "# laconic\n"
                "\n"
                "Lead with the next action.\n"
            )

            prompt = run_evals._condition_prompt("Fix the bug.", "candidate", skill)

            self.assertIn("Lead with the next action.", prompt)
            self.assertIn("Fix the bug.", prompt)
            self.assertNotIn("disable-model-invocation", prompt)
            self.assertNotIn("hermes", prompt)

    def test_condition_prompt_keeps_a_skill_body_that_has_no_frontmatter(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "SKILL.md"
            skill.write_text("# No frontmatter here\n\nLead with the next action.\n")

            prompt = run_evals._condition_prompt("Fix the bug.", "candidate", skill)

            self.assertIn("# No frontmatter here", prompt)

    def test_unmetered_runner_is_rejected_before_any_call(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            marker = tmp_path / "ran"
            runner_config = tmp_path / "runners.json"
            runner_config.write_text(
                json.dumps(
                    {
                        "stub": {
                            "command": [
                                sys.executable,
                                "-c",
                                f"from pathlib import Path; Path({str(marker)!r}).touch(); print('hi')",
                            ],
                            "response_format": "text",
                        }
                    }
                )
            )
            args = argparse.Namespace(
                cases=ROOT / "evals" / "cases.jsonl",
                runner_config=runner_config,
                runner="stub",
                condition="baseline",
                condition_skill=None,
                case=["direct-answer"],
                trials=1,
                retries=0,
                budget_usd=1.0,
                allow_unmetered=False,
                output=tmp_path / "out.jsonl",
            )

            with self.assertRaisesRegex(RuntimeError, "never reports dollar cost"):
                run_evals.run_evaluations(args)

            self.assertFalse(marker.exists(), "runner was invoked before the rejection")
            self.assertFalse((tmp_path / "out.jsonl").exists())

            args.allow_unmetered = True
            self.assertEqual(0, run_evals.run_evaluations(args))
            self.assertTrue(marker.exists())
            # The judge reads this date to resolve 'today' and 'yesterday' in the response.
            [row] = run_evals.read_jsonl(tmp_path / "out.jsonl")
            self.assertEqual(datetime.date.today().isoformat(), row["date"])

    def test_generation_runs_outside_the_repository(self):
        # An agent CLI adopts its working directory as project context. Run it
        # in this checkout and it answers prompts by inspecting the harness,
        # which contaminates the responses being compared.
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            runner_config = tmp_path / "runners.json"
            runner_config.write_text(
                json.dumps({"pwd": {"command": ["sh", "-c", "pwd"], "response_format": "text"}})
            )
            output = tmp_path / "out.jsonl"
            args = argparse.Namespace(
                cases=ROOT / "evals" / "cases.jsonl",
                runner_config=runner_config,
                runner="pwd",
                condition="baseline",
                condition_skill=None,
                case=["direct-answer"],
                trials=1,
                retries=0,
                budget_usd=1.0,
                allow_unmetered=True,
                output=output,
            )

            self.assertEqual(0, run_evals.run_evaluations(args))

            where = Path(run_evals.read_jsonl(output)[0]["response"].strip()).resolve()
            self.assertNotEqual(ROOT.resolve(), where)
            self.assertFalse(str(where).startswith(str(ROOT.resolve())))

    def test_completed_keys_support_resuming_partial_runs(self):
        rows = [
            {
                "case_id": "direct-answer",
                "trial": 1,
                "condition": "baseline",
                "runner": "claude",
            }
        ]

        self.assertEqual(
            {("direct-answer", 1, "baseline", "claude")},
            run_evals.completed_keys(rows),
        )


class NoInjectionTest(unittest.TestCase):
    def test_setup_comparison_sends_the_task_unchanged(self):
        self.assertEqual("Task", run_evals._condition_prompt("Task", "candidate", None, inject=False))

    def test_injection_still_requires_a_skill(self):
        with self.assertRaises(ValueError):
            run_evals._condition_prompt("Task", "candidate", None)


class CallTimeoutTest(unittest.TestCase):
    def test_a_hung_call_becomes_a_failed_attempt(self):
        result = run_evals._run_with_timeout(
            [sys.executable, "-c", "import time; time.sleep(5)"], 0.5
        )
        self.assertEqual(124, result.returncode)
        self.assertIn("Timed out after 0.5 seconds", result.stderr)

    def test_a_finished_call_keeps_its_result(self):
        result = run_evals._run_with_timeout([sys.executable, "-c", "print('ok')"], 30)
        self.assertEqual(0, result.returncode)
        self.assertEqual("ok", result.stdout.strip())


if __name__ == "__main__":
    unittest.main()
