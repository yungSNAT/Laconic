#!/usr/bin/env python3
"""Validate, run, and score paired response-quality evaluations."""

import argparse
from contextlib import contextmanager
import datetime
import json
import math
import re
import shlex
import subprocess
import sys
import tempfile
import time
from collections import Counter, defaultdict
from collections.abc import Iterator
from concurrent.futures import FIRST_COMPLETED, Future, ThreadPoolExecutor, wait
from pathlib import Path
from typing import Any, Optional


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CASES = ROOT / "evals" / "cases.jsonl"
WEIGHTS = {
    "correctness": 0.35,
    "autonomy": 0.25,
    "actionability": 0.20,
    "safety": 0.10,
    "concision": 0.10,
}
CONDITIONS = {"baseline", "candidate", "comparator"}


@contextmanager
def _neutral_cwd() -> Iterator[str]:
    """Yield an empty scratch directory for one runner invocation.

    Agent CLIs adopt their working directory as project context: run one inside
    this checkout and it answers prompts by inspecting the harness instead of
    the task. That contamination is not symmetric across conditions -- a
    response style that discourages exploration wanders less -- so it shows up
    as a score difference that has nothing to do with the skill under test.
    Isolation is the same reason the runners pass `--setting-sources ""`; the
    working directory is simply another channel the operator's world leaks in
    through.
    """
    with tempfile.TemporaryDirectory(prefix="eval-neutral-cwd-") as cwd:
        yield cwd


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}: line {number}: {exc.msg}") from exc
        if not isinstance(row, dict):
            raise ValueError(f"{path}: line {number}: expected a JSON object")
        rows.append(row)
    return rows


def load_cases(path: Path = DEFAULT_CASES) -> list[dict[str, Any]]:
    return read_jsonl(path)


def completed_keys(rows: list[dict[str, Any]]) -> set[tuple[str, int, str, str]]:
    keys: set[tuple[str, int, str, str]] = set()
    for row in rows:
        fields = (row.get("case_id"), row.get("trial"), row.get("condition"), row.get("runner"))
        if isinstance(fields[0], str) and isinstance(fields[1], int) and all(
            isinstance(value, str) for value in fields[2:]
        ):
            keys.add(fields)  # type: ignore[arg-type]
    return keys


