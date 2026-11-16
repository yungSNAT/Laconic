// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `lint` exit statuses, filters and project file, and the exit-status table in README.md.

use std::path::{Path, PathBuf};
use std::process::Command;

use laconic_check::lint::{RULES, RULES_URL};
use laconic_check::mcp::{Json, parse};

/// A fresh directory for one test, holding the given files.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("laconic-check-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

/// Runs `lint` in `dir` and returns its exit status, standard output and standard error.
fn lint(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .current_dir(dir)
        .arg("lint")
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

const TWO: &str = "The color is red.\n\nThe flavor is sweet.\n";

#[test]
fn fails_only_on_more_failing_findings_than_allowed() {
    let dir = folder(
        "count",
        &[
            ("two.md", TWO),
            ("clean.md", "The colour is red.\n"),
            ("advisory.md", "It could potentially rain.\n"),
        ],
    );
    assert_eq!(lint(&dir, &["clean.md"]).0, 0);
    assert_eq!(lint(&dir, &["advisory.md"]).0, 0);
    assert_eq!(lint(&dir, &["two.md"]).0, 1);
    assert_eq!(lint(&dir, &["--max-findings", "1", "two.md"]).0, 1);
    assert_eq!(lint(&dir, &["--max-findings=2", "two.md"]).0, 0);
    assert_eq!(lint(&dir, &["--warn", "two.md"]).0, 0);
}

#[test]
fn filters_drop_findings_before_they_are_counted() {
    let dir = folder(
        "filters",
        &[("mixed.md", "The color is red, e.g. crimson.\n")],
    );
    let (code, out, _) = lint(&dir, &["--disable", "us-spelling", "mixed.md"]);
    assert_eq!(
        (code, out.as_str()),
        (
            1,
            "mixed.md:1:19: latin: Latin abbreviation; write 'for example' or 'that is'\n"
        )
    );
    let (code, out, _) = lint(&dir, &["--only", "us-spelling", "mixed.md"]);
    assert_eq!(code, 1);
    assert!(out.starts_with("mixed.md:1:5: us-spelling: "), "{out}");
    assert_eq!(
        lint(
            &dir,
            &[
                "--only",
                "us-spelling",
                "--disable",
                "us-spelling",
                "mixed.md"
            ]
        )
        .0,
        0
    );
}

#[test]
fn reads_the_project_file_in_the_current_directory() {
    let with = folder(
        "project",
        &[
            ("two.md", TWO),
            (".laconic-check", "# house rules\ndisable = us-spelling\n"),
        ],
    );
    assert_eq!(lint(&with, &["two.md"]), (0, String::new(), String::new()));
    assert_eq!(lint(&with, &["--config", "two.md", "two.md"]).0, 2);
    let without = folder("no-project", &[("two.md", TWO)]);
    assert_eq!(lint(&without, &["two.md"]).0, 1);
}

#[test]
fn accepted_words_in_the_project_file_are_not_flagged() {
    let dir = folder(
        "accept",
        &[
            ("two.md", TWO),
            (".laconic-check", "accept = color\naccept = flavor\n"),
        ],
    );
    assert_eq!(lint(&dir, &["two.md"]), (0, String::new(), String::new()));
    std::fs::write(dir.join("capital.md"), "Color is red.\n").unwrap();
    let (code, out, _) = lint(&dir, &["capital.md"]);
    assert_eq!(code, 1);
    assert!(out.starts_with("capital.md:1:1: us-spelling: "), "{out}");
}

#[test]
fn usage_settings_and_read_errors_exit_2() {
    let dir = folder(
        "errors",
        &[("two.md", TWO), (".laconic-check", "severity = high\n")],
    );
    for args in [
        &[][..],
        &["--disable", "us-spellings", "two.md"],
        &["--colour", "two.md"],
        &["missing.md"],
        &["two.md"],
    ] {
        let (code, out, err) = lint(&dir, args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(!err.is_empty(), "{args:?}");
    }
}

#[test]
fn an_unreadable_file_is_reported_and_the_rest_are_still_linted() {
    let dir = folder("unreadable", &[("two.md", TWO)]);
    let (code, out, err) = lint(&dir, &["missing.md", "two.md", "--warn"]);
    assert_eq!(code, 2);
    assert_eq!(out.lines().count(), 2, "{out}");
    assert!(out.starts_with("two.md:1:5: us-spelling: "), "{out}");
    assert!(err.starts_with("missing.md: "), "{err}");
    let (code, out, _) = lint(&dir, &["--json", "two.md", "missing.md"]);
    assert_eq!(code, 2);
    assert!(
        out.ends_with("],\"failing\":2,\"suppressed\":0}\n"),
        "{out}"
    );
}

#[test]
fn json_reports_every_finding_and_the_counts() {
    let quiet = "<!-- laconic-check-disable-next-line us-spelling -->\nThe flavor is sweet.\n";
    let dir = folder("json", &[("two.md", TWO), ("quiet.md", quiet)]);
    let (code, out, _) = lint(&dir, &["--json", "two.md", "quiet.md", "two.md"]);
    assert_eq!(code, 1);
    let finding = |line: usize, column: usize, message: &str| {
        format!(
            r#"{{"file":"two.md","line":{line},"column":{column},"rule":"us-spelling","advisory":false,"message":"{message}","url":"https://github.com/yungSNAT/Laconic/blob/main/docs/rules.md#us-spelling"}}"#
        )
    };
    let color = finding(1, 5, "'color': Australian form is 'colour'");
    let flavor = finding(3, 5, "'flavor': Australian form is 'flavour'");
    assert_eq!(
        out,
        format!(r#"{{"findings":[{color},{flavor},{color},{flavor}],"failing":4,"suppressed":1}}"#)
            + "\n"
    );
    let clean = folder("json-clean", &[("clean.md", "Fine.\n")]);
    assert_eq!(
        lint(&clean, &["--json", "clean.md"]),
        (
            0,
            "{\"findings\":[],\"failing\":0,\"suppressed\":0}\n".into(),
            String::new()
        )
    );
}

#[test]
fn a_commit_message_is_linted_without_its_comments_or_diff() {
    let dir = folder(
        "commit-msg",
        &[(
            "COMMIT_EDITMSG",
            "fix: stop the flavor\n\n# Changes to be committed: color\n\
             # ------------------------ >8 ------------------------\n+color\n",
        )],
    );
    let (code, out, _) = lint(&dir, &["--commit-msg", "COMMIT_EDITMSG"]);
    assert_eq!(code, 1);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.starts_with("COMMIT_EDITMSG:1:15: us-spelling: "),
        "{out}"
    );
}

/// The member at `path` of a JSON value: a key in an object or an index in an array.
fn at<'a>(value: &'a Json, path: &[&str]) -> &'a Json {
    path.iter().fold(value, |value, key| match value {
        Json::Object(members) => members
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("no '{key}' in {value:?}")),
        Json::Array(items) => &items[key.parse::<usize>().unwrap()],
        _ => panic!("'{key}' in a scalar {value:?}"),
    })
}

