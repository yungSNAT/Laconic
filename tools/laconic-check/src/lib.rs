// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! laconic-check's rules as a library, so the fuzz targets in `fuzz/` can call them. The
//! `laconic-check` binary is the supported interface.

pub mod hook;
pub mod lint;
pub mod lsp;
pub mod mcp;
pub mod preserve;
pub mod sarif;
#[cfg(test)]
mod seeded_faults;
pub mod settings;
