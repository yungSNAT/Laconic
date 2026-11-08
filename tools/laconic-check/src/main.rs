// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! laconic-check: deterministic candidates for Laconic's house style.
//!
//! `lint` reports candidates in the original prose of Markdown files; a match is a
//! reason to look, not proof of a defect. `preserve` compares an original and an
//! edit and reports numbers, quantities, months, code spans, URLs, quotations, meaning words and
//! bounds the edit
//! dropped, and, as advice, those it added. `mcp` serves both as MCP tools on standard
//! input and output.

use std::env;
use std::fs;
use std::process::ExitCode;

use laconic_check::{lint, mcp, preserve};

fn usage() -> ExitCode {
    let advisory: Vec<&str> = lint::RULES
        .iter()
        .filter(|r| r.advisory)
        .map(|r| r.name)
        .collect();
    eprintln!(
        "usage:\n  laconic-check lint [--warn] <file.md | ->...\n  laconic-check preserve [--strict] <original> <edited>\n  laconic-check mcp\n\n\
         lint exits 1 when it finds candidates, unless --warn is given; advisory\n\
         findings ({}) never fail it. Each finding names its rule,\n\
         described under that name in {}\n\
         preserve exits 1 when the edit dropped a protected item. It also lists items the\n\
         edit added, which fail only with --strict (for a rewrite that must add nothing),\n\
         and names to check, which never fail. A clean result does not check meaning.\n\
         mcp serves lint and preserve as MCP tools over standard input and output.",
        advisory.join(", "),
        lint::RULES_URL
    );
    ExitCode::from(2)
}

/// Reads a file, or standard input when the path is `-`.
fn read(path: &str) -> Result<String, ExitCode> {
    let result = if path == "-" {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text).map(|_| text)
    } else {
        fs::read_to_string(path)
    };
    result.map_err(|e| {
        eprintln!("{path}: {e}");
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("lint") => {
            let warn = args.iter().any(|a| a == "--warn");
            let files: Vec<&String> = args[1..].iter().filter(|a| *a != "--warn").collect();
            if files.is_empty() {
                return usage();
            }
            let mut total = 0;
            for file in files {
                let text = match read(file) {
                    Ok(t) => t,
                    Err(code) => return code,
                };
                for f in lint::lint(&text) {
                    println!("{file}:{}:{}: {}: {}", f.line, f.column, f.rule, f.message);
                    if !lint::is_advisory(f.rule) {
                        total += 1;
                    }
                }
            }
            if total > 0 && !warn {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Some("preserve") => {
            // `--strict` may come anywhere before `--`; after it, every argument is a file.
            let (mut strict, mut options, mut files) = (false, true, Vec::new());
            for arg in &args[1..] {
                match arg.as_str() {
                    "--strict" if options => strict = true,
                    "--" if options => options = false,
                    _ => files.push(arg),
                }
            }
            // Standard input can be read once, so only one of the 2 can be `-`.
            let [original, edited] = files[..] else {
                return usage();
            };
            if original == "-" && edited == "-" {
                return usage();
            }
            let (original, edited) = match (read(original), read(edited)) {
                (Ok(o), Ok(e)) => (o, e),
                (Err(code), _) | (_, Err(code)) => return code,
            };
            let report = preserve::report(&original, &edited);
            for line in &report.lines {
                println!("{line}");
            }
            if report.dropped || (strict && report.added) {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Some("mcp") if args.len() == 1 => serve(),
        _ => usage(),
    }
}

/// Answers MCP requests, one JSON-RPC message a line, until standard input closes. A line
/// that is not UTF-8 is read with replacement characters and answered like any other.
fn serve() -> ExitCode {
    match mcp::serve(std::io::stdin().lock(), std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mcp: {e}");
            ExitCode::from(2)
        }
    }
}
