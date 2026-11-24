// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `action-lint` against cases recorded from the Action's shell script,
//! `action/lint.sh`: each case's standard output and exit status are what the script printed
//! and returned for the same files and arguments. `shell_script_gives_the_recorded_results`
//! reruns the script on every case, so the recording cannot drift from it; delete that test
//! with the script.

use std::path::{Path, PathBuf};
use std::process::Command;

const TWO: &str = "The color is red.\n\nThe flavor is sweet.\n";
const FAULT: &str = "# Seeded fault\n\nThe color of the sky changes slowly.\n";
const CLEAN: &str = "The colour is red.\n";
const ADVISORY: &str = "It could potentially rain.\n";

/// What the case's folder holds, besides the files it names.
#[derive(Clone, Copy)]
enum Setup {
    None,
    /// A Git repository where only the files named after `tracked` are added.
    Git(&'static [&'static str]),
    /// 600 clean files with long names between 2 seeded faults: more paths than 1 call takes.
    Many,
}

struct Case {
    name: &'static str,
    files: &'static [(&'static str, &'static str)],
    setup: Setup,
    /// `LACONIC_FILES`.
    list: &'static str,
    /// `LACONIC_ARGS`.
    args: &'static str,
    unix_only: bool,
}

const fn case(name: &'static str, list: &'static str, args: &'static str) -> Case {
    Case {
        name,
        files: &[],
        setup: Setup::None,
        list,
        args,
        unix_only: false,
    }
}

const MANY_FILES: &[(&str, &str)] = &[
    (".laconic-check", "max-findings = 1\n"),
    ("loose.cfg", "max-findings = 2\n"),
];

fn cases() -> Vec<Case> {
    let plain = &[
        ("clean.md", CLEAN),
        ("two.md", TWO),
        ("advisory.md", ADVISORY),
    ];
    let rules = &[
        (".laconic-check", "[rules]\nplain: utilise = use\n"),
        ("rule.md", "We utilise the logs.\n"),
        ("two.md", TWO),
    ];
    let strict = &[("strict.cfg", "max-findings = 1\n"), ("two.md", TWO)];
    vec![
        Case {
            files: plain,
            ..case("clean_file_passes", "clean.md", "")
        },
        Case {
            files: plain,
            ..case("failing_findings_fail", "two.md", "")
        },
        Case {
            files: plain,
            ..case("advisory_finding_warns", "advisory.md", "")
        },
        Case {
            files: plain,
            ..case("warn_keeps_status_zero", "two.md", "--warn")
        },
        Case {
            files: plain,
            ..case(
                "a_path_named_twice_counts_once",
                "two.md\ntwo.md\n",
                "--max-findings 2",
            )
        },
        Case {
            files: plain,
            ..case("a_missing_file_is_status_2", "missing.md\nclean.md", "")
        },
        Case {
            files: plain,
            ..case("json_is_refused", "clean.md", "--warn --json")
        },
        Case {
            files: plain,
            ..case("sarif_is_refused", "clean.md", "--sarif")
        },
        Case {
            files: plain,
            ..case("an_unknown_option_is_status_2", "clean.md", "--bogus")
        },
        Case {
            files: plain,
            ..case(
                "arguments_split_on_new_lines_and_tabs",
                "two.md",
                "--max-findings\n\t3\n",
            )
        },
        Case {
            files: plain,
            ..case(
                "blank_lines_in_the_list_are_skipped",
                "\n\nclean.md\n\n",
                "",
            )
        },
        Case {
            files: &[
                ("a.md", FAULT),
                ("b.markdown", CLEAN),
                ("c.txt", FAULT),
                ("d.md", FAULT),
            ],
            setup: Setup::Git(&["a.md", "b.markdown", "c.txt"]),
            ..case("an_empty_list_takes_the_tracked_markdown", "", "")
        },
        Case {
            files: &[("c.txt", FAULT)],
            setup: Setup::Git(&["c.txt"]),
            ..case("no_tracked_markdown_is_status_0", "", "")
        },
        Case {
            files: plain,
            ..case("outside_a_repository_the_list_fails", "", "")
        },
        Case {
            files: &[("-dash.md", FAULT)],
            ..case("a_name_with_a_leading_dash_gets_a_prefix", "-dash.md", "")
        },
        Case {
            files: &[("a:b.md", FAULT)],
            unix_only: true,
            ..case("colons_in_a_name_are_escaped", "a:b.md", "")
        },
        Case {
            files: &[("a,b%c.md", FAULT)],
            ..case("commas_and_percent_signs_are_escaped", "a,b%c.md", "")
        },
        Case {
            files: plain,
            ..case(
                "a_carriage_return_in_a_name_is_refused",
                "two.md\r\nclean.md\r\n",
                "",
            )
        },
        Case {
            files: &[("a\nb.md", FAULT)],
            setup: Setup::Git(&["a\nb.md"]),
            unix_only: true,
            ..case("a_tracked_name_with_a_line_break_is_refused", "", "")
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case("limit_counts_across_calls", "many", "--max-findings 1")
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case("limit_allows_2_across_calls", "many", "--max-findings 2")
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case("limit_comes_from_the_project_file", "many", "")
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case(
                "the_option_beats_the_project_file",
                "many",
                "--max-findings=5",
            )
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case(
                "warn_skips_the_cross_call_limit",
                "many",
                "--warn --max-findings 1",
            )
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case(
                "an_option_value_is_not_an_option",
                "many",
                "--disable serial-comma --max-findings 2",
            )
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case(
                "limit_after_a_double_dash_is_a_file",
                "many",
                "-- --max-findings",
            )
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case("status_2_after_status_1_wins", "many\nmissing.md", "")
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case("status_2_before_status_1_wins", "missing.md\nmany", "")
        },
        Case {
            files: MANY_FILES,
            setup: Setup::Many,
            ..case(
                "inline_config_names_the_project_file",
                "many",
                "--config=loose.cfg",
            )
        },
        Case {
            files: rules,
            ..case("a_project_rule_is_printed_not_annotated", "rule.md", "")
        },
        Case {
            files: strict,
            ..case(
                "config_option_names_the_project_file",
                "two.md",
                "--config strict.cfg",
            )
        },
    ]
}

