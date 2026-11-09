// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! laconic-check: deterministic candidates for Laconic's house style.
//!
//! `lint` reports candidates in the original prose of Markdown files; a match is a
//! reason to look, not proof of a defect. `preserve` compares an original and an
//! edit and reports numbers, quantities, months, code spans, URLs, quotations, meaning words and
//! bounds the edit
//! dropped, and, as advice, those it added. `mcp` serves both as MCP tools on standard
//! input and output.

use std::env;
use std::fs;
use std::process::ExitCode;

use laconic_check::mcp::Json;
use laconic_check::{lint, mcp, preserve};

fn usage() -> ExitCode {
    let advisory: Vec<&str> = lint::RULES
        .iter()
        .filter(|r| r.advisory)
        .map(|r| r.name)
        .collect();
    eprintln!(
        "usage:\n  laconic-check lint [options] <file.md | ->...\n  laconic-check preserve [--strict] <original> <edited>\n  laconic-check mcp\n\n\
         lint options:\n  \
           --json              print one JSON document instead of a line a finding\n  \
           --disable <rules>   drop these rules' findings (comma-separated, repeatable)\n  \
           --only <rules>      keep only these rules' findings (comma-separated, repeatable)\n  \
           --max-findings <n>  fail only on more than n failing findings (default 0)\n  \
           --warn              never fail on findings\n  \
           --commit-msg        read each file as a Git commit message\n  \
           --config <file>     read settings from this file instead of .laconic-check\n\n\
         lint exits 1 when it finds more candidates than --max-findings allows, unless\n\
         --warn is given; advisory findings ({}) never fail it. Each finding names\n\
         its rule, described under that name in {}\n\
         preserve exits 1 when the edit dropped a protected item. It also lists items the\n\
         edit added, which fail only with --strict (for a rewrite that must add nothing),\n\
         and names to check, which never fail. A clean result does not check meaning.\n\
         mcp serves lint and preserve as MCP tools over standard input and output.",
        advisory.join(", "),
        lint::RULES_URL
    );
    ExitCode::from(2)
}

/// Reads a file, or standard input when the path is `-`.
fn read(path: &str) -> Result<String, ExitCode> {
    let result = if path == "-" {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text).map(|_| text)
    } else {
        fs::read_to_string(path)
    };
    result.map_err(|e| {
        eprintln!("{path}: {e}");
        ExitCode::from(2)
    })
}

/// `lint`'s settings: the project file's, then the command line's.
#[derive(Debug, Default, PartialEq)]
struct Options {
    warn: bool,
    json: bool,
    commit_msg: bool,
    disable: Vec<&'static str>,
    only: Vec<&'static str>,
    max_findings: usize,
    files: Vec<String>,
}

/// The project file `lint` reads from the current directory when `--config` names none.
const PROJECT_FILE: &str = ".laconic-check";

/// `lint`'s arguments, applied over the project file. Options may come anywhere before `--`;
/// after it, every argument is a file.
fn options(args: &[String]) -> Result<Options, String> {
    let mut cli = Options::default();
    let (mut config, mut max_findings, mut operands) = (None, None, false);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if operands || arg == "-" || !arg.starts_with('-') {
            cli.files.push(arg.clone());
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (arg.as_str(), None),
        };
        let takes_value = matches!(name, "--disable" | "--only" | "--max-findings" | "--config");
        if !takes_value && !matches!(name, "--" | "--warn" | "--json" | "--commit-msg") {
            return Err(format!("unknown option '{name}'"));
        }
        let value = match (takes_value, inline) {
            (true, Some(value)) => value,
            (true, None) => rest.next().ok_or(format!("{name} needs a value"))?,
            (false, Some(_)) => return Err(format!("{name} takes no value")),
            (false, None) => "",
        };
        match name {
            "--" => operands = true,
            "--warn" => cli.warn = true,
            "--json" => cli.json = true,
            "--commit-msg" => cli.commit_msg = true,
            "--disable" => cli.disable.extend(rule_names(value)?),
            "--only" => cli.only.extend(rule_names(value)?),
            "--max-findings" => max_findings = Some(count(value)?),
            _ => config = Some(value.to_owned()),
        }
    }
    let path = config.or_else(|| {
        std::path::Path::new(PROJECT_FILE)
            .is_file()
            .then(|| PROJECT_FILE.to_owned())
    });
    let mut options = match path {
        Some(path) => project_file(
            &path,
            &fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?,
        )?,
        None => Options::default(),
    };
    options.disable.extend(cli.disable);
    options.only.extend(cli.only);
    options.max_findings = max_findings.unwrap_or(options.max_findings);
    Ok(Options {
        warn: cli.warn,
        json: cli.json,
        commit_msg: cli.commit_msg,
        files: cli.files,
        ..options
    })
}

