# Laconic for Vale

`vale/Laconic` is a [Vale](https://vale.sh) style generated from `laconic-check`'s rule table and word lists, for a project that already runs Vale. Each rule is named after the `lint` rule it follows and links to that rule in [docs/rules.md](../docs/rules.md). Advisory rules are suggestions and the rest warnings. The files are generated: change `laconic-check` and run `cargo run --manifest-path tools/laconic-tools/Cargo.toml -- vale-style`, never edit them by hand.

Vale finds fewer faults than `lint`: a regular expression cannot see clauses, paragraphs or the whole document, so some rules are missing and others differ. `lint` stays the reference, and `preserve`'s fact checks have no Vale form.

## Use

Each `laconic-check` release after 0.5.0 attaches the style as `Laconic.zip`, with its SHA-256 checksum (on `main`, not yet in a release). In `.vale.ini`, with the release's version:

```ini
StylesPath = styles
MinAlertLevel = suggestion
Packages = https://github.com/yungSNAT/Laconic/releases/download/laconic-check-vVERSION/Laconic.zip

[*.md]
BasedOnStyles = Laconic
```

Then run `vale sync` and `vale <files>`. To use the style from a checkout instead, copy `vale/Laconic` into your `StylesPath`.

## Rules

Generated, each with the `lint` rule it follows:

| Vale rule | What it flags | How it differs from `lint` |
| --- | --- | --- |
| `us-spelling` | US spellings with an Australian form, in lower case | A capitalised word is skipped, since a pattern cannot tell a proper noun from a sentence start; link text that names its own target, such as `[humanizer](https://github.com/blader/humanizer)`, is flagged |
| `us-spelling-ize` | Other -ize and -yze forms of 6 letters or more (part of `us-spelling`) | Stems such as 'size' and 'prize' do not match |
| `word-choice` | 'while' and 'among' for 'whilst' and 'amongst' | 'a while', 'worth your while' and 'while away' are skipped, as in `lint` |
| `double-quotes` | Straight and curly double quotation marks in prose | |
| `symbol-space` | A space before % or after a currency symbol | |
| `em-dash` | Em dashes | |
| `hyphen` | Hyphenated forms the house style writes open or closed | |
| `number-word` | Numbers 2 to 20 written as words | A word at a line start, after '.', '!', '?' or ':', or in a compound such as 'forty-five' is skipped |
| `date` | Ordinals, a comma before the year, the month before the day and a decade with an apostrophe | |
| `time` | Times not written as 9:30 am or 9 am | |
| `latin` | `e.g.`, `i.e.`, `eg` and `ie` | |
| `closer` | Chat closers such as 'hope this helps' | |
| `stock-phrase` | Stock phrases | |
| `stacked-hedge` | Stacked hedges such as 'could potentially' | |
| `staged-contrast` | 'Not X, it's Y' contrasts | Only the openers and turns `lint` lists, within one sentence |
| `reveal-question` | Question-and-answer reveals | Only a capitalised reveal question answered by the next sentence |
| `residue` | Chat-tool markers and tracking parameters, and zero-width, bidirectional and tag characters other than the England, Scotland and Wales flags | |

Not generated, and why:

| `lint` rule | Why there is no Vale rule |
| --- | --- |
| `serial-comma` | Telling a list from joined clauses needs `lint`'s clause analysis |
| `clause-dashes` | It counts dashes across the sentences of a paragraph |
| `hyphen-consistency` | It compares every compound in the document |
| `weekday` | It needs the calendar |
| `acronym-before-definition` | It compares uses and definitions across the document |
| `bold-label` | It counts consecutive list items |
| `unused-suppression` | Vale has its own comments for turning rules off |

## Tests

`vale/tests/faults.md` holds one fault a paragraph and `vale/tests/clean.md` none. `faults.expected` is Vale's output for `faults.md`; `laconic-tools`' tests check that `lint` flags the same lines with the same rules, and the `vale-style` workflow checks that Vale still prints it and finds nothing in `clean.md`.
