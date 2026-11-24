// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `laconic-check action-lint`: what the composite Action's `lint.sh` does. It runs `lint`,
//! with the binary in `LACONIC_CHECK`, on the files named in `LACONIC_FILES`, one path a line,
//! or on every Markdown file Git tracks, and prints each finding as a workflow annotation: an
//! error, or a warning for an advisory finding. `LACONIC_ARGS` holds further `lint` options.
//! The exit status is lint's highest across the calls. The Action is the only caller, so the
//! command is not in `--help` or the completions.

use std::collections::HashSet;
use std::env;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use crate::settings::{self, Settings};

/// Windows caps a command line at 32,767 characters, so `lint` runs on batches of files whose
/// paths take at most this many UTF-16 units, counting 3 for each path's separator and quotes.
/// The rest holds the binary's path and the options.
const BATCH_LIMIT: usize = 24_000;

const CANNOT_LIST: &str = "::error::cannot list the Markdown files Git tracks; check out the repository first, or name files in the files input";

/// The Action's lint step, which passes its inputs in `LACONIC_FILES` and `LACONIC_ARGS`, and
/// the binary the install step verified in `LACONIC_CHECK`.
pub fn from_env() -> ExitCode {
    let var = |name| {
        env::var_os(name)
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };
    let exe = var("LACONIC_CHECK");
    if exe.is_empty() {
        println!("::error::LACONIC_CHECK names no binary to lint with");
        return ExitCode::from(2);
    }
    ExitCode::from(run(
        &var("LACONIC_FILES"),
        &var("LACONIC_ARGS"),
        Path::new(&exe),
        &mut io::stdout().lock(),
    ))
}

/// Runs the Action's lint step for `files` (`LACONIC_FILES`) and `args` (`LACONIC_ARGS`),
/// writing its output to `out`, and returns the exit status.
pub fn run(files: &str, args: &str, exe: &Path, out: &mut impl Write) -> u8 {
    let mut files = match listed(files) {
        Ok(files) => files,
        Err(message) => return fail(out, &message),
    };
    // The Action splits args at spaces, tabs and new lines.
    let args: Vec<&str> = args
        .split([' ', '\t', '\n'])
        .filter(|arg| !arg.is_empty())
        .collect();
    for arg in &args {
        if matches!(*arg, "--json" | "--sarif") {
            let message = format!(
                "::error::the Action reads lint's text output, so args cannot include {arg}"
            );
            return fail(out, &message);
        }
    }
    let warn = args.contains(&"--warn");
    if files.is_empty() {
        let _ = writeln!(out, "No Markdown files to lint.");
        return 0;
    }
    for file in &mut files {
        if file.contains(['\n', '\r']) {
            let message = "::error::the Action cannot annotate a filename containing a line break";
            return fail(out, message);
        }
        if file.starts_with('-') {
            file.insert_str(0, "./");
        }
    }
    let mut totals = Totals::default();
    let (mut start, mut size) = (0, 0);
    for (index, file) in files.iter().enumerate() {
        let units = file.encode_utf16().count() + 3;
        if index > start && size + units > BATCH_LIMIT {
            if let Err(message) = totals.call(exe, &args, &files[start..index], out) {
                return fail(out, &format!("::error::{message}"));
            }
            (start, size) = (index, 0);
        }
        size += units;
    }
    if let Err(message) = totals.call(exe, &args, &files[start..], out) {
        return fail(out, &format!("::error::{message}"));
    }
    let Totals {
        mut status,
        calls,
        failing,
    } = totals;
    // --max-findings applies to each call, so when every call passed without --warn, the
    // failing findings of all the calls are judged against it here.
    if !warn && calls > 1 && status == 0 && failing > 0 {
        let max = max_findings(&args);
        if failing > max {
            let _ = writeln!(
                out,
                "::error::{failing} failing findings across all the files, more than --max-findings allows ({max})"
            );
            status = 1;
        }
    }
    status
}

/// What the `lint` calls came to: the highest exit status wins, 2 (an error) over 1 (too many
/// findings) over 0. A call that cannot run ends the step with 2, the highest status.
#[derive(Default)]
struct Totals {
    status: u8,
    calls: usize,
    failing: usize,
}

impl Totals {
    fn call(
        &mut self,
        exe: &Path,
        args: &[&str],
        batch: &[String],
        out: &mut impl Write,
    ) -> Result<(), String> {
        let (code, text) = lint(exe, args, batch)?;
        self.failing += annotate(&text, out);
        self.status = self.status.max(code);
        self.calls += 1;
        Ok(())
    }
}