/// A project file: `key = value` lines for `disable`, `only` and `max-findings`, with blank
/// lines and lines that start with `#` ignored.
fn project_file(path: &str, text: &str) -> Result<Options, String> {
    let mut options = Options::default();
    for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |message: String| format!("{path}:{}: {message}", index + 1);
        let Some((key, value)) = line.split_once('=') else {
            return Err(at(format!("expected 'key = value', found '{line}'")));
        };
        let value = value.trim();
        match key.trim() {
            "disable" => options.disable.extend(rule_names(value).map_err(at)?),
            "only" => options.only.extend(rule_names(value).map_err(at)?),
            "max-findings" => options.max_findings = count(value).map_err(at)?,
            key => return Err(at(format!("unknown key '{key}'"))),
        }
    }
    Ok(options)
}

/// A comma-separated list of rule names, each one in the rule table.
fn rule_names(list: &str) -> Result<Vec<&'static str>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            lint::RuleId::named(name)
                .map(lint::RuleId::name)
                .ok_or_else(|| {
                    format!(
                        "unknown rule '{name}'; the rules are in {}",
                        lint::RULES_URL
                    )
                })
        })
        .collect()
}

fn count(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("'{value}' is not a whole number"))
}

/// Git's scissors line: it drops this line and everything after it from a commit message.
const SCISSORS: &str = "# ------------------------ >8 ------------------------";

/// The part of a commit message Git keeps when it opens an editor: lines from the scissors
/// line on are dropped and comment lines are blanked, so line numbers still match the file.
fn commit_message(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    for line in text.lines().take_while(|line| *line != SCISSORS) {
        if !line.starts_with('#') {
            kept.push_str(line);
        }
        kept.push('\n');
    }
    kept
}

