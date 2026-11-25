// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! `action-lint` finding parser and workflow annotation formatter on arbitrary text.

use laconic_check::action::annotate;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let _ = annotate(text, &mut std::io::sink());
});
