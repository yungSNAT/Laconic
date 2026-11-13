# Copyright 2026 Ryan Duguid
# SPDX-License-Identifier: FSL-1.1-ALv2

"""Measures `laconic-check preserve` against a published fact checker on public rewrites.

The AI Humanizer Benchmark publishes the AI-written texts of each monthly cycle and every
commercial humaniser's rewrite of them (CC BY 4.0). The paper 'Humanized but Still True?'
(arXiv 2611.00359) judged each number, name and quotation of 3 or more words in the
2026-10 cycle's rewrites as kept or altered, and published those judgements with its
checker, humanizer-fact-audit (code MIT, data CC BY 4.0). This script downloads both at
pinned revisions into a cache folder, runs `preserve` on each source and rewrite, and
reports, per kind, how many documents the paper marks altered that `preserve` also
reports, and on what `preserve` fails the documents the paper marks fully kept. Nothing
downloaded is stored in this repository.

    python3 evals/fact_audit_agreement.py --laconic-check tools/laconic-check/target/release/laconic-check
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import urllib.request
from collections import Counter
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[1]
CYCLE = "2026-10"
AIHB_REVISION = "6306b2043736dd958cf39ee1d9a64956c54dd78a"
AIHB_URL = (
    "https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark/resolve/"
    f"{AIHB_REVISION}/data/cycles/{CYCLE}/data/{{tool}}.jsonl"
)
FACT_AUDIT_COMMIT = "e665dda0506a64c834886c94d8f328d447119a07"
FACT_AUDIT_URL = (
    "https://raw.githubusercontent.com/zwang-devv/humanizer-fact-audit/"
    f"{FACT_AUDIT_COMMIT}/data/fact_units.jsonl"
)
KINDS = {
    "num": ("Numbers", "report a dropped number or quantity"),
    "quote": ("Quotations of 3 or more words", "report a dropped quotation"),
    "ent": ("Names", "list a name to check"),
}


def fetch(url: str, path: Path) -> Path:
    """Downloads `url` to `path` once; a later run reads the cached copy."""
    if not path.is_file():
        path.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(url, timeout=60) as response:
            path.write_bytes(response.read())
    return path


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def judged_documents(rows: list[dict[str, Any]]) -> dict[tuple[str, str], set[str]]:
    """The kinds of unit the paper marks altered, for each (tool, sample) of the cycle.

    A repeated document or a unit whose `kept` is not a boolean is an error, never coerced.
    """
    judged: dict[tuple[str, str], set[str]] = {}
    for row in rows:
        if row["set"] != "commercial" or row["aihb_cycle"] != CYCLE:
            continue
        key = (row["tool"], row["sample_id"])
        if key in judged:
            raise RuntimeError(f"judged twice: {key}")
        if any(not isinstance(unit["kept"], bool) for unit in row["units"]):
            raise RuntimeError(f"a unit of {key} has a non-boolean 'kept'")
        judged[key] = {unit["group"] for unit in row["units"] if not unit["kept"]}
    return judged


def flagged(stdout: str) -> set[str]:
    """What a `preserve` report flags: the paper's kinds, 'meaning' for meaning words and
    'other' for a dropped month, inline code or URL."""
    kinds = set()
    for line in stdout.splitlines():
        if line.startswith(("dropped number:", "dropped quantity:")):
            kinds.add("num")
        elif line.startswith("dropped quotation:"):
            kinds.add("quote")
        elif line.startswith("check name:"):
            kinds.add("ent")
        elif line.startswith("dropped meaning word:"):
            kinds.add("meaning")
        elif line.startswith("dropped "):
            kinds.add("other")
    return kinds


def preserve(binary: str, source: str, rewrite: str) -> tuple[bool, set[str]]:
    """Whether `preserve` fails the rewrite, and what it flags."""
    with tempfile.TemporaryDirectory() as tmp:
        original, edited = Path(tmp, "original.md"), Path(tmp, "edited.md")
        original.write_text(source, encoding="utf-8")
        edited.write_text(rewrite, encoding="utf-8")
        done = subprocess.run(
            [binary, "preserve", str(original), str(edited)], capture_output=True, text=True, encoding="utf-8"
        )
    if done.returncode not in (0, 1):
        raise RuntimeError(f"preserve exited {done.returncode}: {done.stderr.strip()}")
    return done.returncode == 1, flagged(done.stdout)


def tally(results: list[tuple[set[str], bool, set[str]]]) -> Counter[str]:
    """Counts per document: the paper's altered kinds, and what `preserve` did with them."""
    counts: Counter[str] = Counter()
    for altered, failed, kinds in results:
        if altered:
            counts["altered"] += 1
            counts["altered failed"] += failed
            for kind in altered:
                counts[f"{kind} altered"] += 1
                counts[f"{kind} flagged"] += kind in kinds
        else:
            counts["kept"] += 1
            if not failed:
                counts["kept passed"] += 1
            elif kinds & {"num", "quote"}:
                counts["kept failed on numbers or quotations"] += 1
            elif "other" in kinds:
                counts["kept failed on months, code or URLs"] += 1
            else:
                counts["kept failed on meaning words only"] += 1
    return counts


