// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! End-to-end CLI checks for the opt-in `reference-shape` rule.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "laconic-check-refs-cli-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for (file, content) in files {
        let path = dir.join(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
    dir
}

fn lint(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .current_dir(dir)
        .arg("lint")
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn reference_shape_flags_invalid_isbn_and_arxiv_only_when_enabled() {
    let text = concat!(
        "Valid book: ISBN 978-1-74037-738-6 and ISBN-10: 0-8044-2957-X.\n\n",
        "Valid paper: arXiv:2301.12345v2 and https://arxiv.org/abs/0704.0001.\n\n",
        "Broken book: ISBN 978-0-306-40615-8.\n\n",
        "Broken paper: arXiv:2313.12345.\n\n",
        "Code example: `ISBN 978-0-306-40615-8` and `arXiv:2313.12345`.\n",
    );
    let dir = folder("opt-in", &[("doc.md", text)]);

    // Default lint stays quiet: reference-shape is opt-in.
    let (code, out, err) = lint(&dir, &["doc.md"]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));

    // Explicit --only enables it as advisory (exit 0).
    let (code, out, err) = lint(&dir, &["--only", "reference-shape", "doc.md"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(
        lines[0].contains("reference-shape: '978-0-306-40615-8'"),
        "{out}"
    );
    assert!(lines[1].contains("reference-shape: '2313.12345'"), "{out}");
    assert!(lines.iter().all(|l| l.contains("(advisory)")), "{out}");

    // report, essay and docs enable reference-shape; email and chat do not.
    for profile in ["report", "essay", "docs"] {
        let (code, out, err) = lint(&dir, &["--profile", profile, "doc.md"]);
        assert_eq!((code, err.as_str()), (0, ""), "{profile}: {out}");
        assert_eq!(
            out.matches("reference-shape:").count(),
            2,
            "{profile}: {out}"
        );
    }
    for profile in ["email", "chat"] {
        let (code, out, err) = lint(&dir, &["--profile", profile, "doc.md"]);
        assert_eq!((code, err.as_str()), (0, ""), "{profile}: {out}");
        assert!(!out.contains("reference-shape:"), "{profile}: {out}");
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reference_shape_honours_protect_and_suppressions() {
    let dir = folder(
        "protect",
        &[
            (
                "laconic.toml",
                "only = reference-shape\n[protect]\nphrases = [\"ISBN 978-0-306-40615-8\"]\n",
            ),
            (
                "doc.md",
                concat!(
                    "Protected: ISBN 978-0-306-40615-8.\n\n",
                    "<!-- laconic-check-disable-next-line reference-shape -->\n",
                    "Suppressed: arXiv:2313.12345.\n",
                ),
            ),
        ],
    );
    let (code, out, err) = lint(&dir, &["doc.md"]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    let _ = fs::remove_dir_all(&dir);
}
