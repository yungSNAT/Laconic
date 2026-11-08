// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Maintainer tooling for Laconic. Only maintainers and CI run it, so it stays
//! apart from the shipped `laconic-check` binary.
//!
//!   laconic-tools sync-rules [--check]   copy rules/laconic.md into every
//!                                        always-on snippet in INSTALL.md
//!   laconic-tools check-pi [--runtime pi|omp]
//!                                        smoke-test the Pi or OMP package over RPC
//!   laconic-tools gen-au-words <scowl-pre.txt> <style-manual.txt>
//!                                        regenerate laconic-check's US word list

mod au_words;
mod pi_check;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// An always-on snippet is a fenced Markdown block that starts with this heading.
const OPEN: &str = "```markdown\n## Output style\n\n";
const CLOSE: &str = "\n```";
/// One snippet per agent route; a changed marker must not silently drop a route.
const SNIPPETS: usize = 7;
/// rules/laconic.md is loaded into every session; keep it short.
const MAX_WORDS: usize = 240;
const RUN: &str = "cargo run --manifest-path tools/laconic-tools/Cargo.toml -- sync-rules";

/// The repository root: two levels above this crate.
fn root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// `text` with CRLF and lone CR line endings as LF, as the Python script read it.
fn lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The compact rules: rules/laconic.md without its title line.
fn rules_body(rules: &str) -> Result<String, String> {
    let (_, body) = rules
        .split_once('\n')
        .ok_or("rules/laconic.md has no rules under its title")?;
    Ok(body.trim().to_owned())
}

/// `install` with every snippet's body replaced by `body`, and the number of snippets.
fn synced(install: &str, body: &str) -> (String, usize) {
    let mut out = String::with_capacity(install.len());
    let mut rest = install;
    let mut count = 0;
    while let Some(start) = rest.find(OPEN) {
        let inner = start + OPEN.len();
        let Some(len) = rest[inner..].find(CLOSE) else {
            break;
        };
        out.push_str(&rest[..inner]);
        out.push_str(body);
        out.push_str(CLOSE);
        // Past the closing fence, so it cannot open the next snippet.
        rest = &rest[inner + len + CLOSE.len()..];
        count += 1;
    }
    out.push_str(rest);
    (out, count)
}

/// Writes the synced INSTALL.md, or with `check` only reports whether it differs.
fn sync_rules(root: &Path, check: bool) -> Result<String, String> {
    let read = |name: &str| {
        fs::read_to_string(root.join(name))
            .map(|text| lf(&text))
            .map_err(|e| format!("cannot read {name}: {e}"))
    };
    let body = rules_body(&read("rules/laconic.md")?)?;
    let words = body.split_whitespace().count();
    if words > MAX_WORDS {
        return Err(format!(
            "rules/laconic.md has {words} words; it is loaded into every session, so keep it to {MAX_WORDS}"
        ));
    }
    let install = read("INSTALL.md")?;
    let (updated, count) = synced(&install, &body);
    if count != SNIPPETS {
        return Err(format!(
            "expected {SNIPPETS} always-on snippets in INSTALL.md, found {count}"
        ));
    }
    if updated == install {
        Ok("INSTALL.md snippets match rules/laconic.md".into())
    } else if check {
        Err(format!(
            "INSTALL.md snippets differ from rules/laconic.md; run {RUN}"
        ))
    } else {
        fs::write(root.join("INSTALL.md"), updated)
            .map_err(|e| format!("cannot write INSTALL.md: {e}"))?;
        Ok("INSTALL.md snippets updated".into())
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["sync-rules"] => sync_rules(&root(), false),
        ["sync-rules", "--check"] => sync_rules(&root(), true),
        ["check-pi"] => pi_check::run(&root(), "pi"),
        ["check-pi", "--runtime", runtime] => pi_check::run(&root(), runtime),
        ["gen-au-words", esdb, manual] => au_words::run(&root(), esdb, manual),
        _ => Err(
            "usage: laconic-tools <sync-rules [--check] | check-pi [--runtime pi|omp] \
             | gen-au-words <scowl-pre.txt> <style-manual.txt>>"
                .into(),
        ),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(body: &str) -> String {
        format!("{OPEN}{body}{CLOSE}")
    }

    #[test]
    fn repository_snippets_match_the_rules() {
        assert_eq!(
            sync_rules(&root(), true),
            Ok("INSTALL.md snippets match rules/laconic.md".into())
        );
    }

    #[test]
    fn rules_body_drops_the_title_and_surrounding_space() {
        assert_eq!(lf("a\r\nb\rc\n"), "a\nb\nc\n");
        assert_eq!(
            rules_body(&lf("# Clear prose\r\n\r\nOne.\r\nTwo.\r\n")),
            Ok("One.\nTwo.".into())
        );
        assert!(rules_body("# Title only").is_err());
    }

    #[test]
    fn a_closing_fence_does_not_open_the_next_snippet() {
        // The Python regex consumed the closing fence, so this holds 1 snippet, not 2.
        let install = format!("{OPEN}old\n{OPEN}old{CLOSE}");
        let (out, count) = synced(&install, "new");
        assert_eq!(count, 1);
        assert_eq!(
            out,
            format!("{OPEN}new{CLOSE}markdown\n## Output style\n\nold{CLOSE}")
        );
    }

    #[test]
    fn synced_replaces_only_snippet_bodies() {
        let install = format!(
            "intro\n{}\nmiddle\n```markdown\nother block\n```\n{}\nend\n",
            snippet("old one"),
            snippet("old\ntwo")
        );
        let (out, count) = synced(&install, "new");
        assert_eq!(count, 2);
        assert_eq!(
            out,
            format!(
                "intro\n{}\nmiddle\n```markdown\nother block\n```\n{}\nend\n",
                snippet("new"),
                snippet("new")
            )
        );
    }

    #[test]
    fn synced_leaves_an_unclosed_snippet_alone() {
        let install = format!("{}\n{OPEN}never closed", snippet("old"));
        let (out, count) = synced(&install, "new");
        assert_eq!(count, 1);
        assert_eq!(out, format!("{}\n{OPEN}never closed", snippet("new")));
    }

    #[test]
    fn sync_rules_writes_reports_and_refuses() {
        let dir = std::env::temp_dir().join(format!("laconic-tools-{}", std::process::id()));
        fs::create_dir_all(dir.join("rules")).unwrap();
        fs::write(dir.join("rules/laconic.md"), "# Title\n\nKeep it short.\n").unwrap();
        let stale = (0..SNIPPETS)
            .map(|_| snippet("old"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(dir.join("INSTALL.md"), &stale).unwrap();

        assert!(sync_rules(&dir, true).unwrap_err().contains("differ"));
        assert_eq!(fs::read_to_string(dir.join("INSTALL.md")).unwrap(), stale);
        assert_eq!(
            sync_rules(&dir, false),
            Ok("INSTALL.md snippets updated".into())
        );
        assert!(sync_rules(&dir, true).is_ok());

        fs::write(dir.join("INSTALL.md"), snippet("old")).unwrap();
        assert!(sync_rules(&dir, false).unwrap_err().contains("found 1"));
        assert_eq!(
            fs::read_to_string(dir.join("INSTALL.md")).unwrap(),
            snippet("old")
        );

        let long = format!("# Title\n\n{}", "word ".repeat(MAX_WORDS + 1));
        fs::write(dir.join("rules/laconic.md"), long).unwrap();
        assert!(sync_rules(&dir, true).unwrap_err().contains("241 words"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
