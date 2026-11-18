// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Evaluation cases and JSON Lines files: reading them, validating the case catalogue and
//! a case's regular-expression checks.

use crate::py;
use fancy_regex::{Regex, RegexBuilder};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The repository root: two levels above this crate.
pub fn root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// A file's text as Python's `Path.read_text(encoding="utf-8")` gives it.
pub fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map(|text| py::universal_newlines(&text))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes `text` to `path`, replacing the file; an error names the file.
pub fn write(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Appends `text` to `path`, creating the file; an error names the file.
pub fn append(path: &Path, text: &str) -> Result<(), String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// One JSON document, with the parser's message on failure.
pub fn parse(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// The objects of a JSON Lines file, skipping blank lines; a line that is not a JSON object
/// is an error naming its number.
pub fn read_jsonl(path: &Path) -> Result<Vec<Value>, String> {
    let text = read_text(path)?;
    let mut rows = Vec::new();
    for (number, line) in py::split_lines(&text).into_iter().enumerate() {
        if py::strip(line).is_empty() {
            continue;
        }
        let row =
            parse(line).map_err(|e| format!("{}: line {}: {e}", path.display(), number + 1))?;
        if !row.is_object() {
            return Err(format!(
                "{}: line {}: expected a JSON object",
                path.display(),
                number + 1
            ));
        }
        rows.push(row);
    }
    Ok(rows)
}

/// The evaluation cases in `path`.
pub fn load_cases(path: &Path) -> Result<Vec<Value>, String> {
    read_jsonl(path)
}

/// The catalogue's problems, in the order the Python reported them.
pub fn validate_cases(cases: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    let required = ["category", "criteria", "id", "prompt", "risk"];
    for (index, case) in cases.iter().enumerate() {
        let index = index + 1;
        let missing: Vec<&str> = required
            .into_iter()
            .filter(|field| py::get(case, field).is_none())
            .collect();
        if !missing.is_empty() {
            errors.push(format!(
                "Case {index}: missing fields: {}",
                missing.join(", ")
            ));
            continue;
        }
        let id = &case["id"];
        let shown = py::to_str(id);
        match id.as_str() {
            Some(text) if !text.is_empty() => {
                if !seen.insert(text.to_owned()) {
                    errors.push(format!("Duplicate case id: {text}"));
                }
            }
            _ => errors.push(format!("Case {index}: id must be a non-empty string")),
        }
        if !matches!(case["risk"].as_str(), Some("low" | "medium" | "high")) {
            errors.push(format!("Case {shown}: risk must be low, medium, or high"));
        }
        if !case["criteria"].as_array().is_some_and(|c| !c.is_empty()) {
            errors.push(format!("Case {shown}: criteria must be a non-empty list"));
        }
        if let Some(target) = py::get(case, "target_words")
            && !target.as_i64().is_some_and(|t| t >= 1)
            && !target.as_u64().is_some_and(|t| t >= 1)
        {
            errors.push(format!(
                "Case {shown}: target_words must be a positive integer"
            ));
        }
        if let Some(checks) = py::get(case, "checks") {
            errors.extend(
                validate_checks(checks)
                    .into_iter()
                    .map(|error| format!("Case {shown}: {error}")),
            );
        }
    }
    errors
}

/// A pattern compiled as `re.compile(pattern, re.I)`.
pub fn compile(pattern: &str) -> Result<Regex, String> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map_err(|e| e.to_string())
}

/// The strings of a checks field, or None when it is not a list of strings. A missing field
/// is an empty list.
fn strings<'a>(checks: &'a Value, field: &str) -> Option<Vec<&'a str>> {
    match py::get(checks, field) {
        None => Some(Vec::new()),
        Some(Value::Array(items)) => items.iter().map(Value::as_str).collect(),
        Some(_) => None,
    }
}

