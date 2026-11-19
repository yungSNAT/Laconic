// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The README's translations, `README.<language>.md` beside it, carry the English README's
//! commands (its fenced code blocks, in order), links, version numbers and every number written
//! in digits. A change to any of them in README.md fails here until every translation has it too.

use super::{read, root};
use regex::Regex;
use std::fs;
use std::sync::LazyLock;

/// The translations, by file name.
fn translations() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| is_translation(name))
        .collect();
    names.sort();
    names
}

fn is_translation(name: &str) -> bool {
    name.strip_prefix("README.")
        .and_then(|rest| rest.strip_suffix(".md"))
        .is_some_and(|lang| !lang.is_empty() && lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// What a translation must carry from the English README.
#[derive(Debug, PartialEq)]
struct Carried {
    /// Fenced code blocks, fence line included, in order.
    commands: Vec<String>,
    /// Link targets, sorted, except the links between the README's languages.
    links: Vec<String>,
    /// Version numbers outside code blocks, sorted.
    versions: Vec<String>,
    /// Numbers written in digits outside code blocks and link targets, sorted.
    numbers: Vec<String>,
}

static LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\]\(([^)\s]+)(?:\s+"[^"]*")?\)|(?:src|srcset|href)="([^"]+)""#).unwrap()
});
static VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bv?\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?\b").unwrap());
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[+-]?[0-9][0-9,.]*[0-9]|[0-9]").unwrap());

fn carried(text: &str) -> Carried {
    let mut commands = Vec::new();
    let mut prose = String::new();
    let mut block: Option<String> = None;
    for line in text.lines() {
        let fence = line.trim_start().starts_with("```");
        match &mut block {
            Some(b) => {
                b.push_str(line);
                b.push('\n');
                if fence && line.trim() == "```" {
                    commands.push(block.take().unwrap());
                }
            }
            None if fence => block = Some(format!("{line}\n")),
            None => {
                prose.push_str(line);
                prose.push('\n');
            }
        }
    }
    commands.extend(block);
    let mut links: Vec<String> = LINK
        .captures_iter(&prose)
        .map(|c| c.get(1).or(c.get(2)).unwrap().as_str().to_owned())
        .filter(|target| !is_translation(target) && target != "README.md")
        .collect();
    links.sort();
    let mut versions: Vec<String> = VERSION
        .find_iter(&prose)
        .map(|m| m.as_str().to_owned())
        .collect();
    versions.sort();
    let unlinked = LINK.replace_all(&prose, "").replace('\u{2212}', "-");
    let mut numbers: Vec<String> = NUMBER
        .find_iter(&unlinked)
        .map(|m| m.as_str().to_owned())
        .collect();
    numbers.sort();
    Carried {
        commands,
        links,
        versions,
        numbers,
    }
}

/// Items of `want` missing from `got`, and items of `got` not in `want`.
fn differences(want: &[String], got: &[String]) -> (Vec<String>, Vec<String>) {
    let mut extra = got.to_vec();
    let mut missing = Vec::new();
    for w in want {
        match extra.iter().position(|g| g == w) {
            Some(i) => {
                extra.remove(i);
            }
            None => missing.push(w.clone()),
        }
    }
    (missing, extra)
}

/// Each problem with `translation` against `english`; empty when it carries everything.
fn problems(english: &str, translation: &str) -> Vec<String> {
    let (want, got) = (carried(english), carried(translation));
    let mut problems = Vec::new();
    if want.commands != got.commands {
        let at = want
            .commands
            .iter()
            .zip(&got.commands)
            .position(|(w, g)| w != g)
            .unwrap_or(want.commands.len().min(got.commands.len()));
        problems.push(format!(
            "code blocks differ at block {} of {} (translation has {}): want\n{}",
            at + 1,
            want.commands.len(),
            got.commands.len(),
            want.commands.get(at).map_or("(none)", String::as_str)
        ));
    }
    for (kind, w, g) in [
        ("link", &want.links, &got.links),
        ("version", &want.versions, &got.versions),
    ] {
        let (missing, extra) = differences(w, g);
        problems.extend(missing.iter().map(|m| format!("{kind} missing: {m}")));
        problems.extend(extra.iter().map(|e| format!("{kind} not in README.md: {e}")));
    }
    // Extra numbers are allowed: a translation may write in digits what English writes in words.
    let (missing, _) = differences(&want.numbers, &got.numbers);
    problems.extend(missing.iter().map(|m| format!("number missing: {m}")));
    for target in got.links.iter().filter_map(|l| l.strip_prefix('#')) {
        let anchor = format!(r#"id="{target}""#);
        if !translation.contains(&anchor) {
            problems.push(format!("#{target} has no {anchor} to land on"));
        }
    }
    problems
}

#[test]
fn every_translation_carries_the_readmes_commands_links_and_versions() {
    let english = read(&root().join("README.md"));
    let names = translations();
    for name in &names {
        let problems = problems(&english, &read(&root().join(name)));
        assert!(problems.is_empty(), "{name}:\n{}", problems.join("\n"));
    }
    for name in &names {
        assert!(
            english.contains(&format!("]({name})")),
            "README.md does not link {name}"
        );
    }
}

#[test]
fn a_change_to_a_command_link_or_version_fails_until_translated() {
    let english = "Run:\n\n```bash\nlaconic-check lint README.md\n```\n\nSee [the guide](INSTALL.md), [rules](#rules) and 0.6.0; it scored 4.92 of 5.\n";
    let translated = "Ejecute:\n\n```bash\nlaconic-check lint README.md\n```\n\nVea [la guía](INSTALL.md), [reglas](#rules) y 0.6.0; obtuvo 4.92 de 5 en 2 pruebas.\n<a id=\"rules\"></a>\n";
    assert_eq!(problems(english, translated), Vec::<String>::new());
    for (changed, problem) in [
        (english.replace("lint README", "lint --warn README"), "code blocks differ at block 1"),
        (english.replace("(INSTALL.md)", "(docs/INSTALL.md)"), "link missing: docs/INSTALL.md"),
        (english.replace("0.6.0", "0.7.0"), "version missing: 0.7.0"),
        (english.replace("4.92", "4.95"), "number missing: 4.95"),
    ] {
        let found = problems(&changed, translated);
        assert!(found.iter().any(|p| p.starts_with(problem)), "{found:?}");
    }
    let unanchored = translated.replace("<a id=\"rules\"></a>\n", "");
    assert_eq!(
        problems(english, &unanchored),
        [r##"#rules has no id="rules" to land on"##]
    );
}

#[test]
fn language_links_are_not_compared() {
    let english = "English · [Español](README.es.md)\n";
    let translated = "[English](README.md) · Español\n";
    assert_eq!(problems(english, translated), Vec::<String>::new());
}
