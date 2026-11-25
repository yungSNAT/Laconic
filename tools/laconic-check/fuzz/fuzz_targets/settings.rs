// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! `laconic.toml` and `.laconic-check` settings parser on arbitrary text.

use laconic_check::settings::Settings;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let _ = Settings::parse("laconic.toml", text);
    let _ = Settings::parse(".laconic-check", text);
});
