// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! `lsp` JSON-RPC framing and request dispatch on arbitrary input.

use laconic_check::lsp::serve;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serve(std::io::Cursor::new(data), std::io::sink());
});
