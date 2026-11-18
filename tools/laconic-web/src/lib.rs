// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! laconic-check's `lint` and `preserve` for the browser page in `web/`, compiled to
//! WebAssembly with no dependencies and no generated bindings. The page writes each text as
//! UTF-8 into the space `input` returns, calls `lint` or `preserve`, and reads that many bytes
//! of JSON report from `output`.

use std::cell::RefCell;

use laconic_check::mcp::{Json, to_string};

thread_local! {
    /// The texts the page writes: slot 0 holds the text to lint or the original, slot 1 the
    /// edit.
    static INPUTS: RefCell<[Vec<u8>; 2]> = RefCell::default();
    /// The last report, which the page reads before its next call.
    static OUTPUT: RefCell<String> = RefCell::default();
}

/// Space for `len` bytes of text in `slot`, for the page to fill before its next call; null
/// for a slot that does not exist or space the memory cannot hold, which leaves the slot
/// empty.
#[unsafe(no_mangle)]
pub extern "C" fn input(slot: usize, len: usize) -> *mut u8 {
    INPUTS.with_borrow_mut(|inputs| match inputs.get_mut(slot) {
        Some(text) => {
            *text = Vec::new();
            if text.try_reserve_exact(len).is_err() {
                return std::ptr::null_mut();
            }
            text.resize(len, 0);
            text.as_mut_ptr()
        }
        None => std::ptr::null_mut(),
    })
}

/// Lints slot 0. Returns the report's length in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn lint() -> usize {
    respond(1, |texts| lint_report(texts[0]))
}

/// Checks the edit in slot 1 against the original in slot 0, as `preserve`, with `--loose`
/// when `loose` is true. Returns the report's length in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn preserve(loose: bool) -> usize {
    respond(2, |texts| preserve_report(texts[0], texts[1], loose))
}

/// The last report.
#[unsafe(no_mangle)]
pub extern "C" fn output() -> *const u8 {
    OUTPUT.with_borrow(|report| report.as_ptr())
}

/// Runs `check` on the texts of the first `slots` slots and keeps its report, or an error
/// when one of them is not UTF-8.
fn respond(slots: usize, check: impl FnOnce(&[&str]) -> Json) -> usize {
    let report = INPUTS.with_borrow(|inputs| {
        let mut texts = [""; 2];
        for (text, bytes) in texts.iter_mut().zip(&inputs[..slots]) {
            match std::str::from_utf8(bytes) {
                Ok(valid) => *text = valid,
                Err(_) => {
                    return Json::Object(vec![(
                        "error".into(),
                        Json::String("the text is not UTF-8".into()),
                    )]);
                }
            }
        }
        check(&texts[..slots])
    });
    OUTPUT.with_borrow_mut(|out| {
        *out = to_string(&report);
        out.len()
    })
}

/// `lint`'s findings, as `{"findings": [{"line", "column", "rule", "message"}]}`. An advisory
/// rule's message says so.
fn lint_report(text: &str) -> Json {
    let findings = laconic_check::lint::lint(text)
        .into_iter()
        .map(|f| {
            Json::Object(vec![
                ("line".into(), Json::Number(f.line.to_string())),
                ("column".into(), Json::Number(f.column.to_string())),
                ("rule".into(), Json::String(f.rule.to_string())),
                ("message".into(), Json::String(f.message)),
            ])
        })
        .collect();
    Json::Object(vec![("findings".into(), Json::Array(findings))])
}

/// `preserve`'s report, as `{"lines": [...], "dropped", "added"}`: the CLI's lines, whether the
/// edit dropped a protected item, which fails, and whether it added one, which fails only
/// under `--strict`.
fn preserve_report(original: &str, edited: &str, loose: bool) -> Json {
    let report = laconic_check::preserve::report(original, edited, loose);
    Json::Object(vec![
        (
            "lines".into(),
            Json::Array(report.lines.into_iter().map(Json::String).collect()),
        ),
        ("dropped".into(), Json::Bool(report.dropped)),
        ("added".into(), Json::Bool(report.added)),
    ])
}

#[cfg(test)]
mod tests;
