# Edit and audit modes

Read this in full before editing a supplied draft or auditing prose. Apply the shared meaning, voice and workflow rules in [SKILL.md](../SKILL.md).

**Edit.** For a supplied draft, make the minimum effective edit and return the edited text with a short note of what changed and any open evidence gaps, unless the user asks for text only. Keep the note to a few lines; it is for the writer, not a changelog. Leave a clean draft unchanged and say so in one line. When cuts leave a piece that no longer does its job (a post with no point, an email with no answer), rebuild it from the facts the draft supplies and say which claims need evidence before it goes out. Never keep a pointer to content that is not supplied ('Details below', 'See how it works'). When something you write for the writer asserts new knowledge ('I don't know yet', a date, a cause), flag it for them to confirm.

**Detect.** For an audit, scan or review, quote each weak span, name the pattern, explain the problem in context and suggest a local fix. Order findings by severity: unsupported, misleading or contradictory claims first (weasel attribution, guesses stated as fact, a status the content contradicts), then quick pass tells, then judgement calls. Leave the draft and files unchanged. Score only when explicitly requested, using [scoring.md](scoring.md). Never infer authorship from style: a style or substance match is evidence of a defect to inspect, never evidence about who or what wrote the text. If no material problem appears, say so.

**Frozen-findings repair.** When pairing `laconic-check` with an edit pass, separate detection from repair:

1. Run `laconic-check lint --format json --warn [--profile <name>] <file>` to freeze the exact findings list (`file`, `line`, `column`, `rule`, `severity`, `confidence`, `message`).
2. Pass that JSON payload into the repair pass as a fixed worklist. Consume the exact finding records without re-deriving or inventing additional findings during repair.
3. Address high-confidence blocking findings first, then medium-confidence and low-confidence advisories where context confirms a real defect. Respect author idioms exempted under `[protect]` in `laconic.toml` or `.laconic-check`.
4. Verify meaning survival with `laconic-check preserve <original> <edited>` and record any finding intentionally kept with its reason.
