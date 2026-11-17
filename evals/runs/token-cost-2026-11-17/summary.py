"""The published figures, recomputed from results.jsonl.

Usage: python3 -I summary.py < results.jsonl
Means are rounded once, half up, from the computed value. Each 95% interval resamples the cases
with replacement (10,000 resamples, a fresh random.Random(2282) per interval) and takes the
251st and 9,751st of the sorted values.
"""
import functools
import itertools
import json
import operator
import random
import sys
from decimal import ROUND_HALF_UP, Decimal

# evals/rubric.md weights.
WEIGHTS = {"correctness": .35, "autonomy": .25, "actionability": .2, "safety": .1, "concision": .1}

rows = [json.loads(line) for line in sys.stdin if line.strip()]
conditions = ["candidate"] + sorted({r["condition"] for r in rows} - {"candidate"})
setups = {r["condition"]: r["setup"] for r in rows}


def fmt(x, places=2):
    return str(Decimal(repr(x)).quantize(Decimal(1).scaleb(-places), ROUND_HALF_UP))


def mean(xs):
    xs = list(xs)
    return sum(xs) / len(xs)


def weighted(r):
    return sum(r["house"][d] * w for d, w in WEIGHTS.items())


def pick(run, cond):
    return [r for r in rows if r["run"] == run and r["condition"] == cond]


def interval(stat, cases):
    # A seeded generator, so that the published intervals can be recomputed; not for secrets.
    rng = random.Random(2282)  # nosec B311 # noqa: S311
    draws = sorted(stat([rng.choice(cases) for _ in cases]) for _ in range(10000))
    return stat(cases), draws[250], draws[9750]


def total(index, trials, cond, cases, value):
    return sum(value(index[(c, t, cond)]) for c in cases for t in trials)


def ratio(index, trials, first, other, value, cases):
    return total(index, trials, first, cases, value) / total(index, trials, other, cases, value)


def difference(index, trials, first, other, value, cases):
    gap = total(index, trials, first, cases, value) - total(index, trials, other, cases, value)
    return gap / (len(trials) * len(cases))


def show(label, est, lo, hi, places=2, sign=True):
    f = (lambda x: ("+" if x >= 0 else "") + fmt(x, places)) if sign else (lambda x: fmt(x, places))
    print(f"  {label}: {f(est)} ({f(lo)} to {f(hi)})")


for run in ("cold", "warm"):
    if not any(r["run"] == run for r in rows):
        continue
    print(f"== {run} calls")
    for cond in conditions:
        rs = pick(run, cond)
        line = {
            "answers": len(rs),
            "mean output tokens": fmt(mean(r["output_tokens"] for r in rs), 0),
            "mean input tokens, cache included": fmt(mean(
                r["input_tokens"] + r["cache_write_tokens"] + r["cache_read_tokens"] for r in rs), 0),
            "mean cache write": fmt(mean(r["cache_write_tokens"] for r in rs), 0),
            "mean cache read": fmt(mean(r["cache_read_tokens"] for r in rs), 0),
            "mean cost US$": fmt(mean(r["cost_usd"] for r in rs), 4),
            "mean characters": fmt(mean(r["characters"] for r in rs), 0),
            "lint clean": f"{sum(r['lint_clean'] for r in rs)} of {len(rs)}",
        }
        checked = [r for r in rs if r["regex_failed"] is not None]
        line["regex checks passed"] = f"{sum(not r['regex_failed'] for r in checked)} of {len(checked)}"
        if run == "cold":
            line["weighted score, evals/rubric.md"] = fmt(mean(weighted(r) for r in rs))
            line["correctness, evals/rubric.md"] = fmt(mean(r["house"]["correctness"] for r in rs))
            line["blocking findings, evals/rubric.md"] = sum(r["house"]["blocker"] for r in rs)
            line["correctness, evals/rubric-correctness.md"] = fmt(mean(
                r["correctness_only"]["correctness"] for r in rs))
            line["blocking findings, evals/rubric-correctness.md"] = sum(
                r["correctness_only"]["blocker"] for r in rs)
        print(f"{cond} ({setups[cond]}): {json.dumps(line)}")
    trials = sorted({r["trial"] for r in rows if r["run"] == run})
    cases = sorted({r["case_id"] for r in rows if r["run"] == run})
    index = {(r["case_id"], r["trial"], r["condition"]): r for r in rows if r["run"] == run}
    for first, other in itertools.combinations(conditions, 2):
        print(f" {first} against {other}:")
        for label, value in (("output tokens, ratio", operator.itemgetter("output_tokens")),
                             ("cost, ratio", operator.itemgetter("cost_usd"))):
            show(label, *interval(functools.partial(ratio, index, trials, first, other, value), cases),
                 sign=False)
        if run == "cold":
            for label, value in (("weighted score, evals/rubric.md, difference", weighted),
                                 ("correctness, evals/rubric-correctness.md, difference",
                                  lambda r: r["correctness_only"]["correctness"])):
                show(label, *interval(functools.partial(difference, index, trials, first, other, value), cases))
