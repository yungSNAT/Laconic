// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! `lint` on any Markdown text.

use laconic_check::lint::lint;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let lines = text.split('\n').count();
    for f in lint(text) {
        // Every finding points at a place in the text, counted from 1.
        assert!(
            (1..=lines).contains(&f.line) && f.column >= 1,
            "{}:{}: {}",
            f.line,
            f.column,
            f.rule
        );
    }
});
