// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `action-lint` against cases recorded from the Action's shell script, `action/lint.sh`:
//! `tests/action/recorded.txt` holds each case's standard output and exit status as the script
//! gave them. `shell_script_gives_the_recorded_results` reruns the script on every case, so the
//! recording cannot drift from it; delete that test and the recording test with the script.

use std::path::{Path, PathBuf};
use std::process::Command;

type Files = &'static [(&'static str, &'static str)];

const TWO: &str = "The color is red.\n\nThe flavor is sweet.\n";
const FAULT: &str = "# Seeded fault\n\nThe color of the sky changes slowly.\n";
const CLEAN: &str = "The colour is red.\n";
const ADVISORY: &str = "It could potentially rain.\n";

const PLAIN: Files = &[
    ("clean.md", CLEAN),
    ("two.md", TWO),
    ("advisory.md", ADVISORY),
];
const RULES: Files = &[
    (".laconic-check", "[rules]\nplain: utilise = use\n"),
    ("rule.md", "We utilise the logs.\n"),
];
const STRICT: Files = &[("strict.cfg", "max-findings = 1\n"), ("two.md", TWO)];
const MANY: Files = &[
    (".laconic-check", "max-findings = 1\n"),
    ("loose.cfg", "max-findings = 2\n"),
];

/// What the case's folder holds, besides the files it names.
#[derive(Clone, Copy, PartialEq)]
enum Setup {
    None,
    /// A Git repository where only the files named after `tracked` are added.
    Git(&'static [&'static str]),
    /// 600 clean files with long names between 2 seeded faults: more paths than 1 call takes.
    Many,
}

struct Case {
    name: &'static str,
    files: Files,
    setup: Setup,
    /// `LACONIC_FILES`; in a `Many` case, `many` stands for every file under `many/`.
    list: &'static str,
    /// `LACONIC_ARGS`.
    args: &'static str,
    unix_only: bool,
}

const fn case(name: &'static str, files: Files, list: &'static str, args: &'static str) -> Case {
    Case {
        name,
        files,
        setup: Setup::None,
        list,
        args,
        unix_only: false,
    }
}

const fn many(name: &'static str, list: &'static str, args: &'static str) -> Case {
    Case {
        setup: Setup::Many,
        ..case(name, MANY, list, args)
    }
}

const fn tracked(name: &'static str, files: Files, git: &'static [&'static str]) -> Case {
    Case {
        setup: Setup::Git(git),
        ..case(name, files, "", "")
    }
}

const fn unix(case: Case) -> Case {
    Case {
        unix_only: true,
        ..case
    }
}

fn cases() -> Vec<Case> {
    let tree: Files = &[
        ("a.md", FAULT),
        ("b.markdown", CLEAN),
        ("c.txt", FAULT),
        ("d.md", FAULT),
    ];
    vec![
        case("clean_file_passes", PLAIN, "clean.md", ""),
        case("failing_findings_fail", PLAIN, "two.md", ""),
        case("advisory_finding_warns", PLAIN, "advisory.md", ""),
        case("warn_keeps_status_zero", PLAIN, "two.md", "--warn"),
        case(
            "a_path_named_twice_counts_once",
            PLAIN,
            "two.md\ntwo.md\n",
            "--max-findings 2",
        ),
        case(
            "a_missing_file_is_status_2",
            PLAIN,
            "missing.md\nclean.md",
            "",
        ),
        case("json_is_refused", PLAIN, "clean.md", "--warn --json"),
        case("sarif_is_refused", PLAIN, "clean.md", "--sarif"),
        case(
            "an_unknown_option_is_status_2",
            PLAIN,
            "clean.md",
            "--bogus",
        ),
        case(
            "arguments_split_on_new_lines_and_tabs",
            PLAIN,
            "two.md",
            "--max-findings\n\t3\n",
        ),
        case(
            "blank_lines_in_the_list_are_skipped",
            PLAIN,
            "\n\nclean.md\n\n",
            "",
        ),
        tracked(
            "an_empty_list_takes_the_tracked_markdown",
            tree,
            &["a.md", "b.markdown", "c.txt"],
        ),
        tracked(
            "no_tracked_markdown_is_status_0",
            &[("c.txt", FAULT)],
            &["c.txt"],
        ),
        case("outside_a_repository_the_list_fails", PLAIN, "", ""),
        case(
            "a_name_with_a_leading_dash_gets_a_prefix",
            &[("-dash.md", FAULT)],
            "-dash.md",
            "",
        ),
        unix(case(
            "colons_in_a_name_are_escaped",
            &[("a:b.md", FAULT)],
            "a:b.md",
            "",
        )),
        case(
            "commas_and_percent_signs_are_escaped",
            &[("a,b%c.md", FAULT)],
            "a,b%c.md",
            "",
        ),
        case(
            "a_carriage_return_in_a_name_is_refused",
            PLAIN,
            "two.md\r\nclean.md\r\n",
            "",
        ),
        unix(tracked(
            "a_tracked_name_with_a_line_break_is_refused",
            &[("a\nb.md", FAULT)],
            &["a\nb.md"],
        )),
        many("limit_counts_across_calls", "many", "--max-findings 1"),
        many("limit_allows_2_across_calls", "many", "--max-findings 2"),
        many("limit_comes_from_the_project_file", "many", ""),
        many(
            "the_option_beats_the_project_file",
            "many",
            "--max-findings=5",
        ),
        many(
            "warn_skips_the_cross_call_limit",
            "many",
            "--warn --max-findings 1",
        ),
        many(
            "an_option_value_is_not_an_option",
            "many",
            "--disable serial-comma --max-findings 2",
        ),
        many(
            "limit_after_a_double_dash_is_a_file",
            "many",
            "-- --max-findings",
        ),
        many("status_2_after_status_1_wins", "many\nmissing.md", ""),
        many("status_2_before_status_1_wins", "missing.md\nmany", ""),
        many(
            "inline_config_names_the_project_file",
            "many",
            "--config=loose.cfg",
        ),
        case(
            "a_project_rule_is_printed_not_annotated",
            RULES,
            "rule.md",
            "",
        ),
        case(
            "config_option_names_the_project_file",
            STRICT,
            "two.md",
            "--config strict.cfg",
        ),
    ]
}

/// The recorded results: `== <name> <exit status>`, then the standard output.
const RECORDED: &str = include_str!("action/recorded.txt");

/// A case's recorded standard output and exit status. The recording names Linux's message for
/// a missing file; other systems word it differently.
fn expected(case: &Case) -> (String, i32) {
    let os = std::fs::read_to_string("no-such-file-for-the-message")
        .unwrap_err()
        .to_string();
    let header = format!("== {} ", case.name);
    let mut lines = RECORDED.lines();
    let code = lines
        .by_ref()
        .find_map(|line| line.strip_prefix(&header))
        .unwrap_or_else(|| panic!("{} is not recorded", case.name));
    let mut stdout = String::new();
    for line in lines.take_while(|line| !line.starts_with("== ")) {
        stdout.push_str(line);
        stdout.push('\n');
    }
    let stdout = stdout.replace("No such file or directory (os error 2)", &os);
    (stdout, code.parse().unwrap())
}

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_laconic-check")
}

