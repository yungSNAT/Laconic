"""One row per answer from a run's work directory, without the answer text.

Usage: python3 -I results.py <work dir> <cases file> <laconic-check> > results.jsonl
Reads responses.jsonl, scores.jsonl (evals/rubric.md) and correctness.jsonl
(evals/rubric-correctness.md) from <work dir>, and warm.jsonl when it is there.
"""
import json
import re
import subprocess
import sys

work, cases_path, checker = sys.argv[1:4]
DIMENSIONS = ("correctness", "autonomy", "actionability", "safety", "concision")


def read(name):
    try:
        with open(f"{work}/{name}", encoding="utf-8") as f:
            return [json.loads(line) for line in f if line.strip()]
    except FileNotFoundError:
        return []


def by_answer(rows):
    return {(r["case_id"], r["trial"], r["condition"]): r for r in rows}


with open(cases_path, encoding="utf-8") as f:
    cases = {c["id"]: c for c in map(json.loads, f)}


def regex_failed(case_id, text):
    """The case's regex checks the answer fails, as `laconic-evals checks` applies them; None
    when the case has none."""
    checks = cases[case_id].get("checks")
    if not checks:
        return None
    failed = [f"missing {p!r}" for p in checks.get("must_include", [])
              if not re.search(p, text, re.IGNORECASE)]
    failed += [f"has {p!r}" for p in checks.get("must_not_include", [])
               if re.search(p, text, re.IGNORECASE)]
    return failed


def lint_clean(text):
    out = subprocess.run([checker, "lint", "--json", "-"], input=text, capture_output=True,
                         text=True, encoding="utf-8", check=False)
    return not json.loads(out.stdout)["findings"]


house, plain = by_answer(read("scores.jsonl")), by_answer(read("correctness.jsonl"))
for run, rows in (("cold", read("responses.jsonl")), ("warm", read("warm.jsonl"))):
    for r in rows:
        key = (r["case_id"], r["trial"], r["condition"])
        u = r.get("usage") or {}
        row = {
            "run": run,
            "case_id": r["case_id"],
            "trial": r["trial"],
            "condition": r["condition"],
            "setup": r["runner"],
            "input_tokens": u.get("input_tokens", 0),
            "cache_write_tokens": u.get("cache_creation_input_tokens", 0),
            "cache_read_tokens": u.get("cache_read_input_tokens", 0),
            "output_tokens": u.get("output_tokens", 0),
            "cost_usd": r.get("cost_usd"),
            "characters": len(r["response"]),
            "lint_clean": lint_clean(r["response"]),
            "regex_failed": regex_failed(r["case_id"], r["response"]),
        }
        if run == "cold":
            h, p = house[key], plain[key]
            row["house"] = {**{d: h[d] for d in DIMENSIONS}, "blocker": h["blocker"]}
            row["correctness_only"] = {"correctness": p["correctness"], "blocker": p["blocker"]}
        print(json.dumps(row))
