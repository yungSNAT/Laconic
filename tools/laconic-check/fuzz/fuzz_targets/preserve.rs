// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! `preserve` on any original and edit.

use laconic_check::preserve::{added, missing, names};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (&str, &str)| {
    let (original, edited) = input;
    let _ = (
        missing(original, edited),
        added(original, edited),
        names(original, edited),
    );
    // An unchanged text drops and adds nothing.
    assert!(missing(original, original).is_empty(), "{original:?}");
    assert!(added(original, original).is_empty(), "{original:?}");
});
