// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `laconic-tools calibrate`: lints the human-written articles of an outside corpus and writes
//! each rule's counts to `tools/laconic-check/tests/outside-corpus.tsv`, which the rule reference
//! and its advisory gate read. The corpus is `human_detectors.json` from
//! jenna-russell/human_detectors: news articles with no stated licence, so it is read from a
//! local download, pinned by its Git blob id, and never stored here or fetched in CI. Instead
//! the file carries a fingerprint of its counts and of the sources they depend on, which a test
//! checks, so a hand-edited count or a rule change without a rerun fails CI.

use laconic_check::lint::{RULES, lint};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const OUT: &str = "tools/laconic-check/tests/outside-corpus.tsv";
const COMMIT: &str = "afcf03d14d2da4a038d8d0fafa5ec779dd858181";
/// `git hash-object --no-filters human_detectors.json` at COMMIT.
const BLOB: &str = "eb4030e4591de8fd687089b15bf86d6688b128eb";
/// The sources whose changes can change the counts: the rules, their word list and this file.
const SOURCES: &[&str] = &[
    "tools/laconic-check/src/lint.rs",
    "tools/laconic-check/src/us_words.txt",
    "tools/laconic-tools/src/calibrate.rs",
];

/// One split's articles and words, and per rule the articles flagged and the findings.
#[derive(Default)]
struct Split {
    articles: usize,
    words: usize,
    rules: BTreeMap<&'static str, (usize, usize)>,
}

impl Split {
    fn add(&mut self, text: &str) {
        self.articles += 1;
        self.words += text.split_whitespace().count();
        let mut seen = BTreeMap::new();
        for finding in lint(text) {
            *seen.entry(finding.rule.name()).or_insert(0) += 1;
        }
        for (rule, n) in seen {
            let entry = self.rules.entry(rule).or_default();
            entry.0 += 1;
            entry.1 += n;
        }
    }

    fn get(&self, rule: &str) -> (usize, usize) {
        self.rules.get(rule).copied().unwrap_or_default()
    }
}

