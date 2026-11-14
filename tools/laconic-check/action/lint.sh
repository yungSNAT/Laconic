#!/usr/bin/env bash
# Runs the laconic-check binary install.sh verified ($LACONIC_CHECK) with LACONIC_ARGS on
# LACONIC_FILES, one path a line, or on every Markdown file Git tracks, and turns each finding
# into a workflow annotation: an error, or a warning for an advisory finding. Exits with lint's
# status for all the files. Used by the composite Action in action.yml; written for bash 3.2,
# which some macOS runners still use.
set -uo pipefail
files=()
if [[ -z "$LACONIC_FILES" ]]; then
  tracked=$(mktemp) || exit 2
  trap 'rm -f -- "$tracked"' EXIT
  if ! git ls-files -z -- '*.md' '*.markdown' > "$tracked"; then
    echo "::error::cannot list the Markdown files Git tracks; check out the repository first, or name files in the files input"
    exit 2
  fi
  while IFS= read -r -d '' file; do
    files+=("$file")
  done < "$tracked"
else
  # A path named twice is linted once, so the --max-findings check below counts it once.
  unique=$(awk '!seen[$0]++' <<< "$LACONIC_FILES") || exit 2
  while IFS= read -r file; do
    [[ -z "$file" ]] || files+=("$file")
  done <<< "$unique"
fi
args=()
read -r -d '' -a args <<< "$LACONIC_ARGS" || true
warn=
for arg in ${args[@]+"${args[@]}"}; do
  if [[ "$arg" == --json ]]; then
    echo "::error::the Action reads lint's text output, so args cannot include --json"
    exit 2
  fi
  [[ "$arg" != --warn ]] || warn=1
done
if [[ ${#files[@]} -eq 0 ]]; then
  echo "No Markdown files to lint."
  exit 0
fi
for index in "${!files[@]}"; do
  case "${files[index]}" in
    *$'\n'* | *$'\r'*)
      echo "::error::the Action cannot annotate a filename containing a line break"
      exit 2
      ;;
    -*) files[index]="./${files[index]}" ;;
  esac
done
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
pattern='^(.+):([0-9]+):([0-9]+): ([a-z-]+): (.*)$'
failing=0
annotate() {
  local finding level
  while IFS= read -r finding; do
    # Filename and message text must not be interpreted as workflow commands.
    [[ -z "$finding" ]] || printf 'laconic-check: %s\n' "$finding"
    [[ $finding =~ $pattern ]] || continue
    level=error
    if [[ ${BASH_REMATCH[5]} == *'(advisory)'* ]]; then
      level=warning
    else
      failing=$((failing + 1))
    fi
    echo "::$level file=$(property "${BASH_REMATCH[1]}"),line=${BASH_REMATCH[2]},col=${BASH_REMATCH[3]},title=$(property "laconic-check ${BASH_REMATCH[4]}")::$(message "${BASH_REMATCH[5]}")"
  done
}
# Windows caps a command line at 32,767 characters, so lint runs on batches of files whose
# paths take at most 24,000 characters. The highest exit status wins: 2 (an error) over
# 1 (too many findings) over 0.
# shortcut: bash counts characters, but Windows counts a character outside the Basic
# Multilingual Plane, such as an emoji, twice. The 8,767 characters to spare hold the binary's
# path, the options and that many such characters; counting UTF-16 units would lift the limit.
status=0
calls=0
batch=()
size=0
run_batch() {
  local out code
  out=$("$LACONIC_CHECK" lint ${args[@]+"${args[@]}"} "${batch[@]}" 2>&1)
  code=$?
  annotate <<< "$out"
  if (( code > status )); then status=$code; fi
  calls=$((calls + 1))
  batch=()
  size=0
}
for file in "${files[@]}"; do
  if (( ${#batch[@]} > 0 && size + ${#file} + 3 > 24000 )); then run_batch; fi
  batch+=("$file")
  size=$((size + ${#file} + 3))
done
run_batch
# Leading and trailing whitespace off, as lint trims options and project file lines.
trim() {
  local s=$1
  s=${s#"${s%%[![:space:]]*}"}
  printf '%s' "${s%"${s##*[![:space:]]}"}"
}
# A count as lint reads it, digits with an optional '+', without the '+' or leading zeros.
digits() {
  local s=${1#+}
  s=${s#"${s%%[!0]*}"}
  printf '%s' "${s:-0}"
}
# The --max-findings lint applies, as its option parser and project file reader decide: the
# last --max-findings before any '--' in args, else the last max-findings line of the project
# file (--config's, or .laconic-check in this directory), else 0. The first calls already
# rejected a bad option, value or project file, so these are well formed.
max_findings() {
  local i=0 arg max='' config='' line key
  while (( i < ${#args[@]} )); do
    arg=${args[i]}
    case "$arg" in
      --) break ;;
      --max-findings=*) max=${arg#*=} ;;
      --config=*) config=${arg#*=} ;;
      --max-findings) i=$((i + 1)); max=${args[i]} ;;
      --config) i=$((i + 1)); config=${args[i]} ;;
      --disable | --only) i=$((i + 1)) ;;
    esac
    i=$((i + 1))
  done
  if [[ -z "$max" ]]; then
    [[ -n "$config" || ! -f .laconic-check ]] || config=.laconic-check
    if [[ -n "$config" ]]; then
      while IFS= read -r line || [[ -n "$line" ]]; do
        line=$(trim "${line#$'\xef\xbb\xbf'}")
        [[ -n "$line" && $line != '#'* && $line == *=* ]] || continue
        key=$(trim "${line%%=*}")
        [[ "$key" != max-findings ]] || max=$(trim "${line#*=}")
      done < "$config"
    fi
  fi
  digits "${max:-0}"
}
# --max-findings, from args or the project file, applies to each call, so when every call passed
# without --warn, the failing findings of all the calls are judged against it here. This needs no
# further lint call, so no command line grows with the files. A limit of 19 or more digits is
# beyond bash's arithmetic and any count of findings.
if [[ -z "$warn" ]] && (( calls > 1 && status == 0 && failing > 0 )); then
  max=$(max_findings)
  if (( ${#max} < 19 && failing > max )); then
    echo "::error::$failing failing findings across all the files, more than --max-findings allows ($max)"
    status=1
  fi
fi
exit "$status"
