// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

#![no_main]
//! `hook` porcelain-z status parsing, continue-guard handling and decision formatting.

use laconic_check::hook::{decision, markdown, stop};
use laconic_check::mcp::{Json, parse};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = markdown(data);
    if let Ok(text) = std::str::from_utf8(data) {
        if let Some(Json::Object(fields)) = parse(text)
            && fields
                .iter()
                .any(|(k, v)| k == "stop_hook_active" && *v == Json::Bool(true))
        {
            let _ = stop(text);
        }
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let _ = decision(&lines);
    }
});
