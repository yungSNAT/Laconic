// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Candidate findings in the original prose of a Markdown file.
//!
//! Code (fenced and inline), URLs, link targets, HTML tags and quoted spans are
//! masked before the rules run, so protected text never produces a finding, except
//! that a double-quoted span in prose is itself a finding.

use std::collections::HashMap;
use std::sync::OnceLock;

/// One lint rule. `docs/rules.md` is generated from this table and the seeded-fault tests;
/// every finding's `RuleId` has an entry here.
pub struct Rule {
    /// The name every finding carries; also its anchor in `docs/rules.md`.
    pub name: &'static str,
    /// Advisory findings report a pattern worth a look but never fail `lint`.
    pub advisory: bool,
    /// The heading in `skills/laconic-prose/references/style-manual-rules.md` whose rules
    /// this one checks in part; `None` for writing patterns and chat-tool residue.
    pub section: Option<&'static str>,
    /// What the rule flags.
    pub checks: &'static str,
}

const fn rule(name: &'static str, section: Option<&'static str>, checks: &'static str) -> Rule {
    Rule {
        name,
        advisory: false,
        section,
        checks,
    }
}

const fn advisory(name: &'static str, section: Option<&'static str>, checks: &'static str) -> Rule {
    Rule {
        name,
        advisory: true,
        section,
        checks,
    }
}

const SPELLING: Option<&str> = Some("Spelling");
const QUOTES: Option<&str> = Some("Percentages, quotes and titles");
const DASHES: Option<&str> = Some("Dashes");
const DATES: Option<&str> = Some("Dates and time");

/// Declares `RuleId` and `RULES` from one list, in the same order, so a finding can only
/// name a rule that has a table entry.
macro_rules! rules {
    ($($id:ident = $rule:expr,)*) => {
        /// A rule `lint` reports; `rule()` gives its entry in `RULES`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum RuleId {
            $($id,)*
        }

        /// Every rule, in the order of the rule reference.
        pub const RULES: &[Rule] = &[$($rule,)*];

        const IDS: &[RuleId] = &[$(RuleId::$id,)*];
    };
}

rules! {
    UsSpelling = rule(
        "us-spelling",
        SPELLING,
        "US spellings that have an Australian form, such as 'color', 'organized' and 'skeptical', and 'percent' in prose.",
    ),
    WordChoice = rule(
        "word-choice",
        SPELLING,
        "'while' and 'among' as a conjunction or preposition, where the house rule writes 'whilst' and 'amongst'. Noun uses such as 'a while' pass.",
    ),
    DoubleQuotes = rule(
        "double-quotes",
        QUOTES,
        "A pair of double quotation marks, straight or curly, around a short span of prose (under 120 characters), where single marks are the default.",
    ),
    SymbolSpace = rule(
        "symbol-space",
        QUOTES,
        "A space before a percentage sign or after a dollar sign, with or without a currency code, as in '5 %', '$ 27.99' or 'A$ 5'.",
    ),
    SerialComma = rule(
        "serial-comma",
        Some("Punctuation"),
        "A possible comma before the final 'and' or 'or' in a list. Keep it only if it prevents a misreading.",
    ),
    EmDash = rule("em-dash", DASHES, "Em dashes in original prose."),
    ClauseDashes = advisory(
        "clause-dashes",
        DASHES,
        "A paragraph with clause dashes (an em dash, or a spaced en dash, hyphen or double hyphen) in 2 or more sentences, where a pair around one aside counts once.",
    ),
    Hyphen = rule(
        "hyphen",
        Some("Hyphens"),
        "Unnecessary hyphens in open set phrases and closed compounds, such as 'open-source' and 'e-mail'.",
    ),
    HyphenConsistency = advisory(
        "hyphen-consistency",
        Some("Hyphens"),
        "A compound written both closed and hyphenated in one document, such as 'healthcare' and 'health-care'. Some such pairs are different words, such as 'resign' and 're-sign'.",
    ),
    NumberWord = rule(
        "number-word",
        Some("Numbers"),
        "Numbers from 2 to 20 written as words where the Style Manual uses numerals.",
    ),
    Date = rule(
        "date",
        DATES,
        "Dates not written as day month year: an ordinal ('19th October'), the month first ('October 19'), a comma before the year and an apostrophe in a decade ('1980's').",
    ),
    Weekday = rule(
        "weekday",
        DATES,
        "A weekday that does not match its date, such as 'Wednesday 22 October 2026'.",
    ),
    Time = rule(
        "time",
        DATES,
        "Times not written as '9:30 am' or '9 am', such as '9am', '9.30am' and '9:30 a.m.'.",
    ),
    Latin = rule(
        "latin",
        Some("Shortened forms"),
        "The Latin abbreviations 'e.g.' and 'i.e.', with or without full stops, where 'for example' and 'that is' are preferred.",
    ),
    AcronymBeforeDefinition = advisory(
        "acronym-before-definition",
        Some("Shortened forms"),
        "An acronym of 2 to 6 capitals used before the parenthesised definition that introduces it later in the document, as in 'The BOM said ... the Bureau of Meteorology (BOM)'.",
    ),
    Closer = rule(
        "closer",
        None,
        "Chatbot closers such as 'hope this helps', 'let me know if' and 'feel free to'.",
    ),
    StockPhrase = rule(
        "stock-phrase",
        None,
        "Stock phrases that rarely carry meaning, such as 'Bottom line:', 'circle back' and 'it's worth noting', and timeless openers such as 'since the dawn of time'.",
    ),
    StackedHedge = advisory(
        "stacked-hedge",
        None,
        "Stacked hedges such as 'could potentially' and 'may possibly'.",
    ),
    BoldLabel = advisory(
        "bold-label",
        None,
        "3 or more bullets in a row that open with a bold label, such as '**Cause:**'.",
    ),
    StagedContrast = rule(
        "staged-contrast",
        None,
        "Staged contrasts: 'it's not X, it's Y' and 'not just X but Y'.",
    ),
    RevealQuestion = rule(
        "reveal-question",
        None,
        "Verbless reveal questions, such as 'The result? A 40% cut.'",
    ),
    Residue = rule(
        "residue",
        None,
        "Chat-tool residue: 4 known ChatGPT citation and tracking markers (`oaicite`, `contentReference`, `utm_source=chatgpt` and `utm_source=openai`), including inside links, and selected invisible characters that can hide or reorder text: zero-width characters, bidirectional controls and runs of Unicode tag characters (except the England, Scotland and Wales flags).",
    ),
    UnusedSuppression = advisory(
        "unused-suppression",
        None,
        "A suppression comment, `<!-- laconic-check-disable-next-line rule -->` on its own line, that silences nothing: it names no rule or an unknown one, or the next non-blank line has no finding of the rule it names. A suppression silences the named rules (comma or space separated) on that line only.",
    ),
}

impl RuleId {
    pub fn rule(self) -> &'static Rule {
        &RULES[self as usize]
    }

    /// The rule with this name, if there is one.
    pub fn named(name: &str) -> Option<RuleId> {
        IDS.iter().copied().find(|id| id.name() == name)
    }

    /// The name every finding prints; also its anchor in `docs/rules.md`.
    pub fn name(self) -> &'static str {
        self.rule().name
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The rule reference; each rule's name is its anchor there.
pub const RULES_URL: &str = "https://github.com/yungSNAT/Laconic/blob/main/docs/rules.md";

#[derive(Debug, PartialEq)]
pub struct Finding {
    pub line: usize,
    pub column: usize,
    pub rule: RuleId,
    pub message: String,
}

const CLOSERS: &[&str] = &[
    "hope this helps",
    "let me know if",
    "feel free to",
    "happy to help",
    "rest assured",
    "great question",
    "don't hesitate",
];

/// Stock phrases from the pattern catalogue that rarely carry meaning: staged
/// openers, emphasis crutches, summary labels and business jargon. Matched as whole
/// words, so 'circle backwards' is not a finding.
const STOCK_PHRASES: &[&str] = &[
    "here's the thing",
    "let me be clear",
    "make no mistake",
    "let that sink in",
    "the uncomfortable truth",
    "here's what nobody tells you",
    "the part everyone misses",
    "it's worth noting",
    "it is worth noting",
    "worth flagging",
    "it's important to note",
    "it is important to note",
    "bottom line:",
    "in short:",
    "in today's world",
    "in a world where",
    "earns its keep",
    "deep dive",
    "circle back",
    "double down",
    "game changer",
    "game-changer",
    "paradigm shift",
    "innovative",
    "practical solutions",
    "since the dawn of time",
    "since time immemorial",
    "has fascinated people",
    "have fascinated people",
];

/// Stacked hedges: a modal followed at once by a second qualifier. Advisory, as human
/// writers stack them too; 'can possibly' is left out, as it is usually emphatic ('how can
/// we possibly').
const STACKED_HEDGES: &[&str] = &[
    "could potentially",
    "could possibly",
    "could perhaps",
    "could conceivably",
    "could arguably",
    "may potentially",
    "may possibly",
    "may perhaps",
    "may conceivably",
    "may arguably",
    "might potentially",
    "might possibly",
    "might perhaps",
    "might conceivably",
    "might arguably",
    "can potentially",
];

/// Unnecessary hyphens: set phrases the house writes open and established compounds
/// it writes closed, each with the suffixes that keep it the same word. A whole-word
/// match only, so 'code-based' and 'data-setting' are not 'codebase' and 'dataset'.
const HYPHENATED_FORMS: &[(&str, &str, &[&str])] = &[
    ("open-source", "open source", &["", "d"]),
    ("plain-language", "plain language", &[""]),
    ("public-sector", "public sector", &[""]),
    ("best-practice", "best practice", &["", "s"]),
    ("e-mail", "email", &["", "s", "ed", "ing"]),
    ("on-line", "online", &[""]),
    ("web-site", "website", &["", "s"]),
    ("code-base", "codebase", &["", "s"]),
    ("data-set", "dataset", &["", "s"]),
    ("time-frame", "timeframe", &["", "s"]),
    ("road-map", "roadmap", &["", "s"]),
    (
        "co-ordinat",
        "coordinat",
        &["e", "es", "ed", "ing", "ion", "or", "ors"],
    ),
];

/// Openers of a staged contrast ('not just a tool, it's a platform'). 'Not only ...
/// but' is left alone: it is ordinary formal English and common in human prose.
const CONTRAST_OPENERS: &[&str] = &[
    "not just ",
    "not simply ",
    "not merely ",
    "isn't just ",
    "aren't just ",
    "wasn't just ",
    "isn't about ",
    "it's not ",
    "it is not ",
    "this isn't ",
    "this is not ",
    "that's not ",
];

/// What follows the turn of a staged contrast, after a comma, semicolon or dash.
const CONTRAST_TURNS: &[&str] = &[
    "it's ",
    "it is ",
    "it was ",
    "they're ",
    "they are ",
    "we're ",
    "this is ",
    "that's ",
];

/// Reveal questions answered in the next sentence ('The result? A 40% cut.'). An explicit
/// list: a general 'short question' test flags ordinary questions ('The dog died?').
const REVEAL_PHRASES: &[&str] = &[
    "the result",
    "the results",
    "the outcome",
    "the upshot",
    "the catch",
    "the cause",
    "the root cause",
    "the reason",
    "the answer",
    "the fix",
    "the problem",
    "the real problem",
    "the difference",
    "the secret",
    "our secret",
    "the twist",
    "the kicker",
    "the takeaway",
    "the truth",
    "the best part",
    "why",
    "how",
    "honestly",
];

/// Auxiliary verbs that open a question; a reveal's answer never starts with one.
const QUESTION_OPENERS: &[&str] = &[
    "do", "does", "did", "is", "are", "was", "were", "can", "could", "will", "would", "should",
    "has", "have", "had", "don't", "doesn't", "didn't", "isn't", "aren't", "wasn't",
];

/// US forms that are not caught by the -ize/-yze rule, with the Australian form.
const US_WORDS: &[(&str, &str)] = &[
    ("color", "colour"),
    ("colors", "colours"),
    ("colored", "coloured"),
    ("center", "centre"),
    ("centers", "centres"),
    ("centered", "centred"),
    ("behavior", "behaviour"),
    ("behaviors", "behaviours"),
    ("behavioral", "behavioural"),
    ("favor", "favour"),
    ("favorite", "favourite"),
    ("favorable", "favourable"),
    ("honor", "honour"),
    ("labor", "labour"),
    ("neighbor", "neighbour"),
    ("labeled", "labelled"),
    ("labeling", "labelling"),
    ("canceled", "cancelled"),
    ("canceling", "cancelling"),
    ("traveled", "travelled"),
    ("traveling", "travelling"),
    ("traveler", "traveller"),
    ("modeled", "modelled"),
    ("modeling", "modelling"),
    ("catalog", "catalogue"),
    ("defense", "defence"),
    ("offense", "offence"),
    ("artifact", "artefact"),
    ("artifacts", "artefacts"),
    ("toward", "towards"),
    ("percent", "per cent"),
    ("acknowledgment", "acknowledgement"),
];

/// House overrides of the Style Manual, which prefers while and among.
const WORD_CHOICES: &[(&str, &str)] = &[("while", "whilst"), ("among", "amongst")];

/// Words before 'while' that make it a noun: a while, a short while, worth your while.
const WHILE_NOUN_BEFORE: &[&str] = &[
    "a", "the", "this", "that", "short", "long", "little", "good", "great", "whole", "my", "your",
    "his", "her", "its", "our", "their",
];

