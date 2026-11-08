// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Seeded-fault evaluation of `lint`, after snifftest's design
//! (https://github.com/DanRWilloughby/snifftest): a corpus of clean Australian prose,
//! including legitimate forms that look like faults, must produce no findings, and
//! planting one known fault in it must produce exactly that rule, once. The rule reference,
//! `docs/rules.md`, is generated here from the rule table, the seeds and the human corpus.

use crate::lint::{RULES, lint};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

/// Clean prose. Each sentence is correct house style; several are near misses.
const CLEAN: &[&str] = &[
    "The team organised 3 workshops and analysed the results in October.",
    "Bring the logs, metrics and traces to the review.",
    "The industries most affected are retail trade, wholesale trade, and accommodation and food services.",
    "If the build fails, rerun it, and report the error.",
    "We funded forty-five places after the go-ahead.",
    "Doctors practise medicine; the practice opens at 9:30 am on 19 October 2026.",
    "Read the meter before you change the program in the dialog box.",
    "The Labor senator spoke for one minute about the 2026-27 budget.",
    "Prices rose 12% whilst demand fell amongst small businesses, and stayed low for a while.",
    "She said 'the licence is current' and left for the centre.",
    "Two reviewers checked the catalogue, for example the colour plates.",
    "It may 5 times fail before it passes.",
    "The delay may affect the suppliers and possibly the buyers.",
    "Andy wrote to the state senate every week for 6 years.",
    "The patterns credit [humanizer](https://github.com/blader/humanizer) and [the house guide](https://example.com/guide).",
    "Fees were low in the 1980s, but 2020's budget took the rate to 5% and the charge to $27.99 from 21 September.",
    "- **Keep:** a single bold label.",
];

/// One planted fault: the rule it must trigger, a clean fragment and its faulty form.
const SEEDS: &[(&str, &str, &str)] = &[
    ("us-spelling", "organised", "organized"),
    ("us-spelling", "colour plates", "color plates"),
    ("us-spelling", "the centre", "the center"),
    ("us-spelling", "catalogue", "catalog"),
    ("us-spelling", "analysed", "analyzed"),
    ("us-spelling", "the review", "the skeptical review"),
    ("us-spelling", "Prices rose 12%", "Prices rose 12 percent"),
    (
        "us-spelling",
        "[the house guide]",
        "[the house color guide]",
    ),
    (
        "em-dash",
        "after the go-ahead.",
        "after the go-ahead \u{2014} finally.",
    ),
    (
        "double-quotes",
        "'the licence is current'",
        "\"the licence is current\"",
    ),
    (
        "serial-comma",
        "logs, metrics and traces",
        "logs, metrics, and traces",
    ),
    (
        "number-word",
        "organised 3 workshops",
        "organised three workshops",
    ),
    (
        "closer",
        "to the review.",
        "to the review. Hope this helps!",
    ),
    (
        "stock-phrase",
        "to the review.",
        "to the review. Bottom line: it works.",
    ),
    (
        "staged-contrast",
        "Bring the logs, metrics and traces to the review.",
        "It's not a log review, it's a trace review.",
    ),
    (
        "staged-contrast",
        "to the review.",
        "to the review, not just a quick look but a full audit.",
    ),
    (
        "reveal-question",
        "Two reviewers checked",
        "The result? Two reviewers checked",
    ),
    (
        "clause-dashes",
        "and stayed low for a while.",
        "and stayed low - for a while. Supply rose - slowly.",
    ),
    (
        "stacked-hedge",
        "delay may affect",
        "delay could potentially affect",
    ),
    (
        "bold-label",
        "Bring the logs, metrics and traces to the review.",
        "- **Logs:** bring them.\n- **Metrics:** bring them.\n- **Traces:** bring them.",
    ),
    ("hyphen", "the program", "the open-source program"),
    ("hyphen", "left for the centre", "e-mailed the centre"),
    ("word-choice", "whilst demand", "while demand"),
    ("word-choice", "amongst small", "among small"),
    ("date", "on 19 October 2026", "on 19th October 2026"),
    ("date", "on 19 October 2026", "on 19 October, 2026"),
    ("date", "on 19 October 2026", "on October 19"),
    ("date", "from 21 September", "from the 21st of September"),
    ("date", "in the 1980s", "in the 1980's"),
    (
        "weekday",
        "on 19 October 2026",
        "on Tuesday 19 October 2026",
    ),
    ("symbol-space", "the rate to 5%", "the rate to 5 %"),
    (
        "symbol-space",
        "the charge to $27.99",
        "the charge to $ 27.99",
    ),
    ("time", "at 9:30 am", "at 9.30am"),
    ("time", "at 9:30 am", "at 9:30 a.m."),
    ("latin", "for example the colour", "e.g. the colour"),
    (
        "residue",
        "after the go-ahead.",
        "after the go-ahead.\u{200b}",
    ),
    (
        "residue",
        "https://example.com/guide",
        "https://example.com/guide?utm_source=chatgpt.com",
    ),
];

