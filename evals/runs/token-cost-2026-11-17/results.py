"""One row per answer, without the answer text. Run in the work directory.

`python3 -I results.py answers` writes each answer to answers/<run>-<n>.txt for run.sh to lint;
`python3 -I results.py > results.jsonl` then builds the rows from responses.jsonl, scores.jsonl
(evals/rubric.md), correctness.jsonl (evals/rubric-correctness.md), warm.jsonl when it is there,
cases.jsonl and lint/<run>-<n> ('1' when `laconic-check lint` found nothing).
"""
import json
import os
import re
import sys
from pathlib import Path

DIMENSIONS = ("correctness", "autonomy", "actionability", "safety", "concision")


def read(name):
    if not os.path.exists(name):
        return []
    with open(name, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def answers():
    """(run, n, row) for every answer, in file order."""
    for run, name in (("cold", "responses.jsonl"), ("warm", "warm.jsonl")):
        for n, row in enumerate(read(name)):
            yield run, n, row


if sys.argv[1:] == ["answers"]:
    os.makedirs("answers", exist_ok=True)
    for run, n, r in answers():
        Path(f"answers/{run}-{n}.txt").write_text(r["response"], encoding="utf-8", newline="\n")
    sys.exit()

cases = {c["id"]: c for c in read("cases.jsonl")}


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


def by_answer(rows):
    return {(r["case_id"], r["trial"], r["condition"]): r for r in rows}


house, plain = by_answer(read("scores.jsonl")), by_answer(read("correctness.jsonl"))
for run, n, r in answers():
    key = (r["case_id"], r["trial"], r["condition"])
    u = r.get("usage") or {}
    clean = Path(f"lint/{run}-{n}").read_text(encoding="utf-8").strip() == "1"
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
        "lint_clean": clean,
        "regex_failed": regex_failed(r["case_id"], r["response"]),
    }
    if run == "cold":
        h, p = house[key], plain[key]
        row["house"] = {**{d: h[d] for d in DIMENSIONS}, "blocker": h["blocker"]}
        row["correctness_only"] = {"correctness": p["correctness"], "blocker": p["blocker"]}
    print(json.dumps(row))