/// Words that open a subordinate clause; a comma closing one is not a list comma.
const SUBORDINATORS: &[&str] = &[
    "if", "when", "because", "although", "though", "while", "after", "before", "since", "unless",
    "once", "whenever",
];

/// Prepositions that open an introductory phrase ('In the original flow, the server...').
const INTRO_PREPOSITIONS: &[&str] = &[
    "in", "on", "at", "for", "with", "without", "from", "by", "during", "under", "after", "before",
    "within", "across", "through", "outside", "inside",
];

/// Subject pronouns that open an independent clause after 'and' or 'or'.
const SUBJECT_PRONOUNS: &[&str] = &["i", "we", "you", "he", "she", "it", "they", "there"];

/// Auxiliary and linking verbs that mark a finite clause ('the tests are green').
const FINITE_VERBS: &[&str] = &[
    "is", "are", "was", "were", "has", "have", "had", "can", "could", "will", "would", "should",
    "must", "may", "might", "do", "does", "did",
];

/// Determiners after which a verb-like word is a noun or adjective ('the signed token',
/// 'a can of soup'), not a verb. An '-ed' word after 'not' is also not a verb ('sales not
/// connected to it').
const DETERMINERS: &[&str] = &[
    "a", "an", "the", "my", "your", "his", "her", "its", "our", "their", "these", "those",
];

/// Relative pronouns; the verbs after one belong to a relative clause in a list item.
const RELATIVE_PRONOUNS: &[&str] = &["that", "which", "who", "whom", "whose"];

/// Words that end an introductory marker whose comma is not a list comma.
const MARKERS_BEFORE_COMMA: &[&str] = &[
    "for example",
    "for instance",
    "that is",
    "however",
    "in short",
];

/// Words that open a parenthetical segment between commas ('a folder, not a repository,').
const PARENTHETICAL_OPENERS: &[&str] = &["which", "not"];

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Stems whose -ize is part of the word, not a US suffix, alone or compounded and inflected:
/// downsizing, outsized, capsize, prized, seizing. US -ise words with these endings, such as
/// emphasize, are in the word lists, which are checked first.
const IZE_STEMS: &[&str] = &["size", "seize", "prize", "maize", "baize"];

fn ize_is_part_of_word(word: &str) -> bool {
    let base = match word.strip_suffix("ing") {
        Some(stem) => format!("{stem}e"),
        None => word.strip_suffix(['s', 'd']).unwrap_or(word).to_string(),
    };
    IZE_STEMS.iter().any(|stem| base.ends_with(stem))
}

/// Candidate findings in `text`, less those its suppression comments silence.
pub fn lint(text: &str) -> Vec<Finding> {
    lint_counting(text).0
}

/// `lint`'s findings and how many findings suppression comments silenced.
pub fn lint_counting(text: &str) -> (Vec<Finding>, usize) {
    let mut findings = Vec::new();
    let mut suppressions: Vec<Suppression> = Vec::new();
    let mut paragraph = Paragraph::default();
    let mut document = Document::default();
    let mut fence: Option<&str> = None;
    let mut list_indent = None;
    // The indent and length of the current run of bold-label bullets.
    let mut labels = (0, 0);
    // A leading YAML frontmatter block is metadata, not prose. Without its closing line, the
    // opening `---` is a thematic break and the file is prose.
    let mut in_frontmatter = (text.starts_with("---\n") || text.starts_with("---\r\n"))
        && text.lines().skip(1).any(|line| line.trim_end() == "---");
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = raw.trim_start();
        if in_frontmatter {
            if index > 0 && raw.trim_end() == "---" {
                in_frontmatter = false;
            }
            continue;
        }
        // A suppression silences the next non-blank line that is not itself a suppression.
        // Four columns beyond the current list item's content start form indented code.
        let indent = raw.len() - trimmed.len();
        let columns = indent_columns(&raw[..indent]);
        let names = if paragraph.lines.is_empty() && columns >= list_indent.unwrap_or(0) + 4 {
            None
        } else {
            suppression_names(trimmed)
        };
        if !trimmed.is_empty() && names.is_none() {
            for waiting in suppressions
                .iter_mut()
                .rev()
                .take_while(|s| s.target.is_none())
            {
                waiting.target = Some(line_no);
            }
        }
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            paragraph.flush(&mut findings, &mut document);
            fence = Some(if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            });
            continue;
        }
        if trimmed.is_empty() || is_tag_line(trimmed) || trimmed.starts_with('>') {
            // A blank line ends the paragraph; a line of HTML tags holds no prose, and block
            // quotations are not original prose.
            if let Some(names) = names {
                suppressions.push(Suppression {
                    line: line_no,
                    column: raw.len() - trimmed.len() + 1,
                    target: None,
                    rules: names
                        .into_iter()
                        .map(|n| (n, RuleId::named(n), false))
                        .collect(),
                });
            }
            paragraph.flush(&mut findings, &mut document);
            continue;
        }
        let marker_width = if is_break_line(trimmed) {
            None
        } else {
            list_marker_width(trimmed)
        };
        if let Some(width) = marker_width {
            list_indent = Some(columns + width);
        } else if paragraph.lines.is_empty() && indent == 0 {
            list_indent = None;
        }
        let list_item = marker_width.is_some();
        let in_list = list_indent.is_some();
        // Headings, table rows, lines that open with an HTML tag (`<b>Title</b>`), thematic
        // breaks, setext underlines and indented code lines are one line each; a list item
        // opens a block that its wrapped lines continue.
        let indented_code = paragraph.lines.is_empty() && indent >= 4 && !in_list;
        let single_line = trimmed.starts_with('#')
            || trimmed.starts_with('|')
            || trimmed.starts_with('<')
            || is_break_line(trimmed)
            || indented_code;
        if single_line || list_item {
            paragraph.flush(&mut findings, &mut document);
        }
        // Bullets nested under a run leave it open; a sibling without a label ends it.
        if list_item && (labels.1 == 0 || indent <= labels.0) {
            let continues = labels.1 > 0 && indent == labels.0;
            labels = match (opens_bold_label(trimmed), continues) {
                (true, true) => (indent, labels.1 + 1),
                (true, false) => (indent, 1),
                (false, _) => (0, 0),
            };
            if labels.1 == 3 {
                findings.push(Finding {
                    line: line_no,
                    column: indent + 1,
                    rule: RuleId::BoldLabel,
                    message: "3 or more bullets in a row open with a bold label (advisory); use prose or plain bullets unless a template requires the labels".into(),
                });
            }
        } else if !list_item && (single_line || !in_list) {
            labels = (0, 0);
        }
        paragraph.push(raw, line_no);
        // A hard line break (two trailing spaces or a backslash) also ends the run.
        if single_line || raw.ends_with("  ") || raw.trim_end().ends_with('\\') {
            paragraph.flush(&mut findings, &mut document);
        }
    }
    paragraph.flush(&mut findings, &mut document);
    // Rules that compare paragraphs report once the whole document is read.
    findings.extend(document.findings());
    let before = findings.len();
    findings.retain(|f| {
        let hit = suppressions
            .iter_mut()
            .filter(|s| s.target == Some(f.line))
            .flat_map(|s| s.rules.iter_mut())
            .find(|(_, id, _)| *id == Some(f.rule));
        match hit {
            Some((_, _, used)) => {
                *used = true;
                false
            }
            None => true,
        }
    });
    let suppressed = before - findings.len();
    for s in &suppressions {
        let mut messages: Vec<String> = s
            .rules
            .iter()
            .filter(|(_, _, used)| !used)
            .map(|&(name, id, _)| match (id, s.target) {
                (None, _) => format!("'{name}' is not a rule, so the suppression silences nothing (advisory)"),
                (Some(_), None) => format!("no line follows for the suppression of '{name}' (advisory)"),
                (Some(_), Some(target)) => format!(
                    "no '{name}' finding on line {target} to silence (advisory); remove it from the suppression"
                ),
            })
            .collect();
        if s.rules.is_empty() {
            messages.push("the suppression names no rule (advisory)".into());
        }
        findings.extend(messages.into_iter().map(|message| Finding {
            line: s.line,
            column: s.column,
            rule: RuleId::UnusedSuppression,
            message,
        }));
    }
    // Report in line order. The sort is stable, so each line keeps the order its findings were
    // added in: line and paragraph rules, then rules that compare paragraphs, then suppressions.
    findings.sort_by_key(|f| f.line);
    (findings, suppressed)
}

/// A suppression comment, `<!-- laconic-check-disable-next-line rule ... -->` on its own line.
struct Suppression<'a> {
    line: usize,
    column: usize,
    /// The next non-blank line that is not itself a suppression; `None` until it is read.
    target: Option<usize>,
    /// Each name, its rule if it names one, and whether it silenced a finding.
    rules: Vec<(&'a str, Option<RuleId>, bool)>,
}

/// The rule names a suppression comment lists, comma or space separated, or `None` when the
/// line is not a suppression comment.
fn suppression_names(trimmed: &str) -> Option<Vec<&str>> {
    let rest = trimmed
        .trim_end()
        .strip_prefix("<!--")?
        .trim_start()
        .strip_prefix("laconic-check-disable-next-line")?;
    // Whitespace or the comment's end follows the keyword, so a comment such as
    // 'disable-next-lineus-spelling' is not a suppression.
    if !(rest == "-->" || rest.starts_with(char::is_whitespace)) {
        return None;
    }
    let names = rest.strip_suffix("-->")?;
    Some(
        names
            .split([',', ' ', '\t'])
            .filter(|n| !n.is_empty())
            .collect(),
    )
}

/// A thematic break or setext underline: 3 or more of one of - * _ =, spaces allowed.
fn is_break_line(trimmed: &str) -> bool {
    let marks: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    marks.len() >= 3
        && matches!(marks[0], '-' | '*' | '_' | '=')
        && marks.iter().all(|&c| c == marks[0])
}

/// A line of HTML tags and comments only, such as `<p align="center">` or `</details>`. A tag
/// left open runs on to the next line, so the rest of the line is inside it.
fn is_tag_line(trimmed: &str) -> bool {
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
        } else if !opens_tag(&chars, i) {
            return false;
        } else {
            match tag_close(&chars, i) {
                Some(end) => i = end,
                None => return true,
            }
        }
    }
    true
}

/// True when an HTML comment, or a tag or autolink (`<b>`, `</b>`, `<https://...>`), opens at `i`.
fn opens_tag(chars: &[char], i: usize) -> bool {
    let name = if chars.get(i + 1) == Some(&'/') {
        i + 2
    } else {
        i + 1
    };
    chars.get(i) == Some(&'<')
        && (starts_with(chars, i, "<!--") || chars.get(name).is_some_and(char::is_ascii_alphabetic))
}

/// The index after the `>` that closes the tag opening at `i`, or the `-->` of a comment.
fn tag_close(chars: &[char], i: usize) -> Option<usize> {
    if starts_with(chars, i, "<!--") {
        (i + 4..chars.len())
            .find(|&j| starts_with(chars, j, "-->"))
            .map(|j| j + 3)
    } else {
        find(chars, i + 1, |x| x == '>').map(|j| j + 1)
    }
}

fn indent_columns(whitespace: &str) -> usize {
    whitespace.chars().fold(0, |columns, ch| {
        if ch == '\t' {
            columns + (4 - columns % 4)
        } else {
            columns + 1
        }
    })
}

fn list_marker_width(trimmed: &str) -> Option<usize> {
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    let marker = if ["- ", "* ", "+ "].iter().any(|m| trimmed.starts_with(m)) {
        1
    } else if digits > 0
        && [". ", ") "]
            .iter()
            .any(|m| trimmed[digits..].starts_with(m))
    {
        digits + 1
    } else {
        return None;
    };
    let padding = trimmed[marker..].bytes().take_while(|&b| b == b' ').count();
    Some(marker + if padding > 4 { 1 } else { padding })
}

/// True for a bullet that opens with a bold label: `- **Cause:** ...` or `- **Cause**: ...`.
fn opens_bold_label(trimmed: &str) -> bool {
    ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| trimmed.strip_prefix(marker))
        .and_then(|item| item.trim_start().strip_prefix("**"))
        .and_then(|bold| bold.split_once("**"))
        .is_some_and(|(label, rest)| {
            !label.trim().is_empty() && (label.ends_with(':') || rest.starts_with(':'))
        })
}

/// Consecutive prose lines joined with single spaces, so a phrase split by a line wrap
/// is still found. `origin` maps each joined character back to its line and column.
#[derive(Default)]
struct Paragraph {
    chars: Vec<char>,
    origin: Vec<(usize, usize)>,
    lines: Vec<(usize, Vec<char>)>,
}

impl Paragraph {
    fn push(&mut self, raw: &str, line: usize) {
        let kept: Vec<char> = raw.trim_end().chars().collect();
        // A continuation line's indent is layout; the first line keeps its columns.
        let indent = if let Some(&(last_line, last_column)) = self.origin.last() {
            self.chars.push(' ');
            self.origin.push((last_line, last_column + 1));
            kept.iter().take_while(|c| c.is_whitespace()).count()
        } else {
            0
        };
        for (k, &c) in kept.iter().enumerate().skip(indent) {
            self.chars.push(c);
            self.origin.push((line, k + 1));
        }
        self.lines.push((line, raw.chars().collect()));
    }

