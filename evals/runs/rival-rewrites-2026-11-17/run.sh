#!/usr/bin/env bash
# Rewrites of the 33 texts of the AI Humanizer Benchmark's 2026-10 cycle by 4 humaniser skills,
# a one-line request with no skill and patina 9.1.0 with --verify, each judged against its
# source by the paper's factalign.py and by laconic-check preserve; then results.jsonl and the
# summary. Usage: run.sh <work dir>. Needs git, curl, node, npm, uv, Claude Code signed in or
# ANTHROPIC_API_KEY set, python3, and release builds of tools/laconic-evals and
# tools/laconic-check from this checkout.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$1"
W=$(cd "$1" && pwd)
EV=$REPO/tools/laconic-evals/target/release/laconic-evals
LC=$REPO/tools/laconic-check/target/release/laconic-check
export CLAUDE_CONFIG_DIR=$W/claude-config
mkdir -p "$CLAUDE_CONFIG_DIR" "$W/fixed-cwd" "$W/arms" "$W/patina-in" "$W/patina-out"
cd "$W"

# get <file> <sha256> <url>
get() {
  [ -s "$1" ] || curl -fsSL --proto '=https' --tlsv1.2 -o "$1" "$3"
  echo "$2  $1" | sha256sum -c --quiet
}
get aihb-2026-10.jsonl eafee32e4f584aebf73d9f4eb78b5a5a96a2b62093975d2148a44addd5da052f \
  https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark/resolve/6306b2043736dd958cf39ee1d9a64956c54dd78a/data/cycles/2026-10/data/aihumanize.jsonl
get factalign.py 632123df588f99adf95ba43228528603f4b68e0643fc7cedb0bd53e8eb653aca \
  https://raw.githubusercontent.com/zwang-devv/humanizer-fact-audit/e665dda0506a64c834886c94d8f328d447119a07/factalign.py
fetch() { # <name> <url> <commit>
  [ -d "$1" ] || git clone -q "$2" "$1"
  git -C "$1" checkout -q "$3"
}
fetch humanizer https://github.com/blader/humanizer.git b48c832
fetch avoid-ai-writing https://github.com/conorbronsdon/avoid-ai-writing.git f9f626b
fetch work-humanizer https://github.com/phuryn/work-humanizer.git 0a90657
fetch patina https://github.com/devswha/patina.git fb3bd7e
(cd patina && npm ci --omit=dev --ignore-scripts --silent)

# Each skill's SKILL.md, with the files it says to read before a rewrite added as labelled
# sections: Laconic's edit-and-audit.md (read in full for an edit) and eval.md (its quick check,
# always), plus its patterns.md and style-manual-rules.md; avoid-ai-writing's patterns.md (read
# in full before any rewrite); work-humanizer's tells.md. humanizer's SKILL.md is self-contained.
NO_TOOLS='You have no tools in this conversation. Answer in text only; never attempt a tool call.'
section() { printf '\n\n## Reference: %s\n\n' "$1"; cat "$2"; }
P=$REPO/skills/laconic-prose
{ printf '%s\n\n' "$NO_TOOLS"; cat "$P/SKILL.md"
  for f in edit-and-audit eval patterns style-manual-rules; do section "references/$f.md" "$P/references/$f.md"; done
} > arms/laconic.md
{ printf '%s\n\n' "$NO_TOOLS"; cat humanizer/SKILL.md; } > arms/humanizer.md
{ printf '%s\n\n' "$NO_TOOLS"; cat avoid-ai-writing/SKILL.md
  section references/patterns.md avoid-ai-writing/references/patterns.md; } > arms/avoid.md
H=work-humanizer/skills/work-humanizer
{ printf '%s\n\n' "$NO_TOOLS"; cat "$H/SKILL.md"; section tells.md "$H/tells.md"; } > arms/work.md
printf '%s\n' "$NO_TOOLS" > arms/plain.md
printf '#!/usr/bin/env bash\ncd %q && exec claude "$@"\n' "$W/fixed-cwd" > claude-fixed-cwd
chmod +x claude-fixed-cwd
python3 - "$W" <<'PY'
import json, sys
w = sys.argv[1]
base = [f"{w}/claude-fixed-cwd", "--disable-slash-commands", "--print", "--output-format", "json",
        "--no-session-persistence", "--strict-mcp-config", "--setting-sources", "",
        "--model", "claude-opus-5-5", "--effort", "high", "--tools", ""]
with open(f"{w}/setups.json", "w") as f:
    json.dump({n: {"command": base + ["--append-system-prompt-file", f"{w}/arms/{n}.md"],
                   "response_format": "claude-json", "budget_flag": "--max-budget-usd"}
               for n in ("laconic", "humanizer", "avoid", "work", "plain")}, f, indent=2)
ask = "Rewrite this text so it reads as if a person wrote it. Reply with the rewritten text only."
with open(f"{w}/cases.jsonl", "w") as out:
    for line in open(f"{w}/aihb-2026-10.jsonl"):
        r = json.loads(line)
        out.write(json.dumps({"id": f"aihb-2026-10-{r['sample_id']}", "category": "rewrite",
                              "risk": "low", "prompt": f"{ask}\n\n{r['input']}",
                              "criteria": ["Keeps every fact of the source."]}) + "\n")
        with open(f"{w}/patina-in/{r['sample_id']}.md", "w") as p:
            p.write(r["input"] + "\n")
PY
"$EV" validate --cases cases.jsonl
for setup in laconic humanizer avoid work plain; do
  "$EV" run --cases cases.jsonl --runner-config setups.json --runner $setup --condition candidate \
    --no-injection --trials 1 --jobs 4 --budget-usd 10 --output "run-$setup.jsonl"
done
# patina writes its own prompts and calls `claude -p`; it ran at high effort, isolated as above.
mkdir -p patina-bin
printf '#!/usr/bin/env bash\n[ "${1:-}" = -p ] || exec %q "$@"\nexec %q "$@" --setting-sources "" --no-session-persistence --disable-slash-commands --effort high\n' \
  "$(command -v claude)" "$(command -v claude)" > patina-bin/claude
chmod +x patina-bin/claude
for f in patina-in/*.md; do
  id=$(basename "$f" .md)
  [ -s "patina-out/$id.json" ] || PATH="$W/patina-bin:$PATH" node patina/bin/patina.js --verify --lang en \
    --backend claude-cli --model claude-opus-5-5 --format json "$f" > "patina-out/$id.json" || true
done
python3 -I "$HERE/pairs.py" "$W"
for s in laconic humanizer avoid work plain patina; do
  uv run -q --python 3.12 --with spacy==3.8.7 --with click --with numpy \
    --with https://github.com/explosion/spacy-models/releases/download/en_core_web_sm-3.8.0/en_core_web_sm-3.8.0-py3-none-any.whl \
    python -I factalign.py --in "pairs-$s.jsonl" --out "units-$s.jsonl"
done
python3 -I "$HERE/results.py" "$W" "$LC" > results.jsonl
python3 -I "$HERE/summary.py" results.jsonl