/// A case's optional regex checks must compile, and must accept every known-good example and
/// reject every known-bad one, so a broken check fails before a paid run.
pub fn validate_checks(checks: &Value) -> Vec<String> {
    if !checks.is_object() {
        return vec!["checks must be an object".into()];
    }
    let fields = ["must_include", "must_not_include", "pass", "fail"];
    let mut errors: Vec<String> = fields
        .into_iter()
        .filter(|field| strings(checks, field).is_none())
        .map(|field| format!("{field} must be a list of strings"))
        .collect();
    if !errors.is_empty() {
        return errors;
    }
    let list = |field| strings(checks, field).expect("checked above");
    let patterns: Vec<&str> = [list("must_include"), list("must_not_include")].concat();
    if patterns.is_empty() {
        errors.push("checks need at least one must_include or must_not_include pattern".into());
    }
    for pattern in &patterns {
        if let Err(error) = compile(pattern) {
            errors.push(format!(
                "bad check pattern {}: {error}",
                py::str_repr(pattern)
            ));
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    let (pass, fail) = (list("pass"), list("fail"));
    if pass.is_empty() || fail.is_empty() {
        errors.push("checks need at least one pass and one fail example".into());
    }
    for example in pass {
        let failed = check_response(checks, example);
        if !failed.is_empty() {
            let shown: Vec<String> = failed.iter().map(|f| py::str_repr(f)).collect();
            errors.push(format!(
                "pass example {} fails [{}]",
                py::str_repr(example),
                shown.join(", ")
            ));
        }
    }
    for example in fail {
        if check_response(checks, example).is_empty() {
            errors.push(format!(
                "fail example {} passes every check",
                py::str_repr(example)
            ));
        }
    }
    errors
}

/// Whether `pattern` matches anywhere in `text`, case-insensitively. Patterns are validated
/// before use; one that cannot compile or run never matches.
fn found(pattern: &str, text: &str) -> bool {
    compile(pattern).is_ok_and(|re| re.is_match(text).unwrap_or(false))
}

/// The checks a response fails (case-insensitive regex search).
pub fn check_response(checks: &Value, text: &str) -> Vec<String> {
    let list = |field| strings(checks, field).unwrap_or_default();
    let mut failed: Vec<String> = list("must_include")
        .into_iter()
        .filter(|p| !found(p, text))
        .map(|p| format!("missing {}", py::str_repr(p)))
        .collect();
    failed.extend(
        list("must_not_include")
            .into_iter()
            .filter(|p| found(p, text))
            .map(|p| format!("contains {}", py::str_repr(p))),
    );
    failed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_file(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rows.jsonl");
        fs::write(&path, text).unwrap();
        (dir, path)
    }

    #[test]
    fn jsonl_ignores_blank_lines_but_rejects_non_objects() {
        let (_dir, path) = temp_file("\n{\"case_id\": \"probe\"}\n");
        assert_eq!(read_jsonl(&path).unwrap(), [json!({"case_id": "probe"})]);
        let (_dir, path) = temp_file("\n[]\n");
        assert!(
            read_jsonl(&path)
                .unwrap_err()
                .ends_with("line 2: expected a JSON object")
        );
    }

    #[test]
    fn jsonl_loader_reports_invalid_rows() {
        let (_dir, path) = temp_file("{\"id\": \"ok\"}\r\nnot-json\n");
        assert!(read_jsonl(&path).unwrap_err().contains("line 2: "));
        assert!(read_jsonl(&path.with_extension("missing")).is_err());
    }

    #[test]
    fn append_adds_to_the_file_and_names_it_on_failure() {
        let (dir, path) = temp_file("a\n");
        append(&path, "b\n").unwrap();
        append(&dir.path().join("new.jsonl"), "c\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "a\nb\n");
        assert_eq!(
            fs::read_to_string(dir.path().join("new.jsonl")).unwrap(),
            "c\n"
        );
        let error = append(dir.path(), "row\n").unwrap_err();
        assert!(error.starts_with(&format!("{}: ", dir.path().display())));
        write(&path, "c\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "c\n");
        let error = write(dir.path(), "row\n").unwrap_err();
        assert!(error.starts_with(&format!("{}: ", dir.path().display())));
    }

    #[cfg(unix)]
    #[test]
    fn a_full_disk_is_an_error_naming_the_file() {
        let error = append(Path::new("/dev/full"), "row\n").unwrap_err();
        assert!(error.starts_with("/dev/full: "), "{error}");
    }

    #[test]
    fn case_fields_must_match_the_catalogue_schema() {
        let case = json!({"id": "", "category": "direct-answer", "prompt": "Answer.", "risk": "other",
                          "criteria": [], "checks": []});
        let errors = validate_cases(&[case]);
        assert_eq!(errors.len(), 4, "{errors:?}");
        for detail in [
            "id must be",
            "risk must be",
            "criteria must be",
            "checks must be",
        ] {
            assert!(errors.iter().any(|e| e.contains(detail)), "{errors:?}");
        }
        assert_eq!(
            validate_cases(&[json!({})]),
            ["Case 1: missing fields: category, criteria, id, prompt, risk"]
        );
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let case = json!({"id": "duplicate", "category": "direct-answer", "prompt": "What is 2 + 2?",
                          "risk": "low", "criteria": ["Answers 4."]});
        assert_eq!(
            validate_cases(&[case.clone(), case]),
            ["Duplicate case id: duplicate"]
        );
    }

    #[test]
    fn validate_cases_rejects_a_target_that_is_not_a_positive_integer() {
        let base = json!({"id": "draft", "category": "prose-draft", "prompt": "Write.", "risk": "low",
                          "criteria": ["Lands near the target."]});
        let with = |target: Value| {
            let mut case = base.clone();
            case["target_words"] = target;
            validate_cases(&[case])
        };
        assert!(with(json!(400)).is_empty());
        for bad in [json!(0), json!(-5), json!(4.5), json!("400"), json!(true)] {
            assert_eq!(
                with(bad.clone()),
                ["Case draft: target_words must be a positive integer"],
                "{bad}"
            );
        }
    }

    #[test]
    fn regex_checks_must_accept_good_and_reject_bad_examples() {
        let good = json!({"must_include": [r"\b102\b"], "pass": ["102."], "fail": ["112."]});
        assert!(validate_checks(&good).is_empty());
        let mut wrong = good.clone();
        wrong["pass"] = json!(["112."]);
        assert_eq!(
            validate_checks(&wrong),
            [r#"pass example '112.' fails ["missing '\\\\b102\\\\b'"]"#]
        );
        let mut lax = good.clone();
        lax["fail"] = json!(["It is 102."]);
        assert_eq!(
            validate_checks(&lax),
            ["fail example 'It is 102.' passes every check"]
        );
        let bad = validate_checks(&json!({"must_include": ["("], "pass": ["x"], "fail": ["y"]}));
        assert!(bad[0].starts_with("bad check pattern '(': "), "{bad:?}");
        assert_eq!(
            validate_checks(&json!({"pass": ["x"], "fail": ["y"]})),
            ["checks need at least one must_include or must_not_include pattern"]
        );
        assert_eq!(
            validate_checks(&json!({"must_include": ["x"], "pass": ["x"]})),
            ["checks need at least one pass and one fail example"]
        );
    }

    #[test]
    fn malformed_check_fields_are_errors_not_reinterpretations() {
        for checks in [
            json!({"must_include": "102", "pass": ["120"], "fail": ["112"]}),
            json!({"must_include": null, "pass": ["102"], "fail": ["112"]}),
            json!({"must_include": ["102"], "pass": "102", "fail": ["112"]}),
            json!({"must_include": ["102"], "pass": [null], "fail": ["112"]}),
        ] {
            let errors = validate_checks(&checks);
            assert!(
                errors.iter().any(|e| e.contains("list of strings")),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn checks_search_case_insensitively_with_look_behind() {
        let checks = json!({"must_include": [r"(?<![\w.])12(?:%| per cent\b)"], "must_not_include": ["percent"]});
        assert!(check_response(&checks, "Rates rose 12 PER CENT.").is_empty());
        assert_eq!(
            check_response(&checks, "1.12% percent"),
            [
                r"missing '(?<![\\w.])12(?:%| per cent\\b)'",
                "contains 'percent'"
            ]
        );
        assert_eq!(
            check_response(&json!({"must_include": ["("]}), "("),
            ["missing '('"]
        );
    }

    #[test]
    fn root_is_the_repository() {
        assert!(root().join("evals/cases.jsonl").is_file());
    }
}
