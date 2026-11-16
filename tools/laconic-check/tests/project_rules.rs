// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The project file's `[rules]` section through `lint`'s text, JSON and SARIF output, its
//! filters and the MCP `lint` tool.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use laconic_check::lint::RULES;
use laconic_check::mcp::{Json, parse};

const PROJECT: &str = "disable = serial-comma\n\n[rules]\n# Plain words.\n\
                       plain: utilise = use\nplain: leverage = use\nfiller: going forward =\n";
const TEXT: &str = "We utilise the logs.\n\nGoing forward, we leverage the color.\n";

/// A fresh directory for one test, holding the given files.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("rules-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

fn run(dir: &Path, args: &[&str], input: &str) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn get<'a>(value: &'a Json, key: &str) -> &'a Json {
    let Json::Object(members) = value else {
        panic!("not an object: {value:?}");
    };
    &members.iter().rev().find(|(k, _)| k == key).unwrap().1
}

fn items(value: &Json) -> &[Json] {
    let Json::Array(items) = value else {
        panic!("not an array: {value:?}");
    };
    items
}

fn string(value: &Json) -> &str {
    let Json::String(s) = value else {
        panic!("not a string: {value:?}");
    };
    s
}

#[test]
fn lint_reports_project_rules_in_text_and_counts_them_as_failing() {
    let dir = folder("text", &[(".laconic-check", PROJECT), ("a.md", TEXT)]);
    let (code, out, err) = run(&dir, &["lint", "a.md"], "");
    assert_eq!((code, err.as_str()), (1, ""));
    assert_eq!(
        out,
        "a.md:1:4: project/plain: 'utilise': the project file prefers 'use'\n\
         a.md:3:32: us-spelling: 'color': Australian form is 'colour'\n\
         a.md:3:1: project/filler: 'Going forward': the project file bans this phrase\n\
         a.md:3:19: project/plain: 'leverage': the project file prefers 'use'\n"
    );
    assert_eq!(run(&dir, &["lint", "--max-findings", "4", "a.md"], "").0, 0);
    let (code, out, _) = run(&dir, &["lint", "--disable", "project/plain", "a.md"], "");
    assert_eq!((code, out.lines().count()), (1, 2));
    let (_, out, _) = run(&dir, &["lint", "--only=project/filler", "a.md"], "");
    assert_eq!(out.lines().count(), 1);
    let (code, _, err) = run(&dir, &["lint", "--only", "project/other", "a.md"], "");
    assert_eq!(code, 2);
    assert!(err.contains("unknown rule 'project/other'"), "{err}");
}

#[test]
fn json_gives_a_project_rule_no_url() {
    let dir = folder("json", &[(".laconic-check", PROJECT), ("a.md", TEXT)]);
    let (_, out, _) = run(&dir, &["lint", "--json", "a.md"], "");
    let report = parse(out.trim()).unwrap();
    let findings = items(get(&report, "findings"));
    let first = &findings[0];
    assert_eq!(string(get(first, "rule")), "project/plain");
    assert_eq!(get(first, "advisory"), &Json::Bool(false));
    assert_eq!(get(first, "url"), &Json::Null);
    let spelling = &findings[1];
    assert!(string(get(spelling, "url")).ends_with("rules.md#us-spelling"));
    assert_eq!(get(&report, "failing"), &Json::Number("4".into()));
}

#[test]
fn sarif_lists_project_rules_after_the_house_rules() {
    let dir = folder("sarif", &[(".laconic-check", PROJECT), ("a.md", TEXT)]);
    let (code, out, _) = run(&dir, &["lint", "--sarif", "a.md"], "");
    assert_eq!(code, 1);
    let log = parse(out.trim()).unwrap();
    let run = &items(get(&log, "runs"))[0];
    let rules = items(get(get(get(run, "tool"), "driver"), "rules"));
    assert_eq!(rules.len(), RULES.len() + 2);
    let own: Vec<&str> = rules[RULES.len()..]
        .iter()
        .map(|r| string(get(r, "id")))
        .collect();
    assert_eq!(own, ["project/plain", "project/filler"]);
    let plain = &rules[RULES.len()];
    assert_eq!(
        string(get(get(plain, "shortDescription"), "text")),
        "The project file's rule for 'utilise' (write 'use'), 'leverage' (write 'use')."
    );
    let Json::Object(members) = plain else {
        panic!()
    };
    assert!(!members.iter().any(|(k, _)| k == "helpUri"));
    let results = items(get(run, "results"));
    let index = |n: usize| {
        (
            string(get(&results[n], "ruleId")),
            get(&results[n], "ruleIndex"),
        )
    };
    let at = |n: usize| Json::Number(n.to_string());
    assert_eq!(index(0), ("project/plain", &at(RULES.len())));
    assert_eq!(index(2), ("project/filler", &at(RULES.len() + 1)));
    assert_eq!(index(3), ("project/plain", &at(RULES.len())));
    assert_eq!(string(get(&results[2], "level")), "warning");
}

#[test]
fn a_malformed_rules_section_is_an_error_with_its_line() {
    let dir = folder(
        "bad",
        &[
            (
                ".laconic-check",
                "[rules]\nplain: utilise = use\nleverage = use\n",
            ),
            ("a.md", TEXT),
        ],
    );
    let (code, out, err) = run(&dir, &["lint", "a.md"], "");
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(
        err.starts_with("laconic-check: .laconic-check:3: expected 'id: phrase = replacement'"),
        "{err}"
    );
}

/// The MCP `lint` tool's reply text for `text`, run in `dir`.
fn mcp_lint(dir: &Path, text: &str) -> (String, bool) {
    let call = Json::Object(vec![
        ("jsonrpc".into(), Json::String("2.0".into())),
        ("id".into(), Json::Number("1".into())),
        ("method".into(), Json::String("tools/call".into())),
        (
            "params".into(),
            Json::Object(vec![
                ("name".into(), Json::String("lint".into())),
                (
                    "arguments".into(),
                    Json::Object(vec![("text".into(), Json::String(text.into()))]),
                ),
            ]),
        ),
    ]);
    let (code, out, _) = run(
        dir,
        &["mcp"],
        &format!("{}\n", laconic_check::mcp::to_string(&call)),
    );
    assert_eq!(code, 0);
    let reply = parse(out.trim()).unwrap();
    let result = get(&reply, "result");
    let content = string(get(&items(get(result, "content"))[0], "text")).to_owned();
    let error = matches!(result, Json::Object(m) if m.iter().any(|(k, v)| k == "isError" && *v == Json::Bool(true)));
    (content, error)
}

#[test]
fn the_mcp_lint_tool_applies_the_project_file_in_its_directory() {
    let with = folder(
        "mcp",
        &[(
            ".laconic-check",
            "accept = color\n[rules]\nplain: utilise = use\n",
        )],
    );
    assert_eq!(
        mcp_lint(&with, TEXT),
        (
            "line 1:4: project/plain: 'utilise': the project file prefers 'use'".into(),
            false
        )
    );
    let without = folder("mcp-none", &[]);
    assert_eq!(
        mcp_lint(&without, TEXT),
        (
            "line 3:32: us-spelling: 'color': Australian form is 'colour'".into(),
            false
        )
    );
    let bad = folder("mcp-bad", &[(".laconic-check", "[rule]\n")]);
    let (message, error) = mcp_lint(&bad, TEXT);
    assert!(error);
    assert!(
        message.starts_with(".laconic-check:1: unknown section '[rule]'"),
        "{message}"
    );
}
