# Evaluations

The harness compares response quality, not just length. Cases live in `cases.jsonl`; the scoring contract lives in `rubric.md`.

## Validate and plan

```bash
python3 scripts/run_evals.py validate
python3 scripts/run_evals.py plan --trials 3 --include-comparator
```

## Run

Run each condition into the same results file. Candidate and comparator instructions are injected from the supplied skill file; task prompts remain identical.

```bash
python3 scripts/run_evals.py run \
  --runner claude \
  --condition baseline \
  --trials 3 \
  --budget-usd 12.50 \
  --output evals/results/responses.jsonl

python3 scripts/run_evals.py run \
  --runner claude \
  --condition candidate \
  --condition-skill skills/laconic/SKILL.md \
  --trials 3 \
  --budget-usd 12.50 \
  --output evals/results/responses.jsonl
```

The default Claude runner reports dollar cost and receives the remaining condition budget on every call. Runners without cost reporting are rejected unless `--allow-unmetered` is supplied; use that flag only when the provider account has its own hard cap.

The Claude example runner isolates the call from the operator's own agent configuration with `--setting-sources ""`. The Codex example does not fully isolate it: on 30 October 2026 (codex-cli 0.164.0), a call with `--ignore-user-config --ephemeral` still listed the operator's personal instruction files and skills, including Laconic's. Use Codex to compare setups only from a machine or `CODEX_HOME` that holds none of them. Keep isolation when adding runners: without it, user-level plugins, hooks, memory and output styles leak into every condition and shape the responses being judged. The sharpest case is this repo's own always-on flag (`~/.claude/.laconic-always`), which would inject the full laconic ruleset into the **baseline** condition and make the comparison measure the skill against itself.

Isolation also drops the operator's saved model and effort settings, so the claude runner pins `--model` explicitly. Keep a pin when editing the runner: without one, the eval silently runs whatever the operator (or the CLI release) defaults to; the model would vary between operators and over time, and per-token cost varies with it. The pinned model is part of the result: record it with published numbers, as below.

`--jobs N` runs up to N model calls at once, which can reduce wall time: on 20 October 2026, 40 humanising calls for one setup took 2 min 50 s with `--jobs 4`. The budget is a soft threshold: no new call is submitted once recorded cost reaches `--budget-usd`, but up to N - 1 calls may still be running, so it is not a hard spending cap. If a call fails, the calls already running finish and their successful results are saved before the error is raised, so a resumed run skips them. Rows are written as calls complete, not in case order.

Runs are resumable: rerun the same command after a provider failure and completed `(case, trial, condition, runner)` rows are skipped. Each incomplete call is retried twice by default, and the final provider error is preserved.

## Terse control

A skill should beat a plain 'be brief' instruction, not only no instruction at all; comparing against an empty prompt credits the skill with the generic effect of asking for brevity. `evals/controls/terse.md` holds that instruction ('Answer concisely.'). Run it as the comparator, either injected or as a whole setup:

```bash
python3 scripts/run_evals.py run --runner claude --condition comparator --condition-skill evals/controls/terse.md --trials 2 --budget-usd 8 --output evals/results/responses.jsonl
```

## Regex checks

A case may carry `checks`: `must_include` and `must_not_include` regular expressions (case-insensitive search) with known-good `pass` and known-bad `fail` example answers. `validate` rejects a case whose checks reject a good example or accept a bad one, so a broken check fails before a paid run. Score recorded responses without a judge:

```bash
python3 scripts/run_evals.py checks evals/results/responses.jsonl
```

A case with `target_words` asks for a length ('about 400 words'), and the summary gives each response's length in words as a share of it (`target_length`, `mean_target_share`), so a draft that falls well short of a requested length shows. The same responses always get the same result, so checks can be fixed and rescored without new calls. A check sees only what its patterns test: read the failures before citing a pass rate, and keep the blind judge for everything else. The summary does not enforce matching coverage, so confirm that conditions share the same checked case and trial pairs before comparing their rates.

## Compare whole setups

To compare installed setups instead of injected skill text, give each setup its own runner in a runner config (for example `--plugin-dir`, `--add-dir` and `--append-system-prompt-file` for Claude Code), then run each with `--no-injection` so the task is sent unchanged:

```bash
python3 scripts/run_evals.py run --runner-config setups.json --runner setup-current   --condition comparator --no-injection --trials 2 --budget-usd 8 --output evals/results/setups.jsonl
python3 scripts/run_evals.py run --runner-config setups.json --runner setup-laconic   --condition candidate --no-injection --trials 2 --budget-usd 8 --output evals/results/setups.jsonl
```