fn corpus() -> String {
    CLEAN.join("\n\n")
}

fn rules(text: &str) -> Vec<&'static str> {
    lint(text).into_iter().map(|f| f.rule.name()).collect()
}

#[test]
fn clean_corpus_has_no_findings() {
    for sentence in CLEAN {
        assert!(
            rules(sentence).is_empty(),
            "false finding in {sentence:?}: {:?}",
            lint(sentence)
        );
    }
}

#[test]
fn every_seeded_fault_is_caught_exactly_once() {
    let clean = corpus();
    let mut failures = Vec::new();
    for &(rule, from, to) in SEEDS {
        assert_eq!(
            clean.matches(from).count(),
            1,
            "seed fragment {from:?} must occur once"
        );
        let found = rules(&clean.replacen(from, to, 1));
        if found != [rule] {
            failures.push(format!("{rule}: {to:?} gave {found:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} seeds missed or over-flagged:\n{}",
        failures.len(),
        SEEDS.len(),
        failures.join("\n")
    );
}

/// docs/rules.md shows each seed's faulty form as an example, so it must flag its rule alone.
/// An example with invisible characters is printed with escapes and labelled as such.
#[test]
fn every_example_flags_its_rule_on_its_own() {
    let missed: Vec<String> = SEEDS
        .iter()
        .filter(|(rule, _, to)| !rules(to).contains(rule))
        .map(|(rule, _, to)| format!("{rule}: {to:?}"))
        .collect();
    assert!(
        missed.is_empty(),
        "examples that do not flag their rule alone:\n{}",
        missed.join("\n")
    );
}

/// Human Australian Government prose in tests/corpus: 3 Productivity Commission reports
/// released in 2018 and 2021, about 198,500 words under CC BY 3.0 AU (sources in its NOTICE).
const HUMAN_CORPUS: &[&str] = &[
    "rising-inequality-2018.md",
    "right-to-repair-2021.md",
    "vulnerable-supply-chains-2021.md",
];
/// Findings on the human corpus. Most follow the Commission's house style, which Laconic's
/// overrides: spaced em dashes, words for numbers below 10, 'while' and 'among', and serial
/// commas. The ceilings only move down; they guard against a net increase on this corpus,
/// not against every new false finding.
const HUMAN_CORPUS_CEILING: usize = 2246;
/// Advisory findings have their own ceiling: 41 paragraphs put clause dashes in 2 or more
/// sentences when the corpus was calibrated on 2 November 2026, and the stacked-hedge rule
/// added 8 human uses on 4 November ('could potentially' 5 times, 'can potentially' twice,
/// 'may potentially' once).
const HUMAN_CORPUS_ADVISORY_CEILING: usize = 49;
/// AI-tell findings at calibration, all human uses: 'it is important to note' 3 times,
/// 'innovative', 'deep dive', 'not just X, but Y' and 'time-frame'.
const HUMAN_CORPUS_TELL_CEILING: usize = 7;

fn human_corpus() -> Vec<String> {
    HUMAN_CORPUS
        .iter()
        .map(|name| {
            let path = format!("{}/tests/corpus/{name}", env!("CARGO_MANIFEST_DIR"));
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
        })
        .collect()
}

#[test]
fn human_corpus_findings_stay_under_the_ceiling() {
    let (advisory, findings): (Vec<_>, Vec<_>) = human_corpus()
        .iter()
        .flat_map(|text| lint(text))
        .partition(|f| f.rule.rule().advisory);
    assert!(
        findings.len() <= HUMAN_CORPUS_CEILING,
        "{} findings on the human corpus, ceiling {HUMAN_CORPUS_CEILING}",
        findings.len()
    );
    assert!(
        advisory.len() <= HUMAN_CORPUS_ADVISORY_CEILING,
        "{} advisory findings on the human corpus, ceiling {HUMAN_CORPUS_ADVISORY_CEILING}",
        advisory.len()
    );
    let tells: Vec<_> = findings
        .iter()
        .filter(|f| {
            matches!(
                f.rule.name(),
                "stock-phrase" | "staged-contrast" | "reveal-question" | "hyphen"
            )
        })
        .collect();
    assert!(
        tells.len() <= HUMAN_CORPUS_TELL_CEILING,
        "{} AI tells in human prose, ceiling {HUMAN_CORPUS_TELL_CEILING}: {tells:?}",
        tells.len()
    );
}

#[test]
fn every_rule_has_a_seed() {
    for rule in RULES {
        assert!(
            SEEDS.iter().any(|(r, _, _)| *r == rule.name),
            "no seed for {}",
            rule.name
        );
    }
}

#[test]
fn rule_names_are_unique_and_every_seed_names_one() {
    let mut names: Vec<&str> = RULES.iter().map(|r| r.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), RULES.len(), "a rule name is listed twice");
    for (rule, _, _) in SEEDS {
        assert!(
            names.contains(rule),
            "a seed names an unlisted rule: {rule}"
        );
    }
}

const REFERENCE: &str = "docs/rules.md";
const STYLE_RULES: &str = "skills/laconic-prose/references/style-manual-rules.md";
const PATTERNS: &str = "skills/laconic-prose/references/patterns.md";
const ISSUE_FORM: &str = "https://github.com/yungSNAT/Laconic/issues/new?template=";
const OUTSIDE: &str = "tools/laconic-check/tests/outside-corpus.tsv";
const CALIBRATE: &str =
    "cargo run --manifest-path tools/laconic-tools/Cargo.toml -- calibrate <human_detectors.json>";
/// The row of outside-corpus totals: articles and words per split, then local words.
const TOTALS: &str = "(articles and words)";
/// Advisory rules that predate the 5% gate: clause-dashes flags about a fifth of the
/// held-out articles.
const GRANDFATHERED: &[&str] = &["clause-dashes"];

/// tests/outside-corpus.tsv by row name. Each row holds calibration articles and findings,
/// held-out articles and findings, and findings on tests/corpus; the totals row holds
/// articles and words in the same places.
fn outside() -> BTreeMap<String, [usize; 5]> {
    read(OUTSIDE)
        .lines()
        .filter(|l| !l.starts_with('#'))
        .skip(1)
        .map(|line| {
            let mut cells = line.split('\t');
            let name = cells.next().unwrap_or_default().to_owned();
            let counts: Vec<usize> = cells
                .map(|c| {
                    c.parse()
                        .unwrap_or_else(|_| panic!("{OUTSIDE}: bad count in {line:?}"))
                })
                .collect();
            let counts = counts
                .try_into()
                .unwrap_or_else(|_| panic!("{OUTSIDE}: expected 5 counts in {line:?}"));
            (name, counts)
        })
        .collect()
}

fn repo_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// A repository file, with Windows line endings folded.
fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// GitHub's anchor for a Markdown heading: lower case, with spaces as hyphens and other
/// punctuation removed.
fn anchor(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

fn headings(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|l| {
            l.trim_start_matches('#')
                .strip_prefix(' ')
                .filter(|_| l.starts_with('#'))
        })
        .collect()
}

/// The `###` sections under 'Australian English mechanics' in style-manual-rules.md.
fn style_manual_sections() -> Vec<String> {
    let text = read(STYLE_RULES);
    let mechanics = text
        .split("\n## Australian English mechanics\n")
        .nth(1)
        .expect("style-manual-rules.md keeps its mechanics section");
    let mechanics = mechanics.split("\n## ").next().unwrap_or_default();
    mechanics
        .lines()
        .filter_map(|l| l.strip_prefix("### "))
        .map(str::to_owned)
        .collect()
}

/// A seed with its invisible characters escaped.
fn shown(example: &str) -> String {
    example
        .chars()
        .map(|c| match c {
            '\n' => "\n".to_owned(),
            '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2069}'
            | '\u{feff}'
            | '\u{e0000}'..='\u{e007f}' => format!("\\u{{{:x}}}", c as u32),
            c if c.is_control() => format!("\\u{{{:x}}}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

/// 198512 as '198,512'.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// docs/rules.md, built from RULES, SEEDS and the human corpus.
fn reference() -> String {
    let corpus = human_corpus();
    let words: usize = corpus.iter().map(|t| t.split_whitespace().count()).sum();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in corpus.iter().flat_map(|t| lint(t)) {
        *counts.entry(finding.rule.name()).or_default() += 1;
    }
    let human = |name: &str| {
        let n = counts.get(name).copied().unwrap_or(0);
        let rate = n as f64 * 1000.0 / words as f64;
        // A rare finding would round to 0.00, which reads as none.
        let rate = if n > 0 && rate < 0.005 {
            "under 0.01".to_owned()
        } else {
            format!("{rate:.2}")
        };
        (n, rate)
    };
    let outside = outside();
    let totals = outside[TOTALS];
    let outside_rule = |name: &str| outside.get(name).copied().unwrap_or_default();
    let style_rules = format!("../{STYLE_RULES}");
    let section_link = |s: &str| format!("[{s}]({style_rules}#{})", anchor(s));
    let fails = |advisory: bool| if advisory { "no (advisory)" } else { "yes" };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "<!-- Generated by `cargo test` in tools/laconic-check from the rule table in src/lint.rs, the seeds in src/seeded_faults.rs and the corpus in tests/corpus. To update it, run `LACONIC_WRITE_DOCS=1 cargo test rule_reference_is_current`. -->\n\n\
         # laconic-check rules\n\n\
         `laconic-check lint` reports each finding as `file:line:column: rule: message`. Each rule below is headed by its name, so `docs/rules.md#<rule>` links to it. A finding is a reason to look, not proof of a defect. Advisory rules never fail `lint`.\n\n\
         Each example is a seeded fault: the tests plant it in clean prose, and `lint` must flag it exactly once. The human prose figures count findings in 3 Productivity Commission reports released in 2018 and 2021 ({} whitespace-separated words of Australian Government prose under CC BY 3.0 AU, in `tools/laconic-check/tests/corpus`). Most of them are house-style differences the Commission's own style allows, such as spaced dashes, words for numbers below 10 and serial commas, rather than errors.\n\n\
         The outside figures come from the {} human-written news articles in `human_detectors.json` from jenna-russell/human_detectors at commit `afcf03d` ({} words of mostly American prose, not stored here because the texts carry no licence). `laconic-tools calibrate` lints them from a local download. Articles with odd prompt ids are the calibration half, for tuning rules; the even half is held out, and the table shows how many of those {} articles each rule flags. A new advisory rule must flag under 5% of them.\n\n\
         To report a finding that should not have been flagged, use the [false-positive form]({ISSUE_FORM}false-positive.yml).\n\n\
         | Rule | Fails `lint` | Style Manual section | Human prose, per 1,000 words | Outside articles flagged |\n\
         | --- | --- | --- | --- | --- |",
        thousands(words),
        totals[0] + totals[2],
        thousands(totals[1] + totals[3]),
        totals[2]
    );
    for rule in RULES {
        let _ = writeln!(
            out,
            "| [`{0}`](#{0}) | {1} | {2} | {3} | {4} of {5} |",
            rule.name,
            fails(rule.advisory),
            rule.section.unwrap_or("none"),
            human(rule.name).1,
            outside_rule(rule.name)[2],
            totals[2]
        );
    }
    let _ = writeln!(
        out,
        "\n## Style Manual coverage\n\n\
         `lint` checks some forms from each section it names in [style-manual-rules.md]({style_rules}). The rest of those sections, and every section marked skill only, is applied by the laconic-prose skill, not the checker.\n\n\
         | Section | Checked in part by |\n\
         | --- | --- |"
    );
    for section in style_manual_sections() {
        let rules: Vec<String> = RULES
            .iter()
            .filter(|r| r.section == Some(section.as_str()))
            .map(|r| format!("`{}`", r.name))
            .collect();
        let checked = if rules.is_empty() {
            "skill only".to_owned()
        } else {
            rules.join(", ")
        };
        let _ = writeln!(out, "| {} | {checked} |", section_link(&section));
    }
    let others: Vec<String> = RULES
        .iter()
        .filter(|r| r.section.is_none())
        .map(|r| format!("`{}`", r.name))
        .collect();
    let _ = writeln!(
        out,
        "\nThe rules with no Style Manual section flag writing patterns from [patterns.md](../{PATTERNS}) or chat-tool residue: {}.\n\n## Rules",
        others.join(", ")
    );
    for rule in RULES {
        let (n, rate) = human(rule.name);
        let [ca, _, ha, hf, _] = outside_rule(rule.name);
        let section = rule
            .section
            .map_or("Not a Style Manual rule.".to_owned(), |s| {
                format!("Style Manual: {}.", section_link(s))
            });
        // A seed of several lines, such as a run of bullets, keeps them in a fenced block.
        let (blocks, inline): (Vec<String>, Vec<String>) = SEEDS
            .iter()
            .filter(|(r, _, _)| *r == rule.name)
            .map(|(_, _, to)| shown(to))
            .partition(|to| to.contains('\n'));
        let mut examples: Vec<String> = Vec::new();
        if !inline.is_empty() {
            let codes: Vec<String> = inline.iter().map(|to| format!("`{to}`")).collect();
            let label = if codes.len() == 1 {
                "Example"
            } else {
                "Examples"
            };
            examples.push(format!("{label}: {}.", codes.join(", ")));
        }
        for block in blocks {
            examples.push(format!("Example:\n\n```markdown\n{block}\n```"));
        }
        if SEEDS
            .iter()
            .any(|(r, _, to)| *r == rule.name && shown(to) != *to)
        {
            examples.push("Invisible characters appear here as `\\u{...}` escapes, so copying an example does not reproduce them; the tests plant the characters themselves.".to_owned());
        }
        let _ = writeln!(
            out,
            "\n### {}\n\n{}\n\nFails `lint`: {}. {section} Human prose: {} {}, {rate} per 1,000 words. Outside articles: flags {ha} of {} held out ({hf} {}) and {ca} of {} for calibration.\n\n{}",
            rule.name,
            rule.checks,
            fails(rule.advisory),
            thousands(n),
            if n == 1 { "finding" } else { "findings" },
            totals[2],
            if hf == 1 { "finding" } else { "findings" },
            totals[0],
            examples.join("\n\n")
        );
    }
    out
}

/// The outside counts must list every rule and come from the rules as they are now, which
/// the findings on tests/corpus stand in for, since the outside corpus is not stored here.
#[test]
fn outside_corpus_counts_are_current() {
    let outside = outside();
    let corpus = human_corpus();
    let mut local: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in corpus.iter().flat_map(|t| lint(t)) {
        *local.entry(finding.rule.name()).or_default() += 1;
    }
    let words: usize = corpus.iter().map(|t| t.split_whitespace().count()).sum();
    let mut stale: Vec<&str> = Vec::new();
    if outside.get(TOTALS).map(|r| r[4]) != Some(words) {
        stale.push(TOTALS);
    }
    for rule in RULES {
        let now = local.get(rule.name).copied().unwrap_or(0);
        if outside.get(rule.name).map(|r| r[4]) != Some(now) {
            stale.push(rule.name);
        }
    }
    assert!(
        stale.is_empty(),
        "{OUTSIDE} is out of date for {}: run {CALIBRATE}",
        stale.join(", ")
    );
    assert_eq!(
        outside.len(),
        RULES.len() + 1,
        "{OUTSIDE} has a row for a rule not in RULES: run {CALIBRATE}"
    );
}

#[test]
fn advisory_rules_flag_under_5_percent_of_held_out_articles() {
    let outside = outside();
    let held_out = outside[TOTALS][2];
    for rule in RULES
        .iter()
        .filter(|r| r.advisory && !GRANDFATHERED.contains(&r.name))
    {
        let flagged = outside[rule.name][2];
        assert!(
            flagged * 20 < held_out,
            "{} flags {flagged} of {held_out} held-out human articles; an advisory must stay under 5%",
            rule.name
        );
    }
}

/// docs/rules.md must match the rule table; `LACONIC_WRITE_DOCS=1` rewrites it.
#[test]
fn rule_reference_is_current() {
    let expected = reference();
    if std::env::var_os("LACONIC_WRITE_DOCS").is_some() {
        std::fs::write(repo_root().join(REFERENCE), &expected).unwrap();
    }
    assert!(
        read(REFERENCE) == expected,
        "{REFERENCE} is out of date: run `LACONIC_WRITE_DOCS=1 cargo test rule_reference_is_current` in tools/laconic-check"
    );
}

#[test]
fn every_rule_section_is_a_style_manual_heading() {
    let sections = style_manual_sections();
    for rule in RULES {
        if let Some(section) = rule.section {
            assert!(
                sections.iter().any(|s| s == section),
                "{}: no section {section:?} under Australian English mechanics in {STYLE_RULES}",
                rule.name
            );
        }
    }
}

#[test]
fn rule_reference_links_resolve() {
    let text = read(REFERENCE);
    let docs = repo_root().join("docs");
    let mut checked = 0;
    for target in text.split("](").skip(1).filter_map(|s| s.split(')').next()) {
        checked += 1;
        if let Some(form) = target.strip_prefix(ISSUE_FORM) {
            let path = repo_root().join(".github/ISSUE_TEMPLATE").join(form);
            assert!(path.is_file(), "{target}: no {}", path.display());
            continue;
        }
        assert!(!target.contains("://"), "unexpected external link {target}");
        let (file, fragment) = target.split_once('#').unwrap_or((target, ""));
        let body = if file.is_empty() {
            text.clone()
        } else {
            let path = docs.join(file);
            std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{target}: {}: {e}", path.display()))
        };
        if !fragment.is_empty() {
            assert!(
                headings(&body).iter().any(|h| anchor(h) == fragment),
                "{target}: no heading with anchor {fragment}"
            );
        }
    }
    assert!(checked > RULES.len(), "only {checked} links found");
}
