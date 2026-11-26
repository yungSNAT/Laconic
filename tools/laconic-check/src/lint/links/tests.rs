// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::lint::{lint_all, lint_in};

/// Each `broken-anchor` or `broken-link` finding as (line, column, rule, message).
fn found(text: &str, place: Option<(&Workspace, &Path)>) -> Vec<(usize, usize, String, String)> {
    lint_in(text, &[], true, place)
        .0
        .into_iter()
        .filter(|f| f.rule == RuleId::BrokenAnchor || f.rule == RuleId::BrokenLink)
        .map(|f| (f.line, f.column, f.rule.name().to_owned(), f.message))
        .collect()
}

fn names(text: &str, place: Option<(&Workspace, &Path)>) -> Vec<String> {
    found(text, place).into_iter().map(|f| f.2).collect()
}

/// A fresh folder holding `files` (path, contents); folders end in `/`.
fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("laconic-links-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, body) in files {
        let path = root.join(path);
        if path.to_string_lossy().ends_with('/') {
            std::fs::create_dir_all(&path).unwrap();
        } else {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }
    }
    root
}

#[test]
fn a_fragment_must_name_a_heading_or_id_in_the_file() {
    let text = "# Getting started\n\n## The `lint` command\n\n<a id=\"pinned\"></a>\n\nSee [start](#getting-started), [lint](#the-lint-command), [pin](#Pinned), [top](#top) and [gone](#usage).\n";
    assert_eq!(
        found(text, None),
        [(
            7,
            98,
            "broken-anchor".to_owned(),
            "'#usage' matches no heading or id in this file".to_owned()
        )]
    );
}

#[test]
fn repeated_headings_take_numbered_anchors() {
    let text = "## Notes\n\n## Notes\n\nSee [first](#notes), [second](#notes-1) and [third](#notes-2).\n";
    assert_eq!(names(text, None), ["broken-anchor"]);
    assert!(found(text, None)[0].3.contains("#notes-2"));
}

#[test]
fn setext_headings_custom_ids_and_github_prefixes_are_anchors() {
    let text = "Overview\n========\n\nDetails {#more}\n---\n\nSee [a](#overview), [b](#more), [c](#user-content-overview).\n";
    assert!(found(text, None).is_empty(), "{:?}", found(text, None));
}

#[test]
fn headings_in_code_and_frontmatter_are_not_anchors() {
    let text = "---\ntitle: x\n---\n\n```\n# Hidden\n```\n\nSee [h](#hidden).\n";
    assert_eq!(names(text, None), ["broken-anchor"]);
}

#[test]
fn a_reference_link_needs_a_definition() {
    let text = "Read [the guide][guide], [the notes][Missing Notes] and [Setup][].\n\nA [shortcut] and a footnote[^1] pass.\n\n[GUIDE]: docs/guide.md\n[setup]: <docs/set up.md>\n";
    assert_eq!(
        found(text, None),
        [(
            1,
            26,
            "broken-anchor".to_owned(),
            "'[Missing Notes]' has no definition such as '[Missing Notes]: target' in this file"
                .to_owned()
        )]
    );
}

#[test]
fn code_spans_fences_and_escapes_hold_no_links() {
    let text = "Write `[x](#nowhere)` or \\[y\\](#nowhere).\n\n```md\n[z](#nowhere)\n```\n\n    [w](#nowhere)\n";
    assert!(found(text, None).is_empty(), "{:?}", found(text, None));
}

#[test]
fn rules_stay_off_without_opt_in() {
    let text = "See [gone](#usage).\n";
    assert!(crate::lint::lint(text).is_empty());
    assert_eq!(
        lint_all(text)
            .iter()
            .map(|f| f.rule.name())
            .collect::<Vec<_>>(),
        ["broken-anchor"]
    );
}

