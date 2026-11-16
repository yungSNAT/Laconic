// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use laconic_check::mcp::parse;

use super::*;

/// Writes `bytes` into `slot` as the page does.
fn write(slot: usize, bytes: &[u8]) {
    let space = input(slot, bytes.len());
    assert!(!space.is_null());
    // SAFETY: `input` returned space for exactly `bytes.len()` bytes, as the page uses it.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), space, bytes.len()) };
}

/// The report `len` bytes long that the last call left at `output`.
fn read(len: usize) -> Json {
    // SAFETY: `output` holds the last report, `len` bytes of UTF-8 JSON.
    let bytes = unsafe { std::slice::from_raw_parts(output(), len) };
    parse(std::str::from_utf8(bytes).unwrap()).unwrap()
}

#[test]
fn lint_reports_what_laconic_check_lint_finds() {
    let text = "We analyzed the color \u{2014} it\u{2019}s three days.\n";
    write(0, text.as_bytes());
    let report = read(lint());
    let wanted: Vec<Json> = laconic_check::lint::lint(text)
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
    assert_eq!(wanted.len(), 4);
    assert_eq!(
        report,
        Json::Object(vec![("findings".into(), Json::Array(wanted))])
    );
}

#[test]
fn preserve_reports_a_dropped_and_an_added_item() {
    write(0, b"Revenue rose 5% to $47.3 million.");
    write(1, b"Revenue rose to $47.3 billion.");
    let Json::Object(report) = read(preserve(false)) else {
        panic!("not an object");
    };
    let get = |key: &str| &report.iter().find(|(k, _)| k == key).unwrap().1;
    assert_eq!(get("dropped"), &Json::Bool(true));
    assert_eq!(get("added"), &Json::Bool(true));
    let lines = laconic_check::preserve::report(
        "Revenue rose 5% to $47.3 million.",
        "Revenue rose to $47.3 billion.",
        false,
    )
    .lines;
    assert_eq!(
        get("lines"),
        &Json::Array(lines.into_iter().map(Json::String).collect())
    );
}

#[test]
fn text_that_is_not_utf8_is_an_error_and_a_third_slot_does_not_exist() {
    write(0, &[0xff, 0xfe]);
    write(1, b"");
    assert_eq!(
        read(lint()),
        Json::Object(vec![(
            "error".into(),
            Json::String("the text is not UTF-8".into())
        )])
    );
    assert!(input(2, 4).is_null());
}
