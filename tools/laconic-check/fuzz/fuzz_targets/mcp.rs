// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! The MCP server on any request line, and the JSON reader and writer on any text.

use laconic_check::mcp::{handle_line, parse, to_string};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|line: &str| {
    // Every reply is itself valid JSON.
    if let Some(reply) = handle_line(line) {
        assert!(parse(&reply).is_some(), "{reply}");
    }
    // Any JSON read writes back to text that reads the same.
    if let Some(value) = parse(line) {
        assert_eq!(parse(&to_string(&value)), Some(value));
    }
});
