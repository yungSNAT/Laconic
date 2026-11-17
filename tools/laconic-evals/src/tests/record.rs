// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The `record` steps on the fixtures in `src/record/fixtures`. Each `*-expected` file there is
//! what the Python these steps replace (main at 1aa3a3a: the runs' `results.py`, `summary.py`,
//! `pairs.py` and the setup scripts in `run.sh`) wrote from the inputs beside it.

use super::*;
use std::collections::BTreeSet;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/record/fixtures")
        .join(path)
}

fn expected(path: &str) -> String {
    fs::read_to_string(fixture(path)).unwrap()
}

/// The files under `root`, as paths relative to it.
fn files(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                found.insert(path.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    found
}

/// A fresh copy of the fixture folder `name`.
fn work(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let from = fixture(name);
    for file in files(&from) {
        let to = dir.path().join(&file);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(from.join(&file), to).unwrap();
    }
    dir
}

/// Runs `laconic-evals record <args>` and returns what it printed.
fn record(args: &[&str]) -> String {
    let (status, printed) = call(&[&["record"], args].concat(), &NONE);
    assert_eq!(status, Ok(0), "{args:?}");
    printed
}

#[test]
fn style_steps_write_what_the_python_wrote() {
    let dir = work("style");
    let w = arg(dir.path());
    assert_eq!(record(&["answers", &w]), "");
    let mut answers = 0;
    for (run, name) in [("cold", "responses.jsonl"), ("warm", "warm.jsonl")] {
        for (n, row) in cases::read_jsonl(&dir.path().join(name))
            .unwrap()
            .iter()
            .enumerate()
        {
            let file = dir.path().join(format!("answers/{run}-{n}.txt"));
            assert_eq!(fs::read_to_string(file).unwrap(), row["response"]);
            answers += 1;
        }
    }
    assert_eq!(fs::read_dir(dir.path().join("answers")).unwrap().count(), answers);
    assert_eq!(
        record(&["style-results", &w]),
        expected("style-expected/results.jsonl")
    );
    let results = arg(&fixture("style-expected/results.jsonl"));
    assert_eq!(
        record(&["style-summary", &results]),
        expected("style-expected/summary.txt")
    );
}

#[test]
fn rewrite_steps_write_what_the_python_wrote() {
    let dir = work("rewrites");
    let w = arg(dir.path());
    let before = files(dir.path());
    assert_eq!(record(&["rewrite-pairs", &w]), "");
    let pairs = files(&fixture("rewrites-expected/pairs"));
    for file in &pairs {
        let made = fs::read_to_string(dir.path().join(file)).unwrap();
        assert_eq!(made, expected(&format!("rewrites-expected/pairs/{}", file.display())));
    }
    // Each pair's texts, for `laconic-check preserve`.
    let mut preserve = BTreeSet::new();
    for file in pairs.iter().filter(|f| f.starts_with("pairs-")) {
        let s = &file.to_str().unwrap()["pairs-".len()..file.to_str().unwrap().len() - ".jsonl".len()];
        for row in cases::read_jsonl(&dir.path().join(file)).unwrap() {
            let folder = PathBuf::from(format!("preserve/{s}-{}", row["id"].as_str().unwrap()));
            for (name, field) in [("original", "source"), ("edit", "rewrite")] {
                let text = fs::read_to_string(dir.path().join(folder.join(name))).unwrap();
                assert_eq!(text, row[field]);
                preserve.insert(folder.join(name));
            }
        }
    }
    let made: BTreeSet<PathBuf> = files(dir.path()).difference(&before).cloned().collect();
    assert_eq!(made, pairs.union(&preserve).cloned().collect());

    assert_eq!(
        record(&["rewrite-results", &w]),
        expected("rewrites-expected/results.jsonl")
    );
    let results = arg(&fixture("rewrites-expected/results.jsonl"));
    assert_eq!(
        record(&["rewrite-summary", &results]),
        expected("rewrites-expected/summary.txt")
    );
}

#[test]
fn setups_write_what_the_python_wrote() {
    for run in record::RUNS {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("patina-in")).unwrap();
        let texts = dir.path().join("aihb-2026-10.jsonl");
        fs::copy(fixture("rewrites/aihb-2026-10.jsonl"), &texts).unwrap();
        let w = arg(dir.path());
        assert_eq!(record(&["setups", run, &w]), "");
        fs::remove_file(texts).unwrap();
        let want = format!("setups-expected/{run}");
        let written = files(&fixture(&want));
        assert_eq!(files(dir.path()), written, "{run}");
        // The Python ran in a folder its output names; @W@ stands for it.
        let path = serde_json::to_string(&w).unwrap();
        for file in written {
            let made = fs::read_to_string(dir.path().join(&file)).unwrap();
            let want = expected(&format!("{want}/{}", file.display()));
            assert_eq!(made, want.replace("@W@", &path[1..path.len() - 1]), "{run} {file:?}");
        }
    }
    assert!(Cli::try_parse_from(["laconic-evals", "record", "setups", "other", "w"]).is_err());
}

#[test]
fn a_missing_results_file_is_an_error() {
    for step in ["style-summary", "rewrite-summary"] {
        let (status, _) = call(&["record", step, "/nonexistent/results.jsonl"], &NONE);
        assert!(matches!(status, Err(Stop::Failed(_))), "{step}");
    }
}
