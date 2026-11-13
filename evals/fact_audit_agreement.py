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
import shutil
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request
from collections import Counter
from collections.abc import Callable
from pathlib import Path
from typing import Any

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
    """Downloads `url` to `path` once; a later run reads the cached copy. The download is
    written under another name and then renamed, so an interrupted one leaves no `path`.
    Only an http or https URL is opened, never a local file or another scheme."""
    if urllib.parse.urlsplit(url).scheme not in ("http", "https"):
        raise ValueError(f"not an http or https URL: {url}")
    if not path.is_file():
        path.parent.mkdir(parents=True, exist_ok=True)
        # Bandit B310 flags every urlopen; the scheme is checked above.
        with urllib.request.urlopen(url, timeout=60) as response:  # nosec B310
            data = response.read()
        partial = path.with_name(path.name + ".partial")
        partial.write_bytes(data)
        partial.replace(path)
    return path


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    """The rows of a JSON Lines file; a blank line is an error, as it may be a lost row."""
    rows = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            raise RuntimeError(f"{path}: line {number} is blank")
        rows.append(json.loads(line))
    return rows


def judged_documents(rows: list[dict[str, Any]]) -> dict[tuple[str, str], set[str]]:
    """The kinds of unit the paper marks altered, for each (tool, sample) of the cycle.

    A repeated document, units that are not a list of objects, a unit of a group other than
    the paper's 3 or a `kept` that is not a boolean is an error, never coerced.
    """
    judged: dict[tuple[str, str], set[str]] = {}
    for row in rows:
        if row["set"] != "commercial" or row["aihb_cycle"] != CYCLE:
            continue
        key = (row["tool"], row["sample_id"])
        if key in judged:
            raise RuntimeError(f"judged twice: {key}")
        units = row["units"]
        if not isinstance(units, list) or not all(isinstance(unit, dict) for unit in units):
            raise TypeError(f"the units of {key} are not a list of objects")
        for unit in units:
            if unit.get("group") not in KINDS:
                raise RuntimeError(f"a unit of {key} has an unknown group {unit.get('group')!r}")
            if not isinstance(unit.get("kept"), bool):
                raise TypeError(f"a unit of {key} has a non-boolean 'kept'")
        judged[key] = {unit["group"] for unit in units if not unit["kept"]}
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
    """Whether `preserve` fails the rewrite, and what it flags. `binary` is the executable file
    `main` resolved; exit 0 is a pass, 1 a failure and any other status an error."""
    with tempfile.TemporaryDirectory() as tmp:
        original, edited = Path(tmp, "original.md"), Path(tmp, "edited.md")
        original.write_text(source, encoding="utf-8")
        edited.write_text(rewrite, encoding="utf-8")
        # Bandit B603 flags every subprocess call; this one runs the binary main resolved, with no shell.
        done = subprocess.run(  # nosec B603
            [binary, "preserve", str(original), str(edited)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
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
    """The tally over every judged document, and the ids where the 2 checkers differ. Each
    download is cached under its pin, so a changed pin downloads afresh."""
    judged = judged_documents(read_jsonl(fetch(FACT_AUDIT_URL, cache / FACT_AUDIT_COMMIT / "fact_units.jsonl")))
    pairs: dict[tuple[str, str], tuple[str, str]] = {}
    for tool in sorted({tool for tool, _ in judged}):
        path = fetch(AIHB_URL.format(tool=tool), cache / AIHB_REVISION / CYCLE / f"{tool}.jsonl")
        for row in read_jsonl(path):
            key = (row["humanizer"], row["sample_id"])
            if key in pairs:
                raise RuntimeError(f"benchmark rewrite twice: {key}")
            if not isinstance(row["output"], str):
                raise TypeError(f"benchmark rewrite missing or not text for {key}")
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
    binary = shutil.which(args.laconic_check)
    if binary is None:
        parser.error(f"--laconic-check {args.laconic_check} is not an executable file")
    counts, differences = run(binary, args.cache)
    print(f"Judged documents: {counts['altered'] + counts['kept']}\n")
    print(table(counts))
    print(f"\nDocuments where the checkers differ on numbers or quotations: {len(differences)}")
    for line in differences:
        print(f"- {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