    /// Runs the rules over the joined paragraph, records its words in `document` and clears it.
    fn flush(&mut self, findings: &mut Vec<Finding>, document: &mut Document) {
        if self.lines.is_empty() {
            return;
        }
        let (masked, doubles) = mask(&self.chars);
        document.collect(&masked, |column| self.at(column));
        let mut found = Vec::new();
        for column in doubles {
            found.push(self.place(Finding {
                line: 0,
                column,
                rule: RuleId::DoubleQuotes,
                message: "double quotation marks in prose; use single marks".into(),
            }));
        }
        for (line, chars) in &self.lines {
            check_residue(chars, *line, &mut found);
        }
        let mut joined = Vec::new();
        check_line(&masked, 0, &mut joined);
        found.extend(joined.into_iter().map(|f| self.place(f)));
        // Report in line order; the sort is stable, so each line keeps its rule order.
        found.sort_by_key(|f| f.line);
        findings.append(&mut found);
        *self = Self::default();
    }

    /// The original line and column of a 1-based column in the joined paragraph.
    fn at(&self, column: usize) -> (usize, usize) {
        let index = column.saturating_sub(1);
        match self.origin.get(index) {
            Some(&at) => at,
            None => {
                let (line, column) = self.origin.last().copied().unwrap_or((0, 0));
                (line, column + index + 1 - self.origin.len())
            }
        }
    }

    /// Moves a finding from its joined column to its original line and column.
    fn place(&self, mut finding: Finding) -> Finding {
        (finding.line, finding.column) = self.at(finding.column);
        finding
    }
}

/// The words and acronyms of a whole document, for the rules that compare one paragraph with
/// another. Masked text only, so code, links and quotations take no part.
#[derive(Default)]
struct Document {
    /// Each lower-case word's first line and column; a hyphenated word stays whole.
    words: HashMap<String, (usize, usize)>,
    /// Each acronym's first definition in parentheses, as in 'Bureau of Meteorology (BOM)'.
    defined: HashMap<String, (usize, usize)>,
    /// Each acronym's first use outside a definition.
    used: HashMap<String, (usize, usize)>,
}

impl Document {
    /// Records one masked paragraph; `at` maps a 1-based column to its line and column.
    fn collect(&mut self, masked: &[char], at: impl Fn(usize) -> (usize, usize)) {
        let mut k = 0;
        while k < masked.len() {
            if !masked[k].is_alphabetic() {
                k += 1;
                continue;
            }
            // Letters, with single hyphens between letters kept inside the word.
            let start = k;
            while k < masked.len()
                && (masked[k].is_alphabetic()
                    || (masked[k] == '-' && masked.get(k + 1).is_some_and(|c| c.is_alphabetic())))
            {
                k += 1;
            }
            // A bare domain or path, such as 'planetark.org/cartridges-4-planet-ark', is an
            // address, not prose.
            let token_start = masked[..start]
                .iter()
                .rposition(|c| c.is_whitespace())
                .map_or(0, |p| p + 1);
            let token_end = masked[k..]
                .iter()
                .position(|c| c.is_whitespace())
                .map_or(masked.len(), |p| k + p);
            let token = &masked[token_start..token_end];
            if token.iter().any(|&c| c == '/' || c == '@')
                || token
                    .windows(3)
                    .any(|w| w[0].is_alphanumeric() && w[1] == '.' && w[2].is_alphanumeric())
            {
                continue;
            }
            let word: String = masked[start..k].iter().collect();
            let place = at(start + 1);
            if let Some(capitals) = acronym(&word) {
                let defines = start > 0
                    && masked[start - 1] == '('
                    && masked.get(k) == Some(&')')
                    && spells(&masked[..start - 1], &word);
                let first = if defines {
                    &mut self.defined
                } else {
                    &mut self.used
                };
                // 'NGOs' is a use of 'NGO', and '(NGO)' defines both.
                first.entry(capitals.to_owned()).or_insert(place);
            }
            self.words.entry(word.to_lowercase()).or_insert(place);
        }
    }

    /// The findings that need the whole document, in line and column order.
    fn findings(&self) -> Vec<Finding> {
        let mut found = Vec::new();
        for (word, &hyphenated) in &self.words {
            let Some((a, b)) = word.split_once('-') else {
                continue;
            };
            let closed = format!("{a}{b}");
            // The hyphen rule already reports the forms in HYPHENATED_FORMS.
            let reported = HYPHENATED_FORMS.iter().any(|(form, _, suffixes)| {
                word.strip_prefix(*form)
                    .is_some_and(|rest| suffixes.contains(&rest))
            });
            if b.contains('-') || a.chars().count() < 2 || b.chars().count() < 2 || reported {
                continue;
            }
            let Some(&closed_at) = self.words.get(&closed) else {
                continue;
            };
            // Report where the second form first appears.
            let (at, here, there, there_at) = if hyphenated > closed_at {
                (hyphenated, word.as_str(), closed.as_str(), closed_at)
            } else {
                (closed_at, closed.as_str(), word.as_str(), hyphenated)
            };
            found.push(Finding {
                line: at.0,
                column: at.1,
                rule: RuleId::HyphenConsistency,
                message: format!(
                    "'{here}' here and '{there}' on line {} (advisory); use one form unless they are different words",
                    there_at.0
                ),
            });
        }
        for (acronym, &used) in &self.used {
            if let Some(&defined) = self.defined.get(acronym)
                && used < defined
            {
                found.push(Finding {
                    line: used.0,
                    column: used.1,
                    rule: RuleId::AcronymBeforeDefinition,
                    message: format!(
                        "'{acronym}' is used before its definition on line {} (advisory); define it at first use",
                        defined.0
                    ),
                });
            }
        }
        found.sort_by_key(|f| (f.line, f.column));
        found
    }
}

/// The capitals of an acronym: 2 to 6 capital letters, with an optional plural 's' ('BOM',
/// 'NGOs').
fn acronym(word: &str) -> Option<&str> {
    let capitals = word.strip_suffix('s').unwrap_or(word);
    ((2..=6).contains(&capitals.len()) && capitals.bytes().all(|b| b.is_ascii_uppercase()))
        .then_some(capitals)
}

/// True when the few words before a parenthesised acronym spell it, as 'Bureau of
/// Meteorology' spells BOM: its letters appear in order, the first at the start of a word.
/// Labels such as 'Ayoub G. (MIT)' and 'Reading level (AAA)' do not. After Schwartz and
/// Hearst's algorithm for finding abbreviation definitions.
fn spells(before: &[char], acronym: &str) -> bool {
    let capitals = acronym.strip_suffix('s').unwrap_or(acronym);
    let text: String = before.iter().collect();
    let words: Vec<&str> = text.split_whitespace().collect();
    // The definition lies within min(|A| + 5, 2|A|) words before the parenthesis, after any
    // earlier use of the acronym itself, singular or plural.
    let limit = (capitals.len() + 5).min(capitals.len() * 2);
    let mut window = &words[words.len().saturating_sub(limit)..];
    if let Some(last) = window.iter().rposition(|w| {
        let bare = w.trim_matches(|c: char| !c.is_alphanumeric());
        bare.strip_suffix('s').unwrap_or(bare) == capitals
    }) {
        window = &window[last + 1..];
    }
    let long: Vec<char> = window.join(" ").chars().collect();
    let short: Vec<char> = capitals.chars().map(|c| c.to_ascii_lowercase()).collect();
    let mut l = long.len();
    for (s, &letter) in short.iter().enumerate().rev() {
        loop {
            if l == 0 {
                return false;
            }
            l -= 1;
            let word_start = l == 0 || !long[l - 1].is_alphanumeric();
            if long[l].to_ascii_lowercase() == letter && (s > 0 || word_start) {
                break;
            }
        }
    }
    true
}

/// Chat-tool leftovers: citation markers and tracking tags in links.
const RESIDUE_MARKERS: &[&str] = &[
    "oaicite",
    "contentreference",
    "utm_source=chatgpt",
    "utm_source=openai",
];

/// Flags zero-width characters and chat-tool markers. Unlike the other rules this
/// reads URLs and quoted text, where the markers usually sit; inline code, which may
/// name a marker on purpose, is skipped.
fn check_residue(chars: &[char], line: usize, findings: &mut Vec<Finding>) {
    let mut text = chars.to_vec();
    let mut i = 0;
    while i < text.len() {
        if text[i] == '`'
            && let Some(end) = find(chars, i + 1, |x| x == '`')
        {
            text[i..=end].iter_mut().for_each(|x| *x = ' ');
            i = end;
        }
        i += 1;
    }
    let is_tag = |c: char| ('\u{e0000}'..='\u{e007f}').contains(&c);
    let mut k = 0;
    while let Some(&c) = text.get(k) {
        if is_tag(c) {
            // Tag characters are invisible and can spell out a whole hidden instruction, so a
            // run is one finding. The 3 subdivision flags Unicode recommends for interchange
            // (England, Scotland and Wales: the black flag, tags spelling the code, CANCEL TAG)
            // are left alone; any other tag text, a short word included, is flagged.
            let end = text[k..]
                .iter()
                .position(|&x| !is_tag(x))
                .map_or(text.len(), |n| k + n);
            let spec: String = text[k..end]
                .iter()
                .map(|&x| char::from_u32(x as u32 - 0xe0000).unwrap_or('\u{fffd}'))
                .collect();
            let flag = k > 0
                && text[k - 1] == '\u{1f3f4}'
                && matches!(spec.as_str(), "gbeng\u{7f}" | "gbsct\u{7f}" | "gbwls\u{7f}");
            if !flag {
                let n = end - k;
                findings.push(Finding {
                    line,
                    column: k + 1,
                    rule: RuleId::Residue,
                    message: format!(
                        "{n} hidden tag character{} (U+E0000 to U+E007F)",
                        if n == 1 { "" } else { "s" }
                    ),
                });
            }
            k = end;
            continue;
        }
        // A byte order mark that opens the file is an encoding marker, not residue.
        let bom = c == '\u{feff}' && line == 1 && k == 0;
        let kind = match c {
            '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}' if !bom => {
                Some("zero-width character")
            }
            // Embeddings, overrides and isolates reorder what a reader sees.
            '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' => Some("bidirectional control"),
            _ => None,
        };
        if let Some(kind) = kind {
            findings.push(Finding {
                line,
                column: k + 1,
                rule: RuleId::Residue,
                message: format!("{kind} U+{:04X}", c as u32),
            });
        }
        k += 1;
    }
    let lower: String = text.iter().collect::<String>().to_lowercase();
    for marker in RESIDUE_MARKERS {
        let mut from = 0;
        while let Some(pos) = lower[from..].find(marker) {
            findings.push(Finding {
                line,
                column: lower[..from + pos].chars().count() + 1,
                rule: RuleId::Residue,
                message: format!("chat-tool residue '{marker}'"),
            });
            from += pos + marker.len();
        }
    }
}

/// Replaces protected spans with spaces, keeping columns. Returns the masked line and
/// the columns of double-quoted spans.
fn mask(chars: &[char]) -> (Vec<char>, Vec<usize>) {
    let mut out = chars.to_vec();
    let mut doubles = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let blank = |out: &mut Vec<char>, from: usize, to: usize| {
            out[from..to].iter_mut().for_each(|x| *x = ' ')
        };
        if c == '`'
            && let Some(end) = find(chars, i + 1, |x| x == '`')
        {
            blank(&mut out, i, end + 1);
            i = end + 1;
            continue;
        }
        if starts_with(chars, i, "http://") || starts_with(chars, i, "https://") {
            let end = (i..chars.len())
                .find(|&j| chars[j].is_whitespace() || chars[j] == ')' || chars[j] == '>')
                .unwrap_or(chars.len());
            blank(&mut out, i, end);
            i = end;
            continue;
        }
        // Link text that names its own target ([humanizer](https://github.com/x/humanizer))
        // is a name, not prose.
        if c == '['
            && let Some(close) = find(chars, i + 1, |x| x == ']')
            && chars.get(close + 1) == Some(&'(')
            && let Some(end) = find(chars, close + 2, |x| x == ')')
        {
            let text: String = chars[i + 1..close]
                .iter()
                .collect::<String>()
                .to_lowercase();
            let url: String = chars[close + 2..end]
                .iter()
                .collect::<String>()
                .to_lowercase();
            if !text.is_empty() && url.contains(&text) {
                blank(&mut out, i, end + 1);
                i = end + 1;
                continue;
            }
        }
        if c == ']'
            && chars.get(i + 1) == Some(&'(')
            && let Some(end) = find(chars, i + 2, |x| x == ')')
        {
            blank(&mut out, i + 1, end + 1);
            i = end + 1;
            continue;
        }
        // An HTML tag's name and attributes are markup; the text between tags is prose.
        if opens_tag(chars, i)
            && let Some(end) = tag_close(chars, i)
        {
            blank(&mut out, i, end);
            i = end;
            continue;
        }
        if c == '"' || c == '\u{201c}' {
            let close = if c == '"' { '"' } else { '\u{201d}' };
            if let Some(end) = find(chars, i + 1, |x| x == close)
                && end > i + 1
                && end - i <= 120
            {
                doubles.push(i + 1);
                blank(&mut out, i, end + 1);
                i = end + 1;
                continue;
            }
        }
        if (c == '\'' || c == '\u{2018}')
            && opens_quote(chars, i)
            && let Some(end) = (i + 1..chars.len())
                .find(|&j| (chars[j] == '\'' || chars[j] == '\u{2019}') && closes_quote(chars, j))
        {
            blank(&mut out, i, end + 1);
            i = end + 1;
            continue;
        }
        i += 1;
    }
    (out, doubles)
}

fn find(chars: &[char], from: usize, pred: impl Fn(char) -> bool) -> Option<usize> {
    (from..chars.len()).find(|&j| pred(chars[j]))
}

fn starts_with(chars: &[char], at: usize, s: &str) -> bool {
    s.chars()
        .enumerate()
        .all(|(k, c)| chars.get(at + k) == Some(&c))
}