/// Each case's result, recorded from `action/lint.sh`: name, exit status, standard output.
const RECORDED: &[(&str, i32, &str)] = &[
    ("clean_file_passes", 0, ""),
    (
        "failing_findings_fail",
        1,
        "laconic-check: two.md:1:5: us-spelling: 'color': Australian form is 'colour'\n::error file=two.md,line=1,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: two.md:3:5: us-spelling: 'flavor': Australian form is 'flavour'\n::error file=two.md,line=3,col=5,title=laconic-check us-spelling::'flavor': Australian form is 'flavour'\n",
    ),
    (
        "advisory_finding_warns",
        0,
        "laconic-check: advisory.md:1:4: stacked-hedge: stacked hedge 'could potentially' (advisory); keep one qualifier at the confidence the source supports\n::warning file=advisory.md,line=1,col=4,title=laconic-check stacked-hedge::stacked hedge 'could potentially' (advisory); keep one qualifier at the confidence the source supports\n",
    ),
    (
        "warn_keeps_status_zero",
        0,
        "laconic-check: two.md:1:5: us-spelling: 'color': Australian form is 'colour'\n::error file=two.md,line=1,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: two.md:3:5: us-spelling: 'flavor': Australian form is 'flavour'\n::error file=two.md,line=3,col=5,title=laconic-check us-spelling::'flavor': Australian form is 'flavour'\n",
    ),
    (
        "a_path_named_twice_counts_once",
        0,
        "laconic-check: two.md:1:5: us-spelling: 'color': Australian form is 'colour'\n::error file=two.md,line=1,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: two.md:3:5: us-spelling: 'flavor': Australian form is 'flavour'\n::error file=two.md,line=3,col=5,title=laconic-check us-spelling::'flavor': Australian form is 'flavour'\n",
    ),
    (
        "a_missing_file_is_status_2",
        2,
        "laconic-check: missing.md: No such file or directory (os error 2)\n",
    ),
    (
        "json_is_refused",
        2,
        "::error::the Action reads lint's text output, so args cannot include --json\n",
    ),
    (
        "sarif_is_refused",
        2,
        "::error::the Action reads lint's text output, so args cannot include --sarif\n",
    ),
    (
        "an_unknown_option_is_status_2",
        2,
        "laconic-check: laconic-check: unknown option '--bogus'\n",
    ),
    (
        "arguments_split_on_new_lines_and_tabs",
        0,
        "laconic-check: two.md:1:5: us-spelling: 'color': Australian form is 'colour'\n::error file=two.md,line=1,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: two.md:3:5: us-spelling: 'flavor': Australian form is 'flavour'\n::error file=two.md,line=3,col=5,title=laconic-check us-spelling::'flavor': Australian form is 'flavour'\n",
    ),
    ("blank_lines_in_the_list_are_skipped", 0, ""),
    (
        "an_empty_list_takes_the_tracked_markdown",
        1,
        "laconic-check: a.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=a.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "no_tracked_markdown_is_status_0",
        0,
        "No Markdown files to lint.\n",
    ),
    (
        "outside_a_repository_the_list_fails",
        2,
        "::error::cannot list the Markdown files Git tracks; check out the repository first, or name files in the files input\n",
    ),
    (
        "a_name_with_a_leading_dash_gets_a_prefix",
        1,
        "laconic-check: ./-dash.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=./-dash.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "colons_in_a_name_are_escaped",
        1,
        "laconic-check: a:b.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=a%3Ab.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "commas_and_percent_signs_are_escaped",
        1,
        "laconic-check: a,b%c.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=a%2Cb%25c.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "a_carriage_return_in_a_name_is_refused",
        2,
        "::error::the Action cannot annotate a filename containing a line break\n",
    ),
    (
        "a_tracked_name_with_a_line_break_is_refused",
        2,
        "::error::the Action cannot annotate a filename containing a line break\n",
    ),
    (
        "limit_counts_across_calls",
        1,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n::error::2 failing findings across all the files, more than --max-findings allows (1)\n",
    ),
    (
        "limit_allows_2_across_calls",
        0,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "limit_comes_from_the_project_file",
        1,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n::error::2 failing findings across all the files, more than --max-findings allows (1)\n",
    ),
    (
        "the_option_beats_the_project_file",
        0,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "warn_skips_the_cross_call_limit",
        0,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "an_option_value_is_not_an_option",
        0,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "limit_after_a_double_dash_is_a_file",
        2,
        "laconic-check: --max-findings: No such file or directory (os error 2)\nlaconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: --max-findings: No such file or directory (os error 2)\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "status_2_after_status_1_wins",
        2,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: missing.md: No such file or directory (os error 2)\n",
    ),
    (
        "status_2_before_status_1_wins",
        2,
        "laconic-check: missing.md: No such file or directory (os error 2)\nlaconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "inline_config_names_the_project_file",
        0,
        "laconic-check: many/0-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/0-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: many/z-fault.md:3:5: us-spelling: 'color': Australian form is 'colour'\n::error file=many/z-fault.md,line=3,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\n",
    ),
    (
        "a_project_rule_is_printed_not_annotated",
        1,
        "laconic-check: rule.md:1:4: project/plain: 'utilise': the project file prefers 'use'\n",
    ),
    (
        "config_option_names_the_project_file",
        1,
        "laconic-check: two.md:1:5: us-spelling: 'color': Australian form is 'colour'\n::error file=two.md,line=1,col=5,title=laconic-check us-spelling::'color': Australian form is 'colour'\nlaconic-check: two.md:3:5: us-spelling: 'flavor': Australian form is 'flavour'\n::error file=two.md,line=3,col=5,title=laconic-check us-spelling::'flavor': Australian form is 'flavour'\n",
    ),
];

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_laconic-check")
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

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap())
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// The list the Action is given: `list` as written, except `many` is every file under it,
/// in `git ls-files` order, one a line.
fn list(case: &Case, dir: &Path) -> String {
    if !matches!(case.setup, Setup::Many) {
        return case.list.to_owned();
    }
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap())
        .args(["ls-files", "many"])
        .output()
        .unwrap();
    let listing = String::from_utf8(out.stdout).unwrap();
    case.list.replacen("many", &listing, 1)
}

