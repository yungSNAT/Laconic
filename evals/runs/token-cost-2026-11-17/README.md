# Token cost against caveman and Chisle, 17 November 2026

The run behind the README's [Token cost against caveman and Chisle](../../../README.md#token-cost-against-caveman-and-chisle).

- Laconic: `rules/laconic.md` at commit `a462993`; `laconic-evals` and `laconic-check` 0.5.0 built from that commit.
- Rivals: [caveman](https://github.com/JuliusBrussee/caveman) at `1e91f13` (tag `v4.1.0`, file `skills/caveman/SKILL.md`) and [Chisle](https://github.com/JayPokale/Chisle) at `84e8f5a` (tag `v3.12.1`, file `skills/chisle/SKILL.md`), each without its front matter. `run.sh` clones them; their text is not stored here.
- Model: Claude Opus 5.5 (`claude-opus-5-5`, `--effort high`) in Claude Code 2.1.325, for the answers and the judge.
- Each system prompt had the line 'You have no tools in this conversation. Answer in text only; never attempt a tool call.' appended, followed by the rule set. Isolation: `--tools ""`, `--strict-mcp-config`, `--setting-sources ""`, `--disable-slash-commands`, `--no-session-persistence` and a fresh `CLAUDE_CONFIG_DIR`.
- Cases: the 47 in `evals/cases.jsonl`.
- Cold calls: 2 trials, each call from a fresh working directory, as `laconic-evals run` does. Claude Code's system prompt names that directory, which appears to be why each call wrote everything after its first 658 tokens to the prompt cache.
- Cached calls: 1 trial, every call from one fixed working directory after a priming call, so each read the rules from the cache, as calls in one session do. Not judged.
- Judging (cold calls): `laconic-evals judge` shows a case's 3 answers in one call under letters shuffled per case and trial; first against `evals/rubric.md`, then against `evals/rubric-correctness.md`, which tells the judge not to score style, with no calibration set.

## Files

- `run.sh <work dir>`: the whole run, from cloning the rivals to the summary. It calls the model 614 times; this run cost about US$18.
- `results.py`: run in the work directory. `results.py answers` writes each answer to `answers/` for `run.sh` to lint; without an argument it prints one row per answer, without the answer text.
- `results.jsonl`: those rows for this run: `run` is `cold` or `warm` (cached); scores under both rubrics for cold calls, token counts with cache writes and reads, cost, length, whether `laconic-check lint` found anything and which regex checks failed.
- `summary.py < results.jsonl`: every figure in the README section, recomputed. Means are rounded once from the computed value. Each interval resamples the cases (10,000 resamples, a fresh `random.Random(2282)` for each).

The answers themselves are not stored.