fn run_lint(options: &Options) -> ExitCode {
    let (mut findings, mut failing, mut suppressed) = (Vec::new(), 0, 0);
    for file in &options.files {
        let text = match read(file) {
            Ok(t) if options.commit_msg => commit_message(&t),
            Ok(t) => t,
            Err(code) => return code,
        };
        let (found, silenced) = lint::lint_counting(&text);
        suppressed += silenced;
        for f in found {
            let name = f.rule.name();
            if options.disable.contains(&name)
                || !(options.only.is_empty() || options.only.contains(&name))
            {
                continue;
            }
            let advisory = f.rule.rule().advisory;
            if !advisory {
                failing += 1;
            }
            if options.json {
                findings.push(Json::Object(vec![
                    ("file".into(), Json::String(file.clone())),
                    ("line".into(), Json::Number(f.line.to_string())),
                    ("column".into(), Json::Number(f.column.to_string())),
                    ("rule".into(), Json::String(name.into())),
                    ("advisory".into(), Json::Bool(advisory)),
                    ("message".into(), Json::String(f.message)),
                    (
                        "url".into(),
                        Json::String(format!("{}#{name}", lint::RULES_URL)),
                    ),
                ]));
            } else {
                println!("{file}:{}:{}: {name}: {}", f.line, f.column, f.message);
            }
        }
    }
    if options.json {
        let report = Json::Object(vec![
            ("findings".into(), Json::Array(findings)),
            ("failing".into(), Json::Number(failing.to_string())),
            ("suppressed".into(), Json::Number(suppressed.to_string())),
        ]);
        println!("{}", mcp::to_string(&report));
    }
    if failing > options.max_findings && !options.warn {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("lint") => match options(&args[1..]) {
            Ok(options) if !options.files.is_empty() => run_lint(&options),
            Ok(_) => usage(),
            Err(message) => {
                eprintln!("laconic-check: {message}");
                ExitCode::from(2)
            }
        },
        Some("preserve") => {
            // `--strict` may come anywhere before `--`; after it, every argument is a file.
            let (mut strict, mut options, mut files) = (false, true, Vec::new());
            for arg in &args[1..] {
                match arg.as_str() {
                    "--strict" if options => strict = true,
                    "--" if options => options = false,
                    _ => files.push(arg),
                }
            }
            // Standard input can be read once, so only one of the 2 can be `-`.
            let [original, edited] = files[..] else {
                return usage();
            };
            if original == "-" && edited == "-" {
                return usage();
            }
            let (original, edited) = match (read(original), read(edited)) {
                (Ok(o), Ok(e)) => (o, e),
                (Err(code), _) | (_, Err(code)) => return code,
            };
            let report = preserve::report(&original, &edited);
            for line in &report.lines {
                println!("{line}");
            }
            if report.dropped || (strict && report.added) {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Some("mcp") if args.len() == 1 => serve(),
        _ => usage(),
    }
}

/// Answers MCP requests, one JSON-RPC message a line, until standard input closes. A line
/// that is not UTF-8 is read with replacement characters and answered like any other.
fn serve() -> ExitCode {
    match mcp::serve(std::io::stdin().lock(), std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mcp: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_options_in_either_form_and_only_files_after_a_double_dash() {
        let read = options(&args(&[
            "--json",
            "--disable=us-spelling, em-dash",
            "a.md",
            "--only",
            "word-choice",
            "--max-findings",
            "3",
            "-",
            "--",
            "--warn",
        ]))
        .unwrap();
        assert_eq!(
            read,
            Options {
                json: true,
                disable: vec!["us-spelling", "em-dash"],
                only: vec!["word-choice"],
                max_findings: 3,
                files: args(&["a.md", "-", "--warn"]),
                ..Options::default()
            }
        );
    }

    #[test]
    fn rejects_unknown_rules_options_and_numbers() {
        for (list, wanted) in [
            (
                &["--disable", "us-spellings"][..],
                "unknown rule 'us-spellings'",
            ),
            (&["--colour"], "unknown option '--colour'"),
            (&["-x"], "unknown option '-x'"),
            (&["--max-findings=-1"], "'-1' is not a whole number"),
            (&["--max-findings"], "--max-findings needs a value"),
            (&["--warn=yes"], "--warn takes no value"),
        ] {
            let error = options(&args(list)).unwrap_err();
            assert!(error.starts_with(wanted), "{list:?}: {error}");
        }
    }

    #[test]
    fn the_command_line_adds_rules_to_the_project_file_and_overrides_its_count() {
        let dir =
            std::env::temp_dir().join(format!("laconic-check-options-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let config = dir.join("settings");
        fs::write(
            &config,
            "\u{feff}# settings\n\ndisable = em-dash,\nonly = us-spelling\nmax-findings = 4\n",
        )
        .unwrap();
        let config = config.to_str().unwrap();
        let read = options(&args(&["--config", config, "--disable", "latin", "a.md"])).unwrap();
        assert_eq!(read.disable, ["em-dash", "latin"]);
        assert_eq!(read.only, ["us-spelling"]);
        assert_eq!(read.max_findings, 4);
        let read = options(&args(&["--config", config, "--max-findings", "0", "a.md"])).unwrap();
        assert_eq!(read.max_findings, 0);
        let missing = dir.join("missing");
        let error = options(&args(&["--config", missing.to_str().unwrap()])).unwrap_err();
        assert!(error.starts_with(missing.to_str().unwrap()), "{error}");
    }

    #[test]
    fn a_project_file_names_the_line_of_its_first_error() {
        for (text, wanted) in [
            (
                "disable = em-dash\nseverity = high\n",
                "f:2: unknown key 'severity'",
            ),
            (
                "\n\ndisable em-dash\n",
                "f:3: expected 'key = value', found 'disable em-dash'",
            ),
            ("only = em-dashes\n", "f:1: unknown rule 'em-dashes'"),
            ("max-findings = many\n", "f:1: 'many' is not a whole number"),
        ] {
            let error = project_file("f", text).unwrap_err();
            assert!(error.starts_with(wanted), "{text:?}: {error}");
        }
    }

    #[test]
    fn a_commit_message_keeps_what_git_keeps_on_the_same_lines() {
        let message = "fix: stop the color\n\n# Please enter the commit message\nBody text.\n\
                       # ------------------------ >8 ------------------------\n\
                       # Do not modify or remove the line above.\n+color\n";
        assert_eq!(
            commit_message(message),
            "fix: stop the color\n\n\nBody text.\n"
        );
        assert_eq!(commit_message("one\r\n#two\r\n"), "one\n\n");
    }
}