Judge with `--conditions candidate comparator`, then score and measure with `--reference comparator`, which compares the candidate with the comparator in place of a baseline. Give every setup the same tools and the same reachable files, or the comparison measures access rather than instructions.

## Humanising benchmark

`humanise-cases.jsonl` holds 20 AI-written texts to humanise for an Australian reader, each with regex checks for the facts that must survive and the patterns that must go. To compare humanisers, install each as a Claude Code plugin in its own runner (`--plugin-dir`, `--add-dir`, `--tools Skill,Read` and an `--append-system-prompt-file` naming the skill to use), run each with `--no-injection` as one of the 3 conditions, then judge all 3 together:

```bash
python3 scripts/run_evals.py run --cases evals/humanise-cases.jsonl --runner-config setups.json --runner hum-laconic --condition candidate --no-injection --trials 2 --budget-usd 12 --output evals/results/hum-laconic.jsonl
python3 scripts/judge.py --runner claude --cases evals/humanise-cases.jsonl --conditions baseline candidate comparator --responses evals/results/responses.jsonl --output evals/results/scores.jsonl
python3 scripts/judge.py --runner claude --rubric evals/humanise-rubric-neutral.md --cases evals/humanise-cases.jsonl --conditions baseline candidate comparator --responses evals/results/responses.jsonl --output evals/results/scores-neutral.jsonl
python3 scripts/run_evals.py checks evals/results/responses.jsonl --cases evals/humanise-cases.jsonl
python3 evals/humanise_metrics.py evals/results/responses.jsonl --avoid path/to/avoid-ai-writing/bin/avoid-ai-writing.js --dslop
```

`humanise-rubric-neutral.md` is the rubric without Laconic's house mechanics, so a humaniser is not marked down for house rules it never claims to follow. `humanise_metrics.py` reports each condition's length as a share of the source, `laconic-check` lint findings, its `bold-label` runs and `preserve` drops, and optionally the avoid-ai-writing detector and dslop (with negation findings off), beside the same measures on the source texts. Read `preserve` drops before citing them: removing a staged contrast drops its 'not' legitimately. It calls whichever `laconic-check` is on the PATH, so record that build with published counts: later versions add rules. A checker that exits with an error raises rather than counting as zero findings.

## Fact-survival benchmark

This tests the core promise directly: does a shorter rewrite keep what the reader needs? `fact-ledgers.jsonl` lists 99 facts and 33 qualifiers (hedges, conditions, limits and commitments) from the 20 humanising sources. It was written from the sources alone, before any rewrite existed. `fact-cases.jsonl` gives every setup the same instruction, which names none of them.

```bash
python3 evals/fact_setups.py evals/results/facts
python3 evals/fact_setups.py --preflight evals/results/facts
python3 evals/fact_survival.py calibrate --overhead evals/results/facts/overhead.jsonl
for pair in laconic-prose:6 laconic-session:6 humanizer:7 avoid:15 stop-slop:4 caveman:4 concise-control:2 no-skill:2; do
  setup=${pair%:*}; budget=${pair#*:}
  python3 evals/fact_survival.py spend evals/results/facts --cap 48 || break
  python3 scripts/run_evals.py run --cases evals/fact-cases.jsonl --runner-config evals/results/facts/runners.json --runner "$setup" --condition candidate --no-injection --trials 2 --jobs 4 --budget-usd "$budget" --output "evals/results/facts/run-$setup.jsonl"
done
cat evals/results/facts/run-*.jsonl > evals/results/facts/responses.jsonl
python3 evals/fact_survival.py spend evals/results/facts --cap 48 && python3 evals/fact_survival.py judge evals/results/facts/responses.jsonl --output evals/results/facts/scores.jsonl
python3 evals/fact_survival.py spend evals/results/facts --cap 60
python3 evals/fact_survival.py report evals/results/facts/scores.jsonl evals/results/facts/responses.jsonl --cases evals/fact-cases.jsonl --reference laconic-prose
```

The plan is US$46 of generation, about US$2 of preflight and calibration, and US$12 of judging: US$60. `spend` adds up the reported cost of every stage and stops the next one once the US$48 mark is reached. Budgets are soft thresholds, so calls already running can pass them; `spend` reports any overrun. A setup that stops early leaves `report` refusing to compare, because every setup must cover every text and trial exactly once. Preflight and calibration must pass before any result is cited.

