#!/usr/bin/env bash
# Laconic's compact rules, caveman 4.1.0 and Chisle 3.12.1 on the 47 cases in evals/cases.jsonl.
# Cold: 2 trials, each call from a fresh working directory, so each wrote its prompt to the cache;
# judged blind against evals/rubric.md and again against evals/rubric-correctness.md.
# Warm: 1 trial from one fixed working directory, after a priming call, so the rules are read
# from the prompt cache as in a session; not judged. Then results.jsonl and the summary.
# Usage: run.sh <work dir>. Needs git, Claude Code signed in or ANTHROPIC_API_KEY set, python3,
# and release builds of tools/laconic-evals and tools/laconic-check from this checkout.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
# results.py and summary.py are shared with the Concise run.
SCRIPTS=$HERE/../concise-style-2026-11-17
REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$1"
W=$(cd "$1" && pwd)
EV=$REPO/tools/laconic-evals/target/release/laconic-evals
LC=$REPO/tools/laconic-check/target/release/laconic-check
CASES=$REPO/evals/cases.jsonl
export CLAUDE_CONFIG_DIR=$W/claude-config
mkdir -p "$CLAUDE_CONFIG_DIR" "$W/fixed-cwd"
cd "$W"

fetch() { # <name> <url> <commit>
  [[ -d "$1" ]] || git clone -q "$2" "$1"
  git -C "$1" checkout -q "$3"
}
fetch caveman https://github.com/JuliusBrussee/caveman.git 1e91f13
fetch Chisle https://github.com/JayPokale/Chisle.git 84e8f5a

NO_TOOLS='You have no tools in this conversation. Answer in text only; never attempt a tool call.'
# A skill file without its front matter.
body() { awk 'NR==1 && $0=="---" {f=1; next} f && $0=="---" {f=0; next} !f' "$1"; }
{ printf '%s\n\n' "$NO_TOOLS"; cat "$REPO/rules/laconic.md"; } > arm-laconic.md
{ printf '%s\n\n' "$NO_TOOLS"; body caveman/skills/caveman/SKILL.md; } > arm-caveman.md
{ printf '%s\n\n' "$NO_TOOLS"; body Chisle/skills/chisle/SKILL.md; } > arm-chisle.md
printf '%s\n' "$NO_TOOLS" > arm-none.md
printf '#!/usr/bin/env bash\ncd %q && exec claude "$@"\n' "$W/fixed-cwd" > claude-fixed-cwd
chmod +x claude-fixed-cwd
python3 - "$W" <<'PY'
import json, sys
w = sys.argv[1]
base = ["--disable-slash-commands", "--print", "--output-format", "json",
        "--no-session-persistence", "--strict-mcp-config", "--setting-sources", "",
        "--model", "claude-opus-5-5", "--effort", "high", "--tools", ""]
for name, claude in (("setups.json", "claude"), ("setups-warm.json", f"{w}/claude-fixed-cwd")):
    def runner(arm):
        return {"command": [claude] + base + ["--append-system-prompt-file", f"{w}/arm-{arm}.md"],
                "response_format": "claude-json", "budget_flag": "--max-budget-usd"}
    with open(f"{w}/{name}", "w") as f:
        json.dump({"claude": runner("none"), "setup-laconic": runner("laconic"),
                   "setup-caveman": runner("caveman"), "setup-chisle": runner("chisle")}, f, indent=2)
PY

for pair in candidate:setup-laconic comparator:setup-caveman baseline:setup-chisle; do
  run=("$EV" run --cases "$CASES" --runner "${pair#*:}" --condition "${pair%%:*}" --no-injection --jobs 4)
  "${run[@]}" --runner-config setups.json --trials 2 --budget-usd 6 --output responses.jsonl
  "${run[@]}" --runner-config setups-warm.json --case direct-answer --trials 1 --budget-usd 1 --output warm-prime.jsonl
  "${run[@]}" --runner-config setups-warm.json --trials 1 --budget-usd 4 --output warm.jsonl
done
for rubric in rubric:scores rubric-correctness:correctness; do
  "$EV" judge --runner claude --runner-config setups.json --cases "$CASES" \
    --rubric "$REPO/evals/${rubric%%:*}.md" --conditions baseline candidate comparator \
    --responses responses.jsonl --output "${rubric#*:}.jsonl"
done
cp "$CASES" cases.jsonl
python3 -I "$SCRIPTS/results.py" answers
mkdir -p lint
for answer in answers/*.txt; do
  if "$LC" lint --json - < "$answer" | grep -qF '{"findings":[]'; then echo 1; else echo 0; fi \
    > "lint/$(basename "$answer" .txt)"
done
python3 -I "$SCRIPTS/results.py" > results.jsonl
python3 -I "$SCRIPTS/summary.py" < results.jsonl
