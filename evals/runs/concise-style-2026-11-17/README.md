# Laconic against Claude Code's Concise style, 17 November 2026

The run behind the README's [Against Claude Code's Concise style](../../../README.md#against-claude-codes-concise-style).

- Laconic: `rules/laconic.md` at commit `a462993`; `laconic-evals` and `laconic-check` 0.5.0 built from that commit.
- Model: Claude Opus 5.5 (`claude-opus-5-5`, `--effort high`) in Claude Code 2.1.325, for the answers and the judge.
- Setups: Laconic's compact rules, Claude Code's Concise output style (`--settings '{"outputStyle":"Concise"}'`) and `evals/controls/terse.md` ('Answer concisely.'). Each system prompt had the line 'You have no tools in this conversation. Answer in text only; never attempt a tool call.' appended, followed by the setup's rules.
- Isolation: `--tools ""`, `--strict-mcp-config`, `--setting-sources ""`, `--disable-slash-commands`, `--no-session-persistence` and a fresh `CLAUDE_CONFIG_DIR`; each call ran from a fresh working directory, as `laconic-evals run` does.
- Cases: the 10 in `evals/holdout-cases.jsonl`, 3 trials each.
- Judging: `laconic-evals judge` shows a case's 3 answers in one call under letters shuffled per case and trial. First against `evals/rubric.md`, after its calibration (`evals/judge-calibration.jsonl`: 10 of 10 agreed); then against `evals/rubric-correctness.md`, which tells the judge not to score style, with no calibration set.

## Files

- `run.sh <work dir>`: the whole run, from the arm files to the summary. It calls the model 160 times; this run cost about US$4.
- `results.py`: run in the work directory. `results.py answers` writes each answer to `answers/` for `run.sh` to lint; without an argument it prints one row per answer, without the answer text.
- `results.jsonl`: those rows for this run: scores under both rubrics, token counts, cost, length, whether `laconic-check lint` found anything and which regex checks failed.
- `summary.py < results.jsonl`: every figure in the README section, recomputed. Means are rounded once from the computed value. Each interval resamples the cases (10,000 resamples, a fresh `random.Random(2282)` for each).

The answers themselves are not stored.
