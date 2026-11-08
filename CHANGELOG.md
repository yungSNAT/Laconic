# Changelog

This file follows the shape of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and is built from `git log`. The plugin and `laconic-check` are versioned separately. Dates are commit dates. Numbers in brackets are pull requests.

The manifests still declare plugin 0.5.0, so a build from `main` carries that version plus the Unreleased changes. `v0.4.1`, `laconic-check-v0.1.0`, `laconic-check-v0.2.0` and `laconic-check-v0.3.0` are tagged; plugin 0.5.0 has no tag or release.

## [Unreleased]

### Plugin: added

- Warn when always-on mode cannot locate the plugin (#18).
- Run the full Python test suite and eval validation in CI (#19).
- Run eval model calls in parallel with `--jobs` (#11).
- Benchmark Laconic against `humanizer` and `avoid-ai-writing` (#12).
- A revisit trigger for each shortcut marker (#24).
- A README badge row, CodeQL, `CITATION.cff` and `llms.txt` (#37).
- Calibrate the eval judge with `judge.py --calibrate` against 10 hand-labelled responses; add a social post venue and 10 frozen holdout cases (#55).
- Report each response's length as a share of its case's `target_words` in `run_evals.py checks`, with 2 drafting cases that ask for about 400 and 150 words.
- `allowed-tools` in both skills pre-approves the `laconic-check` checks that read only the prose they are given: `laconic-check lint --warn -` on standard input and the MCP `lint` tool, plus the MCP `preserve` tool for `laconic-prose`. A `laconic-check` command that names files still asks. Claude Code clears the grant at the next message; tools not listed keep their usual permissions.

### Plugin: changed

- Make Laconic a full replacement for stop-slop and No AI Slop.
- Forbid typical-range estimates in place of evidence.
- Move the compact rule's mechanics into a before-sending checklist (#34).
- Relicense under FSL-1.1-ALv2 (#25), replacing the Apache-2.0 licence adopted the same day (#22).
- Point repository links at EquityBot (#31).
- Keep figurative language in fiction, judging only its density (#55).
- State the fact-keeping promise as an aim until the fact-survival benchmark reports, and name 'humanizer' and 'no AI slop' in the README.
- Date the Style Manual rules with a 'rules last checked' line.

### Plugin: fixed

- Validate eval inputs and record requested model metadata (#20).
- Make hook trimming linear (#29).
- Remove the copy of the Australian Government Style Manual and correct the claim that it is CC BY 4.0: the manual is not openly licensed. The references now send agents to stylemanual.gov.au for rules they do not cover (#62).

### Plugin: security

- Pin CI agent installs and disable lifecycle scripts (#28).
- Keep scenario prompts out of Claude CLI arguments and reject model values that could become options (#30).

### laconic-check: added

- Advise, without failing `lint`, on stacked hedges such as 'could potentially' and 'may possibly'.
- Flag a space in '5 %' and '$ 27.99', the decade form '1980's' and the date form '21st of September'.
- Advise, without failing `lint`, on 3 or more bullets in a row that open with a bold label; `humanise_metrics.py` reports these runs.
- A rule reference, `docs/rules.md`, generated from one rule table: what each rule flags, examples, its Style Manual section and its rate on human prose, with a Style Manual coverage table. The usage text and the MCP `lint` description link to it, and a false-positive issue form links back.
- A seed for `weekday`; the seed check now covers every rule in the table, each example flags its rule on its own, and a finding can name only a rule in the table.

### laconic-check: fixed

- Stop reading 'may' after an ordinal as the month in 'the 13th may be rooted', found by a run over 150 human articles.
- End `preserve`'s report with both lengths in words and the edit's as a share of the original's.

## [laconic-check 0.3.0] - 2026-11-04

### Added

- Serve `lint` and `preserve` as MCP tools with `laconic-check mcp`, with nothing else to install (#65).
- Flag hidden Unicode tag runs and bidirectional embeddings, overrides and isolates as residue, leaving the England, Scotland and Wales flags alone (#66).

### Deprecated

- The Node MCP server, `tools/laconic-check/mcp/server.mjs`, kept for 0.2.0 binaries; it will be removed in a later release (#65).

## [laconic-check 0.2.0] - 2026-11-02

### Added

- Flag staged contrasts and reveal questions (#10).
- Flag unnecessary hyphens (#13).
- Report protected items an edit added, failing on them only with `--strict` (#26).
- Report dropped quantifiers, modals and bounds in `preserve` (#35).
- Report changed sizes, units and dated months, and dropped commitments, hedges, conditions and order words, in `preserve`; list possibly lost names as advice (#42). `preserve` counts from earlier runs are not comparable.
- Advise, without failing `lint`, on clause dashes in 2 or more sentences of one paragraph; flag the stock phrases 'innovative' and 'practical solutions' and timeless openers (#55).
- Fuzz `lint` and `preserve` weekly in CI, through a new library target (#60).

### Changed

- Relicense under FSL-1.1-ALv2 (#25); 0.1.0 was MIT.
- Calibrate the human-prose test on 3 Productivity Commission reports (CC BY 3.0 AU) instead of the Style Manual copy, and make `gen_au_words.py` take the manual's text as an argument (#62).

### Fixed

- Catch prose patterns across wrapped Markdown lines (#16).
- Stop date checks from joining separate clauses (#17).
- Make dictionary parsing linear (#29).
- Stop flagging commas that join 2 clauses as serial commas (#36).
- Stop suggesting -ise forms for words built on size, seize and prize, such as downsizing, outsized, resized, prized and seizing (#62).

## [laconic-check 0.1.0] - 2026-10-20

### Added

- The `lint` and `preserve` commands (#2).
- The MCP server and seeded-fault measurement.
- Stock-phrase linting (#5).
- Flags for mismatched weekdays (#6).
- Prebuilt binaries published on a release tag (#8).

### Changed

- Make the serial-comma rule precise enough to act on (#7).

## [plugin 0.5.0] - 2026-10-19

### Changed

- Turn the i-have-adhd fork into Laconic: concise, answer-first output in correct Australian English, with the No AI Slop prose skill merged into the response rules.
- Rename every adapter, hook, flag, manifest, test and doc to `laconic`.

### Removed

- The stale translations, upstream eval results and logo.

For versions before 0.5.0, see the history from tag `v0.4.1`.

[Unreleased]: https://github.com/yungSNAT/laconic/commits/main
[laconic-check 0.3.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.3.0
[laconic-check 0.2.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.2.0
[laconic-check 0.1.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.1.0
[plugin 0.5.0]: https://github.com/yungSNAT/laconic/commit/3eb0a03
