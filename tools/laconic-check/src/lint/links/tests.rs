// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::lint::{lint_all, lint_in};

mod definitions;

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
    let text =
        "## Notes\n\n## Notes\n\nSee [first](#notes), [second](#notes-1) and [third](#notes-2).\n";
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
        [
            "'docs/guide.md': 'guide.md' is 'Guide.md' on disk; a case-sensitive host will not find it"
        ]
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
    let root = tree(
        "symlink",
        &[("ws/a.md", ""), ("ws/sub/c.md", ""), ("out/b.md", "# B\n")],
    );
    std::os::unix::fs::symlink(root.join("out"), root.join("ws/link")).unwrap();
    // Even a link that stays inside is not followed: learning where it leads means looking.
    std::os::unix::fs::symlink(root.join("ws/sub"), root.join("ws/inner")).unwrap();
    let workspace = Workspace::new(&root.join("ws")).unwrap();
    let folder = workspace.folder_of(&root.join("ws/a.md")).unwrap();
    assert!(
        found(
            "[b](link/b.md#nope) [c](link/none.md) [d](inner/none.md)\n",
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
    assert_eq!(
        slug("[Docs](x.md) and <b>bold</b> my_var"),
        "docs-and-bold-my_var"
    );
    assert_eq!(slug("Café olé"), "café-olé");
    // GitHub slugs the rendered text: emphasis marks and entity escapes go first.
    assert_eq!(slug("_Emph_ words"), "emph-words");
    assert_eq!(slug("__init__ and snake_case"), "init-and-snake_case");
    assert_eq!(
        slug("A &amp; B &#38; C &#x26; D &unknown; E & F"),
        "a--b--c--d-unknown-e--f"
    );
    // A code span and a backslash keep their text literal: no entity, tag, link or emphasis.
    assert_eq!(slug("The `&nbsp;` entity"), "the-nbsp-entity");
    assert_eq!(slug("A \\&eacute; b"), "a-eacute-b");
    assert_eq!(slug("The `__init__` method"), "the-__init__-method");
    assert_eq!(slug("Tag `<b>` and \\<i>"), "tag-b-and-i");
}

#[test]
fn entities_in_code_spans_and_after_a_backslash_stay_literal() {
    let wrong: Vec<_> = [
        ("The `&nbsp;` entity", "#the-nbsp-entity"),
        ("Use `&copy;`", "#use-copy"),
        ("A \\&eacute; b", "#a-eacute-b"),
    ]
    .into_iter()
    .filter(|(heading, fragment)| {
        let text = format!("## {heading}\n\nSee [e]({fragment}) and [f](#nowhere).\n");
        names(&text, None) != ["broken-anchor"]
    })
    .collect();
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn heading_anchors_match_github_for_emphasis_entities_and_attributes() {
    let text = "## Title {#custom}\n\n## _Emph_ words\n\n## A &amp; B\n\nSee [a](#custom), [b](#title-custom), [c](#emph-words), [d](#a--b) and [e](#title).\n";
    assert_eq!(names(text, None), ["broken-anchor"]);
    assert!(found(text, None)[0].3.contains("'#title'"));
}

#[test]
fn links_in_html_comments_are_not_read() {
    let text = "<!-- [a](#gone-one) -->\n\nText <!-- [b](#gone-two) --> and [c](#gone-three).\n\n<!--\n[d](#gone-four) [e][nowhere]\n\n[f](#gone-five)\n-->\n\n`<!--` opens nothing in code: [g](#gone-six).\n";
    let hits = found(text, None);
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert!(hits[0].3.contains("#gone-three"), "{hits:?}");
    assert!(hits[1].3.contains("#gone-six"), "{hits:?}");
    // A comment left open hides the rest of the text.
    assert!(found("<!-- [h](#gone)\n", None).is_empty());
}

#[cfg(unix)]
#[test]
fn a_fifo_is_never_opened() {
    let root = tree("fifo", &[("ws/a.md", "")]);
    let ws = root.join("ws");
    let made = std::process::Command::new("mkfifo")
        .arg(ws.join("pipe.md"))
        .status()
        .unwrap();
    assert!(made.success());
    // Opening a FIFO waits for a writer, so the check runs on its own thread with a deadline.
    let (sent, received) = std::sync::mpsc::channel();
    let probe = ws.clone();
    std::thread::spawn(move || {
        let workspace = Workspace::new(&probe).unwrap();
        let folder = workspace.folder_of(&probe.join("a.md")).unwrap();
        let _ = sent.send(names("[f](pipe.md#x)\n", Some((&workspace, &folder))));
    });
    let result = received.recv_timeout(std::time::Duration::from_secs(10));
    assert_eq!(
        result.expect("reading a FIFO hung the run"),
        Vec::<String>::new()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn a_junction_out_of_the_workspace_is_not_followed() {
    let root = tree(
        "junction",
        &[("ws/a.md", ""), ("ws/sub/c.md", ""), ("out/b.md", "# B\n")],
    );
    // Even a junction that stays inside is not followed: learning where it leads means looking.
    for (name, to) in [
        ("link", root.join("out")),
        ("inner", root.join("ws").join("sub")),
    ] {
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(root.join("ws").join(name))
            .arg(to)
            .status()
            .unwrap();
        assert!(made.success());
    }
    let workspace = Workspace::new(&root.join("ws")).unwrap();
    let folder = workspace.folder_of(&root.join("ws/a.md")).unwrap();
    assert!(
        found(
            "[b](link/b.md#nope) [c](link/none.md) [d](inner/none.md)\n",
            Some((&workspace, &folder))
        )
        .is_empty()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unc_and_drive_letter_targets_are_refused_without_a_read() {
    let root = tree("unc", &[("ws/a.md", "")]);
    let workspace = Workspace::new(&root.join("ws")).unwrap();
    let folder = workspace.folder_of(&root.join("ws/a.md")).unwrap();
    let text = r"[a](\\server\share\x.md) [b](%5C%5Cserver%5Cshare%5Cx.md) [c](C:/x.md) [d](C%3A/x.md) [e](//server/share/x.md)";
    assert!(found(text, Some((&workspace, &folder))).is_empty());
    assert_eq!(workspace.reads.get(), 0);
    std::fs::remove_dir_all(root).unwrap();
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

#[test]
fn named_entities_in_headings_decode_or_leave_the_file_unchecked() {
    // GitHub slugs the rendered heading, so a named entity becomes its character.
    let text =
        "## Caf&eacute; &mdash; menu\n\nSee [a](#café--menu) and [b](#cafeacute-mdash-menu).\n";
    let reported = found(text, None);
    assert_eq!(
        reported.len(),
        1,
        "only the entity spelling is reported: {reported:?}"
    );
    // An entity this rule cannot decode leaves the heading's anchor unknown: nothing is reported.
    let text = "## The &bigstar; list\n\nSee [a](#the--list) and [b](#elsewhere).\n";
    assert!(found(text, None).is_empty());
}

#[test]
fn unclosed_links_and_definitions_are_read_in_linear_time() {
    // Each `](` once scanned to the paragraph's end for its `)`, so a paragraph of unclosed links
    // took time in the square of its length. Quadrupling every hostile paragraph must take
    // about 4 times as long, well under the 16 a square gives; the fastest of 5 runs keeps a
    // busy machine from deciding the ratio. Only the link rules' own reading is timed: the rest of
    // lint has costs of its own on such a paragraph. Each shape (a heading prefix and a repeated
    // unit) is timed alone, so a fast shape cannot hide a slow one.
    let shapes = [
        ("", "a](b c "),
        ("", "](a"),
        ("", "](a("),
        ("", "[a\n"),
        ("", "x][a] "),
        ("", "[a]: <b\n"),
        ("", "](<a "),
        ("", "`a` <!-- c --> [b](#x) "),
        ("", "<!-- c -->\n[b](#x)\n"),
        ("", "\\"),
        ("# ", "_"),
        ("# ", "&"),
    ];
    // As lint reads a paragraph: its lines joined by a space, which belongs to the line before.
    let read = |text: &str| {
        let mut line = 1;
        let lines: Vec<usize> = text
            .chars()
            .map(|c| {
                let at = line;
                line += usize::from(c == '\n');
                at
            })
            .collect();
        let chars: Vec<char> = text
            .chars()
            .map(|c| if c == '\n' { ' ' } else { c })
            .collect();
        let mut links = Links::default();
        links.collect(&chars, true, |k| {
            (lines.get(k).copied().unwrap_or(line), k + 1)
        });
        links.findings(text, None)
    };
    let fastest = |text: &str| {
        (0..5)
            .map(|_| {
                let start = std::time::Instant::now();
                read(text);
                start.elapsed()
            })
            .min()
            .unwrap()
    };
    let slow: Vec<String> = shapes
        .iter()
        .filter_map(|&(head, unit)| {
            let text = |chars: usize| format!("{head}{}", unit.repeat(chars / unit.len()));
            let (small, large) = (fastest(&text(16_000)), fastest(&text(64_000)));
            (large >= small * 8).then(|| {
                format!(
                    "{unit:?}: 4 times the input took {:.1} times as long ({small:?} to {large:?})",
                    large.as_secs_f64() / small.as_secs_f64()
                )
            })
        })
        .collect();
    assert!(slow.is_empty(), "{slow:#?}");
}

#[test]
fn an_inline_link_ends_at_its_title_and_closing_parenthesis() {
    // Each of these links names a heading the file lacks, so each that is read as a link is
    // reported once.
    for link in [
        "[a](#x)",
        "[a](#x \"t (1)\")",
        "[a](#x 't')",
        "[a](#x (t))",
        "[a](<#x> \"t\")",
        "[a](#x\n  \"t\")",
    ] {
        assert_eq!(names(&format!("{link}\n"), None).len(), 1, "{link:?}");
    }
    // Text after the destination that is no title makes no link, nor does a line break in an
    // angle-bracket destination, inline or in a definition.
    for text in [
        "[a](#x junk)",
        "[a](#x \"t\" junk)",
        "[a](<#x\ny>)",
        "[a]: <#b\nc>",
    ] {
        assert!(names(&format!("{text}\n"), None).is_empty(), "{text:?}");
    }
    // A destination may start on the next line: the definition's label is defined, and the
    // link rules now see the line break itself.
    assert!(names("[t][a]\n\n[a]:\nx.md\n", None).is_empty());
    let target = |text: &str| inline_target(&text.chars().collect::<Vec<_>>(), 0);
    assert_eq!(target("\n#x)").map(|t| t.0), Some("#x".to_owned()));
}

#[test]
fn a_large_folder_is_searched_once_per_target() {
    // Each target once scanned the whole listing of its folder, so missing links into a large
    // folder took time in the product of the two. Quadrupling both must take about 4 times as
    // long, not 16.
    let time = |n: usize| {
        let files: Vec<(String, &str)> = (0..n).map(|k| (format!("f{k}.md"), "")).collect();
        let mut listed: Vec<(&str, &str)> = files.iter().map(|(f, b)| (f.as_str(), *b)).collect();
        listed.push(("README.md", ""));
        let root = tree(&format!("large-{n}"), &listed);
        let workspace = Workspace::new(&root).unwrap();
        let folder = workspace.folder_of(&root.join("README.md")).unwrap();
        let text = "[a](Gone) ".repeat(n);
        let chars: Vec<char> = text.chars().collect();
        let mut links = Links::default();
        links.collect(&chars, true, |k| (1, k + 1));
        // The first run lists the folder; the fastest of 3 more times the lookups alone.
        links.findings(&text, Some((&workspace, &folder)));
        let fastest = (0..3)
            .map(|_| {
                let start = std::time::Instant::now();
                links.findings(&text, Some((&workspace, &folder)));
                start.elapsed()
            })
            .min()
            .unwrap();
        let _ = std::fs::remove_dir_all(&root);
        fastest
    };
    let (small, large) = (time(1_000), time(4_000));
    assert!(
        large < small * 8,
        "4 times the links took {:.1} times as long ({small:?} to {large:?})",
        large.as_secs_f64() / small.as_secs_f64()
    );
}

#[test]
fn a_bracket_in_a_bare_destination_is_part_of_it() {
    // As in CommonMark, so a missing target there is still reported.
    for text in ["[a](#x]y)", "[a](#x[1])", "[a](#notes[1]x)"] {
        assert_eq!(names(&format!("{text}\n"), None).len(), 1, "{text:?}");
    }
}
