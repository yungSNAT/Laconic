#!/usr/bin/env bash
# Laconic's compact rules, Claude Code's Concise output style and 'Answer concisely.' on the 10
# held-out cases, 3 trials each, judged blind against evals/rubric.md and again against
# evals/rubric-correctness.md; then results.jsonl and the summary.
# Usage: run.sh <work dir>. Needs Claude Code signed in or ANTHROPIC_API_KEY set, python3, and
# release builds of tools/laconic-evals and tools/laconic-check from this checkout.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$1"
W=$(cd "$1" && pwd)
EV=$REPO/tools/laconic-evals/target/release/laconic-evals
LC=$REPO/tools/laconic-check/target/release/laconic-check
CASES=$REPO/evals/holdout-cases.jsonl
export CLAUDE_CONFIG_DIR=$W/claude-config
mkdir -p "$CLAUDE_CONFIG_DIR"
cd "$W"

NO_TOOLS='You have no tools in this conversation. Answer in text only; never attempt a tool call.'
printf '%s\n' "$NO_TOOLS" > arm-none.md
{ printf '%s\n\n' "$NO_TOOLS"; cat "$REPO/rules/laconic.md"; } > arm-laconic.md
{ printf '%s\n\n' "$NO_TOOLS"; tr -d '\r' < "$REPO/evals/controls/terse.md"; } > arm-terse.md
python3 - "$W" > setups.json <<'PY'
import json, sys
w = sys.argv[1]
base = ["claude", "--disable-slash-commands", "--print", "--output-format", "json",
        "--no-session-persistence", "--strict-mcp-config", "--setting-sources", "",
        "--model", "claude-opus-5-5", "--effort", "high", "--tools", ""]
def runner(extra):
    return {"command": base + extra, "response_format": "claude-json", "budget_flag": "--max-budget-usd"}
print(json.dumps({
    "claude": runner(["--append-system-prompt-file", f"{w}/arm-none.md"]),
    "setup-laconic": runner(["--append-system-prompt-file", f"{w}/arm-laconic.md"]),
    "setup-concise": runner(["--settings", '{"outputStyle":"Concise"}',
                             "--append-system-prompt-file", f"{w}/arm-none.md"]),
    "setup-terse": runner(["--append-system-prompt-file", f"{w}/arm-terse.md"]),
}, indent=2))
PY

"$EV" judge --runner claude --runner-config setups.json --cases "$REPO/evals/cases.jsonl" \
  --rubric "$REPO/evals/rubric.md" --calibrate "$REPO/evals/judge-calibration.jsonl" > calibration.json
for pair in candidate:setup-laconic comparator:setup-concise baseline:setup-terse; do
  "$EV" run --cases "$CASES" --runner-config setups.json --runner "${pair#*:}" \
    --condition "${pair%%:*}" --no-injection --trials 3 --jobs 4 --budget-usd 2 --output responses.jsonl
done
for rubric in rubric:scores rubric-correctness:correctness; do
  "$EV" judge --runner claude --runner-config setups.json --cases "$CASES" \
    --rubric "$REPO/evals/${rubric%%:*}.md" --conditions baseline candidate comparator \
    --responses responses.jsonl --output "${rubric#*:}.jsonl"
done
python3 -I "$HERE/results.py" "$W" "$CASES" "$LC" > results.jsonl
python3 -I "$HERE/summary.py" results.jsonl