fn opens_quote(chars: &[char], i: usize) -> bool {
    let before = if i == 0 { ' ' } else { chars[i - 1] };
    let after = chars.get(i + 1).copied().unwrap_or(' ');
    !before.is_alphanumeric() && !after.is_whitespace()
}

fn closes_quote(chars: &[char], j: usize) -> bool {
    let before = chars[j - 1];
    let after = chars.get(j + 1).copied().unwrap_or(' ');
    !before.is_whitespace() && !after.is_alphanumeric()
}

/// Columns (0-based) of clause dashes in a paragraph: an em dash after a word, or an en
/// dash, hyphen or double hyphen spaced between words. Spans ('2017–2019'), compounds,
/// list markers and table rows hold none.
fn clause_dashes(chars: &[char]) -> Vec<usize> {
    if chars.iter().find(|c| !c.is_whitespace()) == Some(&'|') {
        return Vec::new();
    }
    let word_before = |i: usize| {
        chars[..i]
            .iter()
            .rev()
            .find(|c| !c.is_whitespace())
            .is_some_and(|&c| c.is_alphanumeric() || matches!(c, ')' | '\'' | '\u{2019}' | '%'))
    };
    let word_after = |i: usize| {
        chars[i..]
            .iter()
            .find(|c| !c.is_whitespace())
            .is_some_and(|&c| c.is_alphanumeric() || matches!(c, '(' | '\'' | '\u{2018}' | '$'))
    };
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let width = if chars[i..].starts_with(&['-', '-']) {
            2
        } else {
            1
        };
        let dash = match chars[i] {
            '\u{2014}' => word_before(i),
            '\u{2013}' | '-' => {
                i > 0
                    && chars[i - 1].is_whitespace()
                    && chars.get(i + width).is_some_and(|c| c.is_whitespace())
                    && word_before(i)
                    && word_after(i + width)
            }
            _ => false,
        };
        if dash {
            found.push(i);
        }
        i += width;
    }
    found
}