fn fail(out: &mut impl Write, message: &str) -> u8 {
    let _ = writeln!(out, "{message}");
    2
}

/// The files to lint: those `list` names, one a line and each once, or else every Markdown file
/// Git tracks.
fn listed(list: &str) -> Result<Vec<String>, String> {
    if list.is_empty() {
        let output = Command::new("git")
            .args(["ls-files", "-z", "--", "*.md", "*.markdown"])
            .stderr(Stdio::inherit())
            .output();
        return match output {
            Ok(output) if output.status.success() => Ok(String::from_utf8_lossy(&output.stdout)
                .split('\0')
                .filter(|file| !file.is_empty())
                .map(str::to_owned)
                .collect()),
            _ => Err(CANNOT_LIST.into()),
        };
    }
    let mut seen = HashSet::new();
    Ok(list
        .split('\n')
        .filter(|file| !file.is_empty() && seen.insert(*file))
        .map(str::to_owned)
        .collect())
}

/// Runs `lint` on `files`, returning its exit status and its standard output and error in the
/// order it wrote them.
fn lint(exe: &Path, args: &[&str], files: &[String]) -> Result<(u8, String), String> {
    let (mut reader, writer) = io::pipe().map_err(|e| format!("cannot read lint's output: {e}"))?;
    let mut command = Command::new(exe);
    command.arg("lint").args(args).args(files);
    command.stdout(writer.try_clone().map_err(|e| e.to_string())?);
    command.stderr(writer);
    let spawned = command.spawn();
    // The command holds the pipe's write ends; the reader sees the end once it is dropped.
    drop(command);
    let mut child = spawned.map_err(|e| format!("cannot run {}: {e}", exe.display()))?;
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read lint's output: {e}"))?;
    let status = child.wait().map_err(|e| e.to_string())?;
    Ok((
        exit_code(status.code(), signal(&status)),
        String::from_utf8_lossy(&bytes).into_owned(),
    ))
}

/// The signal that ended a process, where the system says.
#[cfg(unix)]
fn signal(status: &std::process::ExitStatus) -> Option<i32> {
    std::os::unix::process::ExitStatusExt::signal(status)
}

#[cfg(not(unix))]
fn signal(_: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// `lint`'s exit status, as a shell reports it: 128 plus the signal that ended it. Any other
/// status outside 0 to 255, such as a Windows crash code, counts as an error.
fn exit_code(code: Option<i32>, signal: Option<i32>) -> u8 {
    match (code, signal) {
        (Some(code), _) => u8::try_from(code).unwrap_or(2),
        (None, Some(signal)) => u8::try_from(128 + signal).unwrap_or(2),
        (None, None) => 2,
    }
}

/// Prints each line of lint's output, and an annotation for each finding in it. Returns the
/// number of failing findings.
fn annotate(text: &str, out: &mut impl Write) -> usize {
    let mut failing = 0;
    for line in text.split('\n').filter(|line| !line.is_empty()) {
        // Filename and message text must not be interpreted as workflow commands.
        let _ = writeln!(out, "laconic-check: {line}");
        let Some(finding) = Finding::parse(line) else {
            continue;
        };
        let level = if finding.message.contains("(advisory)") {
            "warning"
        } else {
            failing += 1;
            "error"
        };
        let _ = writeln!(
            out,
            "::{level} file={},line={},col={},title={}::{}",
            property(finding.file),
            finding.line,
            finding.column,
            property(&format!("laconic-check {}", finding.rule)),
            message(finding.message)
        );
    }
    failing
}

/// A finding line, `file:line:column: rule: message`.
struct Finding<'a> {
    file: &'a str,
    line: &'a str,
    column: &'a str,
    rule: &'a str,
    message: &'a str,
}