/// Runs `command` in the case's folder as the Action would, returning its standard output
/// and exit status; standard error must be empty unless the case expects git's message.
fn run(case: &Case, dir: &Path, mut command: Command) -> (String, i32) {
    let out = command
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap())
        .env("LACONIC_CHECK", exe())
        .env("LACONIC_FILES", list(case, dir))
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

/// The recorded text names Linux's message for a missing file; other systems word it
/// differently.
fn expected(case: &Case) -> (String, i32) {
    let os = std::fs::read_to_string("no-such-file-for-the-message")
        .unwrap_err()
        .to_string();
    let (_, code, stdout) = RECORDED
        .iter()
        .find(|(name, ..)| *name == case.name)
        .unwrap();
    (
        stdout.replace("No such file or directory (os error 2)", &os),
        *code,
    )
}

#[test]
fn action_lint_gives_the_recorded_results() {
    for case in cases() {
        if case.unix_only && !cfg!(unix) {
            continue;
        }
        let dir = folder(&case, "rust");
        let mut command = Command::new(exe());
        command.arg("action-lint");
        let (stdout, code) = run(&case, &dir, command);
        assert_eq!((stdout, code), expected(&case), "{}", case.name);
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
        let dir = folder(&case, "shell");
        let mut command = Command::new("bash");
        command.arg(&script);
        let (stdout, code) = run(&case, &dir, command);
        assert_eq!((stdout, code), expected(&case), "{}", case.name);
    }
}

/// Prints each case's result from the script, as a Rust literal, for pasting above.
#[cfg(unix)]
#[test]
#[ignore = "records the script's results; run by hand with --nocapture"]
fn record_shell_results() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("action/lint.sh");
    for case in cases() {
        let dir = folder(&case, "record");
        let mut command = Command::new("bash");
        command.arg(&script);
        let (stdout, code) = run(&case, &dir, command);
        println!("RECORD {} | {code} | {stdout:?}", case.name);
    }
}