fn check_line(masked: &[char], line: usize, findings: &mut Vec<Finding>) {
    for (k, &c) in masked.iter().enumerate() {
        if c == '\u{2014}' {
            findings.push(Finding {
                line,
                column: k + 1,
                rule: RuleId::EmDash,
                message: "em dash in original prose".into(),
            });
        }
    }
    // A pair of dashes around one aside is one device; dashes count once per sentence.
    let mut sentences = clause_dashes(masked).into_iter().map(|at| {
        let mut start = 0;
        while sentence_end(masked, start) <= at {
            start = sentence_end(masked, start);
        }
        (start, at)
    });
    let first = sentences.next();
    if let Some((_, at)) = sentences.find(|&(start, _)| Some(start) != first.map(|f| f.0)) {
        findings.push(Finding {
            line,
            column: at + 1,
            rule: RuleId::ClauseDashes,
            message: "clause dashes in 2 or more sentences of one paragraph (advisory); use a comma, colon or full stop where one reads as well".into(),
        });
    }
    let text: String = masked.iter().collect();
    let lower = text.to_lowercase();
    let lower_chars: Vec<char> = lower.chars().collect();
    for phrase in CLOSERS {
        let mut from = 0;
        while let Some(pos) = lower[from..].find(phrase) {
            let col = lower[..from + pos].chars().count() + 1;
            findings.push(Finding {
                line,
                column: col,
                rule: RuleId::Closer,
                message: format!("chatbot phrase '{phrase}'"),
            });
            from += pos + phrase.len();
        }
    }
    // One output char per input char, so columns stay exact (to_lowercase can expand
    // a character); every listed phrase is ASCII.
    let plain: String = masked
        .iter()
        .map(|&c| {
            if c == '\u{2019}' {
                '\''
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    for phrase in STOCK_PHRASES {
        let mut from = 0;
        while let Some(pos) = plain[from..].find(phrase) {
            let at = from + pos;
            let end = at + phrase.len();
            let bounded = !plain[..at].ends_with(|c: char| c.is_alphanumeric())
                && (phrase.ends_with(':')
                    || !plain[end..].starts_with(|c: char| c.is_alphanumeric()));
            if bounded {
                findings.push(Finding {
                    line,
                    column: plain[..at].chars().count() + 1,
                    rule: RuleId::StockPhrase,
                    message: format!("stock phrase '{phrase}'; state the point directly"),
                });
            }
            from = end;
        }
    }
    for phrase in STACKED_HEDGES {
        let mut from = 0;
        while let Some(pos) = plain[from..].find(phrase) {
            let at = from + pos;
            let end = at + phrase.len();
            if !plain[..at].ends_with(|c: char| c.is_alphanumeric())
                && !plain[end..].starts_with(|c: char| c.is_alphanumeric())
            {
                findings.push(Finding {
                    line,
                    column: plain[..at].chars().count() + 1,
                    rule: RuleId::StackedHedge,
                    message: format!("stacked hedge '{phrase}' (advisory); keep one qualifier at the confidence the source supports"),
                });
            }
            from = end;
        }
    }
    for (hyphenated, house, suffixes) in HYPHENATED_FORMS {
        let mut from = 0;
        while let Some(pos) = plain[from..].find(hyphenated) {
            let at = from + pos;
            from = at + hyphenated.len();
            if plain[..at].ends_with(|c: char| c.is_alphanumeric() || c == '-') {
                continue;
            }
            let rest = &plain[from..];
            let Some(suffix) = suffixes.iter().find(|s| {
                rest.starts_with(**s) && !rest[s.len()..].starts_with(char::is_alphanumeric)
            }) else {
                continue;
            };
            findings.push(Finding {
                line,
                column: plain[..at].chars().count() + 1,
                rule: RuleId::Hyphen,
                message: format!("'{hyphenated}{suffix}': write '{house}{suffix}'"),
            });
        }
    }
    check_structure(masked, &plain, line, findings);
    // Words, for spelling.
    let mut start = None;
    for k in 0..=lower_chars.len() {
        let is_word = k < lower_chars.len() && lower_chars[k].is_alphabetic();
        match (start, is_word) {
            (None, true) => start = Some(k),
            (Some(s), false) => {
                let word: String = lower_chars[s..k].iter().collect();
                // A capital inside a sentence marks a proper noun (the Labor Party, Pearl
                // Harbor), whose spelling is not a house-style choice.
                let proper_noun = masked.get(s).is_some_and(|c| c.is_uppercase())
                    && !starts_sentence(&lower_chars, s);
                if let Some(au) = au_form(&word).filter(|_| !proper_noun) {
                    findings.push(Finding {
                        line,
                        column: s + 1,
                        rule: RuleId::UsSpelling,
                        message: format!("'{word}': Australian form is '{au}'"),
                    });
                }
                let noun_while = word == "while"
                    && (WHILE_NOUN_BEFORE.contains(&word_before(&lower_chars, s).as_str())
                        || word_after(&lower_chars, k) == "away");
                if let Some((_, house)) = WORD_CHOICES.iter().find(|(w, _)| *w == word)
                    && !noun_while
                {
                    findings.push(Finding {
                        line,
                        column: s + 1,
                        rule: RuleId::WordChoice,
                        message: format!("'{word}': the house style prefers '{house}'"),
                    });
                }
                // A number word inside a hyphenated compound (forty-five) is a fixed form.
                if let Some(numeral) = number_word(&word)
                    && !starts_sentence(&lower_chars, s)
                    && !lower_chars.get(k).is_some_and(|c| *c == '-')
                    && !(s > 0 && lower_chars[s - 1] == '-')
                {
                    findings.push(Finding {
                        line,
                        column: s + 1,
                        rule: RuleId::NumberWord,
                        message: format!("'{word}': use the numeral {numeral} (words only at the start of a sentence)"),
                    });
                }
                start = None;
            }
            _ => {}
        }
    }
    check_style_manual_forms(&lower, &text, line, findings);
    // Serial comma candidates: ", and" or ", or" with another comma earlier in the same clause.
    for (k, window) in lower_chars.windows(6).enumerate() {
        let text: String = window.iter().collect();
        let is_and = text.starts_with(", and ");
        let is_or = text.starts_with(", or ");
        if !(is_and || is_or) {
            continue;
        }
        // The clause starts after sentence punctuation or the bracket that encloses this
        // comma; a bracket pair that closes before it belongs to the clause.
        let mut depth = 0;
        let mut clause_start = 0;
        for j in (0..k).rev() {
            match lower_chars[j] {
                ')' => depth += 1,
                '(' if depth == 0 => {
                    clause_start = j + 1;
                    break;
                }
                '(' => depth -= 1,
                '.' if j + 1 < lower_chars.len()
                    && lower_chars[j + 1].is_ascii_digit()
                    && j > 0
                    && lower_chars[j - 1].is_ascii_digit() => {}
                '.' | ';' | ':' | '!' | '?' if depth == 0 => {
                    clause_start = j + 1;
                    break;
                }
                _ => {}
            }
        }
        let list_commas = list_commas(&lower_chars, clause_start, k);
        // A final item that itself contains 'and' or 'or' may need the comma.
        let item_start = k + if is_and { 6 } else { 5 };
        let item_end = (item_start..lower_chars.len())
            .find(|&j| matches!(lower_chars[j], '.' | ';' | ':' | '!' | '?' | ','))
            .unwrap_or(lower_chars.len());
        let final_item: String = lower_chars[item_start..item_end].iter().collect();
        let compound_final = final_item.contains(" and ") || final_item.contains(" or ");
        // So may a list whose earlier items contain 'and' or 'or'.
        let earlier: String = lower_chars[clause_start..k].iter().collect();
        let compound_earlier = earlier.contains(" and ") || earlier.contains(" or ");
        // 'If it fails, rerun it, and report' joins 2 clauses after an introductory clause.
        let first_word = earlier
            .split(|c: char| !c.is_alphabetic())
            .find(|w| !w.is_empty())
            .unwrap_or("");
        // So does 'In the original flow, the server sends a code, and the client...'.
        let words_before_first_comma = list_commas.first().map_or(0, |&c| {
            lower_chars[clause_start..c]
                .iter()
                .collect::<String>()
                .split_whitespace()
                .count()
        });
        let introductory = list_commas.len() == 1
            && (SUBORDINATORS.contains(&first_word)
                || (INTRO_PREPOSITIONS.contains(&first_word)
                    && words_before_first_comma >= 3
                    && starts_sentence_at(&lower_chars, clause_start)));
        // 'We fixed it, as you suggested, and we reran it' joins a clause to a clause. A list
        // of clauses ('it may be gitignored, or the build may be wrong') has a clause in every
        // item, and a list of nouns as subject ('red, green, and blue are') has none.
        let item_clauses: Vec<bool> = list_commas
            .iter()
            .map(|&c| c + 1)
            .chain(std::iter::once(k))
            .scan(clause_start, |from, to| {
                let item = &lower_chars[*from..to];
                *from = to;
                Some(starts_clause(item))
            })
            .collect();
        let clause_join = starts_clause(&lower_chars[item_start..])
            && item_clauses.contains(&true)
            && item_clauses.contains(&false);
        if !list_commas.is_empty()
            && !compound_final
            && !compound_earlier
            && !introductory
            && !clause_join
        {
            findings.push(Finding { line, column: k + 1, rule: RuleId::SerialComma, message: "possible comma before a final 'and' or 'or'; keep it only if it prevents a misreading".into() });
        }
    }
}

/// True when `chars` opens an independent clause: a subject pronoun, or a finite verb after
/// a subject of up to 4 words ('the build now passes', 'recovery couldn't confirm').
// shortcut: irregular past tenses ('it broke') and present tenses without an adverb before
// them ('the build passes') go unrecognised, so those clause joins are still flagged.
fn starts_clause(chars: &[char]) -> bool {
    let text: String = chars
        .iter()
        .map(|&c| if c == '\u{2019}' { '\'' } else { c })
        .collect();
    // The words up to the first one that ends the item ('audio descriptions*, WebAIM').
    let mut words = Vec::new();
    for raw in text.split_whitespace().take(5) {
        words.push(raw.trim_matches(|c: char| !c.is_alphanumeric()));
        let closed = raw.trim_end_matches(['*', '_', ')', '\'', '"']);
        if closed.ends_with([',', ';', ':', '.', '!', '?']) {
            break;
        }
    }
    let Some(&first) = words.first() else {
        return false;
    };
    if SUBJECT_PRONOUNS.contains(&first) {
        return true;
    }
    if SUBORDINATORS.contains(&first) || INTRO_PREPOSITIONS.contains(&first) || first == "as" {
        return false;
    }
    // Drop a relative pronoun and the verb group after it ('that has failed'), so only a
    // main verb can mark a clause ('the build that failed has been rerun').
    let is_auxiliary = |w: &str| FINITE_VERBS.contains(&w.strip_suffix("n't").unwrap_or(w));
    let is_participle = |w: &str| w.len() > 3 && w.ends_with("ed") && !w.ends_with("eed");
    let mut main = vec![first];
    let mut rest = words[1..].iter().copied().peekable();
    while let Some(word) = rest.next() {
        if RELATIVE_PRONOUNS.contains(&word) {
            while rest.next_if(|w| is_auxiliary(w)).is_some() {}
            rest.next_if(|w| is_participle(w));
        } else {
            main.push(word);
        }
    }
    let words = main;
    (1..words.len()).any(|j| {
        let (word, previous) = (words[j], words[j - 1]);
        // An auxiliary after an article or possessive is a noun ('a can of soup'); after
        // 'these' or 'those' it has a subject ('these may fail').
        let noun = DETERMINERS.contains(&previous) && !matches!(previous, "these" | "those");
        let auxiliary = word.strip_suffix("n't").unwrap_or(word);
        !noun && (FINITE_VERBS.contains(&auxiliary) || matches!(word, "can't" | "won't" | "cannot"))
            || (j > 1 && ["now", "still", "also", "never", "already"].contains(&previous))
            || word.len() > 3
                && word.ends_with("ed")
                && !word.ends_with("eed")
                && !DETERMINERS.contains(&previous)
                && previous != "not"
    })
}

/// Commas in `chars[from..to]` that could separate list items. Skips commas inside numbers
/// (75,000), inside a bracket pair, after markers such as 'for example', and the pair of
/// commas around a 'which' or 'not' segment.
fn list_commas(chars: &[char], from: usize, to: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let mut depth = 0i32;
    let mut closes_parenthetical = false;
    for j in from..to {
        match chars[j] {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                let in_number = j > 0
                    && chars[j - 1].is_ascii_digit()
                    && chars.get(j + 1).is_some_and(|c| c.is_ascii_digit());
                if in_number {
                    continue;
                }
                let before: String = chars[from..j].iter().collect();
                let after_marker = MARKERS_BEFORE_COMMA
                    .iter()
                    .any(|m| before.trim_end().ends_with(m));
                let opens = PARENTHETICAL_OPENERS.contains(&word_after(chars, j + 1).as_str());
                if closes_parenthetical {
                    closes_parenthetical = false;
                } else if opens {
                    closes_parenthetical = true;
                } else if !after_marker {
                    found.push(j);
                }
            }
            _ => {}
        }
    }
    found
}

/// End (exclusive) of the sentence starting at `start`: just past a '.', '!' or '?'
/// followed by whitespace or the end of the line, or the end of the line.
fn sentence_end(chars: &[char], start: usize) -> usize {
    let mut stop = start;
    while stop < chars.len()
        && !(matches!(chars[stop], '.' | '!' | '?')
            && chars.get(stop + 1).is_none_or(|c| c.is_whitespace()))
    {
        stop += 1;
    }
    (stop + 1).min(chars.len())
}

/// Structural tells, one finding per sentence at most: a staged contrast ('it's not
/// X, it's Y'; 'not just X but Y') and a reveal question answered by the next
/// sentence ('The result? A 40% cut.'). `plain` is `masked` lowercased char for char.
fn check_structure(masked: &[char], plain: &str, line: usize, findings: &mut Vec<Finding>) {
    let chars: Vec<char> = plain.chars().collect();
    let skip_space = |mut at: usize| {
        while at < chars.len() && chars[at].is_whitespace() {
            at += 1;
        }
        at
    };
    let mut start = skip_space(0);
    while start < chars.len() {
        let stop = sentence_end(&chars, start);
        let sentence: String = chars[start..stop].iter().collect();
        let lead = sentence.len() - sentence.trim_start_matches([' ', '-', '*', '#']).len();
        if let Some(at) = staged_contrast(&sentence, lead) {
            findings.push(Finding {
                line,
                column: start + sentence[..at].chars().count() + 1,
                rule: RuleId::StagedContrast,
                message: "staged contrast ('not X, it's Y'); state what it is".into(),
            });
        }
        // The answer is the next sentence on the line, and it must not be a question:
        // no '?' in its closing punctuation ('Did it pass?!') and no auxiliary verb
        // opening it, which also covers a question cut short by 'Mr.' or 'Dr.'.
        let next = skip_space(stop);
        let answer: String = chars[next..sentence_end(&chars, next)].iter().collect();
        let asks = answer
            .trim_end()
            .chars()
            .rev()
            .take_while(|c| matches!(c, '?' | '!' | '.'))
            .any(|c| c == '?')
            || answer
                .split_whitespace()
                .next()
                .is_some_and(|w| QUESTION_OPENERS.contains(&w));
        let answered = answer.contains(char::is_alphanumeric) && !asks;
        let capital = masked.get(start + lead).is_some_and(|c| c.is_uppercase());
        let question = sentence[lead..].trim_end();
        let reveal = question
            .strip_suffix('?')
            .is_some_and(|phrase| REVEAL_PHRASES.contains(&phrase.trim()));
        if reveal && answered && capital {
            findings.push(Finding {
                line,
                column: start + lead + 1,
                rule: RuleId::RevealQuestion,
                message: "question-and-answer reveal; state the answer directly".into(),
            });
        }
        start = next;
    }
}

/// Byte offset of a staged-contrast opener in `sentence`, if its turn follows. Openers
/// beginning 'it's not' or 'this is not' count only at the start of the sentence and
/// only with an 'it's' turn, so 'If it is not listed, it is excluded' and 'This is not
/// a firm rule, but one example' are not findings.
fn staged_contrast(sentence: &str, lead: usize) -> Option<usize> {
    for (index, opener) in CONTRAST_OPENERS.iter().enumerate() {
        let sentence_start_only =
            index >= CONTRAST_OPENERS.iter().position(|o| *o == "it's not ")?;
        let mut from = 0;
        while let Some(pos) = sentence[from..].find(opener) {
            let at = from + pos;
            from = at + opener.len();
            let bounded = !sentence[..at].ends_with(|c: char| c.is_alphanumeric());
            if !bounded || (sentence_start_only && at != lead) {
                continue;
            }
            let rest = &sentence[from..];
            let turned = (!sentence_start_only && rest.contains(" but "))
                || rest
                    .match_indices([',', ';', '\u{2014}', '\u{2013}'])
                    .any(|(i, sep)| {
                        let after = rest[i + sep.len()..].trim_start();
                        CONTRAST_TURNS.iter().any(|t| after.starts_with(t))
                    });
            if turned {
                return Some(at);
            }
        }
    }
    None
}

/// True when position `at` opens a sentence or the line, not a clause after ':' or ';'.
fn starts_sentence_at(chars: &[char], at: usize) -> bool {
    chars[..at]
        .iter()
        .rev()
        .find(|c| !c.is_whitespace())
        .is_none_or(|c| matches!(c, '.' | '!' | '?'))
}

/// True for a year ending in 0 with an apostrophe s, as in `1980's` or `mid-1980's.`.
fn apostrophe_decade(word: &str) -> bool {
    word.split(['-', '\u{2013}']).any(|part| {
        let part = part.trim_end_matches(|c: char| !c.is_alphanumeric());
        part.strip_suffix("'s")
            .or_else(|| part.strip_suffix("\u{2019}s"))
            .is_some_and(|year| {
                year.len() == 4 && year.ends_with('0') && year.chars().all(|c| c.is_ascii_digit())
            })
    })
}

/// Dates, times, number symbols and Latin abbreviations that break the Style Manual forms.
fn check_style_manual_forms(lower: &str, text: &str, line: usize, findings: &mut Vec<Finding>) {
    let tokens: Vec<(usize, &str)> = lower
        .split_whitespace()
        .map(|w| (w.as_ptr() as usize - lower.as_ptr() as usize, w))
        .collect();
    let core = |w: &str| -> String {
        w.trim_matches(|c: char| matches!(c, ',' | ';' | '(' | ')' | '*' | '_'))
            .to_string()
    };
    let digits = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit());
    let ordinal = |w: &str| {
        ["st", "nd", "rd", "th"]
            .iter()
            .any(|s| w.strip_suffix(s).is_some_and(digits))
    };
    // Whole amounts only ('27.99', '1,000'), so '$ 2x + 1 $' and a closing '$ (1)' in maths
    // are not money.
    let amount = |raw: &str| {
        raw.starts_with(|c: char| c.is_ascii_digit())
            && core(raw)
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, ',' | '.'))
    };
    let mut push = |at: usize, rule: RuleId, message: &str| {
        findings.push(Finding {
            line,
            column: lower[..at].chars().count() + 1,
            rule,
            message: message.into(),
        })
    };
    for (i, &(at, raw)) in tokens.iter().enumerate() {
        let word = core(raw);
        let bare = word.trim_end_matches(['.', ':']);
        let prev = i.checked_sub(1).map(|j| core(tokens[j].1));
        let next = tokens.get(i + 1).map(|t| core(t.1));
        if word.starts_with("e.g.") || word.starts_with("i.e.") || bare == "eg" || bare == "ie" {
            push(
                at,
                RuleId::Latin,
                "Latin abbreviation; write 'for example' or 'that is'",
            );
        }
        // 9am, 9:30pm, 9.30am, 9 a.m.: write 9:30 am.
        let time = bare
            .strip_suffix("am")
            .or_else(|| bare.strip_suffix("pm"))
            .filter(|t| {
                !t.is_empty()
                    && t.chars()
                        .all(|c| c.is_ascii_digit() || c == ':' || c == '.')
            })
            .is_some_and(|t| t.starts_with(|c: char| c.is_ascii_digit()));
        let dotted = matches!(word.as_str(), "a.m." | "p.m." | "a.m" | "p.m");
        let period_separator = word.contains('.')
            && word.split('.').count() == 2
            && word.split('.').all(digits)
            && next
                .as_deref()
                .is_some_and(|n| matches!(n.trim_end_matches('.'), "am" | "pm"));
        if time || dotted || period_separator {
            push(at, RuleId::Time, "write times as 9:30 am or 9 am");
        }
        // A digit after the sign is a remainder ('5 % 3'), and a year before it a table
        // heading ('2020 % change').
        let percent_sign = word.starts_with('%') && !word[1..].contains(char::is_alphanumeric);
        if percent_sign
            && prev.as_deref().is_some_and(|p| {
                p.ends_with(|c: char| c.is_ascii_digit())
                    && !p.contains(char::is_alphabetic)
                    && !(digits(p) && p.len() == 4 && (p.starts_with("19") || p.starts_with("20")))
            })
            && !next
                .as_deref()
                .is_some_and(|n| n.starts_with(|c: char| c.is_ascii_digit()))
        {
            push(
                at,
                RuleId::SymbolSpace,
                "no space before the percentage sign: write 12%",
            );
        }
        let currency = word
            .strip_suffix('$')
            .is_some_and(|code| code.chars().all(char::is_alphabetic));
        if currency && tokens.get(i + 1).is_some_and(|t| amount(t.1)) {
            push(
                at,
                RuleId::SymbolSpace,
                "no space after a currency symbol: write $27.99",
            );
        }
        // A year's possessive ('2020's budget') has a noun after it and no 'the' before it.
        // shortcut: so a bare decade before a noun ('1980's music') passes; telling the 2 apart
        // needs a parser.
        let noun_follows = next.is_some()
            && !raw
                .trim_end_matches(['*', '_', ')', ']', '\'', '"'])
                .ends_with(['.', ',', ';', ':', '!', '?']);
        let decade_word = word.contains(['-', '\u{2013}'])
            || prev
                .as_deref()
                .is_some_and(|p| matches!(p, "the" | "early" | "mid" | "late" | "those" | "these"));
        if apostrophe_decade(&word) && (decade_word || !noun_follows) {
            push(at, RuleId::Date, "decades take no apostrophe: write 1980s");
        }
        if MONTHS.contains(&word.trim_end_matches('.')) {
            let month = word.trim_end_matches('.');
            // A month that ends a clause ('20 October; 14 new') takes no year or day after it.
            let next = next.as_deref().filter(|_| {
                !raw.trim_end_matches(['*', '_', ')', ']', '\'', '"'])
                    .ends_with(['.', ';', ':', '!', '?'])
            });
            let prev_day = prev.as_deref().filter(|p| digits(p) || ordinal(p));
            let ordinal_of = prev.as_deref() == Some("of")
                && i.checked_sub(2)
                    .is_some_and(|j| ordinal(&core(tokens[j].1)));
            // 'Friday the 13th may be rooted in fear': the month takes a capital, the verb
            // doesn't, so 'the 1st May meeting' is still a date. `lower` has lost the case.
            let modal = month == "may"
                && text
                    .split_whitespace()
                    .nth(i)
                    .is_some_and(|w| w.contains("may"));
            if (prev_day.is_some_and(ordinal) && !modal) || ordinal_of {
                push(
                    at,
                    RuleId::Date,
                    "no ordinal in dates: write 19 October 2026",
                );
            }
            if prev_day.is_some()
                && raw.ends_with(',')
                && next.is_some_and(|n| {
                    let n = n.trim_end_matches(['.', ',']);
                    digits(n) && n.len() == 4
                })
            {
                push(
                    at,
                    RuleId::Date,
                    "no comma before the year: write 19 October 2026",
                );
            }
            // 'Wednesday 22 October 2026' names a weekday the calendar can check.
            let year = next
                .map(|n| n.trim_end_matches(['.', ',']))
                .filter(|n| digits(n) && n.len() == 4)
                .and_then(|n| n.parse::<i32>().ok());
            let day = prev_day
                .filter(|p| digits(p))
                .and_then(|p| p.parse::<u32>().ok());
            let named = i
                .checked_sub(2)
                .map(|j| core(tokens[j].1))
                .and_then(|w| WEEKDAYS.iter().position(|d| *d == w));
            if let (Some(year), Some(day), Some(named)) = (year, day, named) {
                let month_index = MONTHS.iter().position(|m| *m == month).unwrap_or(0) as u32 + 1;
                if let Some(actual) = weekday(year, month_index, day)
                    && actual != named
                {
                    push(
                        tokens[i - 2].0,
                        RuleId::Weekday,
                        &format!(
                            "{day} {} {year} is a {}, not a {}",
                            capitalise(month),
                            capitalise(WEEKDAYS[actual]),
                            capitalise(WEEKDAYS[named])
                        ),
                    );
                }
            }
            if month != "may"
                && next.is_some_and(|n| {
                    let n = n.trim_end_matches(['.', ',']);
                    (digits(n) && n.len() <= 2) || ordinal(n)
                })
            {
                push(at, RuleId::Date, "day before month: write 19 October 2026");
            }
        }
    }
}

