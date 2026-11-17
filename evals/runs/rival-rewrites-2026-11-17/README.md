# Rival humaniser skills and patina on the 2026-10 cycle, 17 November 2026

The run behind [By humaniser](../../README.md#by-humaniser) in `evals/README.md`, from its paragraph that starts 'On 17 November 2026'.

- Texts: the 33 sources of the [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark)'s 2026-10 cycle (CC BY 4.0), the `input` field of `data/cycles/2026-10/data/aihumanize.jsonl` at revision `6306b2043736dd958cf39ee1d9a64956c54dd78a`. `run.sh` downloads it and checks its SHA-256; the texts are not stored here.
- Checker: `factalign.py` from [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit) at `e665dda0506a64c834886c94d8f328d447119a07` (SHA-256 `632123df…`), with Python 3.12, spaCy 3.8.7 and `en_core_web_sm` 3.8.0 on CPU; and `laconic-check preserve` 0.5.0.
- Rewriters: Laconic's prose skill at `a462993`; [humanizer](https://github.com/blader/humanizer) at `b48c832` (tag `v4.0.0`); [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing) at `f9f626b` (tag `v3.38.0`); [work-humanizer](https://github.com/phuryn/work-humanizer) at `0a90657`; the request alone; [patina](https://github.com/devswha/patina) at `fb3bd7e` (tag `v9.1.0`). `run.sh` clones them; their text is not stored here.
- Arms: the line 'You have no tools in this conversation. Answer in text only; never attempt a tool call.', then the skill's `SKILL.md`, then each file it says to read before a rewrite as a section headed `## Reference: <path>` (`run.sh` lists them), appended to the system prompt. Each text went in its own call with the request 'Rewrite this text so it reads as if a person wrote it. Reply with the rewritten text only.'
- Model: Claude Opus 5.5 (`claude-opus-5-5`, `--effort high`) in Claude Code 2.1.325, with `--tools ""`, `--strict-mcp-config`, `--setting-sources ""`, `--disable-slash-commands`, `--no-session-persistence` and a fresh `CLAUDE_CONFIG_DIR`. patina called the same Claude Code through a stand-in on its `PATH` that added the same isolation and `--effort high`; its own prompts were unchanged.
- The published rows for `humanizer`, `work-humanizer` and the request alone ran each call from a fresh working directory; Laconic's and avoid-ai-writing's ran from one fixed directory, which `run.sh` uses for all, and which changes only what is read from the prompt cache.

## Files

- `run.sh <work dir>`: the whole run, from the downloads to the summary. It makes 165 rewrite calls and 3 patina calls a text; the published rewrites cost about US$11 and patina's about US$5.
- `pairs.py`: source and rewrite pairs for `factalign.py`.
- `results.py`: one row per rewrite, without the texts: units, units altered and their groups, whether `preserve` fails, lengths in words, cost, and patina's own `--verify` result.
- `results.jsonl`: those rows for this run.
- `summary.py results.jsonl`: every figure in the table, recomputed.

The rewrites themselves are not stored.
