"""The published figures, recomputed from results.jsonl.

Usage: python3 -I summary.py < results.jsonl
"""
import collections
import json
import sys
from decimal import ROUND_HALF_UP, Decimal

rows = [json.loads(line) for line in sys.stdin if line.strip()]


def fmt(x, places):
    return str(Decimal(repr(x)).quantize(Decimal(1).scaleb(-places), ROUND_HALF_UP))


for s in dict.fromkeys(r["rewriter"] for r in rows):
    rs = [r for r in rows if r["rewriter"] == s]
    altered = [r for r in rs if r["units_altered"]]
    kept = [r for r in rs if not r["units_altered"]]
    line = {
        "texts": len(rs),
        "texts with a unit": sum(bool(r["units"]) for r in rs),
        "units": sum(r["units"] for r in rs),
        "documents with a unit altered": len(altered),
        "units altered": sum(r["units_altered"] for r in rs),
        "documents altered, by group": dict(collections.Counter(g for r in altered for g in r["altered_groups"])),
        "preserve fails, altered": f"{sum(r['preserve_fails'] for r in altered)} of {len(altered)}",
        "preserve fails, kept": f"{sum(r['preserve_fails'] for r in kept)} of {len(kept)}",
        "mean length against source, in words": fmt(
            sum(r["rewrite_words"] / r["source_words"] for r in rs) / len(rs), 2),
        "altered": [r["sample_id"] for r in altered],
    }
    if any("cost_usd" in r for r in rs):
        line["cost US$"] = fmt(sum(r.get("cost_usd") or 0 for r in rs), 2)
    if s == "patina":
        v = [r["patina_verify"] for r in rs]
        line["--verify passed"] = sum(x["verified"] is True for x in v)
        line["--verify reasons"] = dict(collections.Counter(x["reason"] for x in v if x["verified"] is not True))
        line["meaning score"] = dict(collections.Counter(x["mps"] for x in v))
        line["fidelity"] = dict(collections.Counter(x["fidelity"] for x in v))
        line["retried"] = sum(bool(x["retried"]) for x in v)
    print(f"{s}: {json.dumps(line)}")
