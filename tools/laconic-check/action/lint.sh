#!/usr/bin/env bash
# Runs laconic-check lint with LACONIC_ARGS on LACONIC_FILES, or on every Markdown file Git
# tracks, and turns each finding into a workflow annotation: an error, or a warning for an
# advisory finding. Exits with lint's status. Used by the composite Action in action.yml;
# written for bash 3.2, which some macOS runners still use.
set -uo pipefail
files=()
if [ -n "$LACONIC_FILES" ]; then
  read -r -d '' -a files <<< "$LACONIC_FILES" || true
else
  while IFS= read -r file; do files+=("$file"); done < <(git ls-files -- '*.md')
fi
args=()
read -r -a args <<< "$LACONIC_ARGS" || true
if [ ${#files[@]} -eq 0 ]; then
  echo "No Markdown files to lint."
  exit 0
fi
# Workflow commands need '%', CR and LF escaped in messages, and ',' and ':' too in properties.
message() {
  local s=${1//%/%25}
  s=${s//$'\r'/%0D}
  printf '%s' "${s//$'\n'/%0A}"
}
property() {
  local s
  s=$(message "$1")
  s=${s//,/%2C}
  printf '%s' "${s//:/%3A}"
}
out=$(laconic-check lint ${args[@]+"${args[@]}"} "${files[@]}")
status=$?
[ -z "$out" ] || printf '%s\n' "$out"
pattern='^(.+):([0-9]+):([0-9]+): ([a-z-]+): (.*)$'
while IFS= read -r finding; do
  [[ $finding =~ $pattern ]] || continue
  level=error
  [[ ${BASH_REMATCH[5]} == *'(advisory)'* ]] && level=warning
  echo "::$level file=$(property "${BASH_REMATCH[1]}"),line=${BASH_REMATCH[2]},col=${BASH_REMATCH[3]},title=$(property "laconic-check ${BASH_REMATCH[4]}")::$(message "${BASH_REMATCH[5]}")"
done <<< "$out"
exit "$status"