def validate_cases(cases: list[dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    seen: set[str] = set()
    required = {"id", "category", "prompt", "risk", "criteria"}
    for index, case in enumerate(cases, start=1):
        missing = sorted(required - set(case))
        if missing:
            errors.append(f"Case {index}: missing fields: {', '.join(missing)}")
            continue
        case_id = case["id"]
        if not isinstance(case_id, str) or not case_id:
            errors.append(f"Case {index}: id must be a non-empty string")
        elif case_id in seen:
            errors.append(f"Duplicate case id: {case_id}")
        else:
            seen.add(case_id)
        if case["risk"] not in {"low", "medium", "high"}:
            errors.append(f"Case {case_id}: risk must be low, medium, or high")
        if not isinstance(case["criteria"], list) or not case["criteria"]:
            errors.append(f"Case {case_id}: criteria must be a non-empty list")
        if "target_words" in case:
            target = case["target_words"]
            if isinstance(target, bool) or not isinstance(target, int) or target < 1:
                errors.append(f"Case {case_id}: target_words must be a positive integer")
        if "checks" in case:
            errors.extend(f"Case {case_id}: {error}" for error in _validate_checks(case["checks"]))
    return errors


def _validate_checks(checks: Any) -> list[str]:
    """A case's optional regex checks must compile, and must accept every known-good
    example and reject every known-bad one, so a broken check fails before a paid run."""
    if not isinstance(checks, dict):
        return ["checks must be an object"]

    def strings(field: str) -> bool:
        value = checks.get(field, [])
        return isinstance(value, list) and all(isinstance(item, str) for item in value)

    errors = [f"{field} must be a list of strings" for field in ("must_include", "must_not_include", "pass", "fail") if not strings(field)]
    if errors:
        return errors
    patterns = [*checks.get("must_include", []), *checks.get("must_not_include", [])]
    if not patterns:
        errors.append("checks need at least one must_include or must_not_include pattern")
    for pattern in patterns:
        try:
            re.compile(pattern, re.I)
        except re.error as exc:
            errors.append(f"bad check pattern {pattern!r}: {exc}")
    if errors:
        return errors
    if not checks.get("pass") or not checks.get("fail"):
        errors.append("checks need at least one pass and one fail example")
    for example in checks.get("pass", []):
        if failed := check_response(checks, example):
            errors.append(f"pass example {example!r} fails {failed}")
    for example in checks.get("fail", []):
        if not check_response(checks, example):
            errors.append(f"fail example {example!r} passes every check")
    return errors


def check_response(checks: dict[str, Any], text: str) -> list[str]:
    """Return the checks a response fails (case-insensitive regex search)."""
    failed = [f"missing {p!r}" for p in checks.get("must_include", []) if not re.search(p, text, re.I)]
    failed += [f"contains {p!r}" for p in checks.get("must_not_include", []) if re.search(p, text, re.I)]
    return failed


def summarize_checks(cases: list[dict[str, Any]], responses: list[dict[str, Any]]) -> dict[str, Any]:
    """Pass rate per condition on the cases that carry regex checks, and each response's
    length in words as a share of its case's `target_words`. No judge is involved, so the
    same responses always get the same result."""
    checks = {case["id"]: case["checks"] for case in cases if "checks" in case}
    targets = {case["id"]: case["target_words"] for case in cases if "target_words" in case}
    summary: dict[str, Any] = {}
    for row in responses:
        case_id = row.get("case_id")
        if case_id not in checks and case_id not in targets:
            continue
        entry = summary.setdefault(row["condition"], {"checked": 0, "passed": 0, "failures": []})
        text = row.get("response", "")
        if case_id in targets:
            words = len(text.split())
            entry.setdefault("target_length", []).append(
                {"case_id": case_id, "trial": row.get("trial"), "words": words, "share": round(words / targets[case_id], 3)}
            )
        if case_id not in checks:
            continue
        failed = check_response(checks[case_id], text)
        entry["checked"] += 1
        if failed:
            entry["failures"].append({"case_id": case_id, "trial": row.get("trial"), "failed": failed})
        else:
            entry["passed"] += 1
    for entry in summary.values():
        entry["pass_rate"] = entry["passed"] / entry["checked"] if entry["checked"] else None
        if "target_length" in entry:
            shares = [item["share"] for item in entry["target_length"]]
            entry["mean_target_share"] = round(sum(shares) / len(shares), 3)
    return summary


def _validate_score(row: dict[str, Any], index: int) -> None:
    required = {"case_id", "trial", "condition", *WEIGHTS, "blocker", "notes"}
    missing = sorted(required - set(row))
    if missing:
        raise ValueError(f"Score row {index}: missing fields: {', '.join(missing)}")
    if row["condition"] not in CONDITIONS:
        raise ValueError(f"Score row {index}: unsupported condition {row['condition']!r}")
    for metric in WEIGHTS:
        value = row[metric]
        if type(value) not in (int, float) or not 1 <= value <= 5:
            raise ValueError(f"Score row {index}: {metric} must be between 1 and 5")
    if not isinstance(row["blocker"], bool):
        raise ValueError(f"Score row {index}: blocker must be boolean")


def _describe_rows(keys: list[tuple[str, Any]]) -> str:
    return ", ".join(f"{case_id}/trial {trial}" for case_id, trial in keys)


def _check_pairing(grouped: dict[str, list[dict[str, Any]]], reference: str = "baseline") -> None:
    """Conditions are only comparable when judged on identical rows."""
    coverage = {
        condition: Counter((row["case_id"], row["trial"]) for row in rows)
        for condition, rows in grouped.items()
    }
    for condition, counts in sorted(coverage.items()):
        repeated = sorted(key for key, count in counts.items() if count > 1)
        if repeated:
            raise ValueError(
                f"{condition}: duplicate score rows for {_describe_rows(repeated)}"
            )
    baseline = coverage[reference]
    for condition, counts in sorted(coverage.items()):
        if condition == reference or counts == baseline:
            continue
        details = []
        missing = sorted(set(baseline) - set(counts))
        if missing:
            details.append(f"missing {_describe_rows(missing)}")
        unmatched = sorted(set(counts) - set(baseline))
        if unmatched:
            details.append(f"unmatched {_describe_rows(unmatched)}")
        raise ValueError(
            f"{condition} was not judged on the same rows as {reference}: "
            + "; ".join(details)
        )


def summarize_scores(scores: list[dict[str, Any]], reference: str = "baseline") -> dict[str, Any]:
    """Score every condition and gate the candidate against `reference`.

    The reference is usually the baseline; comparing 2 installed setups often
    makes it the comparator instead.
    """
    if reference == "candidate":
        raise ValueError("The reference must be a condition other than candidate")
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for index, row in enumerate(scores, start=1):
        _validate_score(row, index)
        grouped[row["condition"]].append(row)
    if reference not in grouped or "candidate" not in grouped:
        raise ValueError(f"Scores must include {reference} and candidate conditions")
    _check_pairing(grouped, reference)

    conditions: dict[str, dict[str, Any]] = {}
    for condition, rows in sorted(grouped.items()):
        metrics = {
            metric: sum(float(row[metric]) for row in rows) / len(rows)
            for metric in WEIGHTS
        }
        conditions[condition] = {
            "rows": len(rows),
            **metrics,
            "weighted_score": sum(metrics[metric] * weight for metric, weight in WEIGHTS.items()),
            "blocking_findings": sum(bool(row["blocker"]) for row in rows),
        }

    baseline = conditions[reference]
    candidate = conditions["candidate"]
    reasons: list[str] = []
    if candidate["blocking_findings"]:
        reasons.append("Candidate has blocking safety or correctness findings.")
    # Round the gap so a regression of exactly 0.1 is not tipped over by float error.
    if round(baseline["correctness"] - candidate["correctness"], 9) > 0.1:
        reasons.append("Candidate correctness regressed by more than 0.1 points.")
    if round(baseline["safety"] - candidate["safety"], 9) > 0.1:
        reasons.append("Candidate safety regressed by more than 0.1 points.")
    if candidate["weighted_score"] <= baseline["weighted_score"]:
        reasons.append(f"Candidate weighted score did not beat {reference}.")

    return {
        "weights": WEIGHTS,
        "reference": reference,
        "conditions": conditions,
        "release_gate": {"passed": not reasons, "reasons": reasons},
    }


def summarize_usage(rows: list[dict[str, Any]], reference: str = "baseline") -> dict[str, Any]:
    if reference == "candidate":
        raise ValueError("The reference must be a condition other than candidate")
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for index, row in enumerate(rows, start=1):
        for field in ("case_id", "runner", "condition"):
            if not isinstance(row.get(field), str) or not row[field]:
                raise ValueError(f"Response {index}: {field} must be a non-empty string")
        if row["condition"] not in CONDITIONS:
            raise ValueError(f"Response {index}: unsupported condition")
        if type(row.get("trial")) is not int or row["trial"] < 1:
            raise ValueError(f"Response {index}: trial must be a positive integer")
        if not isinstance(row.get("response"), str):
            raise ValueError(f"Response {index}: response must be a string")
        cost = row.get("cost_usd")
        if cost is not None and (
            type(cost) not in (int, float) or cost < 0 or not math.isfinite(cost)
        ):
            raise ValueError(f"Response {index}: cost_usd must be finite and non-negative")
        model = row.get("model")
        if model is not None and (not isinstance(model, str) or not model.strip()):
            raise ValueError(f"Response {index}: model must be a non-empty string")
        grouped[row["condition"]].append(row)
    if reference not in grouped or "candidate" not in grouped:
        raise ValueError(f"Responses must include {reference} and candidate conditions")

    runners = {row["runner"] for row in rows}
    if len(runners) > 1:
        names = ", ".join(sorted(str(runner) for runner in runners))
        raise ValueError(f"Responses must use the same runner; found: {names}")
    models = {row.get("model") for row in rows}
    if len(models) > 1:
        raise ValueError("Responses must report the same model on every row, or omit it on all rows")
    _check_pairing(grouped, reference)

    conditions: dict[str, dict[str, Any]] = {}
    for condition, condition_rows in sorted(grouped.items()):
        tokens = [_usage_tokens(row.get("usage", {})) for row in condition_rows]
        input_counts = [counts[0] for counts in tokens]
        output_counts = [counts[1] for counts in tokens]
        input_total = _reported_token_total(input_counts)
        output_total = _reported_token_total(output_counts)
        response_chars = sum(len(row["response"]) for row in condition_rows)
        reported_costs = [row.get("cost_usd") for row in condition_rows]
        unreported_costs = sum(cost is None for cost in reported_costs)
        cost_total = (
            None
            if unreported_costs
            else sum(float(cost) for cost in reported_costs)
        )
        if cost_total is not None and not math.isfinite(cost_total):
            raise ValueError(f"{condition}: cost total is not finite")
        summary = {
            "rows": len(condition_rows),
            "input_tokens": input_total,
            "mean_input_tokens": (
                None if input_total is None else input_total / len(condition_rows)
            ),
            "output_tokens": output_total,
            "mean_output_tokens": (
                None if output_total is None else output_total / len(condition_rows)
            ),
            "cost_usd": cost_total,
            "response_chars": response_chars,
            "mean_response_chars": response_chars / len(condition_rows),
        }
        if unreported_costs:
            summary["cost_usd_unreported_rows"] = unreported_costs
        conditions[condition] = summary

    baseline = conditions[reference]
    deltas: dict[str, dict[str, Any]] = {}
    for condition, summary in sorted(conditions.items()):
        if condition == reference:
            continue
        deltas[condition] = {}
        for metric in ("input_tokens", "output_tokens", "cost_usd", "mean_response_chars"):
            delta = _difference(summary[metric], baseline[metric])
            deltas[condition][metric] = delta
            deltas[condition][f"{metric}_pct"] = _percent_change(delta, baseline[metric])

    return {
        "runner": next(iter(runners)),
        "model": next(iter(models)),
        "reference": reference,
        "conditions": conditions,
        "delta": deltas,
    }


def _reported_token_total(values: list[int | None]) -> int | None:
    if any(value is None for value in values):
        return None
    return sum(value for value in values if value is not None)


def _difference(
    value: int | float | None, baseline: int | float | None
) -> int | float | None:
    if value is None or baseline is None:
        return None
    return value - baseline


def _percent_change(
    delta: int | float | None, baseline: int | float | None
) -> float | None:
    if delta is None or baseline in (None, 0):
        return None
    return delta / baseline * 100


def _strip_frontmatter(text: str) -> str:
    """Drop a leading YAML frontmatter block, mirroring hooks/always-on.sh.

    The hook injects the ruleset without frontmatter, so the eval has to grade
    the same text. A file that opens a block but never closes it is left alone
    rather than emptied.
    """
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return text
    for index in range(1, len(lines)):
        if lines[index].strip() == "---":
            return "\n".join(lines[index + 1 :]).lstrip("\n")
    return text


def _run_with_timeout(invocation: list[str], timeout: float, **kwargs: Any) -> subprocess.CompletedProcess:
    """Run one model call, turning a hang into an ordinary failed attempt.

    A CLI that silently waits on provider retries would otherwise stall the
    whole condition with no output.
    """
    try:
        return subprocess.run(invocation, check=False, capture_output=True, text=True, timeout=timeout, **kwargs)
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(invocation, 124, "", f"Timed out after {timeout:g} seconds")


def _condition_prompt(
    task: str, condition: str, skill_path: Optional[Path], inject: bool = True
) -> str:
    # Without injection the runner itself supplies the setup (plugins, rules files),
    # so whole installations can be compared under their real condition names.
    if condition == "baseline" or not inject:
        return task
    if skill_path is None:
        raise ValueError(f"--condition-skill is required for the {condition} condition")
    instructions = _strip_frontmatter(skill_path.read_text(encoding="utf-8"))
    return (
        "Follow the response-style skill below while completing the task. "
        "Do not discuss or quote the skill.\n\n"
        f"<response_style>\n{instructions}\n</response_style>\n\n"
        f"<task>\n{task}\n</task>"
    )


def _parse_response(
    output: str, response_format: str
) -> tuple[str, dict[str, Any], Optional[float]]:
    if response_format == "text":
        return output.strip(), {}, None
    if response_format == "claude-json":
        payload, _ = json.JSONDecoder().raw_decode(output.lstrip())
        return (
            str(payload.get("result", "")).strip(),
            payload.get("usage", {}) or {},
            payload.get("total_cost_usd"),
        )
    if response_format == "codex-jsonl":
        events = [json.loads(line) for line in output.splitlines() if line.strip()]
        text = ""
        usage: dict[str, Any] = {}
        for event in events:
            item = event.get("item", {})
            if event.get("type") == "item.completed" and item.get("type") == "agent_message":
                text = item.get("text", text)
            if event.get("type") == "turn.completed":
                usage = event.get("usage", usage)
        return str(text).strip(), usage, None
    raise ValueError(f"Unsupported response format: {response_format}")


_INPUT_TOKEN_KEYS = (
    "input_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
)


def _usage_tokens(usage: Optional[dict[str, Any]]) -> tuple[int | None, int | None]:
    """Return (input, output) token counts, or None where they were not reported."""
    if usage is None:
        return None, None
    if not isinstance(usage, dict):
        raise ValueError("usage must be an object or null")
    for key in (*_INPUT_TOKEN_KEYS, "cached_input_tokens", "output_tokens"):
        value = usage.get(key)
        if value is not None and (type(value) is not int or value < 0):
            raise ValueError(f"{key} must be a non-negative integer or null")
    # Claude cache counts are additional input; Codex cached_input_tokens is a subset.
    input_values = [usage.get("input_tokens")] + [
        usage.get(key, 0) for key in _INPUT_TOKEN_KEYS[1:]
    ]
    return _reported_token_total(input_values), usage.get("output_tokens")


def _runner_model(command: list[str]) -> str | None:
    """Report a requested model only for a directly invoked CLI we recognise."""
    executable = Path(command[0]).stem.lower()
    if executable not in ("claude", "codex"):
        return None  # A wrapper's -m may select a Python module, not a model.
    model = None
    arguments = iter(command[1:])
    for arg in arguments:
        if arg == "--":
            break
        if arg == "--model" or (executable == "codex" and arg == "-m"):
            model = next(arguments, None)
        elif arg.startswith("--model="):
            model = arg.partition("=")[2]
    return model if model and not model.startswith("-") else None


def run_evaluations(args: argparse.Namespace) -> int:
    cases = load_cases(args.cases)
    errors = validate_cases(cases)
    if errors:
        raise ValueError("\n".join(errors))
    unknown = sorted(set(args.case or []) - {case["id"] for case in cases})
    if unknown:
        raise ValueError(f"--case matched no evaluation case: {', '.join(unknown)}")
    config = json.loads(args.runner_config.read_text(encoding="utf-8"))
    runner = config[args.runner]
    command = list(runner["command"])
    response_format = runner.get("response_format", "text")
    if response_format != "claude-json" and not args.allow_unmetered:
        raise RuntimeError(
            f"The {response_format!r} response format never reports dollar cost; rerun with "
            "--allow-unmetered only when the provider has a separate hard spending cap."
        )
    reported_cost = 0.0
    prior_rows = read_jsonl(args.output) if args.output.exists() else []
    done = completed_keys(prior_rows)
    reported_cost = sum(
        float(row.get("cost_usd") or 0)
        for row in prior_rows
        if row.get("condition") == args.condition and row.get("runner") == args.runner
    )

    if not 0 < args.budget_usd <= 25:
        raise ValueError("--budget-usd must be greater than 0 and no more than 25")

    jobs = getattr(args, "jobs", 1)
    if jobs < 1:
        raise ValueError("--jobs must be at least 1")

    # The requested model, not an attestation of the provider's resolved backend.
    model = _runner_model(command)

    def call(case: dict[str, Any], trial: int, remaining: float) -> dict[str, Any]:
        prompt = _condition_prompt(
            case["prompt"], args.condition, args.condition_skill, not getattr(args, "no_injection", False)
        )
        invocation = [*command]
        if runner.get("budget_flag"):
            invocation.extend([runner["budget_flag"], f"{remaining:.4f}"])
        invocation.append(prompt)
        completed = None
        # The model reads today's date from its environment, so 'today' and
        # 'yesterday' in a response mean this date, which the judge needs.
        called_on = datetime.date.today().isoformat()
        for attempt in range(args.retries + 1):
            with _neutral_cwd() as cwd:
                completed = _run_with_timeout(
                    invocation, getattr(args, "call_timeout", 300.0), cwd=cwd, stdin=subprocess.DEVNULL
                )
            if completed.returncode == 0:
                break
            if attempt < args.retries:
                time.sleep(min(2**attempt, 5))
        assert completed is not None
        if completed.returncode:
            detail = completed.stderr.strip() or completed.stdout.strip()
            if completed.stdout.strip():
                try:
                    parsed_text, _, _ = _parse_response(completed.stdout, response_format)
                    detail = parsed_text or detail
                except (ValueError, json.JSONDecodeError):
                    pass
            raise RuntimeError(
                f"Runner failed after {args.retries + 1} attempts "
                f"({shlex.join(invocation[:-1])}):\n{detail}"
            )
        text, usage, cost = _parse_response(completed.stdout, response_format)
        if cost is None and not args.allow_unmetered:
            raise RuntimeError(
                "Runner did not report dollar cost; rerun with --allow-unmetered only when "
                "the provider has a separate hard spending cap."
            )
        return {
            "case_id": case["id"],
            "trial": trial,
            "condition": args.condition,
            "runner": args.runner,
            "response": text,
            "usage": usage,
            "cost_usd": cost,
            "model": model,
            "date": called_on,
        }

    pending = []
    for trial in range(1, args.trials + 1):
        for case in cases:
            if args.case and case["id"] not in args.case:
                continue
            if (case["id"], trial, args.condition, args.runner) in done:
                print(f"skip completed {args.condition} trial {trial}: {case['id']}")
                continue
            pending.append((case, trial))

    # shortcut: the budget is a soft threshold. No new call is submitted once recorded
    # cost reaches it, but up to N - 1 calls may still be running; it is not a hard cap.
    # Reserve per-call cost before submitting if a run overshoots materially.
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("a", encoding="utf-8") as destination, ThreadPoolExecutor(jobs) as pool:
        running: dict[Future[dict[str, Any]], None] = {}
        queue = iter(pending)
        exhausted = False
        # After a failure, calls already running still finish and are saved, so a
        # resumed run neither repeats nor loses their responses and costs.
        failure: BaseException | None = None
        while True:
            while failure is None and not exhausted and len(running) < jobs:
                remaining = args.budget_usd - reported_cost
                if remaining <= 0:
                    exhausted = True
                    break
                item = next(queue, None)
                if item is None:
                    break
                running[pool.submit(call, item[0], item[1], remaining)] = None
            if not running:
                break
            finished, _ = wait(running, return_when=FIRST_COMPLETED)
            for future in finished:
                del running[future]
                try:
                    row = future.result()
                except BaseException as error:
                    failure = failure or error
                    continue
                reported_cost += float(row["cost_usd"] or 0)
                destination.write(json.dumps(row, ensure_ascii=False) + "\n")
                destination.flush()
                print(f"{args.condition} trial {row['trial']}: {row['case_id']}")
        if failure is not None:
            raise failure
        if exhausted and next(queue, None) is not None:
            print("Budget exhausted; stopping.", file=sys.stderr)
            return 2
    print(f"Reported cost: ${reported_cost:.4f}")
    return 0


def _positive_int(text: str) -> int:
    value = int(text)
    if value < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return value


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser("validate", help="Validate the case catalog")
    validate.add_argument("--cases", type=Path, default=DEFAULT_CASES)

    plan = subparsers.add_parser("plan", help="Print the paired run matrix as JSONL")
    plan.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    plan.add_argument("--trials", type=_positive_int, default=3)
    plan.add_argument("--include-comparator", action="store_true")

    reference_help = "Condition the candidate is compared with (default: baseline)"
    score = subparsers.add_parser("score", help="Aggregate manually judged score rows")
    score.add_argument("scores", type=Path)
    score.add_argument("--reference", choices=sorted(CONDITIONS - {"candidate"}), default="baseline", help=reference_help)

    measure = subparsers.add_parser(
        "measure", help="Aggregate token and cost usage from recorded responses"
    )
    measure.add_argument("responses", type=Path)
    measure.add_argument("--reference", choices=sorted(CONDITIONS - {"candidate"}), default="baseline", help=reference_help)

    checks = subparsers.add_parser(
        "checks", help="Score recorded responses against the cases' regex checks, without a judge"
    )
    checks.add_argument("responses", type=Path)
    checks.add_argument("--cases", type=Path, default=DEFAULT_CASES)

    run = subparsers.add_parser("run", help="Run one evaluation condition")
    run.add_argument("--cases", type=Path, default=DEFAULT_CASES)
    run.add_argument("--runner-config", type=Path, default=ROOT / "evals" / "runners.example.json")
    run.add_argument("--runner", required=True)
    run.add_argument("--condition", choices=sorted(CONDITIONS), required=True)
    run.add_argument("--condition-skill", type=Path)
    run.add_argument("--case", action="append")
    run.add_argument("--trials", type=_positive_int, default=3)
    run.add_argument("--retries", type=int, default=2)
    run.add_argument(
        "--no-injection",
        action="store_true",
        help="Send the task unchanged; the runner config supplies the setup being compared",
    )
    run.add_argument("--call-timeout", type=float, default=300.0, help="Seconds before one model call counts as failed")
    run.add_argument("--jobs", type=int, default=1, help="Model calls to run at once; the budget is a soft threshold that up to jobs - 1 running calls can pass")
    run.add_argument("--budget-usd", type=float, default=25.0)
    run.add_argument("--allow-unmetered", action="store_true")
    run.add_argument("--output", type=Path, required=True)
    run.set_defaults(handler=run_evaluations)
    return parser


def main(argv: Optional[list[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    if hasattr(args, "handler"):
        return args.handler(args)
    if args.command == "validate":
        errors = validate_cases(load_cases(args.cases))
        if errors:
            for error in errors:
                print(f"ERROR: {error}", file=sys.stderr)
            return 1
        print("Evaluation cases are valid.")
        return 0
    if args.command == "plan":
        cases = load_cases(args.cases)
        errors = validate_cases(cases)
        if errors:
            raise ValueError("\n".join(errors))
        conditions = ["baseline", "candidate"]
        if args.include_comparator:
            conditions.append("comparator")
        for trial in range(1, args.trials + 1):
            for case in cases:
                for condition in conditions:
                    print(json.dumps({"case_id": case["id"], "trial": trial, "condition": condition}))
        return 0
    if args.command == "score":
        print(json.dumps(summarize_scores(read_jsonl(args.scores), args.reference), indent=2))
        return 0
    if args.command == "measure":
        print(json.dumps(summarize_usage(read_jsonl(args.responses), args.reference), indent=2, allow_nan=False))
        return 0
    # The parser rejects unknown commands. Only "checks" remains.
    cases = load_cases(args.cases)
    errors = validate_cases(cases)
    if errors:
        raise ValueError("\n".join(errors))
    print(json.dumps(summarize_checks(cases, read_jsonl(args.responses)), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