fn blob_id(path: &Path) -> Result<String, String> {
    let out = Command::new("git")
        .args(["hash-object", "--no-filters", "--"])
        .arg(path)
        .output()
        .map_err(|e| format!("git hash-object: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git hash-object {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// SOURCES by path, with LF line endings so checkouts on Windows give the same fingerprint.
fn sources(root: &Path) -> Result<Vec<(&'static str, String)>, String> {
    SOURCES
        .iter()
        .map(|&path| {
            let file = root.join(path);
            fs::read_to_string(&file)
                .map(|text| (path, text.replace("\r\n", "\n")))
                .map_err(|e| format!("{}: {e}", file.display()))
        })
        .collect()
}

/// 64-bit FNV-1a over the sources and the counts, each followed by a zero byte. It detects
/// a change made without a rerun; it is not a defence against someone forging the file.
fn fingerprint(sources: &[(&str, String)], counts: &str) -> String {
    let parts = sources
        .iter()
        .flat_map(|(path, text)| [*path, text.as_str()])
        .chain([counts]);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in parts.flat_map(|part| part.bytes().chain([0])) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The TSV for the parsed corpus. Human-written articles with odd prompt ids calibrate; even
/// ones are held out.
fn tsv(data: &Value, sources: &[(&str, String)]) -> Result<String, String> {
    let entries = data
        .as_object()
        .ok_or("expected a JSON object of articles")?;
    let (mut calibration, mut held_out) = (Split::default(), Split::default());
    for (key, entry) in entries {
        if entry["ground_truth"] != "Human-written" {
            continue;
        }
        let prompt = entry["prompt_id"]
            .as_u64()
            .ok_or(format!("article {key} has no numeric prompt_id"))?;
        let text = entry["article"]
            .as_str()
            .ok_or(format!("article {key} has no text"))?;
        if prompt % 2 == 1 {
            calibration.add(text);
        } else {
            held_out.add(text);
        }
    }
    let mut counts = format!(
        "rule\tcalibration articles\tcalibration findings\theld-out articles\theld-out findings\n\
         (articles and words)\t{}\t{}\t{}\t{}\n",
        calibration.articles, calibration.words, held_out.articles, held_out.words,
    );
    for rule in RULES {
        let (ca, cf) = calibration.get(rule.name);
        let (ha, hf) = held_out.get(rule.name);
        counts.push_str(&format!("{}\t{ca}\t{cf}\t{ha}\t{hf}\n", rule.name));
    }
    Ok(format!(
        "# Written by `laconic-tools calibrate`; do not edit. Counts for the {} human-written\n\
         # articles in human_detectors.json (jenna-russell/human_detectors at {COMMIT},\n\
         # blob {BLOB}), which are not stored here. Calibration articles have odd prompt ids and\n\
         # held-out articles even ones. The fingerprint covers the counts and the rule sources;\n\
         # a test fails when either changes without a rerun.\n\
         # fingerprint {}\n\
         {counts}",
        calibration.articles + held_out.articles,
        fingerprint(sources, &counts),
    ))
}

/// Writes OUT from the corpus at `json`, or with `check` fails when OUT differs from a fresh run.
pub fn run(root: &Path, json: &Path, check: bool) -> Result<String, String> {
    let blob = blob_id(json)?;
    if blob != BLOB {
        return Err(format!(
            "{} has blob {blob}, not {BLOB}: download human_detectors.json from \
             jenna-russell/human_detectors at commit {COMMIT}",
            json.display()
        ));
    }
    let text = fs::read_to_string(json).map_err(|e| format!("{}: {e}", json.display()))?;
    let data: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", json.display()))?;
    let fresh = tsv(&data, &sources(root)?)?;
    let path = root.join(OUT);
    if !check {
        fs::write(&path, fresh).map_err(|e| format!("{}: {e}", path.display()))?;
        return Ok(format!("wrote {OUT}"));
    }
    let current = fs::read_to_string(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .replace("\r\n", "\n");
    if current == fresh {
        Ok(format!("{OUT} is current"))
    } else {
        Err(format!("{OUT} is out of date: rerun without --check"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The fingerprint a written table records, and the counts it covers: everything from
    /// the column header on.
    fn recorded(table: &str) -> Option<(&str, &str)> {
        let stored = table
            .lines()
            .find_map(|line| line.strip_prefix("# fingerprint "))?;
        let start = table.find("\nrule\t")? + 1;
        Some((stored, &table[start..]))
    }

    fn row<'a>(tsv: &'a str, rule: &str) -> Vec<&'a str> {
        tsv.lines()
            .find(|l| l.split('\t').next() == Some(rule))
            .unwrap_or_else(|| panic!("no row for {rule}"))
            .split('\t')
            .collect()
    }

    #[test]
    fn splits_human_articles_by_prompt_parity_and_counts_articles_once() {
        let data = json!({
            "1": { "ground_truth": "Human-written", "prompt_id": 3, "article": "The color and the color." },
            "2": { "ground_truth": "Human-written", "prompt_id": 4, "article": "A plain sentence." },
            "3": { "ground_truth": "Human-written", "prompt_id": 6, "article": "The color." },
            "4": { "ground_truth": "AI-generated", "prompt_id": 5, "article": "The color." }
        });
        let out = tsv(&data, &[]).unwrap();
        assert_eq!(
            row(&out, "(articles and words)"),
            ["(articles and words)", "1", "5", "2", "5"]
        );
        assert_eq!(
            row(&out, "us-spelling"),
            ["us-spelling", "1", "2", "1", "1"]
        );
        assert_eq!(row(&out, "em-dash"), ["em-dash", "0", "0", "0", "0"]);
        assert_eq!(
            out.lines().filter(|l| !l.starts_with('#')).count(),
            RULES.len() + 2
        );
        let (stored, counts) = recorded(&out).unwrap();
        assert_eq!(stored, fingerprint(&[], counts));
    }

    /// The committed table must come from calibrate run on the rules as they are now: a
    /// hand-edited count, or a change to SOURCES without a rerun, changes the fingerprint.
    #[test]
    fn the_outside_counts_match_their_fingerprint() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let table = fs::read_to_string(root.join(OUT))
            .unwrap()
            .replace("\r\n", "\n");
        let (stored, counts) = recorded(&table).expect("no fingerprint line");
        assert!(
            stored == fingerprint(&sources(&root).unwrap(), counts),
            "{OUT} is out of date or was edited by hand: run \
             `laconic-tools calibrate <human_detectors.json>` with the corpus from \
             jenna-russell/human_detectors at {COMMIT}"
        );
    }

    #[test]
    fn rejects_articles_without_a_numeric_prompt_id() {
        let data =
            json!({ "9": { "ground_truth": "Human-written", "prompt_id": "x", "article": "a" } });
        assert!(tsv(&data, &[]).unwrap_err().contains("article 9"));
    }

    #[test]
    fn refuses_a_file_other_than_the_pinned_corpus() {
        let path =
            std::env::temp_dir().join(format!("laconic-calibrate-{}.json", std::process::id()));
        fs::write(&path, "{}").unwrap();
        let err = run(Path::new("."), &path, true).unwrap_err();
        fs::remove_file(&path).unwrap();
        assert!(err.contains(BLOB), "{err}");
    }
}
