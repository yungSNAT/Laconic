// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The README's translations, `README.<language>.md` beside it, carry the English README's
//! commands (its fenced code blocks, in order, and its inline code), links and every number
//! written in digits, versions included. A change to any of them in README.md fails here until
//! every translation has it too.

use super::{read, root};
use regex::Regex;
use std::fs;
use std::sync::LazyLock;

fn is_translation(name: &str) -> bool {
    name.strip_prefix("README.")
        .and_then(|rest| rest.strip_suffix(".md"))
        .is_some_and(|lang| {
            !lang.is_empty() && lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// What a translation must carry from the English README.
#[derive(Debug, PartialEq)]
struct Carried {
    /// Fenced code blocks, fence line included, in order.
    commands: Vec<String>,
    /// Inline code spans outside fenced blocks, sorted.
    code: Vec<String>,
    /// Link targets, sorted, except the links between the README's languages.
    links: Vec<String>,
    /// Numbers written in digits outside code blocks and link targets, sorted.
    numbers: Vec<String>,
}

static LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\]\(([^)\s]+)(?:\s+"[^"]*")?\)|(?:src|srcset|href)="([^"]+)""#).unwrap()
});
static CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`[^`\n]+`").unwrap());
// ASCII digits and no word boundaries, so a number followed by Hangul or kana still counts.
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[+-]?[0-9][0-9,.]*[0-9]|[0-9]").unwrap());

fn sorted<'a>(items: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut items: Vec<String> = items.map(str::to_owned).collect();
    items.sort();
    items
}

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
    let links = sorted(
        LINK.captures_iter(&prose)
            .map(|c| c.get(1).or(c.get(2)).unwrap().as_str())
            .filter(|target| !is_translation(target) && *target != "README.md"),
    );
    let unlinked = LINK.replace_all(&prose, "").replace('\u{2212}', "-");
    Carried {
        commands,
        code: sorted(CODE.find_iter(&prose).map(|m| m.as_str())),
        links,
        numbers: sorted(NUMBER.find_iter(&unlinked).map(|m| m.as_str())),
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
        ("inline code", &want.code, &got.code),
        ("link", &want.links, &got.links),
    ] {
        let (missing, extra) = differences(w, g);
        problems.extend(missing.iter().map(|m| format!("{kind} missing: {m}")));
        problems.extend(
            extra
                .iter()
                .map(|e| format!("{kind} not in README.md: {e}")),
        );
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
fn every_linked_translation_carries_the_readmes_commands_links_and_numbers() {
    let english = read(&root().join("README.md"));
    let linked = sorted(
        LINK.captures_iter(&english)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str())
            .filter(|target| is_translation(target)),
    );
    let mut on_disk: Vec<String> = fs::read_dir(root())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| is_translation(name))
        .collect();
    on_disk.sort();
    assert_eq!(
        linked, on_disk,
        "README.md's language links and the README.<language>.md files differ"
    );
    for name in &on_disk {
        let problems = problems(&english, &read(&root().join(name)));
        assert!(problems.is_empty(), "{name}:\n{}", problems.join("\n"));
    }
}

#[test]
fn a_change_to_a_command_link_or_number_fails_until_translated() {
    let english = "Run:\n\n```bash\nlaconic-check lint README.md\n```\n\nThen `laconic-check fix`. See [the guide](INSTALL.md), [rules](#rules) and 0.6.0; it scored 4.92 of 5.\n";
    let translated = "Ejecute:\n\n```bash\nlaconic-check lint README.md\n```\n\nLuego `laconic-check fix`. Vea [la guía](INSTALL.md), [reglas](#rules) y 0.6.0; obtuvo 4.92 de 5 en 2 pruebas.\n<a id=\"rules\"></a>\n";
    assert_eq!(problems(english, translated), Vec::<String>::new());
    for (changed, problem) in [
        (
            english.replace("lint README", "lint --warn README"),
            "code blocks differ at block 1",
        ),
        (
            english.replace("check fix`", "check fix --all`"),
            "inline code missing: `laconic-check fix --all`",
        ),
        (
            english.replace("(INSTALL.md)", "(docs/INSTALL.md)"),
            "link missing: docs/INSTALL.md",
        ),
        (english.replace("0.6.0", "0.7.0"), "number missing: 0.7.0"),
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
fn a_number_followed_by_hangul_or_kana_counts() {
    let english = "Since 0.6.0, 4.92 of 5.\n";
    assert_eq!(
        problems(english, "0.6.0부터 5점 만점에 4.92점.\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        problems(english, "0.6.0から、5 点中 4.92。\n"),
        Vec::<String>::new()
    );
    assert_eq!(
        problems(
            &english.replace("0.6.0", "0.7.0"),
            "0.6.0부터 5점 만점에 4.92점.\n"
        ),
        ["number missing: 0.7.0"]
    );
}

#[test]
fn language_links_are_not_compared() {
    let english = "English · [Español](README.es.md)\n";
    let translated = "[English](README.md) · Español\n";
    assert_eq!(problems(english, translated), Vec::<String>::new());
}