fn string(value: &Json) -> &str {
    match value {
        Json::String(s) if !s.is_empty() => s,
        _ => panic!("not a non-empty string: {value:?}"),
    }
}

fn array(value: &Json) -> &[Json] {
    match value {
        Json::Array(items) => items,
        _ => panic!("not an array: {value:?}"),
    }
}

/// SARIF 2.1.0 as GitHub code scanning reads it: the log's version, one run whose driver
/// lists every rule with the descriptions and help code scanning requires, and one result a
/// finding with its rule, level, message and a location: the file as an absolute `file` URI,
/// and a region of one character whose columns count UTF-16 code units, as SARIF's do by
/// default.
#[test]
fn sarif_reports_each_finding_with_its_rule_level_and_location() {
    let dir = folder(
        "sarif",
        &[
            ("two.md", TWO),
            ("my notes.md", "\u{1f600} The color is red.\n"),
            ("advisory.md", "It could potentially rain.\n"),
        ],
    );
    let (code, out, err) = lint(&dir, &["--sarif", "two.md", "my notes.md", "advisory.md"]);
    assert_eq!((code, err.as_str()), (1, ""));
    assert!(out.ends_with('\n') && out.lines().count() == 1, "{out}");
    let log = parse(&out).expect("SARIF output is JSON");
    assert_eq!(
        string(at(&log, &["$schema"])),
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(string(at(&log, &["version"])), "2.1.0");
    assert_eq!(array(at(&log, &["runs"])).len(), 1);
    let run = at(&log, &["runs", "0"]);
    let driver = at(run, &["tool", "driver"]);
    assert_eq!(string(at(driver, &["name"])), "laconic-check");
    assert_eq!(string(at(driver, &["version"])), env!("CARGO_PKG_VERSION"));
    string(at(driver, &["informationUri"]));
    let rules = array(at(driver, &["rules"]));
    assert_eq!(rules.len(), RULES.len());
    for (rule, wanted) in rules.iter().zip(RULES) {
        assert_eq!(string(at(rule, &["id"])), wanted.name);
        for key in ["shortDescription", "fullDescription", "help"] {
            string(at(rule, &[key, "text"]));
        }
        assert_eq!(
            string(at(rule, &["helpUri"])),
            format!("{RULES_URL}#{}", wanted.name)
        );
        let level = if wanted.advisory { "note" } else { "warning" };
        assert_eq!(string(at(rule, &["defaultConfiguration", "level"])), level);
    }
    assert_eq!(
        at(run, &["invocations", "0", "executionSuccessful"]),
        &Json::Bool(true)
    );
    let results: Vec<(String, String, String, String, String, String, String)> =
        array(at(run, &["results"]))
            .iter()
            .map(|result| {
                let rule = string(at(result, &["ruleId"]));
                let index = match at(result, &["ruleIndex"]) {
                    Json::Number(n) => n.parse::<usize>().unwrap(),
                    other => panic!("ruleIndex {other:?}"),
                };
                assert_eq!(string(at(&rules[index], &["id"])), rule);
                assert_eq!(
                    string(at(result, &["level"])),
                    string(at(&rules[index], &["defaultConfiguration", "level"]))
                );
                assert_eq!(array(at(result, &["locations"])).len(), 1);
                let location = at(result, &["locations", "0", "physicalLocation"]);
                let number = |key: &str| match at(location, &["region", key]) {
                    Json::Number(n) => n.clone(),
                    other => panic!("{key} {other:?}"),
                };
                assert_eq!(number("endLine"), number("startLine"));
                (
                    file_name(string(at(location, &["artifactLocation", "uri"]))),
                    number("startLine"),
                    number("startColumn"),
                    number("endColumn"),
                    rule.to_owned(),
                    string(at(result, &["level"])).to_owned(),
                    string(at(result, &["message", "text"])).to_owned(),
                )
            })
            .collect();
    let row = |uri: &str, line: &str, column: &str, rule: &str, level: &str, message: &str| {
        let end = (column.parse::<usize>().unwrap() + 1).to_string();
        (
            uri.to_owned(),
            line.to_owned(),
            column.to_owned(),
            end,
            rule.to_owned(),
            level.to_owned(),
            message.to_owned(),
        )
    };
    assert_eq!(
        results,
        [
            row(
                "two.md",
                "1",
                "5",
                "us-spelling",
                "warning",
                "'color': Australian form is 'colour'"
            ),
            row(
                "two.md",
                "3",
                "5",
                "us-spelling",
                "warning",
                "'flavor': Australian form is 'flavour'"
            ),
            // The emoji is one column in the text output and 2 UTF-16 code units here.
            row(
                "my%20notes.md",
                "1",
                "8",
                "us-spelling",
                "warning",
                "'color': Australian form is 'colour'"
            ),
            row(
                "advisory.md",
                "1",
                "4",
                "stacked-hedge",
                "note",
                "stacked hedge 'could potentially' (advisory); keep one qualifier at the \
                 confidence the source supports",
            ),
        ]
    );
}

/// The last segment of an absolute `file` URI with no `.` or `..` segments.
fn file_name(uri: &str) -> String {
    assert!(uri.starts_with("file:///"), "{uri}");
    assert!(!uri.contains("/./") && !uri.contains("/../"), "{uri}");
    uri.rsplit('/').next().unwrap().to_owned()
}

/// A path given from a subdirectory names the same file as one given from its parent, so
/// code scanning maps both to the same repository path.
#[test]
fn sarif_names_a_file_the_same_from_any_directory() {
    let dir = folder("sarif-dirs", &[("two.md", TWO)]);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let uri = |cwd: &Path, file: &str| {
        let (code, out, _) = lint(cwd, &["--sarif", file]);
        assert_eq!(code, 1, "{out}");
        let log = parse(&out).unwrap();
        let location = at(
            &log,
            &[
                "runs",
                "0",
                "results",
                "0",
                "locations",
                "0",
                "physicalLocation",
            ],
        );
        string(at(location, &["artifactLocation", "uri"])).to_owned()
    };
    let from_parent = uri(&dir, "two.md");
    assert_eq!(file_name(&from_parent), "two.md");
    assert_eq!(uri(&dir.join("sub"), "../two.md"), from_parent);
    assert_eq!(uri(&dir.join("sub"), "./../sub/../two.md"), from_parent);
}

/// A finding that starts at a character outside the Basic Multilingual Plane, here a run of
/// tag characters, has a region 2 UTF-16 code units wide, and one such character earlier on
/// the line moves its start one unit further than the text output's column.
#[test]
fn sarif_counts_a_supplementary_character_at_a_finding_as_2_columns() {
    let tags = "\u{e0068}\u{e0069}";
    let text = format!("{tags} Fine text.\n\u{1f600} Fine {tags} text.\n");
    let dir = folder("sarif-supplementary", &[("tags.md", &text)]);
    let (code, out, _) = lint(&dir, &["tags.md"]);
    assert_eq!(code, 1, "{out}");
    let columns: Vec<&str> = out.lines().map(|l| l.split(": ").next().unwrap()).collect();
    assert_eq!(columns, ["tags.md:1:1", "tags.md:2:8"], "{out}");
    let (code, out, _) = lint(&dir, &["--sarif", "tags.md"]);
    assert_eq!(code, 1, "{out}");
    let log = parse(&out).unwrap();
    let regions: Vec<[String; 3]> = array(at(&log, &["runs", "0", "results"]))
        .iter()
        .map(|result| {
            assert_eq!(string(at(result, &["ruleId"])), "residue");
            let region = at(result, &["locations", "0", "physicalLocation", "region"]);
            ["startLine", "startColumn", "endColumn"].map(|key| match at(region, &[key]) {
                Json::Number(n) => n.clone(),
                other => panic!("{key} {other:?}"),
            })
        })
        .collect();
    assert_eq!(
        regions,
        [["1", "1", "3"], ["2", "9", "11"]].map(|r| r.map(String::from))
    );
}

/// `--sarif` changes what `lint` prints, never its exit status or what its filters keep, and
/// cannot be combined with `--json`.
#[test]
fn sarif_keeps_lints_exit_statuses() {
    let dir = folder(
        "sarif-status",
        &[
            ("two.md", TWO),
            ("clean.md", "The colour is red.\n"),
            ("advisory.md", "It could potentially rain.\n"),
        ],
    );
    for args in [
        &["clean.md"][..],
        &["advisory.md"],
        &["two.md"],
        &["--max-findings", "1", "two.md"],
        &["--max-findings=2", "two.md"],
        &["--warn", "two.md"],
        &["missing.md", "two.md", "--warn"],
        &["--only", "latin", "two.md"],
        &["--disable", "us-spelling", "two.md", "advisory.md"],
        &["--only", "stacked-hedge", "two.md", "advisory.md"],
    ] {
        let (plain, text, _) = lint(&dir, args);
        let sarif: Vec<&str> = ["--sarif"]
            .into_iter()
            .chain(args.iter().copied())
            .collect();
        let (code, out, _) = lint(&dir, &sarif);
        assert_eq!(code, plain, "{args:?}");
        let log = parse(&out).unwrap_or_else(|| panic!("{args:?}: {out}"));
        let complete = at(
            &log,
            &["runs", "0", "invocations", "0", "executionSuccessful"],
        );
        assert_eq!(complete, &Json::Bool(plain != 2), "{args:?}");
        let results = array(at(&log, &["runs", "0", "results"]));
        assert_eq!(results.len(), text.lines().count(), "{args:?}");
    }
    let (code, out, err) = lint(&dir, &["--sarif", "--json", "two.md"]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.contains("--json and --sarif"), "{err}");
}

/// README.md's exit-status table lists exactly the statuses `main.rs` can return.
#[test]
fn the_readme_lists_every_exit_status() {
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")).unwrap();
    let table: Vec<&str> = readme
        .lines()
        .skip_while(|line| !line.starts_with("| Exit status |"))
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|row| row.split('|').nth(1).unwrap().trim())
        .collect();
    let main = include_str!("../src/main.rs");
    let mut codes: Vec<&str> = main
        .split("ExitCode::from(")
        .skip(1)
        .map(|rest| rest.split(')').next().unwrap())
        .collect();
    if main.contains("ExitCode::SUCCESS") {
        codes.push("0");
    }
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(table, codes);
}

/// Golden outputs: `tests/golden/<case>.out` holds `lint`'s exact output on
/// `tests/golden/input.md` for each case, so any change to either format fails here. To
/// update them after a deliberate change, run `LACONIC_WRITE_GOLDEN=1 cargo test golden`.
#[test]
fn golden_outputs_are_unchanged() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut changed = Vec::new();
    for (case, args, status) in [
        ("text", &[][..], 1),
        ("json", &["--json"], 1),
        (
            "only",
            &["--json", "--only", "us-spelling,stacked-hedge"],
            1,
        ),
    ] {
        let args: Vec<&str> = args.iter().copied().chain(["input.md"]).collect();
        let (code, out, err) = lint(&dir, &args);
        assert_eq!((code, err.as_str()), (status, ""), "{case}");
        let path = dir.join(format!("{case}.out"));
        if std::env::var_os("LACONIC_WRITE_GOLDEN").is_some() {
            std::fs::write(&path, &out).unwrap();
        } else if std::fs::read_to_string(&path)
            .map(|s| s.replace("\r\n", "\n"))
            .ok()
            != Some(out)
        {
            changed.push(case);
        }
    }
    assert!(changed.is_empty(), "golden outputs differ: {changed:?}");
}