def table(counts: Counter[str]) -> str:
    rows = [
        "| Documents the paper marks as | Documents | `preserve` result |",
        "| --- | --- | --- |",
        f"| Any unit altered | {counts['altered']} | {counts['altered failed']} fail |",
    ]
    for kind, (label, result) in KINDS.items():
        rows.append(f"| {label} altered | {counts[f'{kind} altered']} | {counts[f'{kind} flagged']} {result} |")
    rows.append(
        f"| Every unit kept | {counts['kept']} | {counts['kept passed']} pass; "
        f"{counts['kept failed on meaning words only']} fail only on meaning words; "
        f"{counts['kept failed on months, code or URLs']} fail on a month, inline code or URL; "
        f"{counts['kept failed on numbers or quotations']} fail on a number, quantity or quotation |"
    )
    return "\n".join(rows)


def run(
    binary: str, cache: Path, check: Callable[[str, str, str], tuple[bool, set[str]]] = preserve
) -> tuple[Counter[str], list[str]]:
    """The tally over every judged document, and the ids where the 2 checkers differ."""
    judged = judged_documents(read_jsonl(fetch(FACT_AUDIT_URL, cache / "fact_units.jsonl")))
    pairs: dict[tuple[str, str], tuple[str, str]] = {}
    for tool in sorted({tool for tool, _ in judged}):
        path = fetch(AIHB_URL.format(tool=tool), cache / CYCLE / f"{tool}.jsonl")
        for row in read_jsonl(path):
            key = (row["humanizer"], row["sample_id"])
            if key in pairs:
                raise RuntimeError(f"benchmark rewrite twice: {key}")
            if not isinstance(row["output"], str):
                raise RuntimeError(f"benchmark rewrite missing for {key}")
            pairs[key] = (row["input"], row["output"])
    missing = sorted(set(judged) - set(pairs))
    if missing:
        raise RuntimeError(f"no benchmark rewrite for {missing}")
    results, differences = [], []
    for key in sorted(judged):
        failed, kinds = check(binary, *pairs[key])
        altered = judged[key]
        results.append((altered, failed, kinds))
        missed = altered - kinds - {"ent"}
        extra = kinds & {"num", "quote"} - altered
        if missed or extra:
            differences.append(f"{key[0]} {key[1]}: missed {sorted(missed)}, extra {sorted(extra)}")
    return tally(results), differences


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--laconic-check", default="laconic-check", help="the laconic-check binary")
    parser.add_argument("--cache", type=Path, default=ROOT / "evals" / "results" / "fact-audit")
    args = parser.parse_args(argv)
    counts, differences = run(args.laconic_check, args.cache)
    print(f"Judged documents: {counts['altered'] + counts['kept']}\n")
    print(table(counts))
    print(f"\nDocuments where the checkers differ on numbers or quotations: {len(differences)}")
    for line in differences:
        print(f"- {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