const WEEKDAYS: [&str; 7] = [
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
];

/// Day of the week (0 is Sunday) for a Gregorian date, or None if the date does not exist.
fn weekday(year: i32, month: u32, day: u32) -> Option<usize> {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let length = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month) || day == 0 || day > length[month as usize - 1] {
        return None;
    }
    // Sakamoto's method.
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let d = y + y / 4 - y / 100 + y / 400 + OFFSETS[month as usize - 1] + day as i32;
    Some(d.rem_euclid(7) as usize)
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Numbers from 2 to 20 written as words. 'Zero' and 'one' stay as words.
fn number_word(word: &str) -> Option<u8> {
    const WORDS: [&str; 19] = [
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];
    WORDS.iter().position(|w| *w == word).map(|i| i as u8 + 2)
}

/// The word before position `s`, lower case, or an empty string.
fn word_before(chars: &[char], s: usize) -> String {
    let end = chars[..s]
        .iter()
        .rposition(|c| c.is_alphabetic())
        .map_or(0, |e| e + 1);
    let gap = &chars[end..s];
    if !gap.iter().all(|c| c.is_whitespace()) {
        return String::new();
    }
    let start = chars[..end]
        .iter()
        .rposition(|c| !c.is_alphabetic())
        .map_or(0, |b| b + 1);
    chars[start..end].iter().collect()
}

/// The word starting after position `k` past spaces, lower case, or an empty string.
fn word_after(chars: &[char], k: usize) -> String {
    let start = (k..chars.len())
        .find(|&j| !chars[j].is_whitespace())
        .unwrap_or(chars.len());
    chars[start..]
        .iter()
        .take_while(|c| c.is_alphabetic())
        .collect()
}

/// True when the word at `s` begins a sentence (or the line).
fn starts_sentence(chars: &[char], s: usize) -> bool {
    // Skip spaces and Markdown markers (list bullets, emphasis, headings, table pipes).
    let previous = chars[..s]
        .iter()
        .rev()
        .find(|c| !(c.is_whitespace() || matches!(c, '*' | '_' | '#' | '-' | '|' | '(')));
    previous.is_none_or(|c| matches!(c, '.' | '!' | '?' | ':'))
}

/// US forms with their Australian spelling, generated from the English Speller Database
/// by `laconic-tools gen-au-words`, one "us au" pair per line.
const ESDB_US_WORDS: &str = include_str!("us_words.txt");

fn esdb_us_words() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        ESDB_US_WORDS
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once(' '))
            .collect()
    })
}