#[test]
fn file_targets_are_checked_only_in_a_workspace() {
    let root = tree(
        "files",
        &[
            ("docs/guide.md", "# Install\n\n## Use it\n"),
            ("docs/img/", ""),
            ("docs/set up.md", "x"),
            ("README.md", ""),
        ],
    );
    let workspace = Workspace::new(&root).unwrap();
    let folder = workspace.folder_of(&root.join("README.md")).unwrap();
    let text = "[a](docs/guide.md) [b](docs/guide.md#use-it) [c](./docs/img/) [d](docs/set%20up.md?raw=1)\n\n[e](docs/missing.md) [f](docs/guide.md#nope) [g](nowhere/x.md)\n\n[h](https://example.com/x.md) [i](/docs/x.md) [j](mailto:a@b.c) [k](../outside.md)\n\n<img src=\"docs/logo.png\" alt=\"\">\n";
    let got = found(text, Some((&workspace, &folder)));
    let messages: Vec<&str> = got.iter().map(|f| f.3.as_str()).collect();
    assert_eq!(
        messages,
        [
            "'docs/missing.md': no file or folder named 'missing.md'",
            "'docs/guide.md#nope': '#nope' matches no heading or id in that file",
            "'nowhere/x.md': no file or folder named 'nowhere'",
            "'docs/logo.png': no file or folder named 'logo.png'",
        ]
    );
    assert!(got.iter().all(|f| f.2 == "broken-link"));
    assert_eq!((got[0].0, got[0].1), (3, 5));
    assert_eq!((got[3].0, got[3].1), (7, 11));
    // Without a workspace, as on standard input, file targets go unchecked.
    assert!(found(text, None).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_target_in_another_case_is_reported() {
    let root = tree("case", &[("docs/Guide.md", "x"), ("a.md", "")]);
    let workspace = Workspace::new(&root).unwrap();
    let folder = workspace.folder_of(&root.join("a.md")).unwrap();
    assert_eq!(
        found("[g](docs/guide.md)\n", Some((&workspace, &folder)))
            .into_iter()
            .map(|f| f.3)
            .collect::<Vec<_>>(),
        ["'docs/guide.md': 'guide.md' is 'Guide.md' on disk; a case-sensitive host will not find it"]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nothing_outside_the_workspace_is_read() {
    let root = tree("outside", &[("ws/a.md", ""), ("secret.md", "# Key\n")]);
    let workspace = Workspace::new(&root.join("ws")).unwrap();
    let folder = workspace.folder_of(&root.join("ws/a.md")).unwrap();
    assert!(
        found(
            "[s](../secret.md#key) [t](../absent.md)\n",
            Some((&workspace, &folder))
        )
        .is_empty()
    );
    // A file outside the workspace has no place in it.
    assert_eq!(workspace.folder_of(&root.join("secret.md")), None);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_out_of_the_workspace_is_not_followed() {
    let root = tree("symlink", &[("ws/a.md", ""), ("out/b.md", "# B\n")]);
    std::os::unix::fs::symlink(root.join("out"), root.join("ws/link")).unwrap();
    let workspace = Workspace::new(&root.join("ws")).unwrap();
    let folder = workspace.folder_of(&root.join("ws/a.md")).unwrap();
    assert!(
        found(
            "[b](link/b.md#nope) [c](link/none.md)\n",
            Some((&workspace, &folder))
        )
        .is_empty()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn targets_that_are_not_relative_paths_are_skipped() {
    for target in [
        "https://example.com",
        "//cdn.example.com/x",
        "/abs.md",
        "mailto:a@b.c",
        "C:\\x.md",
        "{{ site.url }}/x",
        "%FF.md",
        "",
    ] {
        assert_eq!(split_target(target), None, "{target}");
    }
    assert_eq!(
        split_target("a%20b.md?raw=1#Part%202"),
        Some(("a b.md".to_owned(), Some("Part 2".to_owned())))
    );
    assert_eq!(split_target("#"), Some((String::new(), None)));
}

#[test]
fn slugs_follow_github() {
    assert_eq!(slug("The `lint` command"), "the-lint-command");
    assert_eq!(slug("What's new in 0.6?"), "whats-new-in-06");
    assert_eq!(slug("[Docs](x.md) and <b>bold</b> my_var"), "docs-and-bold-my_var");
    assert_eq!(slug("Café olé"), "café-olé");
}

#[test]
fn reading_stops_at_the_budget() {
    let root = tree("budget", &[("a.md", ""), ("b.md", "# B\n")]);
    let workspace = Workspace::new(&root).unwrap();
    workspace.reads.set(MAX_READS);
    let folder = workspace.folder_of(&root.join("a.md")).unwrap();
    assert!(found("[x](missing.md)\n", Some((&workspace, &folder))).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
