// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! laconic-check: deterministic candidates for Laconic's house style.
//!
//! `lint` reports candidates in the original prose of Markdown files; a match is a
//! reason to look, not proof of a defect. `preserve` compares an original and an
//! edit and reports numbers, quantities, months, code spans, URLs, quotations, meaning words and
//! bounds the edit
//! dropped, and, as advice, those it added. `mcp` serves both as MCP tools on standard
//! input and output, and `lsp` publishes `lint`'s findings to an editor.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::process::ExitCode;

use laconic_check::mcp::Json;
use laconic_check::settings::{self, PROJECT_FILE, Settings};
use laconic_check::{completions, hook, lint, lsp, mcp, preserve, sarif};

fn usage_text() -> String {
    let advisory: Vec<&str> = lint::RULES
        .iter()
        .filter(|r| r.advisory)
        .map(|r| r.name)
        .collect();
    format!(
        "usage:\n  laconic-check lint [options] <file.md | ->...\n  laconic-check preserve [--strict] [--loose] <original> <edited>\n  laconic-check mcp\n  laconic-check lsp\n  laconic-check hook\n  laconic-check completions <bash|zsh|fish|powershell>\n\n\
         lint options:\n  \
           --json              print one JSON document instead of a line a finding\n  \
           --sarif             print a SARIF 2.1.0 log instead of a line a finding\n  \
           --disable <rules>   drop these rules' findings (comma-separated, repeatable)\n  \
           --only <rules>      keep only these rules' findings (comma-separated, repeatable)\n  \
           --max-findings <n>  fail only on more than n failing findings (default 0)\n  \
           --warn              never fail on findings\n  \
           --commit-msg        read each file as a Git commit message\n  \
           --config <file>     read settings from this file instead of .laconic-check\n\n\
         lint exits 1 when it finds more candidates than --max-findings allows, unless\n\
         --warn is given; advisory findings ({}) never fail it. Each finding names\n\
         its rule, described under that name in {}\n\
         or a rule of the project file's [rules] section, named project/<id>.\n\
         preserve exits 1 when the edit dropped a protected item. It also lists items the\n\
         edit added, which fail only with --strict (for a rewrite that must add nothing),\n\
         and names to check, which never fail. With --loose, it lists a lost word of\n\
         frequency, quantity, ability, prediction, cause or hedging, such as 'often' or\n\
         'can', to check instead; a swap such as 'all' to 'some' adds a word, which --strict\n\
         fails on.\n\
         A clean result does not check meaning.\n\
         mcp serves lint and preserve as MCP tools over standard input and output; its\n\
         lint applies the .laconic-check file in the directory it runs in.\n\
         lsp is a language server over standard input and output: it publishes lint's\n\
         findings for open Markdown documents, and their fixes as quick fixes, with the\n\
         settings of the .laconic-check file in the workspace folder that holds each one.\n\
         hook reads a Claude Code or Codex Stop event on standard input and, once a turn,\n\
         asks the agent to look at lint's failing findings in the uncommitted Markdown of\n\
         the repository that holds the event's directory before it finishes.\n\
         completions prints a shell completion script for bash, zsh, fish or powershell.\n\
         --version prints the version and --help this text.",
        advisory.join(", "),
        lint::RULES_URL
    )
}

