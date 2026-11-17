#!/usr/bin/env bash
# Laconic's compact rules, Claude Code's Concise output style and 'Answer concisely.' on the 10
# held-out cases, 3 trials each, judged blind against evals/rubric.md and again against
# evals/rubric-correctness.md; then results.jsonl and the summary.
# Usage: run.sh <work dir>. Needs Claude Code signed in or ANTHROPIC_API_KEY set, and release
# builds of tools/laconic-evals and tools/laconic-check from this checkout.
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
"$EV" record setups concise-style-2026-11-17 "$W"

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
cp "$CASES" cases.jsonl
"$EV" record answers "$W"
mkdir -p lint
for answer in answers/*.txt; do
  if "$LC" lint --json - < "$answer" | grep -qF '{"findings":[]'; then echo 1; else echo 0; fi \
    > "lint/$(basename "$answer" .txt)"
done
"$EV" record style-results "$W" > results.jsonl
"$EV" record style-summary results.jsonl