fn git(dir: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap())
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    out.stdout
}

/// A fresh folder for one case.
fn folder(case: &Case, tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("action-{tag}-{}", case.name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in case.files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    match case.setup {
        Setup::None => {}
        Setup::Git(tracked) => {
            git(&dir, &["init", "-q"]);
            for name in tracked {
                git(&dir, &["add", "--", name]);
            }
        }
        Setup::Many => {
            std::fs::create_dir_all(dir.join("many")).unwrap();
            std::fs::write(dir.join("many/0-fault.md"), FAULT).unwrap();
            for i in 1..=600 {
                let name =
                    format!("many/a-clean-file-with-a-long-name-to-fill-a-command-line-{i}.md");
                std::fs::write(dir.join(name), CLEAN).unwrap();
            }
            std::fs::write(dir.join("many/z-fault.md"), FAULT).unwrap();
            git(&dir, &["init", "-q"]);
            git(&dir, &["add", "many"]);
        }
    }
    dir
}

/// `LACONIC_FILES`: the case's list, with `many` replaced by `git ls-files` order, one a line.
fn list(case: &Case, dir: &Path) -> String {
    if case.setup != Setup::Many {
        return case.list.to_owned();
    }
    let listing = String::from_utf8(git(dir, &["ls-files", "many"])).unwrap();
    case.list.replacen("many", &listing, 1)
}

/// Runs `command` in the case's folder as the Action would, returning its standard output
/// and exit status. Standard error must be empty unless Git is what failed.
fn run(case: &Case, tag: &str, mut command: Command) -> (String, i32) {
    let dir = folder(case, tag);
    let out = command
        .current_dir(&dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap())
        .env("LACONIC_CHECK", exe())
        .env("LACONIC_FILES", list(case, &dir))
        .env("LACONIC_ARGS", case.args)
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let from_git = case.name.contains("repository") || case.name.contains("line_break");
    assert!(
        stderr.is_empty() || from_git,
        "{}: unexpected standard error: {stderr}",
        case.name
    );
    (
        String::from_utf8(out.stdout).unwrap(),
        out.status.code().unwrap(),
    )
}

#[test]
fn action_lint_gives_the_recorded_results() {
    for case in cases().iter().filter(|c| !c.unix_only || cfg!(unix)) {
        let mut command = Command::new(exe());
        command.arg("action-lint");
        assert_eq!(run(case, "rust", command), expected(case), "{}", case.name);
    }
}

/// While `action/lint.sh` exists: the same cases through the script itself.
#[cfg(unix)]
#[test]
fn shell_script_gives_the_recorded_results() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("action/lint.sh");
    for case in cases() {
        let mut command = Command::new("bash");
        command.arg(&script);
        assert_eq!(
            run(&case, "shell", command),
            expected(&case),
            "{}",
            case.name
        );
    }
}

/// Prints each case's result from the script in `recorded.txt`'s form.
#[cfg(unix)]
#[test]
#[ignore = "records the script's results; run by hand with --nocapture"]
fn record_shell_results() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("action/lint.sh");
    for case in cases() {
        let mut command = Command::new("bash");
        command.arg(&script);
        let (stdout, code) = run(&case, "record", command);
        println!("RECORD== {} {code}\nRECORD{stdout}", case.name);
    }
}

/// The recording lists every case, once.
#[test]
fn every_case_is_recorded_once() {
    for case in cases() {
        let header = format!("\n== {} ", case.name);
        assert_eq!(
            format!("\n{RECORDED}").matches(&header).count(),
            1,
            "{}",
            case.name
        );
    }
}
