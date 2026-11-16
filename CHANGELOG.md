# Changelog

This file follows the shape of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and is built from `git log`. The plugin and `laconic-check` are versioned separately. Dates are commit dates. Numbers in brackets are pull requests.

The manifests other than Claude Code's still declare plugin 0.5.0, so a build from `main` carries that version plus the Unreleased changes. `v0.4.1`, `laconic-check-v0.1.0`, `laconic-check-v0.2.0`, `laconic-check-v0.3.0`, `laconic-check-v0.4.0` and `laconic-check-v0.5.0` are tagged; plugin 0.5.0 has no tag or release.

## [Unreleased]

### Plugin: added

- A skills-only edition for the Claude plugin directory: `laconic-tools build-plugin-dist` builds it from `main` without the hooks, tools or MCP server, refuses any file the directory would refuse from an unverified organisation, and a workflow validates it on pull requests and publishes it to the `claude-plugin` branch at each release, when the version in `package.json` rises.
- A listing icon for the Claude plugin directory, `.claude-plugin/icon.png`, cut from the banner's helmet: the directory build ships it and refuses any other image, or an icon that is not a complete square PNG of 512 to 2048 px under 2 MB.
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

- The GitHub prose reference keeps session IDs, local paths, private host names and internal IDs out of posted text, and points the reader to the public artefact instead.
- Make Laconic a full replacement for stop-slop and No AI Slop.
- Forbid typical-range estimates in place of evidence.
- Move the compact rule's mechanics into a before-sending checklist (#34).
- Relicense under FSL-1.1-ALv2 (#25), replacing the Apache-2.0 licence adopted the same day (#22).
- Point repository links at EquityBot (#31).
- Keep figurative language in fiction, judging only its density (#55).
- State the fact-keeping promise as an aim until the fact-survival benchmark reports, and name 'humanizer' and 'no AI slop' in the README.
- Date the Style Manual rules with a 'rules last checked' line.
- Rule 8 sizes work by what is left and gives a time only with a measured basis; its example, 'about 15 minutes if the tests cover this', was a time with no measured basis.

### Plugin: fixed

- The always-on banner names 'normal mode' beside 'stop laconic', as the skill turns off on either, and INSTALL.md shows how to run the shell and PowerShell versions of the hook where Node is missing.
- Claude Code installs follow each commit: `.claude-plugin/plugin.json` no longer declares a version, so `claude plugin update` stops skipping changes made since 0.5.0.
- Validate eval inputs and record requested model metadata (#20).
- Make hook trimming linear (#29).
- Remove the copy of the Australian Government Style Manual and correct the claim that it is CC BY 4.0: the manual is not openly licensed. The references now send agents to stylemanual.gov.au for rules they do not cover (#62).

### Plugin: security

- Pin CI agent installs and disable lifecycle scripts (#28).
- Keep scenario prompts out of Claude CLI arguments and reject model values that could become options (#30).

### laconic-check: added

- `preserve` lists a possible reversal to check on a `check reversal:` line, which never fails: a sentence of the edit that states without a negation the word a sentence of the original negated, when the 2 share at least half of the original's content words. The original's negations are 'not', 'never', 'cannot', 'no', 'none' and words ending in 'n't', and 'no more seats' negates 'seats'. 'not just' and 'not only' add rather than negate, and an edit sentence with its own negation or 'rather than' is not a reversal.
- `preserve` reports a dropped or added day name, such as 'Thursday', and reads a plural such as 'Sundays' as its day.
- `laconic-check lsp`, a language server over standard input and output: it publishes `lint`'s findings for each open Markdown document as diagnostics, warnings or hints for advisory rules, each linked to its rule, with the settings of the `.laconic-check` file in the innermost workspace folder that holds the document. A project file it cannot read or parse is one error diagnostic. Positions are in UTF-8 when the client offers it and UTF-16 otherwise.
- An `accept` line in `.laconic-check` lists words of letters only that `us-spelling` does not flag, each matched as a whole word in the same case.
- `lint --sarif` prints a SARIF 2.1.0 log for GitHub code scanning: every rule with its description and link, and each finding at level `warning`, or `note` if advisory, located by the file's absolute `file` URI and a one-character region in UTF-16 code units. The exit status is unchanged; `--sarif` cannot be combined with `--json`.
- The MCP `preserve` tool takes an optional boolean `loose`, which does what `preserve --loose` does; without it, or with `false`, a lost word such as 'often' is still reported as dropped. Any other value is an error.
- `preserve --loose` lists a lost word of frequency, quantity, ability, prediction, cause or hedging, such as 'often', 'some', 'can', 'will', 'because' or 'likely', on a `check meaning word:` line that does not fail; without it, these losses fail as before. On the fact audit, 64 of the 258 rewrites the paper marks fully kept pass with `--loose` (23 without), and all 105 it marks altered still fail. `evals/fact_audit_agreement.py --loose` measures it.

### laconic-check: changed

- Builds need Rust 1.100 or later: `rust-toolchain.toml` pins 1.100.0 for every crate in the repository, and each crate declares `rust-version = "1.100"`.
- Release builds use link-time optimisation and one codegen unit, so the Linux binary is about a fifth smaller and starts slightly faster (#119).
- The Action's `args` input documents that it splits at spaces, tabs and new lines and keeps quotation marks as written, so an option's value cannot contain a space, and that a project file whose path has one can go in the working directory as `.laconic-check`.

### laconic-check: fixed

- `preserve` no longer reports '00' as dropped when an edit writes a time on the hour without its minutes, such as '5:00 p.m.' as '5pm'. A colon number is a time only after a word such as 'at' or 'until', before 'am' or 'pm', or after 'to' or 'and' that follows a time, so a ratio such as '5:30' stays 2 numbers.
- `lint` reads back only the few words that could spell a parenthesised acronym, not the whole paragraph before it, so many definitions in one paragraph take linear time: 4,000 in a debug build took 8.1 seconds and now take about as long as 8 times 500.
- `lint` no longer reads an apostrophe that elides the start of a year or word ('90s, '22, 'em, 'til, 'tis, 'twas) as an opening quotation mark, which hid the prose up to the next closing apostrophe: in `In the '90s, organized teams' work`, 'organized' is now flagged.
- The MCP `lint` tool's description names every rule, listed from the rule table, and `preserve`'s names quotations, which it has always checked.
- `preserve` reads a range written with a spaced dash ('30 – 60 grams') or a hyphen that waits for the upper bound's unit ('a 45- to 60-minute ride') in that unit, and compares a quotation set as its own sentence without its new capital or question mark, so 'can I afford to publish this book' to 'Can I afford to publish this book?' no longer reports a dropped quotation. Each was a false alarm on a rewrite the fact audit marks fully kept.
- `preserve` looks each name up in a set of the edit's runs of 2 and 3 words instead of searching the whole edit for it, so 16 times the names no longer takes about 240 times as long in a debug build; 32,000 names took 1.3 seconds and now take 0.7 in a release build. A name now counts as kept only as whole words: 'Jane Smith' is kept in '(Jane Smith)', 'Jane Smith's' and 'Jane Smith-Jones', but no longer in 'Jane Smithson'. The fact audit's results are unchanged.
- `lint` no longer rescans a whitespace-free token for each word it holds, so minified text takes linear time: 80,000 characters of 'a,' took about 6 seconds and twice the text 4 times as long (#119).
- `preserve` compares numbers and quantities through hash sets, so a text of many numbers takes linear time: 16,000 numbers took about 1 second and twice as many 5 times as long.
- `lint` checks the body of a GitHub alert, such as `> [!NOTE]`, as prose; it skipped every block quotation (#111).
- `lint` lints the remaining files after one it cannot read, then exits 2; it stopped at that file, so `--json` printed nothing (#112).
- The Action refuses `--sarif` in `args`, as it refuses `--json`: with a release that has `--sarif`, it would log the SARIF and annotate no finding.
- The Action lints a large repository in several `laconic-check` calls, so the Windows command-line limit no longer stops it. It counts the failing findings of all the calls against `--max-findings`, taken from `args` or the project file as `lint` takes it, so no call has to hold every file with a failing finding. A path named twice in `files` is linted once.
- `preserve` reads 'dollars', 'euros' and 'yen' after a number, or after both bounds of a range, as its currency, so '$30 billion' to '30 billion dollars' and '$5-10' to '5-10 dollars' no longer fail, and reads every mark in the Unicode Currency Symbols block, so '₹500' to '₩500' now fails. 'Pounds' after a number is money when either text writes '£' with that number, so '£5' to '5 pounds' no longer fails or lists an added '5 lb'. Report lines change with it: a dropped '30 dollars' is now 'dropped number: $30'.
- `preserve` reads both bounds of a range in its unit, whether written with a hyphen, an en dash, 'to', 'and' or 'or', so '45 to 60 minutes' to '45-60 minutes' or '45–60 minutes' no longer reports 'dropped quantity: 60 minute', '30 to 60 grams' to '30-60 grams' no longer reports '60 g', and '5-10 kg' to '5 to 10 g' now fails.

## [laconic-check 0.5.0] - 2026-11-13

### Added

- `preserve` lists names of 2 or 3 capitalised words that an edit added on a new `check added name:` line, which never fails, and reads ms, mg, g, L, mL, °C and °F, and 'degrees Celsius' and 'degrees Fahrenheit', as units (#105).

### Changed

- `preserve` keeps a number's sign, accounting brackets around money or a comma-grouped figure, such as '($250)' or '(1,200)', and currency, so '-3%' to '3%', '(1,200)' to '1,200' and '$500' to '£500' now fail. Its report lines change with them: a dropped '$1,200' was 'dropped number: 1,200' and is now 'dropped number: $1,200', and a bracketed '(1,200)' is '-1,200', so a script that parses the report sees new values (#105).

### Fixed

- `--version` and `--help` print to standard output and exit 0; they printed usage to standard error and exited 2 (#104).
- `lint` reads a file whose frontmatter never closes, and prose on a line that opens with an HTML tag, such as `<sub>Note</sub>`; it skipped both without saying so. Attribute values inside tags no longer give findings (#109).
- `preserve` reads every negative contraction, such as 'didn't' and 'wasn't', as 'not', keeps the modal in 'can't' and 'won't', and skips bold ordered list markers such as '**1.**' (#105).

## [laconic-check 0.4.0] - 2026-11-12

### Added

- `preserve` checks quotations word for word, skipping elisions and reading a quotation after a colon or dash (#81).

- Advise, without failing `lint`, on stacked hedges such as 'could potentially' and 'may possibly'.
- `laconic-tools calibrate` counts each rule's findings on 150 human-written articles from jenna-russell/human_detectors, split into calibration and held-out halves. The rule reference shows the held-out rate, and a test keeps each new advisory under 5% of the held-out articles.
- Advise, without failing `lint`, on a compound written both closed and hyphenated in one document (`hyphen-consistency`) and on an acronym used before the parenthesised definition that introduces it (`acronym-before-definition`). Neither flags any of the 150 outside human articles in the rule reference.
- Flag a space in '5 %' and '$ 27.99', the decade form '1980's' and the date form '21st of September'.
- Advise, without failing `lint`, on 3 or more bullets in a row that open with a bold label; `humanise_metrics.py` reports these runs.
- Attest each release archive's build provenance, and attach an MCP bundle and Scoop, Homebrew, winget and MCP Registry manifests to each release, written by `laconic-tools package-manifests`.
- A rule reference, `docs/rules.md`, generated from one rule table: what each rule flags, examples, its Style Manual section and its rate on human prose, with a Style Manual coverage table. The usage text and the MCP `lint` description link to it, and a false-positive issue form links back.
- A seed for `weekday`; the seed check now covers every rule in the table, each example flags its rule on its own, and a finding can name only a rule in the table.
- `lint` options for use as a gate: `--json`, `--disable` and `--only` for rules, `--max-findings`, `--commit-msg` for Git commit messages and `--config`, with a `.laconic-check` project file. A `<!-- laconic-check-disable-next-line rule -->` comment silences the named rules on the next line, and an `unused-suppression` advisory flags one that silences nothing. An unknown option, rule name or count exits 2. README lists the exit statuses.
- Pre-commit hooks for Markdown files and commit messages, and a composite GitHub Action that downloads a laconic-check release, checks its SHA-256 and annotates each finding.

### Fixed

- Link the C runtime statically in the Windows binary, which needed the VC++ redistributable.
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
[laconic-check 0.5.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.5.0
[laconic-check 0.4.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.4.0
[laconic-check 0.3.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.3.0
[laconic-check 0.2.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.2.0
[laconic-check 0.1.0]: https://github.com/yungSNAT/laconic/releases/tag/laconic-check-v0.1.0
[plugin 0.5.0]: https://github.com/yungSNAT/laconic/commit/3eb0a03