fn usage() -> ExitCode {
    eprintln!("{}", usage_text());
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

/// `lint`'s options: the project file's settings, then the command line's.
#[derive(Debug, Default, PartialEq)]
struct Options {
    warn: bool,
    json: bool,
    sarif: bool,
    commit_msg: bool,
    settings: Settings,
    files: Vec<String>,
}

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
        if !takes_value
            && !matches!(
                name,
                "--" | "--warn" | "--json" | "--sarif" | "--commit-msg"
            )
        {
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
            "--sarif" => cli.sarif = true,
            "--commit-msg" => cli.commit_msg = true,
            "--disable" => cli.settings.disable.extend(settings::rule_names(value)),
            "--only" => cli.settings.only.extend(settings::rule_names(value)),
            "--max-findings" => max_findings = Some(settings::count(value)?),
            _ => config = Some(value.to_owned()),
        }
    }
    if cli.json && cli.sarif {
        return Err("--json and --sarif cannot be combined".into());
    }
    let path = config.or_else(|| {
        std::path::Path::new(PROJECT_FILE)
            .is_file()
            .then(|| PROJECT_FILE.to_owned())
    });
    let mut settings = match path {
        Some(path) => Settings::read(&path)?,
        None => Settings::default(),
    };
    // A rule named on the command line may be one of the project file's.
    for name in cli.settings.disable.iter().chain(&cli.settings.only) {
        settings.check_rule(name)?;
    }
    settings.disable.extend(cli.settings.disable);
    settings.only.extend(cli.settings.only);
    settings.max_findings = max_findings.unwrap_or(settings.max_findings);
    Ok(Options { settings, ..cli })
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
    let (mut findings, mut failing, mut suppressed, mut unread) = (Vec::new(), 0, 0, false);
    for file in &options.files {
        // `read` reports an unreadable file; the rest are still linted, and lint exits 2.
        let text = match read(file) {
            Ok(t) if options.commit_msg => commit_message(&t),
            Ok(t) => t,
            Err(_) => {
                unread = true;
                continue;
            }
        };
        // SARIF and accepted words need the text of a finding's line.
        let lines: Vec<&str> = if options.sarif || !options.settings.accept.is_empty() {
            text.lines().collect()
        } else {
            Vec::new()
        };
        let uri = if options.sarif {
            sarif::artifact_uri(file)
        } else {
            String::new()
        };
        let (found, silenced) = lint::lint_counting(&text, &options.settings.rules);
        suppressed += silenced;
        for f in found {
            let line = f.line.checked_sub(1).and_then(|i| lines.get(i)).copied();
            if !options.settings.keeps(&f, line.unwrap_or("")) {
                continue;
            }
            let name = f.rule.name().to_owned();
            let advisory = f.rule.advisory();
            if !advisory {
                failing += 1;
            }
            if options.json {
                findings.push(Json::Object(vec![
                    ("file".into(), Json::String(file.clone())),
                    ("line".into(), Json::Number(f.line.to_string())),
                    ("column".into(), Json::Number(f.column.to_string())),
                    ("rule".into(), Json::String(name)),
                    ("advisory".into(), Json::Bool(advisory)),
                    ("message".into(), Json::String(f.message)),
                    ("url".into(), f.rule.url().map_or(Json::Null, Json::String)),
                ]));
            } else if options.sarif {
                findings.push(sarif::result(
                    &uri,
                    f,
                    line.unwrap_or(""),
                    &options.settings.rules,
                ));
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
    } else if options.sarif {
        println!(
            "{}",
            mcp::to_string(&sarif::log(findings, !unread, &options.settings.rules))
        );
    }
    if unread {
        ExitCode::from(2)
    } else if failing > options.settings.max_findings && !options.warn {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    // `env::args` panics on an argument that is not valid Unicode.
    let args: Vec<String> = match env::args_os().skip(1).map(OsString::into_string).collect() {
        Ok(args) => args,
        Err(arg) => {
            eprintln!(
                "laconic-check: an argument is not valid Unicode: {}",
                arg.to_string_lossy()
            );
            return ExitCode::from(2);
        }
    };
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
            // `--strict` and `--loose` may come anywhere before `--`; after it, every argument
            // is a file.
            let (mut strict, mut loose, mut options, mut files) = (false, false, true, Vec::new());
            for arg in &args[1..] {
                match arg.as_str() {
                    "--strict" if options => strict = true,
                    "--loose" if options => loose = true,
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
            let report = preserve::report(&original, &edited, loose);
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
        Some("--version") if args.len() == 1 => {
            println!("laconic-check {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("lsp") if args.len() == 1 => {
            match lsp::serve(std::io::stdin().lock(), std::io::stdout().lock()) {
                // The protocol's exit status: 0 after shutdown, 1 for an exit without one.
                Ok(true) => ExitCode::SUCCESS,
                Ok(false) => ExitCode::from(1),
                Err(e) => {
                    eprintln!("lsp: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Some("hook") if args.len() == 1 => stop_hook(),
        Some("completions") if args.len() == 2 => match completions::Shell::parse(&args[1]) {
            Some(shell) => {
                print!("{}", completions::script(shell));
                ExitCode::SUCCESS
            }
            None => {
                eprintln!(
                    "laconic-check: unknown shell '{}' (expected bash, zsh, fish or powershell)",
                    args[1]
                );
                ExitCode::from(2)
            }
        },
        Some("--help") if args.len() == 1 => {
            println!("{}", usage_text());
            ExitCode::SUCCESS
        }
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

/// Answers one Stop event on standard input with a block decision or nothing. Claude Code
/// reports exit status 1 as a hook error without blocking the turn.
fn stop_hook() -> ExitCode {
    let mut input = String::new();
    let read = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input);
    match read
        .map_err(|e| e.to_string())
        .and_then(|_| hook::stop(&input))
    {
        Ok(decision) => {
            if let Some(decision) = decision {
                println!("{decision}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("laconic-check hook: {message}");
            ExitCode::from(1)
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
                settings: Settings {
                    disable: vec!["us-spelling".into(), "em-dash".into()],
                    only: vec!["word-choice".into()],
                    max_findings: 3,
                    ..Settings::default()
                },
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
        assert_eq!(read.settings.disable, ["em-dash", "latin"]);
        assert_eq!(read.settings.only, ["us-spelling"]);
        assert_eq!(read.settings.max_findings, 4);
        let read = options(&args(&["--config", config, "--max-findings", "0", "a.md"])).unwrap();
        assert_eq!(read.settings.max_findings, 0);
        let missing = dir.join("missing");
        let error = options(&args(&["--config", missing.to_str().unwrap()])).unwrap_err();
        assert!(error.starts_with(missing.to_str().unwrap()), "{error}");
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
