// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Tests of the plugin directory build.

use super::icon::tests::png;
use super::*;
use crate::pi_check::TempDir;

fn plugin(extra: &[(&str, &str)]) -> Files {
    let mut files = Files::new();
    for (path, text) in [
        (MANIFEST, "{\"name\": \"laconic\"}\n"),
        ("README.md", "See [the skill](skills/laconic/SKILL.md).\n"),
        ("LICENSE", "licence\n"),
        ("NOTICE", "notice\n"),
        ("skills/laconic/SKILL.md", "---\nname: laconic\n---\n"),
        ("skills/laconic/LICENSE", "licence\n"),
        ("skills/laconic/references/a.yaml", "a: 1\n"),
    ]
    .iter()
    .chain(extra)
    {
        files.insert((*path).to_owned(), text.as_bytes().to_vec());
    }
    files.insert(ICON.to_owned(), png(1024, 1024));
    files
}

fn refused(path: &str, text: &str) -> String {
    check(&plugin(&[(path, text)])).expect_err(path)
}

#[test]
fn only_the_listing_icon_ships_as_an_image() {
    for (path, reason) in [
        ("icon.png", "outside skills/"),
        (".claude-plugin/logo.png", "outside skills/"),
        ("assets/icon.png", "outside skills/"),
        ("skills/laconic/icon.png", "the only image the build ships"),
    ] {
        let mut files = plugin(&[]);
        files.insert(path.to_owned(), png(1024, 1024));
        let error = check(&files).unwrap_err();
        assert!(error.contains(reason), "{path}: {error}");
    }
    let mut oblong = plugin(&[]);
    oblong.insert(ICON.to_owned(), png(1024, 1000));
    assert!(check(&oblong).unwrap_err().contains("square"));
}

#[test]
fn the_repository_builds_a_skills_only_plugin() {
    let temp = TempDir::new("laconic-plugin-dist-").unwrap();
    let out = temp.0.join("plugin");
    build(&crate::root(), &out).unwrap();
    for path in [
        MANIFEST,
        ICON,
        "README.md",
        "LICENSE",
        "NOTICE",
        "skills/laconic/SKILL.md",
        "skills/laconic-prose/SKILL.md",
    ] {
        assert!(out.join(path).is_file(), "{path}");
    }
    assert_eq!(
        fs::read(out.join(ICON)).unwrap(),
        fs::read(crate::root().join(ICON)).unwrap()
    );
    for path in ["hooks", "tools", ".cursor", "skills/laconic/agents"] {
        assert!(!out.join(path).exists(), "{path}");
    }
    let json = |path: &Path| -> Value { serde_json::from_slice(&fs::read(path).unwrap()).unwrap() };
    let version = json(&out.join(MANIFEST))["version"].clone();
    assert!(version.is_string());
    assert_eq!(
        version,
        json(&crate::root().join(VERSION_SOURCE))["version"]
    );
    assert!(build(&crate::root(), &out).is_err(), "an existing folder");
}

#[test]
fn a_skills_only_plugin_passes() {
    assert_eq!(check(&plugin(&[])), Ok(12));
}

#[test]
fn disallowed_paths_are_refused() {
    for path in [
        "hooks/hooks.json",
        "tools/laconic-check/mcp/server.mjs",
        ".mcp.json",
        "skills/README.md",
        ".cursor/skills/laconic/SKILL.md",
        "skills/laconic/.claude/settings.json",
        "skills/laconic/.DS_Store",
        "skills/laconic/references/SKILL.md",
    ] {
        refused(path, "{}\n");
    }
}

#[test]
fn disallowed_file_types_are_refused() {
    for path in [
        "skills/laconic/scripts/run.sh",
        "skills/laconic/scripts/check.py",
        "skills/laconic/server.mjs",
        "skills/laconic/agents/gemini.toml",
        "skills/laconic/icon.png",
        "skills/laconic/bin/tool",
    ] {
        assert!(refused(path, "x\n").contains("not Markdown"), "{path}");
    }
}

#[test]
fn unreadable_large_or_script_text_is_refused() {
    let large = "x".repeat(MAX_BYTES + 1);
    assert!(refused("skills/laconic/big.md", &large).contains("bytes"));
    assert!(refused("skills/laconic/nul.md", "a\0b").contains("NUL"));
    assert!(refused("skills/laconic/run.md", "#!/bin/sh\n").contains("#!"));
    let mut binary = plugin(&[]);
    binary.insert("skills/laconic/bad.txt".into(), vec![0xff, 0xfe]);
    assert!(check(&binary).unwrap_err().contains("UTF-8"));
}