fn au_form(word: &str) -> Option<String> {
    if let Some((_, au)) = US_WORDS.iter().find(|(us, _)| *us == word) {
        return Some((*au).to_string());
    }
    if let Some(au) = esdb_us_words().get(word) {
        return Some((*au).to_string());
    }
    if word.len() < 6 || ize_is_part_of_word(word) {
        return None;
    }
    for (us, au) in [
        ("ization", "isation"),
        ("izations", "isations"),
        ("izing", "ising"),
        ("ized", "ised"),
        ("izes", "ises"),
        ("ize", "ise"),
        ("yzing", "ysing"),
        ("yzed", "ysed"),
        ("yzes", "yses"),
        ("yze", "yse"),
    ] {
        if let Some(stem) = word.strip_suffix(us) {
            return Some(format!("{stem}{au}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> Vec<&'static str> {
        lint(text).into_iter().map(|f| f.rule.name()).collect()
    }

    #[test]
    fn flags_unnecessary_hyphens_but_keeps_meaningful_ones() {
        let f = &lint("We e-mailed the open-source maintainers.")[..];
        assert_eq!(
            f.iter().map(|f| f.message.as_str()).collect::<Vec<_>>(),
            vec![
                "'open-source': write 'open source'",
                "'e-mailed': write 'emailed'"
            ]
        );
        assert_eq!(
            rules("Co-ordination of the data-sets"),
            vec!["hyphen", "hyphen"]
        );
        for text in [
            "The code-base, code-bases, data-set and e-mail-based forms.",
            "They co-ordinated it.",
        ] {
            assert!(rules(text).iter().all(|r| *r == "hyphen"), "{text}");
        }
        assert_eq!(
            rules("The code-base, code-bases, data-set and e-mail-based forms.").len(),
            4
        );
        for text in [
            "Re-sign the up-to-date co-op agreement.",
            "Reopen-source is not a word, nor is pre-e-mail.",
            "See https://example.com/open-source and `e-mail`.",
            "We use code-based authentication.",
            "The data-setting step is complete.",
            "Decode code-base64 strings.",
            "The co-ordinatorship lapsed.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
        assert_eq!(lint("Caf\u{e9} e-mail.")[0].column, 6);
    }

    #[test]
    fn flags_staged_contrasts_but_not_ordinary_negation() {
        for text in [
            "It's not a tool, it's a platform.",
            "It\u{2019}s not a tool; it\u{2019}s a platform.",
            "The deadline isn't just a date, it's a milestone.",
            "We ship not just code but working software.",
            "This isn't about speed. It is about trust, and this is not just about speed but trust.",
        ] {
            assert_eq!(rules(text), vec!["staged-contrast"], "{text}");
        }
        for text in [
            "If it is not listed, it is excluded.",
            "This is not a firm rule, but one example of how things can change.",
            "Not only the family but the carers attended.",
            "It's not ready yet.",
            "The fix is not just a restart.",
            "She said 'it's not a tool, it's a platform' twice.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
        let f = &lint("Read this. It's not X, it's Y.")[0];
        assert_eq!((f.column, f.rule.name()), (12, "staged-contrast"));
    }

    #[test]
    fn flags_verbless_reveal_questions_followed_by_their_answer() {
        for text in [
            "The root cause? A misconfigured pool.",
            "- Our secret? Testing.",
            "Why? Because the cache was cold.",
        ] {
            assert_eq!(rules(text), vec!["reveal-question"], "{text}");
        }
        for text in [
            "Who's still there? A few staff.",
            "Did it pass? Yes.",
            "Is the build ready?",
            "the result? a cut, on a wrapped line.",
            "What does the 401 mean in this request? It means no token.",
            "Your payment cleared? Check the account.",
            "The dog died? I had no idea.",
            "Really? Did it pass?",
            "The result? Did it pass?",
            "The result? Did Mr. Smith pass?",
            "The result? Did it pass?!",
            "The result? Is it ready.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
        assert_eq!(lint("Done. The result? A cut.")[0].column, 7);
        // Columns count characters, not bytes.
        assert_eq!(lint("Caf\u{e9}. The result? A cut.")[0].column, 7);
    }

    #[test]
    fn any_whitespace_ends_a_sentence() {
        for gap in [" ", "\t", "\u{a0}"] {
            let text = format!("The fix is not just a restart.{gap}It is slow, but it works.");
            assert!(rules(&text).is_empty(), "{gap:?}: {:?}", rules(&text));
            let text = format!("Done.{gap}It's not a tool, it's a platform.");
            assert_eq!(rules(&text), vec!["staged-contrast"], "{gap:?}");
        }
        let f = &lint("Caf\u{e9}. It\u{2019}s not X, it\u{2019}s Y.")[0];
        assert_eq!((f.column, f.rule.name()), (7, "staged-contrast"));
    }

    #[test]
    fn flags_house_style_candidates_in_prose() {
        assert_eq!(
            rules("We will organize the colors \u{2014} soon."),
            vec!["em-dash", "us-spelling", "us-spelling"]
        );
        assert_eq!(rules("He said \"approved\" today."), vec!["double-quotes"]);
        assert_eq!(
            rules(
                "They prized the resized, outsized and capsized boats, seizing them whilst downsizing."
            ),
            Vec::<&str>::new()
        );
        assert_eq!(
            rules("We emphasized and synthesized it."),
            vec!["us-spelling", "us-spelling"]
        );
        assert_eq!(
            rules("Bring apples, pears, and plums."),
            vec!["serial-comma"]
        );
        assert_eq!(rules("Hope this helps!"), vec!["closer"]);
    }

    #[test]
    fn flags_stock_phrases_as_whole_words_only() {
        assert_eq!(
            rules("Here\u{2019}s the thing: we should circle back."),
            vec!["stock-phrase", "stock-phrase"]
        );
        assert_eq!(rules("Bottom line: it works."), vec!["stock-phrase"]);
        assert!(rules("Draw a circle backwards. The deep diver surfaced.").is_empty());
        assert!(rules("Avoid 'circle back' and `deep dive` in reports.").is_empty());
        assert!(
            rules("Reconcile the till at the end of the day. The load-bearing wall is cracked.")
                .is_empty()
        );
        assert_eq!(
            rules("Recircle back. Bottom line:works."),
            vec!["stock-phrase"]
        );
    }

    #[test]
    fn flags_timeless_openers_and_vague_praise() {
        assert_eq!(
            rules("Tax has fascinated people for centuries. We offer practical solutions."),
            vec!["stock-phrase", "stock-phrase"]
        );
        assert_eq!(rules("An innovative, timely fix."), vec!["stock-phrase"]);
        assert!(rules("Innovation grants open in March. The comet fascinated people.").is_empty());
    }

    #[test]
    fn flags_clause_dashes_in_two_sentences_of_a_paragraph() {
        // One aside in a pair of dashes is one device, as in the Style Manual.
        assert!(
            rules("The manual is for the people \u{2013} our audience \u{2013} who read it.")
                .is_empty()
        );
        assert!(rules("It failed \u{2013} twice. Then it passed.").is_empty());
        for dash in ["\u{2014}", " \u{2013} ", " - ", " -- "] {
            let text = format!("It failed{dash}twice. Then it{dash}passed.");
            let found = lint(&text);
            let rules: Vec<_> = found
                .iter()
                .map(|f| f.rule.name())
                .filter(|r| *r != "em-dash")
                .collect();
            assert_eq!(rules, vec!["clause-dashes"], "{dash:?}");
            let at = found
                .iter()
                .find(|f| f.rule == RuleId::ClauseDashes)
                .unwrap()
                .column;
            assert_eq!(
                at,
                text.rfind(dash.trim())
                    .map(|b| text[..b].chars().count() + 1)
                    .unwrap(),
                "{dash:?}"
            );
        }
        // Wrapped lines join into one paragraph; a blank line separates two.
        assert_eq!(
            rules("It failed - twice.\nThen it - passed."),
            vec!["clause-dashes"]
        );
        assert!(rules("It failed - twice.\n\nThen it - passed.").is_empty());
        // Spans, compounds, list markers, table rows and code are not clause dashes.
        assert!(rules("Run 2017\u{2013}2019 and 2026-27. The well-known go-ahead.").is_empty());
        assert!(rules("- First item. - Second item.").is_empty());
        assert!(rules("| a - b. | c - d. |").is_empty());
        assert!(rules("Use `a - b`. Then `c - d`.").is_empty());
    }

    #[test]
    fn stock_phrase_columns_count_characters() {
        let f = &lint("We\u{2019}ll circle back.")[0];
        assert_eq!((f.column, f.rule.name()), (7, "stock-phrase"));
        let f = &lint("\u{130} circle back.")[0];
        assert_eq!((f.column, f.rule.name()), (3, "stock-phrase"));
    }

    #[test]
    fn leaves_protected_and_correct_text_alone() {
        let clean = "Set `color: red` and quote 'Choose a color' at https://example.com/color.\n\
                     Bring apples, pears and plums. The size is fine; we organise and analyse.\n\
                     ```\nbehavior = \"organize\"\n```\n> A quoted line with \"double\" marks.";
        assert!(lint(clean).is_empty(), "{:?}", lint(clean));
    }

    #[test]
    fn numbers_from_2_take_numerals_except_at_a_sentence_start() {
        assert_eq!(rules("We made two commits."), vec!["number-word"]);
        assert!(rules("Two commits landed. One more is due. A two-thirds share.").is_empty());
        assert!(rules("- Three items remain.").is_empty());
    }

    #[test]
    fn skips_frontmatter_and_compound_final_items() {
        assert!(lint("---\ntags: \"A, B\"\n---\nFine text.").is_empty());
        assert!(rules("Read the draft, the notes, and the facts or figures.").is_empty());
    }

    #[test]
    fn lints_a_file_whose_frontmatter_never_closes() {
        assert_eq!(rules("---\nThe color is red.\n"), ["us-spelling"]);
        assert_eq!(
            rules("---\r\ntitle: x\r\nThe color is red.\r\n"),
            ["us-spelling"]
        );
    }

    #[test]
    fn lints_the_text_between_html_tags_but_not_the_tags() {
        assert_eq!(rules("<sub>The color is red.</sub>\n"), ["us-spelling"]);
        let found = lint("<p align=\"center\"><b>The</b> color is red.</p>\n");
        assert_eq!(
            found
                .iter()
                .map(|f| (f.column, f.rule.name()))
                .collect::<Vec<_>>(),
            [(30, "us-spelling")]
        );
        assert_eq!(rules("See <a href=\"x\">the color</a>.\n"), ["us-spelling"]);
        assert_eq!(rules("<3 the color.\n"), ["us-spelling"]);
        // Only CommonMark's tag syntax is markup: a comparison, an escaped '<' and a '>' in a
        // quoted attribute leave the prose visible and the attribute masked.
        for text in [
            "If x<y, the color is y>z.\n",
            "Type \\<b the color> here.\n",
            "See <a title=\"x > y\">the color</a>.\n",
            "See <a title='x > color'>the color</a>.\n",
        ] {
            assert_eq!(rules(text), ["us-spelling"], "{text:?}");
        }
        // A line that opens with a tag stands alone, as a heading does.
        assert_eq!(
            rules(
                "<b>Isoflavones and Anthocyanins</b>\nIsoflavones, sometimes called \
                 phytoestrogens, are found in soy, lentils, and legumes.\n"
            ),
            ["serial-comma"]
        );
        for tags in [
            "<p align=\"center\">\n",
            "<img src=\"a.png\" alt=\"A color chart\">\n",
            "  </details> <br>\n",
            "<!-- -->\n",
            "<!-- the color -->\n",
            "<https://example.com/color>\n",
            "<img src=\"a.png\"\n",
            "<img alt=\"a > b\" title=\"color\" />\n",
        ] {
            assert!(lint(tags).is_empty(), "{tags:?}: {:?}", lint(tags));
        }
    }

    #[test]
    fn a_comma_joining_two_clauses_is_not_a_serial_comma() {
        assert!(rules("The build passed, and the deploy started.").is_empty());
        // An earlier comma that does not separate clauses leaves the final join a clause
        // join (the first 2 adapted from agent output, 30 October 2026).
        let joins = [
            "As planned, all 3 release runs passed, and au-fpa-pack 0.4.0 is on PyPI.",
            "The backup started overnight, finished within an hour, and recovery couldn't confirm it.",
            "The job retried twice, as expected, and it failed.",
            "We fixed the lockfile, as you suggested, and we reran it.",
            "Thanks to the new runner, the tests are green, and the build now passes.",
            "As expected, the job retried, and these may fail.",
            "As expected, the job retried, and the build that failed has been rerun.",
        ];
        for sentence in joins {
            assert!(rules(sentence).is_empty(), "should not flag: {sentence}");
        }
        for list in [
            "Collect logs, metrics, and traces.",
            "Pick red, green, or blue.",
            "Review logs, the build that failed, and the test that passed.",
            "Review logs, the build that has failed, and the test that has passed.",
            "Buy bread, a can of soup, and a can of beans.",
            "The +, \u{2212}, \u{d7}, and \u{f7} symbols are called operators.",
        ] {
            assert_eq!(rules(list), vec!["serial-comma"], "should flag: {list}");
        }
    }

    #[test]
    fn flags_style_manual_word_choices_dates_times_and_latin() {
        assert_eq!(
            rules("While among friends."),
            vec!["word-choice", "word-choice"]
        );
        assert!(
            rules("Whilst amongst friends we waited a while. It was worth your while to while away an hour.")
                .is_empty()
        );
        assert_eq!(
            rules("Leave at 9.30am on 5th October, 2026."),
            vec!["time", "date", "date"]
        );
        assert_eq!(
            rules("Due October 5 at 9 a.m. today."),
            vec!["date", "time"]
        );
    }

    /// Labelled from the serial-comma findings on 164 eval responses (20 October 2026):
    /// real serial commas, then sentences the rule used to flag wrongly.
    #[test]
    fn serial_comma_rule_matches_labelled_eval_responses() {
        let real = [
            "Risks include `id > :cursor` pagination, sharding by `id % n`, and integer-typed fields in clients.",
            "It's your gross business income, excluding GST, input-taxed sales such as residential rent, and sales not connected to an enterprise.",
            "The client creates a random secret, sends a hash of it when the login starts, and reveals the secret itself when it redeems the code.",
            "The file may never have been created, it may be gitignored, or the build may be resolving the path wrongly.",
            "Re-key caches with a dual-read: try the UUID key first, then the integer key, and write the UUID key.",
            "It excludes GST-free sales such as basic food, input-taxed sales, and sales not connected with your business.",
            "It's expired, revoked, mistyped, or issued for a different environment.",
            "Use whatever your other tests use, such as a fixture, a test helper that signs a JWT, or a seeded test user's login.",
            "Add the UUID next to the integer, write both, move readers one at a time behind flags, and remove the integer at the end.",
            "So I assumed PostgreSQL, a Kafka-style stream with a schema registry, and a REST API with external clients.",
            "Wait until the gate has passed, v1 is sunset, event retention has expired, and the longest token lifetime has passed.",
            // A list after a colon, and a list of 'wh' clauses, are lists (held-out responses).
            "Store the nonce locally: in session storage for an SPA, secure storage for a mobile app, or the server-side session.",
            "The paragraph never says what the platform does, who it's for, or what result it produces.",
        ];
        for sentence in real {
            assert_eq!(
                rules(sentence),
                vec!["serial-comma"],
                "should flag: {sentence}"
            );
        }
        let not_lists = [
            "In Australia you must register once your GST turnover reaches $75,000, and $68,000 is under that.",
            "This includes taxi and ride-sourcing drivers (for example, Uber), and anyone claiming fuel tax credits.",
            "Find the entry from just before the operation (for example, `feature@{2}`, or the line before it).",
            "The working directory is an empty temporary folder, not a git repository, and nothing names a project.",
            "Without a secret, the authorisation code is the only protection, and the code can leak in several ways.",
            "The authorisation server signs the user in, which may include multi-factor authentication, and asks them to approve.",
            "It is a single file. 10,000 notes is a small load for it, and its FTS5 extension gives you fast search.",
            "For example, a payment entered twice in the ledger, or a receipt left out.",
            "You made 2 commits (C, D), and meanwhile someone added E to `main`.",
            "In the original flow, the AS returns a short-lived code via redirect, and the client exchanges it for tokens.",
            "They do not have the verifier, which was never sent anywhere until step 6, and step 6 goes straight to the AS.",
            "Use base64url, not standard base64, and strip the padding.",
            "Which project should I deploy (for example, `site`), and how do you normally deploy it?",
            "Check for standard base64 (`+`, `/`, `=`) instead of base64url, or hashing a hex string.",
            "The threshold is $75,000 ($150,000 for non-profit organisations), and $68,000 is below it.",
            "For public clients, the code itself was the only credential, and it travels through the browser.",
        ];
        for sentence in not_lists {
            assert!(rules(sentence).is_empty(), "should not flag: {sentence}");
        }
    }

    #[test]
    fn flags_a_space_beside_a_number_symbol() {
        for text in [
            "Prices rose 5 %.",
            "A rise of 0.4 % over the year.",
            "Up by 5\u{a0}%, then down.",
            "Costs (up 1,000 %) rose.",
            "A band of 5-10 % applies.",
            "| 12 % |",
            "It fell -3 %!",
            "It cost $ 27.99.",
            "It cost $ 1,000, then rose.",
            "It cost US$ 20,000 in all.",
            "It cost **$ 5** at most.",
            "A 1200 % rise.",
        ] {
            assert_eq!(rules(text), vec!["symbol-space"], "{text}");
        }
        assert_eq!(rules("From 5 % to $ 27.99."), vec!["symbol-space"; 2]);
        let found = lint("Caf\u{e9} rose 5 %.");
        assert_eq!((found[0].line, found[0].column), (1, 13));
        assert_eq!(
            found[0].message,
            "no space before the percentage sign: write 12%"
        );
        assert_eq!(
            lint("It cost $ 27.99.")[0].message,
            "no space after a currency symbol: write $27.99"
        );
        for text in [
            "Prices rose 5% to $27.99, or US$20,000 for 20c.",
            "The remainder of 5 % 3 is 2.",
            "The 2020 % change was small.",
            "The Q4 % column and the CPU % column.",
            "Print 5 %d items.",
            "A $ sign, the US$ and $ signs.",
            "Solve $ 2x + 1 = 3 $ for x.",
            "Close the sum with $ (1) as its number.",
            "Set `x = 5 %` and `$ 27.99`.",
            "She quoted '5 %' and '$ 27.99' as faults.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
    }

    #[test]
    fn flags_decades_with_an_apostrophe_but_not_a_year_possessive() {
        for text in [
            "The 1980's were loud.",
            "In the 1980\u{2019}s music was loud.",
            "It began in the mid-1980's.",
            "A 1980's-style fit.",
            "By the late 1990's prices fell.",
            "It dates from 1980's.",
            "After 1980's, it changed.",
        ] {
            assert_eq!(rules(text), vec!["date"], "{text}");
        }
        assert_eq!(
            lint("The 1980's were loud.")[0].message,
            "decades take no apostrophe: write 1980s"
        );
        for text in [
            "The 1980s were loud.",
            "In the mid-1980s and the '80s, it was loud.",
            "2020's budget raised the rate.",
            "It matched 1990's growth.",
            "The 2021's total is wrong, but not in this rule.",
            "The 198's and 19800's are not years.",
            "Quote '1980's' in the guide.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
    }

    #[test]
    fn flags_an_ordinal_joined_to_its_month_by_of() {
        for text in [
            "Lodge it by the 21st of September.",
            "Lodge it by the **21st** of September 2026.",
            "The 1st of May was a Friday.",
            "It fell on the 1st May.",
            "It fell on 1st May 2026.",
            "The 1st May meeting went ahead.",
        ] {
            assert_eq!(rules(text), vec!["date"], "{text}");
        }
        for text in [
            "Lodge it by 21 September.",
            "She will leave by the 20th.",
            "On the 21st of the month.",
            "A 3rd of them may fail.",
            "Fear of Friday the 13th may be rooted in belief.",
            "The 13th may not come.",
            "The 13th may fall on a Friday.",
            "The 2nd of 3 reports came in September.",
            "Of the 21 of us, 5 left in September.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
    }

    /// Findings of one rule as (line, column, message).
    fn of(rule: RuleId, text: &str) -> Vec<(usize, usize, String)> {
        lint(text)
            .into_iter()
            .filter(|f| f.rule == rule)
            .map(|f| (f.line, f.column, f.message))
            .collect()
    }

    #[test]
    fn flags_a_compound_written_closed_and_hyphenated_in_one_document() {
        let found = |text: &str| of(RuleId::HyphenConsistency, text);
        assert!(RuleId::HyphenConsistency.rule().advisory);
        // Across paragraphs, once, where the second form first appears.
        assert_eq!(
            found("A healthcare plan.\n\nA health-care card and a health-care fund."),
            vec![(
                3,
                3,
                "'health-care' here and 'healthcare' on line 1 (advisory); use one form unless they are different words".to_owned()
            )]
        );
        assert_eq!(
            found("Health-care first.\n\nThen Healthcare.")[0].0,
            3,
            "either order, any case"
        );
        // The hyphen rule keeps its own forms.
        assert_eq!(rules("We e-mailed and then emailed."), ["hyphen"]);
        for clean in [
            "A health-care card and a health-care fund.",
            "Long-term plans for the long term.",
            "An e-book and an ebook.",
            "An up-to-date list and an upto list.",
            "The `health-care` key in code and healthcare in prose.",
            "She said 'health-care' once; we write healthcare.",
            "Cartridges 4 Planet Ark, planetark.org/programs/cartridges-4-planet-ark.",
            "Write to planet-ark@example.org or planetark@example.org.",
            "See docs/health-care for healthcare.",
            "Follow @health-care for healthcare news.",
        ] {
            assert!(found(clean).is_empty(), "{clean}: {:?}", found(clean));
        }
    }

    #[test]
    fn flags_an_acronym_used_before_its_definition() {
        let found = |text: &str| of(RuleId::AcronymBeforeDefinition, text);
        assert!(RuleId::AcronymBeforeDefinition.rule().advisory);
        assert_eq!(
            found("The BOM issued a warning.\n\nThe Bureau of Meteorology (BOM) said so."),
            vec![(
                1,
                5,
                "'BOM' is used before its definition on line 3 (advisory); define it at first use"
                    .to_owned()
            )]
        );
        assert_eq!(
            found("BOM and BOM. The Bureau of Meteorology (BOM).").len(),
            1,
            "one finding however often it is used first"
        );
        // Singular and plural are one acronym, whichever comes first.
        for plural in [
            "NGOs met. Non-government organisations (NGOs) met.",
            "NGOs were consulted. The non-government organisation (NGO) responded.",
            "The NGO met. Non-government organisations (NGOs) met.",
        ] {
            assert_eq!(found(plural).len(), 1, "{plural}");
        }
        assert_eq!(
            found("The ABS counts. The Australian Bureau of Statistics (ABS) counts.").len(),
            1
        );
        for clean in [
            "The Bureau of Meteorology (BOM) said so. The BOM agreed.",
            "The BOM agreed.",
            "DNA (deoxyribonucleic acid) is a molecule.",
            "The `BOM` key, then the Bureau of Meteorology (BOM).",
            // An earlier use of the acronym ends the words that may spell it.
            "BOM was mentioned. It was discussed (BOM).",
            "NGOs were mentioned. It was discussed (NGO).",
            "Plan A. The Australian (A) league.",
            // A parenthesised label the words before it do not spell is not a definition.
            "Vale is MIT. The fork is by Ayoub G. (MIT).",
            "Level AAA applies. Reading level (AAA) is stricter.",
        ] {
            assert!(found(clean).is_empty(), "{clean}: {:?}", found(clean));
        }
    }

    #[test]
    fn a_suppression_silences_its_rules_on_the_next_non_blank_line_only() {
        let text = "<!-- laconic-check-disable-next-line us-spelling, em-dash -->\n\n\
                    The color changed \u{2014} slowly, e.g. today.\n\nThe color stayed.";
        assert_eq!(rules(text), ["latin", "us-spelling"]);
        assert_eq!(lint_counting(text).1, 2);
        // Stacked suppressions share the next line; space-separated names work too.
        let stacked = "<!-- laconic-check-disable-next-line us-spelling -->\n\
                       <!--laconic-check-disable-next-line latin em-dash-->\n\
                       The color changed \u{2014} slowly, e.g. today.";
        assert_eq!(lint_counting(stacked), (Vec::new(), 3));
        // Paragraph and document findings are silenced on the line they are reported on.
        let later = "The BOM met.\n\n\
                     <!-- laconic-check-disable-next-line acronym-before-definition -->\n\
                     The Bureau of Meteorology (BOM) met.";
        assert_eq!(
            rules(later),
            ["acronym-before-definition", "unused-suppression"]
        );
        // In code it is an example, not a suppression.
        let code = "```\n<!-- laconic-check-disable-next-line us-spelling -->\n```\nThe color.";
        assert_eq!(rules(code), ["us-spelling"]);
        // Without a space after the keyword it is an ordinary comment.
        let joined = "<!-- laconic-check-disable-next-lineus-spelling -->\nThe color.";
        assert_eq!(rules(joined), ["us-spelling"]);
    }

    #[test]
    fn suppression_examples_in_indented_code_do_not_silence_prose() {
        for indent in ["    ", "        ", "\t", "  \t"] {
            let text = format!(
                "{indent}<!-- laconic-check-disable-next-line us-spelling -->\n\nThe color is red."
            );
            let (findings, suppressed) = lint_counting(&text);
            assert_eq!(suppressed, 0, "{indent:?}");
            assert_eq!(findings.len(), 1, "{indent:?}: {findings:?}");
            assert_eq!(findings[0].rule, RuleId::UsSpelling);
            assert_eq!(findings[0].line, 3);
        }
        // Up to 3 spaces leave a comment outside a code block.
        let text = "   <!-- laconic-check-disable-next-line us-spelling -->\nThe color is red.";
        assert_eq!(lint_counting(text), (Vec::new(), 1));
        // An indented comment within a list remains a suppression.
        let text = "- The colour is red.\n\n    <!-- laconic-check-disable-next-line us-spelling -->\n    The color is red.";
        assert_eq!(lint_counting(text), (Vec::new(), 1));
        for (item, content_indent) in [("- ", 2), ("12. ", 4)] {
            let code = " ".repeat(content_indent + 4);
            let prose = " ".repeat(content_indent);
            let text = format!(
                "{item}The colour is red.\n\n{code}<!-- laconic-check-disable-next-line us-spelling -->\n\n{prose}The color is red."
            );
            let (findings, suppressed) = lint_counting(&text);
            assert_eq!(suppressed, 0, "{item:?}");
            assert_eq!(findings.len(), 1, "{item:?}: {findings:?}");
            assert_eq!(findings[0].rule, RuleId::UsSpelling);
            assert_eq!(findings[0].line, 5);
        }
    }

    #[test]
    fn flags_a_suppression_that_silences_nothing() {
        let unused = |text: &str| of(RuleId::UnusedSuppression, text);
        assert!(RuleId::UnusedSuppression.rule().advisory);
        assert_eq!(
            unused("  <!-- laconic-check-disable-next-line us-spellings em-dash -->\nThe color."),
            [
                (1, 3, "'us-spellings' is not a rule, so the suppression silences nothing (advisory)".to_owned()),
                (1, 3, "no 'em-dash' finding on line 2 to silence (advisory); remove it from the suppression".to_owned()),
            ]
        );
        assert_eq!(
            rules("<!-- laconic-check-disable-next-line us-spellings -->\nThe color."),
            ["unused-suppression", "us-spelling"]
        );
        assert_eq!(
            unused("Text.\n\n<!-- laconic-check-disable-next-line -->\nThe colour."),
            [(3, 1, "the suppression names no rule (advisory)".to_owned())]
        );
        assert_eq!(
            unused("<!-- laconic-check-disable-next-line latin -->\n"),
            [(
                1,
                1,
                "no line follows for the suppression of 'latin' (advisory)".to_owned()
            )]
        );
        // Other comments are not suppressions.
        assert!(unused("<!-- laconic-check-disable-next-line latin\nThe colour.").is_empty());
        assert!(unused("<!-- prettier-ignore -->\nThe colour.").is_empty());
    }

    #[test]
    fn document_findings_follow_the_findings_on_their_line() {
        assert_eq!(
            rules("The BOM color.\n\nThe Bureau of Meteorology (BOM)."),
            ["us-spelling", "acronym-before-definition"]
        );
        assert_eq!(
            rules("The BOM.\n\nThe color.\n\nThe Bureau of Meteorology (BOM)."),
            ["acronym-before-definition", "us-spelling"]
        );
    }

    #[test]
    fn flags_a_run_of_3_bold_label_bullets_as_advice() {
        let at = |text: &str| -> Vec<(usize, usize, &'static str)> {
            lint(text)
                .into_iter()
                .map(|f| (f.line, f.column, f.rule.name()))
                .collect()
        };
        assert!(RuleId::BoldLabel.rule().advisory);
        let three = "- **Cause:** a bad key.\n- **Fix:** replace it.\n- **Risk:** low.";
        assert_eq!(at(three), vec![(3, 1, "bold-label")]);
        // Each marker, the colon outside the bold, and one finding however long the run.
        assert_eq!(
            at("* **A**: x\n+ **B**: y\n- **C:** z\n- **D:** w\n- **E:** v"),
            vec![(3, 1, "bold-label")]
        );
        // Wrapped lines, blank lines, indented paragraphs and nested bullets stay in the run.
        assert_eq!(
            at("- **A:** x\n  wrapped\n\n- **B:** y\n\n  More on B.\n\n  - detail\n- **C:** z"),
            vec![(9, 1, "bold-label")]
        );
        // A nested list has its own run, and each list its own finding.
        assert_eq!(
            at("- Parent\n  - **A:** x\n  - **B:** y\n  - **C:** z"),
            vec![(4, 3, "bold-label")]
        );
        assert_eq!(at(&format!("{three}\n\nBetween.\n\n{three}")).len(), 2);
        for text in [
            "- **A:** x\n- **B:** y",
            "- **A** x\n- **B** y\n- **C** z",
            "- A: x\n- B: y\n- C: z",
            "- ****: x\n- **  **: y\n- ****: z",
            "1. **A:** x\n2. **B:** y\n3. **C:** z",
            "**A:** x\n\n**B:** y\n\n**C:** z",
            "- **A:** x\n- **B:** y\n- plain\n- **C:** z",
            "- **A:** x\n- **B:** y\n\nBetween.\n\n- **C:** z",
            "- **A:** x\n- **B:** y\n# Heading\n- **C:** z",
            "> - **A:** x\n> - **B:** y\n> - **C:** z",
            "```\n- **A:** x\n- **B:** y\n- **C:** z\n```",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
    }

    #[test]
    fn flags_a_weekday_that_does_not_match_its_date() {
        let found = lint("Reply by Wednesday 22 October 2026, please.");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, RuleId::Weekday);
        assert_eq!(found[0].column, 10);
        assert_eq!(
            found[0].message,
            "22 October 2026 is a Thursday, not a Wednesday"
        );
        assert_eq!(rules("Lodge by Friday, 31 October 2026."), vec!["weekday"]);
        // Correct weekdays, leap days and dates without a year or weekday pass.
        assert!(rules("Reply by Thursday 22 October 2026.").is_empty());
        assert!(rules("It was Tuesday 29 February 2028.").is_empty());
        assert!(rules("Reply by Wednesday 22 October.").is_empty());
        assert!(rules("Reply by 22 October 2026.").is_empty());
        // A date that does not exist is not checked against a weekday.
        assert!(rules("Monday 30 February 2026 is not a date.").is_empty());
    }

    #[test]
    fn a_date_ends_at_clause_punctuation() {
        for sentence in [
            "Peers were rechecked since 20 October; 14 new.",
            "It shipped in October: 12 fixes.",
            "We met in October. 3 of us stayed.",
            "Was it October? 2 people thought so.",
            "Not in October! 2 weeks later.",
            "Reply by Wednesday 22 October; 2026 will be busy.",
            "Since 20 **October;** 14 new.",
            "(Since 20 October;) 14 new.",
        ] {
            assert!(rules(sentence).is_empty(), "should not flag: {sentence}");
        }
        let found = lint("Peers since October 20; 14 new.");
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].rule.name(), found[0].column), ("date", 13));
    }

    #[test]
    fn weekday_follows_the_gregorian_calendar() {
        assert_eq!(weekday(2026, 10, 19), Some(1));
        assert_eq!(weekday(2000, 1, 1), Some(6));
        assert_eq!(weekday(1900, 3, 1), Some(4));
        assert_eq!(weekday(2024, 2, 29), Some(4));
        assert_eq!(weekday(2025, 2, 29), None);
        assert_eq!(weekday(2026, 13, 1), None);
        assert_eq!(
            rules("Use e.g. a token, i.e. the key."),
            vec!["latin", "latin"]
        );
        assert!(
            rules("Meet on 19 October 2026 at 9:30 am or 5 pm. It may 5 times fail.").is_empty()
        );
    }

    #[test]
    fn hyphenated_numbers_and_introductory_clauses_are_not_findings() {
        assert!(rules("We funded forty-five places.").is_empty());
        assert!(rules("If the tool is available, run it, and fix each finding.").is_empty());
    }

    #[test]
    fn flags_chat_tool_residue_but_not_markers_named_in_code() {
        assert_eq!(
            rules("See [the guide](https://example.com/?utm_source=chatgpt.com) now."),
            vec!["residue"]
        );
        assert_eq!(rules("Done\u{200b} today."), vec!["residue"]);
        assert_eq!(
            rules("Cited :contentReference[oaicite:0] here."),
            vec!["residue", "residue"]
        );
        assert!(rules("\u{feff}Scan for `oaicite` and `utm_source=` before sending.").is_empty());
    }

    #[test]
    fn flags_hidden_tag_runs_and_bidi_controls_but_not_subdivision_flags() {
        // 'ignore' spelt in tag characters: one finding for the run.
        let hidden: String = "ignore"
            .chars()
            .map(|c| char::from_u32(0xe0000 + c as u32).unwrap())
            .collect();
        let found = lint(&format!("Approve it.{hidden} Thanks."));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].column, 12);
        assert_eq!(
            found[0].message,
            "6 hidden tag characters (U+E0000 to U+E007F)"
        );
        assert_eq!(
            rules("Total \u{202e}cba\u{202c} due."),
            vec!["residue", "residue"]
        );
        assert_eq!(
            rules("Name: \u{2067}abc\u{2069}."),
            vec!["residue", "residue"]
        );
        // England's flag, and the same flag carrying a longer hidden message.
        let england = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}";
        let scotland = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}";
        assert!(rules(&format!("Go {england} or {scotland} today.")).is_empty());
        // A hidden word in the shape of a flag is still flagged, however short.
        for smuggled in [
            format!("\u{1f3f4}{hidden}\u{e007f}"),
            format!("\u{1f3f4}{hidden}{hidden}\u{e007f}"),
        ] {
            assert_eq!(rules(&format!("Go {smuggled} today.")), vec!["residue"]);
        }
        assert_eq!(rules("A lone \u{e007f} cancel tag."), vec!["residue"]);
    }

    #[test]
    fn reports_line_and_column() {
        let f = &lint("ok\nThe color.")[0];
        assert_eq!((f.line, f.column, f.rule.name()), (2, 5, "us-spelling"));
    }

    #[test]
    fn finds_phrases_split_by_a_line_wrap() {
        let at = |text: &str| -> Vec<(usize, usize, &'static str)> {
            lint(text)
                .into_iter()
                .map(|f| (f.line, f.column, f.rule.name()))
                .collect()
        };
        // Present day, present time: the wrap falls inside each phrase.
        assert_eq!(
            at("Present day, present time. Hope this\nhelps."),
            vec![(1, 28, "closer")]
        );
        assert_eq!(
            at("The logs, metrics,\n  and traces."),
            vec![(1, 18, "serial-comma")]
        );
        assert_eq!(
            at("Ship it\non\n\"Friday\"."),
            vec![(3, 1, "double-quotes")]
        );
        assert_eq!(at("A game\nchanger."), vec![(1, 3, "stock-phrase")]);
        // Blank lines, list items, headings, table rows and fences end a paragraph.
        for text in [
            "Hope this\n\nhelps.",
            "- logs, metrics\n- and traces",
            "# Hope this\nhelps",
            "Hope this\n# helps",
            "| logs, metrics |\n| and traces |",
            "Hope this\n```\nhelps\n```",
            // Hard breaks, thematic breaks, setext underlines and indented code too.
            "Hope this  \nhelps.",
            "Hope this\\\nhelps.",
            "Hope this\n***\nhelps.",
            "A heading\n=========\nhelps.",
            "Prose.\n\n    Hope this\n    helps.",
        ] {
            assert!(rules(text).is_empty(), "{text}: {:?}", rules(text));
        }
        // Indented code is still checked line by line, as before; list continuations,
        // nested or after a blank line, still join.
        assert_eq!(at("Prose.\n\n    color"), vec![(3, 5, "us-spelling")]);
        assert_eq!(at("- Hope this\n  helps."), vec![(1, 3, "closer")]);
        assert_eq!(
            at("- item\n\n    Hope this\n    helps."),
            vec![(3, 5, "closer")]
        );
        assert_eq!(
            at("- item\n  - Hope this\n    helps."),
            vec![(2, 5, "closer")]
        );
    }
}
