// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `lint --sarif`: findings as a SARIF 2.1.0 log with the properties GitHub code scanning
//! reads.

use std::path::Path;

use crate::lint::{self, Finding, RULES, RULES_URL};
use crate::mcp::{Json, object, text};

/// A rule's level: advisory findings are notes and the rest warnings, since a finding is a
/// reason to look, not proof of a defect; `lint`'s exit status is the gate.
fn level(rule: &lint::Rule) -> Json {
    text(if rule.advisory { "note" } else { "warning" })
}

fn message(s: &str) -> Json {
    object(vec![("text", text(s))])
}

/// One finding in `file`, whose text on the finding's line is `line`. SARIF counts columns
/// in UTF-16 code units by default; `lint` counts characters.
pub fn result(file: &str, finding: Finding, line: &str) -> Json {
    let rule = finding.rule.rule();
    let column: usize = line
        .chars()
        .map(char::len_utf16)
        .chain(std::iter::repeat(1))
        .take(finding.column.saturating_sub(1))
        .sum::<usize>()
        + 1;
    let number = |n: usize| Json::Number(n.to_string());
    object(vec![
        ("ruleId", text(rule.name)),
        ("ruleIndex", number(finding.rule as usize)),
        ("level", level(rule)),
        ("message", message(&finding.message)),
        (
            "locations",
            Json::Array(vec![object(vec![(
                "physicalLocation",
                object(vec![
                    (
                        "artifactLocation",
                        object(vec![("uri", text(&artifact_uri(file)))]),
                    ),
                    (
                        "region",
                        object(vec![
                            ("startLine", number(finding.line)),
                            ("startColumn", number(column)),
                        ]),
                    ),
                ]),
            )])]),
        ),
    ])
}

/// The log of one `lint` run: every rule, then `results`. `complete` is false when a file
/// could not be read.
pub fn log(results: Vec<Json>, complete: bool) -> Json {
    let rules = RULES
        .iter()
        .map(|rule| {
            let url = format!("{RULES_URL}#{}", rule.name);
            object(vec![
                ("id", text(rule.name)),
                ("shortDescription", message(rule.checks)),
                ("fullDescription", message(rule.checks)),
                ("help", message(&format!("{} See {url}", rule.checks))),
                ("helpUri", text(&url)),
                ("defaultConfiguration", object(vec![("level", level(rule))])),
            ])
        })
        .collect();
    object(vec![
        (
            "$schema",
            text("https://json.schemastore.org/sarif-2.1.0.json"),
        ),
        ("version", text("2.1.0")),
        (
            "runs",
            Json::Array(vec![object(vec![
                (
                    "tool",
                    object(vec![(
                        "driver",
                        object(vec![
                            ("name", text("laconic-check")),
                            ("version", text(env!("CARGO_PKG_VERSION"))),
                            (
                                "informationUri",
                                text("https://github.com/yungSNAT/Laconic"),
                            ),
                            ("rules", Json::Array(rules)),
                        ]),
                    )]),
                ),
                (
                    "invocations",
                    Json::Array(vec![object(vec![(
                        "executionSuccessful",
                        Json::Bool(complete),
                    )])]),
                ),
                ("results", Json::Array(results)),
            ])]),
        ),
    ])
}

/// A path as a URI reference: a relative path stays relative, which code scanning reads
/// from the repository root, and an absolute one becomes a `file` URI. Characters outside
/// RFC 3986's unreserved set and `/` are percent-encoded.
fn artifact_uri(path: &str) -> String {
    let slashed = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
    let (prefix, rest) = if !Path::new(path).is_absolute() {
        ("", slashed.as_str())
    } else if !cfg!(windows) {
        ("file://", slashed.as_str())
    } else if slashed.starts_with("//") {
        // A UNC path, `\\server\share\...`.
        ("file:", slashed.as_str())
    } else {
        // `C:/...`: the drive's colon stays as it is.
        return format!("file:///{}{}", &slashed[..2], encode(&slashed[2..]));
    };
    format!("{prefix}{}", encode(rest))
}

fn encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_becomes_a_uri_reference() {
        assert_eq!(artifact_uri("docs/a b#1%.md"), "docs/a%20b%231%25.md");
        assert_eq!(artifact_uri("a:b.md"), "a%3Ab.md");
        assert_eq!(artifact_uri("caf\u{e9}.md"), "caf%C3%A9.md");
        assert_eq!(artifact_uri("-"), "-");
        if cfg!(windows) {
            assert_eq!(artifact_uri(r"docs\a.md"), "docs/a.md");
            assert_eq!(artifact_uri(r"C:\work\a b.md"), "file:///C:/work/a%20b.md");
            assert_eq!(artifact_uri(r"\\host\share\a.md"), "file://host/share/a.md");
        } else {
            assert_eq!(artifact_uri(r"docs\a.md"), "docs%5Ca.md");
            assert_eq!(artifact_uri("/work/a b.md"), "file:///work/a%20b.md");
        }
    }
}
