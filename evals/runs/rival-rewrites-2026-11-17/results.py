"""One row per rewrite, without the texts: factalign.py's units, preserve's verdict, lengths,
cost and, for patina, its own --verify result.

Usage: python3 -I results.py <work dir> <laconic-check> > results.jsonl
Reads pairs-<rewriter>.jsonl, units-<rewriter>.jsonl, run-<rewriter>.jsonl and
patina-verify.jsonl from <work dir>.
"""
import json
import os
import subprocess
import sys
import tempfile

work, checker = sys.argv[1:3]
REWRITERS = ("laconic", "humanizer", "avoid", "work", "plain", "patina")


def read(name):
    path = f"{work}/{name}"
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def preserve_fails(source, rewrite):
    with tempfile.TemporaryDirectory() as d:
        for name, text in (("original", source), ("edit", rewrite)):
            with open(f"{d}/{name}", "w", encoding="utf-8", newline="\n") as f:
                f.write(text)
        done = subprocess.run([checker, "preserve", f"{d}/original", f"{d}/edit"], capture_output=True, check=False)
        return done.returncode != 0


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
            "preserve_fails": preserve_fails(p["source"], p["rewrite"]),
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