`--preflight` makes one call per skill setup and fails unless the setup called its own skill; read the replies it prints too. Caveman's rules exempt third-party messages and other text persisted outside chat, so check that it applied its style to this task. `calibrate` runs the judge on the hand-labelled rewrites in `fact-calibration.jsonl`: a verbatim copy, a faithful paraphrase, a removed hedge, omitted facts, a contradicted figure, a fabricated claim and an empty rewrite. Read its disagreements before judging.

The judge sees the source, one rewrite and the ledger, never the setup's name. It marks each item kept, hardened (present but stronger or wider than the source) or dropped, and must quote the rewrite for kept and hardened. A quote not found in the rewrite counts as dropped; the report counts those. It also lists passages that state material facts the source does not support, again verified as quotes. `report` gives survival by text, facts and qualifiers separately, invented passages, length against the source and items `laconic-check preserve` lists as added, with paired 95% intervals on survival by text that resample whole texts.

Survival alone proves little, since copying the source scores 100%. Decided before any run: a setup is better than another on this benchmark only if its survival interval excludes 0, it is no longer and it invents no more. Anything else is reported as a trade-off or a tie. The setups and the ledgers come from Laconic's author, and the ledgers had one independent review, so read them before relying on a margin. Report the result whichever way it falls.

## Fact-audit agreement

