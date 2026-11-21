// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The command line: an argument that is not valid Unicode is an error, not a panic.

use std::process::Command;

/// An argument that is not valid Unicode: 'caf' and '.md' around a lone Latin-1 byte on Unix,
/// or around an unpaired surrogate on Windows.
#[cfg(unix)]
fn not_unicode() -> std::ffi::OsString {
    std::os::unix::ffi::OsStringExt::from_vec(b"caf\xe9.md".to_vec())
}

#[cfg(windows)]
fn not_unicode() -> std::ffi::OsString {
    std::os::windows::ffi::OsStringExt::from_wide(&[0x63, 0x61, 0x66, 0xD800, 0x2E, 0x6D, 0x64])
}

#[test]
fn an_argument_that_is_not_unicode_is_an_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-tools"))
        .arg("calibrate")
        .arg(not_unicode())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "laconic-tools: an argument is not valid Unicode: caf\u{fffd}.md\n"
    );
}