#[test]
fn skills_that_run_commands_are_refused() {
    let bang = refused("skills/laconic/more.md", "Status: !`git status`\n");
    assert!(bang.contains("!`"), "{bang}");
    let hooks = "---\nname: laconic-x\nhooks:\n  Stop: []\n---\nBody\n";
    assert!(refused("skills/laconic-x/SKILL.md", hooks).contains("hooks"));
    let crlf = hooks.replace('\n', "\r\n");
    assert!(refused("skills/laconic-x/SKILL.md", &crlf).contains("hooks"));
    let body = "---\nname: laconic-x\n---\nhooks: are fine in the body\n";
    assert!(check(&plugin(&[("skills/laconic-x/SKILL.md", body)])).is_ok());
}

#[test]
fn only_regular_files_from_every_source_are_listed() {
    let record = |mode: &str, path: &str| format!("{mode} blob 0a1b\t{path}\0");
    let sources: String = [
        MANIFEST,
        ICON,
        "skills/laconic/SKILL.md",
        "LICENSE",
        "NOTICE",
        "docs/plugin-directory-readme.md",
    ]
    .iter()
    .map(|path| record("100644", path))
    .collect();
    let files = listed(&sources).unwrap();
    assert_eq!(files.len(), SOURCES.len());
    assert_eq!(files["README.md"], "0a1b");
    for mode in ["100755", "120000", "160000"] {
        let listing = sources.clone() + &record(mode, "skills/laconic/x.md");
        assert!(listed(&listing).unwrap_err().contains(mode), "{mode}");
    }
    // Other hosts' metadata is left out before its mode is read.
    assert!(listed(&(sources.clone() + &record("120000", "skills/laconic/agents/a"))).is_ok());
    let without_notice = sources.replace(&record("100644", "NOTICE"), "");
    let error = listed(&without_notice).unwrap_err();
    assert!(error.contains("NOTICE is not committed"), "{error}");
    let without_icon = sources.replace(&record("100644", ICON), "");
    let error = listed(&without_icon).unwrap_err();
    assert!(error.contains("icon.png is not committed"), "{error}");
}

#[test]
fn links_must_stay_in_the_plugin() {
    for link in [
        "[r](../../rules/laconic.md)",
        "[x](missing.md#top)",
        "[o](../../../a)",
    ] {
        assert!(
            refused("skills/laconic/more.md", link).contains("does not ship"),
            "{link}"
        );
    }
    let fine = "[u](https://example.com) [a](#use) [s](SKILL.md)\n```\n[f](gone.md)\n```\n";
    assert!(check(&plugin(&[("skills/laconic/more.md", fine)])).is_ok());
}

#[test]
fn too_many_entries_are_refused() {
    let names: Vec<String> = (0..MAX_ENTRIES)
        .map(|n| format!("skills/laconic/references/{n}.md"))
        .collect();
    let extra: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "x\n")).collect();
    assert!(
        check(&plugin(&extra))
            .unwrap_err()
            .contains("files and folders")
    );
}

#[test]
fn the_version_must_be_plain() {
    assert_eq!(
        package_version(br#"{"name": "x", "version": "0.5.10"}"#),
        Ok("0.5.10".to_owned())
    );
    for version in [
        r#""0.5""#,
        r#""0.5.0.1""#,
        r#""0.5.0-rc.1""#,
        r#""0.5.00""#,
        r#""01.2.3""#,
        r#""v1.2.3""#,
        r#"" 1.2.3""#,
        r#""1..3""#,
        r#""""#,
        "1",
        "null",
    ] {
        let package = format!(r#"{{"version": {version}}}"#);
        assert!(package_version(package.as_bytes()).is_err(), "{version}");
    }
    for package in ["{}", "[]", "not json"] {
        assert!(package_version(package.as_bytes()).is_err(), "{package}");
    }
}

#[test]
fn the_manifest_ships_only_descriptive_keys() {
    let shipped = manifest(
        br#"{"name": "laconic", "author": {"name": "yungSNAT"}}"#,
        "1.2.3",
    )
    .unwrap();
    let shipped: Value = serde_json::from_slice(&shipped).unwrap();
    assert_eq!(shipped["author"]["name"], "yungSNAT");
    assert_eq!(shipped["version"], "1.2.3");
    for key in ["hooks", "mcpServers", "commands", "skills", "version"] {
        let text = format!(r#"{{"name": "laconic", "{key}": "./x"}}"#);
        assert!(
            manifest(text.as_bytes(), "1.2.3")
                .unwrap_err()
                .contains(key),
            "{key}"
        );
    }
}

#[test]
fn other_hosts_metadata_and_loose_files_are_left_out() {
    assert_eq!(shipped_path("skills/laconic/agents/gemini.toml"), None);
    assert_eq!(shipped_path("skills/plugin.json"), None);
    assert_eq!(
        shipped_path("skills/laconic/SKILL.md").as_deref(),
        Some("skills/laconic/SKILL.md")
    );
    assert_eq!(
        shipped_path("skills/laconic/agents").as_deref(),
        Some("skills/laconic/agents")
    );
    assert_eq!(
        shipped_path("docs/plugin-directory-readme.md").as_deref(),
        Some("README.md")
    );
}
