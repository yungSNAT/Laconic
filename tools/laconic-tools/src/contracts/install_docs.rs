// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Installation paths in INSTALL.md and its translations. Replaces `tests/test_install_docs.py`.

use super::{re, read, root};
use std::fs;
use std::path::PathBuf;

/// INSTALL.md, then any translations alongside it in `.github/install` (they are optional).
fn install_docs() -> Vec<PathBuf> {
    let mut translations: Vec<PathBuf> = fs::read_dir(root().join(".github/install"))
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("INSTALL.") && n.ends_with(".md"))
                })
                .collect()
        })
        .unwrap_or_default();
    translations.sort();
    let mut docs = vec![root().join("INSTALL.md")];
    docs.extend(translations);
    docs
}

/// The text between `marker` and the next `</details>`, failing when either is missing.
fn section<'a>(text: &'a str, marker: &str, file: &str) -> &'a str {
    let (_, after) = text
        .split_once(marker)
        .unwrap_or_else(|| panic!("{file} lacks {marker}"));
    let (section, _) = after
        .split_once("</details>")
        .unwrap_or_else(|| panic!("{file}: {marker} has no </details>"));
    section
}

#[test]
fn marketplace_refresh_is_paired_with_plugin_update() {
    // `claude plugin marketplace update` refreshes only the marketplace listing; the
    // installed copy stays at its old version until `claude plugin update` runs, so the
    // docs must never offer the first alone.
    let refresh = "claude plugin marketplace update laconic";
    let update = "claude plugin update laconic@laconic";
    let blank_line = re(r"\n[^\S\n]*\n");
    for path in install_docs() {
        let text = read(&path);
        let file = path.display();
        assert!(text.contains(update), "{file}");
        for block in blank_line.split(&text) {
            if block.contains(refresh) {
                assert!(block.contains(update), "{file}: {block}");
            }
        }
    }
}

#[test]
fn zed_install_paths() {
    for path in install_docs() {
        let text = read(&path);
        let file = path.display().to_string();
        let section = section(&text, "<summary><strong>Zed</strong></summary>", &file);
        assert!(!section.contains("~/.config/zed/skills"), "{file}");
        assert!(
            section.contains(
                "mkdir -p ~/.agents/skills\ncp -R laconic/skills/laconic ~/.agents/skills/"
            ),
            "{file}"
        );
        assert!(section.contains("~/.agents/skills/laconic"), "{file}");
    }
}

#[test]
fn grok_install_commands() {
    let text = read(&root().join("INSTALL.md"));
    let section = section(
        &text,
        "<summary><strong>Grok (<code>grok</code>)</strong></summary>",
        "INSTALL.md",
    );
    for wanted in [
        "grok plugin install yungSNAT/laconic --trust",
        "grok plugin enable laconic",
        "grok plugin update laconic",
        "grok plugin uninstall laconic --confirm",
        "/laconic",
        "~/.grok/AGENTS.md",
        "~/.grok/rules/laconic.md",
    ] {
        assert!(section.contains(wanted), "missing {wanted}");
    }
    assert!(!section.contains("~/.claude/.laconic-always"));
}