This measures `laconic-check preserve` against a fact checker Laconic's author did not write, on rewrites Laconic did not make. The [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) publishes the AI-written texts of each monthly cycle and each commercial humaniser's rewrite of them. The paper [Humanized but Still True?](https://arxiv.org/abs/2611.00359) judged each number, name and quotation of 3 or more words in the 2026-10 cycle's rewrites as kept or altered, and published the judgements with its checker, [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit). `fact_audit_agreement.py` downloads both at pinned revisions into `evals/results/fact-audit`, which Git ignores, runs `preserve` on each source and rewrite, and prints the table below and the documents where the 2 checkers differ on numbers or quotations:

```bash
cargo build --release --manifest-path tools/laconic-check/Cargo.toml
python3 evals/fact_audit_agreement.py --laconic-check tools/laconic-check/target/release/laconic-check
```

Measured on 13 November 2026 over the 363 rewrites the paper judged (11 commercial humanisers, 33 texts each), benchmark revision `6306b2043736dd958cf39ee1d9a64956c54dd78a` and `humanizer-fact-audit` commit `e665dda0506a64c834886c94d8f328d447119a07`. A document counts under a kind when the paper marks at least one of its units of that kind altered:

| Documents the paper marks as | Documents | laconic-check 0.4.0 | Unreleased |
| --- | --- | --- | --- |
| Any unit altered | 105 | 105 fail | 105 fail |
| Numbers altered | 47 | 44 report a dropped number or quantity | 43 |
| Quotations of 3 or more words altered | 58 | 58 report a dropped quotation | 58 |
| Names altered | 28 | 20 list a name to check | 20 |
| Every unit kept | 258 | 17 pass; 224 fail only on meaning words; 17 fail on a number, quantity or quotation | 23 pass; 217; 18 |

Both versions fail every document the paper marks altered. In each of the 4 documents whose altered number the unreleased version misses, the altered unit is one 'first' of several and the rewrite still has a 'first', so `preserve`, which reports a value only when the edit has none of it, cannot see it. 0.4.0 reported one of the 4, cleverhumanizer's s17, only because it read the source's bold list markers ('**1.**') as numbers the rewrite dropped. The unreleased version reads them as layout, as it does in 5 documents the paper marks fully kept. It also reports 'first' when a rewrite drops every one, as 5 other kept documents do ('safety first', 'at first').

The paper's judgements are the reference here, so the table measures agreement with them, not the accuracy of either checker. `preserve` never fails on a name, and it also fails on words such as 'not', 'must' and 'before', which the paper does not judge. The texts are CC BY 4.0 (AI Humanizer Benchmark); `humanizer-fact-audit` is by Ziyuan Wang, Genliang Zhu and Xiaobo Chen, its code MIT and its data CC BY 4.0. Nothing from either is stored in this repository.

## Agent cost benchmark

`evals/agent_bench.py` measures whether a rule set changes how an agent works, not just how it writes. It builds 5 small Python repositories with one failing test each and asks Claude Code to fix them with tools limited to Read, Edit, Glob, Grep and `python -m unittest`. It records whether the tests pass afterwards, plus turns, tokens, cost and permission denials. It compares 2 rule files:

```bash
python3 evals/agent_bench.py rules-a.md rules-b.md 2 evals/results/agent-bench.jsonl
```

Everything runs in temporary folders, and each call is capped with `--max-budget-usd 1.5`.

## Measure

Aggregate token usage, reported cost and response length from the completed responses file:

```bash
python3 scripts/run_evals.py measure evals/results/responses.jsonl
```

The summary reports input/output token totals, reported generation cost, stored
response length and candidate-minus-baseline deltas. Positive deltas mean more
usage or cost; negative deltas mean less. Read these alongside quality scores.

Comparisons require the same runner and identical `(case_id, trial)` coverage,
without duplicates. If rows include `model`, every row must name the same model.
`run` records `--model` (or Codex's `-m`) for a direct Claude or Codex command,
including `--model=value`. Unknown wrappers, commands without a model flag and
older runner output leave `model: null`, which means model
comparability is unverified. Check the original model/CLI settings yourself;
a matching runner alias alone does not establish the same model or configuration.

Missing costs or token counts produce `null` totals and deltas, not zero. Invalid
negative, boolean, non-finite or fractional token counts are rejected. A zero
baseline has no meaningful percentage change, so that percentage is `null`.
Claude input totals include cache creation/read tokens; Codex cached input is
already part of its input count and is not added again.

This measures recorded generation rows, not total provider billing: judge costs
and unrecorded failed/retried calls are excluded. Scenario captures currently omit
token usage, and their response length includes transcript JSON and user prompts.
The command is read-only and makes no model calls.

## Judge and score

`scripts/judge.py` grades the responses and writes the score rows for you:

```bash
python3 scripts/judge.py \
  --runner claude \
  --responses evals/results/responses.jsonl \
  --output evals/results/scores.jsonl
```

It groups responses by `(case_id, trial)` and grades every condition for a case
in one call, so the conditions are compared against each other rather than
scored in isolation. Blinding is structural, not a convention the grader is
asked to respect: each condition is relabelled `A`/`B`/`C` before the prompt is
built, and the label order is permuted per group, so position carries no signal.
The permutation comes from a digest of the group key rather than a random
source, so a resumed run reproduces the labels it used the first time.

Only the region of `rubric.md` between the `<!-- judge:begin -->` and
`<!-- judge:end -->` markers reaches the grader. The release-gate rules below
those markers name the conditions, and sending them to a blind grader would
leak the vocabulary the blinding exists to hide. Keep anything condition-identifying
outside that block.

Each response row records the date it was generated, and the judge sees that date beside the response. A model reads today's date from its environment. Without the recorded date, a run that crosses midnight or a judge run on a later day would mark a correct 'today' or 'yesterday' as a factual error. Rows from older runs have no date and are judged as before.

Runs are resumable the same way generation is: groups already present in the
output file are skipped. A group missing a condition cannot be scored, because the
conditions would no longer be judged on identical rows. It is reported on
stderr and left out rather than silently dropped.

Before citing blocker counts, check the judge against hand labels. `judge-calibration.jsonl` holds 10 responses to existing cases, each labelled blocker or not: a correct and a wrong product, a destructive command with and without confirmation, a list missing Tasmania, a US spelling left in an edit, a status note with and without invented verification, a brief reply and a long but sound recommendation. `--calibrate` judges each alone with the same prompt and reports agreement:

```bash
python3 scripts/judge.py --runner claude --calibrate evals/judge-calibration.jsonl
```

On 1 November 2026 (Claude Code 2.1.312, `claude-opus-5-5`, 1 trial) it agreed on all 10, at a reported US$0.34. Read any disagreement before judging; the labels cover the blocker field only, not the 1 to 5 scores.

`holdout-cases.jsonl` holds 10 frozen cases written on 1 November 2026 and never used to choose rule wording. Use them for the next public comparison (`--cases evals/holdout-cases.jsonl`), and replace the whole set rather than editing it after seeing results; a test pins its contents.

Judging by hand instead is still supported: blind the `condition` field
yourself and write one JSON object per response with these fields.

```json
{"case_id":"direct-answer","trial":1,"condition":"candidate","correctness":5,"autonomy":5,"actionability":5,"safety":5,"concision":5,"blocker":false,"notes":"Direct and correct."}
```

Either way, apply the release gate:

```bash
python3 scripts/run_evals.py score evals/results/scores.jsonl
```

Record the exact CLI and model versions with published results, including measured token and cost numbers. Do not compare conditions produced with different cases, models, trial counts or rubrics.
