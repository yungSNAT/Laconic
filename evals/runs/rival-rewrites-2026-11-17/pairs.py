"""Source and rewrite pairs, one file per rewriter. Run in the work directory.

Writes pairs-<rewriter>.jsonl ({id, source, rewrite}) for factalign.py from run-<rewriter>.jsonl
and, for patina, patina-out/<id>.json, whose `verification` goes to patina-verify.jsonl; and
preserve/<rewriter>-<id>/original and edit for run.sh to check with `laconic-check preserve`.
"""
import json
from pathlib import Path

MODEL_REWRITERS = ("laconic", "humanizer", "avoid", "work", "plain")


def read(name):
    with open(name, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def write(name, rows):
    with open(name, "w", encoding="utf-8") as f:
        f.writelines(json.dumps(row) + "\n" for row in rows)


sources = {r["sample_id"]: r["input"] for r in read("aihb-2026-10.jsonl")}
pairs = {}
for s in MODEL_REWRITERS:
    pairs[s] = [{"id": sid, "source": sources[sid], "rewrite": r["response"]}
                for r in read(f"run-{s}.jsonl") for sid in [r["case_id"].rsplit("-", 1)[1]]]
patina = {sid: json.loads(Path(f"patina-out/{sid}.json").read_text(encoding="utf-8")) for sid in sources}
pairs["patina"] = [{"id": sid, "source": sources[sid], "rewrite": d.get("output") or ""}
                   for sid, d in patina.items()]
write("patina-verify.jsonl", ({"id": sid, **(d.get("verification") or {})} for sid, d in patina.items()))
for s, rows in pairs.items():
    write(f"pairs-{s}.jsonl", rows)
    for row in rows:
        folder = Path(f"preserve/{s}-{row['id']}")
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "original").write_text(row["source"], encoding="utf-8", newline="\n")
        (folder / "edit").write_text(row["rewrite"], encoding="utf-8", newline="\n")
