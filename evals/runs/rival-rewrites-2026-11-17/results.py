"""One row per rewrite, without the texts: factalign.py's units, preserve's verdict, lengths,
cost and, for patina, its own --verify result. Run in the work directory.

Usage: python3 -I results.py > results.jsonl
Reads pairs-<rewriter>.jsonl, units-<rewriter>.jsonl, run-<rewriter>.jsonl, patina-verify.jsonl
and preserve/<rewriter>-<id>/fails ('1' when `laconic-check preserve` failed).
"""
import json
import os
from pathlib import Path

REWRITERS = ("laconic", "humanizer", "avoid", "work", "plain", "patina")


def read(name):
    if not os.path.exists(name):
        return []
    with open(name, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


verify = {r["id"]: r for r in read("patina-verify.jsonl")}
for s in REWRITERS:
    pairs = {r["id"]: r for r in read(f"pairs-{s}.jsonl")}
    runs = {r["case_id"].rsplit("-", 1)[1]: r for r in read(f"run-{s}.jsonl")}
    for u in read(f"units-{s}.jsonl"):
        sid, units = u["item_id"], u.get("units", [])
        p = pairs[sid]
        row = {
            "rewriter": s,
            "sample_id": sid,
            "units": len(units),
            "units_altered": sum(not x["kept"] for x in units),
            "altered_groups": sorted({x["group"] for x in units if not x["kept"]}),
            "preserve_fails": Path(f"preserve/{s}-{sid}/fails").read_text(encoding="utf-8").strip() == "1",
            "source_words": len(p["source"].split()),
            "rewrite_words": len(p["rewrite"].split()),
        }
        if sid in runs:
            usage = runs[sid].get("usage") or {}
            row["output_tokens"] = usage.get("output_tokens", 0)
            row["cost_usd"] = runs[sid].get("cost_usd")
        if s == "patina":
            v = verify[sid]
            row["patina_verify"] = {k: v.get(k) for k in ("verified", "reason", "mps", "fidelity", "retried")}
        print(json.dumps(row))
