// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Structural checks for the 2 Laconic skills and their references. Replaces
//! `tests/test_skills.py`.

use super::{re, read, root};
use std::fs;
use std::path::{Path, PathBuf};

fn skills() -> PathBuf {
    root().join("skills")
}

/// The frontmatter of a skill file: the text between its first 2 `---` lines.
fn frontmatter(path: &Path) -> String {
    read(path)
        .split("---\n")
        .nth(1)
        .unwrap_or_else(|| panic!("{} has no frontmatter", path.display()))
        .to_owned()
}

/// Every Markdown file under `dir`, as `Path.rglob("*.md")` finds them.
fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![];
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|e| e == "md") {
            files.push(path);
        }
    }
    files
}

#[test]
fn skill_names_match_their_folders() {
    for name in ["laconic", "laconic-prose"] {
        let path = skills().join(name).join("SKILL.md");
        assert!(
            frontmatter(&path).contains(&format!("name: {name}\n")),
            "{name}"
        );
    }
}

#[test]
fn session_mode_is_explicit_and_prose_skill_is_model_invocable() {
    assert!(
        frontmatter(&skills().join("laconic/SKILL.md")).contains("disable-model-invocation: true")
    );
    assert!(
        !frontmatter(&skills().join("laconic-prose/SKILL.md")).contains("disable-model-invocation")
    );
}

#[test]
fn prose_skill_does_not_import_session_persistence() {
    let text = read(&skills().join("laconic-prose/SKILL.md"));
    assert!(!text.contains("../laconic/"));
    assert!(!text.contains("stop laconic"));
}

#[test]
fn relative_links_resolve() {
    let fence = re(r"(?s)```.*?```");
    let link = re(r"\]\(([^)\s#]+)(?:#[^)]*)?\)");
    let scheme = re(r"^[a-z]+:");
    for path in markdown_files(&skills()) {
        let text = read(&path);
        let text = fence.replace_all(&text, "");
        for captures in link.captures_iter(&text) {
            let target = &captures[1];
            if scheme.is_match(target) {
                continue;
            }
            let resolved = path.parent().unwrap().join(target);
            assert!(resolved.exists(), "{}: {target}", path.display());
        }
    }
}

#[test]
fn no_em_dashes_in_skill_files_or_compact_rules() {
    for path in [
        skills().join("laconic/SKILL.md"),
        skills().join("laconic-prose/SKILL.md"),
        root().join("rules/laconic.md"),
    ] {
        assert!(!read(&path).contains('\u{2014}'), "{}", path.display());
    }
}

#[test]
fn gemini_command_keeps_the_canonical_error_rule() {
    let skill = read(&skills().join("laconic/SKILL.md"));
    let rule = re(r"(?m)^7\. \*\*(State errors plainly\.)\*\* (.+)$")
        .captures(&skill)
        .expect("SKILL.md keeps rule 7");
    let line = format!("7. {} {}\n", &rule[1], &rule[2]);
    let relative = "skills/laconic/agents/gemini.toml";
    for path in [root().join(relative), root().join(".cursor").join(relative)] {
        assert!(read(&path).contains(&line), "{}", path.display());
    }
}
