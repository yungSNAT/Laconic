"""Source and rewrite pairs for factalign.py, one file per rewriter.

Usage: python3 -I pairs.py <work dir>
Writes pairs-<rewriter>.jsonl ({id, source, rewrite}) from run-<rewriter>.jsonl and, for patina,
patina-out/<id>.json, whose `verification` it keeps in patina-verify.jsonl.
"""
import json
import sys

work = sys.argv[1]
with open(f"{work}/aihb-2026-10.jsonl", encoding="utf-8") as f:
    sources = {r["sample_id"]: r["input"] for r in map(json.loads, f)}


def write(name, rows):
    with open(f"{work}/{name}", "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row) + "\n")


for s in ("laconic", "humanizer", "avoid", "work", "plain"):
    with open(f"{work}/run-{s}.jsonl", encoding="utf-8") as f:
        runs = [json.loads(line) for line in f if line.strip()]
    write(f"pairs-{s}.jsonl", ({"id": r["case_id"].rsplit("-", 1)[1], "source": sources[r["case_id"].rsplit("-", 1)[1]],
                                "rewrite": r["response"]} for r in runs))
patina = {}
for sid in sources:
    with open(f"{work}/patina-out/{sid}.json", encoding="utf-8") as f:
        patina[sid] = json.load(f)
write("pairs-patina.jsonl", ({"id": sid, "source": sources[sid], "rewrite": d.get("output") or ""}
                             for sid, d in patina.items()))
write("patina-verify.jsonl", ({"id": sid, **(d.get("verification") or {})} for sid, d in patina.items()))