impl<'a> Finding<'a> {
    /// The file name is as long as it can be, so a message holding `:1:2: rule:` stays in the
    /// message; a rule is lower-case letters and hyphens, so a project rule, `project/<id>`,
    /// is no finding here.
    fn parse(line: &'a str) -> Option<Finding<'a>> {
        line.match_indices(':').rev().find_map(|(at, _)| {
            if at == 0 {
                return None;
            }
            let (number, rest) = digits(&line[at + 1..])?;
            let (column, rest) = digits(rest.strip_prefix(':')?)?;
            let rest = rest.strip_prefix(": ")?;
            let end = rest.find(|c: char| !(c.is_ascii_lowercase() || c == '-'))?;
            let message = rest[end..].strip_prefix(": ")?;
            (end > 0).then_some(Finding {
                file: &line[..at],
                line: number,
                column,
                rule: &rest[..end],
                message,
            })
        })
    }
}

/// A leading run of ASCII digits, and the rest.
fn digits(text: &str) -> Option<(&str, &str)> {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    (end > 0).then(|| text.split_at(end))
}

/// Workflow commands need '%', CR and LF escaped in messages.
fn message(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// And ',' and ':' too in properties.
fn property(text: &str) -> String {
    message(text).replace(',', "%2C").replace(':', "%3A")
}

/// The `--max-findings` that `lint` applies: the last one before any `--` in `args`, else the
/// last in the project file (`--config`'s, or `.laconic-check` in this directory), else 0.
/// The calls already rejected a bad option, value or project file, so these are well formed.
fn max_findings(args: &[&str]) -> usize {
    let (mut max, mut config) = (None, None);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match *arg {
            "--" => break,
            "--max-findings" => max = rest.next().copied(),
            "--config" => config = rest.next().copied(),
            "--disable" | "--only" | "--profile" => {
                rest.next();
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--max-findings=") {
                    max = Some(value);
                } else if let Some(value) = arg.strip_prefix("--config=") {
                    config = Some(value);
                }
            }
        }
    }
    // A count too large to hold is beyond any count of findings.
    if let Some(value) = max.filter(|value| !value.is_empty()) {
        return settings::count(value).unwrap_or(usize::MAX);
    }
    let path = config
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .or_else(|| settings::find_in(Path::new(".")).map(|p| p.to_string_lossy().into_owned()));
    path.as_deref()
        .and_then(|path| Settings::read(path).ok())
        .map_or(0, |settings| settings.max_findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_finding_with_the_longest_file_name() {
        let found = Finding::parse("a:1:2: x: b:3:4: color-rule: say 'colour'").unwrap();
        assert_eq!(
            (
                found.file,
                found.line,
                found.column,
                found.rule,
                found.message
            ),
            ("a:1:2: x: b", "3", "4", "color-rule", "say 'colour'")
        );
    }

    #[test]
    fn a_line_that_is_no_finding_has_no_annotation() {
        for line in [
            "",
            ":1:2: rule: text",
            "a.md:1:2: project/plain: text",
            "a.md:1:2: Rule: text",
            "a.md:x:2: rule: text",
            "a.md:1:2:rule: text",
            "a.md:1:2: : text",
            "a.md: unreadable",
        ] {
            assert!(Finding::parse(line).is_none(), "{line:?}");
        }
    }

    #[test]
    fn escapes_follow_the_workflow_command_rules() {
        assert_eq!(message("100%\r\nx,y:z"), "100%25%0D%0Ax,y:z");
        assert_eq!(property("100%\r\nx,y:z"), "100%25%0D%0Ax%2Cy%3Az");
    }

    #[test]
    fn the_last_max_findings_before_a_double_dash_counts() {
        let args = [
            "--max-findings",
            "9",
            "--max-findings",
            "+02",
            "--",
            "--max-findings",
            "1",
        ];
        assert_eq!(max_findings(&args), 2);
        assert_eq!(max_findings(&["--max-findings=3", "--max-findings=007"]), 7);
        // An option's value is not an option, as in lint.
        assert_eq!(
            max_findings(&["--only", "--max-findings", "--max-findings", "4"]),
            4
        );
        assert_eq!(
            max_findings(&["--max-findings=99999999999999999999999"]),
            usize::MAX
        );
    }

    #[test]
    fn a_status_outside_0_to_255_is_an_error() {
        let given = [
            Some(0),
            Some(1),
            Some(2),
            Some(-1_073_741_571),
            Some(256),
            None,
        ];
        assert_eq!(given.map(|code| exit_code(code, None)), [0, 1, 2, 2, 2, 2]);
        assert_eq!(
            [exit_code(None, Some(15)), exit_code(None, Some(9))],
            [143, 137]
        );
    }

    #[test]
    fn a_list_names_each_file_once() {
        assert_eq!(listed("a.md\n\nb.md\na.md\n").unwrap(), ["a.md", "b.md"]);
    }
}
