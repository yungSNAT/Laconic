// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Measures `laconic-check preserve` against a published fact checker on public rewrites.
//!
//! The AI Humanizer Benchmark publishes the AI-written texts of each monthly cycle and every
//! commercial humaniser's rewrite of them (CC BY 4.0). The paper 'Humanized but Still True?'
//! (arXiv 2611.00359) judged each number, name and quotation of 3 or more words in the
//! 2026-10 cycle's rewrites as kept or altered, and published those judgements with its
//! checker, humanizer-fact-audit (code MIT, data CC BY 4.0). This command downloads both at
//! pinned revisions into a cache folder, runs `preserve` on each source and rewrite, and
//! reports, per kind, how many documents the paper marks altered that `preserve` also
//! reports, and on what `preserve` fails the documents the paper marks fully kept. Nothing
//! downloaded is stored in this repository.

use crate::cases::{parse, read_text, write};
use crate::proc::{self, Input};
use crate::py;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CYCLE: &str = "2026-10";
pub const AIHB_REVISION: &str = "6306b2043736dd958cf39ee1d9a64956c54dd78a";
pub const FACT_AUDIT_COMMIT: &str = "e665dda0506a64c834886c94d8f328d447119a07";
/// The paper's kinds of unit: (group, table label, what `preserve` does with it).
pub const KINDS: [(&str, &str, &str); 3] = [
    ("num", "Numbers", "report a dropped number or quantity"),
    (
        "quote",
        "Quotations of 3 or more words",
        "report a dropped quotation",
    ),
    ("ent", "Names", "list a name to check"),
];

pub fn aihb_url(tool: &str) -> String {
    format!(
        "https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark/resolve/\
         {AIHB_REVISION}/data/cycles/{CYCLE}/data/{tool}.jsonl"
    )
}

pub fn fact_audit_url() -> String {
    format!(
        "https://raw.githubusercontent.com/zwang-devv/humanizer-fact-audit/\
         {FACT_AUDIT_COMMIT}/data/fact_units.jsonl"
    )
}

/// A (tool, sample) pair.
type Key = (String, String);
/// What `preserve` flagged, or the paper altered.
pub type Kinds = BTreeSet<String>;
/// Counts by name; a missing name counts 0, as with Python's `Counter`.
pub type Counts = BTreeMap<String, u64>;

fn key_repr((tool, sample): &Key) -> String {
    format!("({}, {})", py::str_repr(tool), py::str_repr(sample))
}

fn list_repr<'a>(items: impl IntoIterator<Item = &'a String>) -> String {
    let shown: Vec<String> = items.into_iter().map(|s| py::str_repr(s)).collect();
    format!("[{}]", shown.join(", "))
}

/// The body at `url`, following redirects, with Python's 60-second socket timeout for
/// connecting and for the response.
pub fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(60)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| format!("{url}: {e}"))?;
    response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|e| format!("{url}: {e}"))
}

/// Downloads `url` to `path` once; a later run reads the cached copy. The download is written
/// under another name and then renamed, so an interrupted one leaves no `path`. Only an http
/// or https URL is opened, never a local file or another scheme.
pub fn fetch(
    url: &str,
    path: &Path,
    get: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<PathBuf, String> {
    let scheme = url.split_once(':').map(|(s, _)| s.to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http" | "https")) {
        return Err(format!("not an http or https URL: {url}"));
    }
    if !path.is_file() {
        let parent = path.parent().expect("a cache file has a folder");
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        let data = get(url)?;
        let mut name = path.file_name().expect("a file name").to_os_string();
        name.push(".partial");
        let partial = path.with_file_name(name);
        fs::write(&partial, data).map_err(|e| format!("{}: {e}", partial.display()))?;
        fs::rename(&partial, path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(path.to_path_buf())
}

/// The rows of a JSON Lines file; a blank line is an error, as it may be a lost row.
pub fn read_jsonl(path: &Path) -> Result<Vec<Value>, String> {
    let text = read_text(path)?;
    py::split_lines(&text)
        .into_iter()
        .enumerate()
        .map(|(number, line)| {
            if py::strip(line).is_empty() {
                Err(format!("{}: line {} is blank", path.display(), number + 1))
            } else {
                parse(line)
            }
        })
        .collect()
}

/// The kinds of unit the paper marks altered, for each (tool, sample) of the cycle.
///
/// A repeated document, units that are not a list of objects, a unit of a group other than
/// the paper's 3 or a `kept` that is not a boolean is an error, never coerced.
pub fn judged_documents(rows: &[Value]) -> Result<BTreeMap<Key, Kinds>, String> {
    let mut judged = BTreeMap::new();
    for row in rows {
        if py::field(row, "set")?.as_str() != Some("commercial")
            || py::field(row, "aihb_cycle")?.as_str() != Some(CYCLE)
        {
            continue;
        }
        let key = (
            py::text(row, "tool")?.to_owned(),
            py::text(row, "sample_id")?.to_owned(),
        );
        if judged.contains_key(&key) {
            return Err(format!("judged twice: {}", key_repr(&key)));
        }
        let units = py::field(row, "units")?
            .as_array()
            .filter(|units| units.iter().all(Value::is_object))
            .ok_or_else(|| format!("the units of {} are not a list of objects", key_repr(&key)))?;
        let mut altered = Kinds::new();
        for unit in units {
            let group = py::get(unit, "group").unwrap_or(&Value::Null);
            let Some(group) = group.as_str().filter(|g| KINDS.iter().any(|k| k.0 == *g)) else {
                return Err(format!(
                    "a unit of {} has an unknown group {}",
                    key_repr(&key),
                    py::repr(group)
                ));
            };
            match py::get(unit, "kept") {
                Some(Value::Bool(true)) => {}
                Some(Value::Bool(false)) => {
                    altered.insert(group.to_owned());
                }
                _ => {
                    return Err(format!(
                        "a unit of {} has a non-boolean 'kept'",
                        key_repr(&key)
                    ));
                }
            }
        }
        judged.insert(key, altered);
    }
    Ok(judged)
}

/// What a `preserve` report flags: the paper's kinds, 'meaning' for meaning words and
/// 'other' for a dropped month, inline code or URL.
pub fn flagged(stdout: &str) -> Kinds {
    let mut kinds = Kinds::new();
    for line in py::split_lines(stdout) {
        let kind = if line.starts_with("dropped number:") || line.starts_with("dropped quantity:") {
            "num"
        } else if line.starts_with("dropped quotation:") {
            "quote"
        } else if line.starts_with("check name:") {
            "ent"
        } else if line.starts_with("dropped meaning word:") {
            "meaning"
        } else if line.starts_with("dropped ") {
            "other"
        } else {
            continue;
        };
        kinds.insert(kind.to_owned());
    }
    kinds
}

/// Runs a command, as `subprocess.run` with no shell.
pub type Exec<'a> = &'a dyn Fn(&[String], &Input) -> Result<proc::Output, proc::Failure>;

/// The production `Exec`: no working directory or timeout.
pub fn exec(command: &[String], input: &Input) -> Result<proc::Output, proc::Failure> {
    proc::run(command, input, None, None)
}

/// `source` and `rewrite` as `original.md` and `edited.md` in a fresh folder, for a `preserve`
/// call; the folder goes when the returned guard drops.
pub fn preserve_files(
    source: &str,
    rewrite: &str,
) -> Result<(tempfile::TempDir, [String; 2]), String> {
    let dir = crate::run_evals::neutral_cwd(".tmp")?;
    let (original, edited) = (dir.path().join("original.md"), dir.path().join("edited.md"));
    write(&original, source)?;
    write(&edited, rewrite)?;
    Ok((dir, [original, edited].map(|p| p.display().to_string())))
}

/// Whether `preserve` fails the rewrite, and what it flags, run with `--loose` when `loose`.
/// `binary` is the executable file `main` resolved; exit 0 is a pass, 1 a failure and any
/// other status an error.
pub fn preserve(
    binary: &str,
    source: &str,
    rewrite: &str,
    loose: bool,
    exec: Exec,
) -> Result<(bool, Kinds), String> {
    let (_dir, files) = preserve_files(source, rewrite)?;
    let mut command = vec![binary.to_owned(), "preserve".to_owned()];
    if loose {
        command.push("--loose".to_owned());
    }
    command.extend(files);
    let done = exec(&command, &Input::Inherit).map_err(|e| e.message(binary))?;
    if done.code != 0 && done.code != 1 {
        let stderr = proc::text(&done.stderr)?;
        return Err(format!(
            "preserve exited {}: {}",
            done.code,
            py::strip(&stderr)
        ));
    }
    Ok((done.code == 1, flagged(&proc::text(&done.stdout)?)))
}

fn bump(counts: &mut Counts, name: &str, by: bool) {
    *counts.entry(name.to_owned()).or_default() += u64::from(by);
}

/// Counts per document: the paper's altered kinds, and what `preserve` did with them.
pub fn tally(results: &[(Kinds, bool, Kinds)]) -> Counts {
    let mut counts = Counts::new();
    for (altered, failed, kinds) in results {
        if altered.is_empty() {
            bump(&mut counts, "kept", true);
            let name = if !failed {
                "kept passed"
            } else if kinds.contains("num") || kinds.contains("quote") {
                "kept failed on numbers or quotations"
            } else if kinds.contains("other") {
                "kept failed on months, code or URLs"
            } else {
                "kept failed on meaning words only"
            };
            bump(&mut counts, name, true);
        } else {
            bump(&mut counts, "altered", true);
            bump(&mut counts, "altered failed", *failed);
            for kind in altered {
                bump(&mut counts, &format!("{kind} altered"), true);
                bump(
                    &mut counts,
                    &format!("{kind} flagged"),
                    kinds.contains(kind),
                );
            }
        }
    }
    counts
}

fn count(counts: &Counts, name: &str) -> u64 {
    counts.get(name).copied().unwrap_or(0)
}

/// The Markdown table `evals/README.md` publishes.
pub fn table(counts: &Counts) -> String {
    let c = |name: &str| count(counts, name);
    let mut rows = vec![
        "| Documents the paper marks as | Documents | `preserve` result |".to_owned(),
        "| --- | --- | --- |".to_owned(),
        format!(
            "| Any unit altered | {} | {} fail |",
            c("altered"),
            c("altered failed")
        ),
    ];
    for (kind, label, result) in KINDS {
        rows.push(format!(
            "| {label} altered | {} | {} {result} |",
            c(&format!("{kind} altered")),
            c(&format!("{kind} flagged"))
        ));
    }
    rows.push(format!(
        "| Every unit kept | {} | {} pass; {} fail only on meaning words; {} fail on a month, \
         inline code or URL; {} fail on a number, quantity or quotation |",
        c("kept"),
        c("kept passed"),
        c("kept failed on meaning words only"),
        c("kept failed on months, code or URLs"),
        c("kept failed on numbers or quotations")
    ));
    rows.join("\n")
}

/// Checks one source and rewrite: `preserve` with the binary.
pub type Check<'a> = &'a dyn Fn(&str, &str, &str) -> Result<(bool, Kinds), String>;

/// The tally over every judged document, and the ids where the 2 checkers differ. Each
/// download is cached under its pin, so a changed pin downloads afresh.
pub fn run(
    binary: &str,
    cache: &Path,
    check: Check,
    get: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(Counts, Vec<String>), String> {
    let units = fetch(
        &fact_audit_url(),
        &cache.join(FACT_AUDIT_COMMIT).join("fact_units.jsonl"),
        get,
    )?;
    let judged = judged_documents(&read_jsonl(&units)?)?;
    let tools: BTreeSet<&String> = judged.keys().map(|(tool, _)| tool).collect();
    let mut pairs: BTreeMap<Key, (String, String)> = BTreeMap::new();
    for tool in tools {
        let path = cache
            .join(AIHB_REVISION)
            .join(CYCLE)
            .join(format!("{tool}.jsonl"));
        for row in read_jsonl(&fetch(&aihb_url(tool), &path, get)?)? {
            let key = (
                py::text(&row, "humanizer")?.to_owned(),
                py::text(&row, "sample_id")?.to_owned(),
            );
            if pairs.contains_key(&key) {
                return Err(format!("benchmark rewrite twice: {}", key_repr(&key)));
            }
            let output = py::field(&row, "output")?.as_str().ok_or_else(|| {
                format!(
                    "benchmark rewrite missing or not text for {}",
                    key_repr(&key)
                )
            })?;
            pairs.insert(
                key,
                (py::text(&row, "input")?.to_owned(), output.to_owned()),
            );
        }
    }
    let missing: Vec<String> = judged
        .keys()
        .filter(|key| !pairs.contains_key(*key))
        .map(key_repr)
        .collect();
    if !missing.is_empty() {
        return Err(format!("no benchmark rewrite for [{}]", missing.join(", ")));
    }
    let mut results = Vec::new();
    let mut differences = Vec::new();
    for (key, altered) in &judged {
        let (source, rewrite) = &pairs[key];
        let (failed, kinds) = check(binary, source, rewrite)?;
        let missed: Vec<&String> = altered
            .iter()
            .filter(|kind| !kinds.contains(*kind) && *kind != "ent")
            .collect();
        let extra: Vec<&String> = kinds
            .iter()
            .filter(|kind| (*kind == "num" || *kind == "quote") && !altered.contains(*kind))
            .collect();
        if !missed.is_empty() || !extra.is_empty() {
            differences.push(format!(
                "{} {}: missed {}, extra {}",
                key.0,
                key.1,
                list_repr(missed),
                list_repr(extra)
            ));
        }
        results.push((altered.clone(), failed, kinds));
    }
    Ok((tally(&results), differences))
}

/// `shutil.which(name)`: a name with a folder in it is checked as given; a bare name is
/// looked up on the PATH. The result is an executable file.
pub fn which(name: &str) -> Option<PathBuf> {
    if name.contains('/') || (cfg!(windows) && name.contains('\\')) {
        return executable(Path::new(name));
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .find_map(|dir| executable(&dir.join(name)))
}

/// `path` if it is an executable file: one with an execute bit.
#[cfg(unix)]
fn executable(path: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path).ok()?;
    (metadata.is_file() && metadata.permissions().mode() & 0o111 != 0).then(|| path.to_path_buf())
}

/// `path`, or `path` with an extension from PATHEXT, if that is a file with such an extension.
#[cfg(windows)]
fn executable(path: &Path) -> Option<PathBuf> {
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let extensions: Vec<&str> = extensions.split(';').filter(|e| !e.is_empty()).collect();
    let known = |p: &Path| {
        p.extension().is_some_and(|ext| {
            let ext = format!(".{}", ext.to_string_lossy());
            extensions
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&ext))
        })
    };
    std::iter::once(path.to_path_buf())
        .chain(
            extensions
                .iter()
                .map(|ext| PathBuf::from(format!("{}{ext}", path.display()))),
        )
        .find(|p| p.is_file() && known(p))
}

/// The report the command prints.
pub fn report(counts: &Counts, differences: &[String]) -> String {
    let mut out = format!(
        "Judged documents: {}\n\n{}\n\nDocuments where the checkers differ on numbers or \
         quotations: {}\n",
        count(counts, "altered") + count(counts, "kept"),
        table(counts),
        differences.len()
    );
    for line in differences {
        let _ = writeln!(out, "- {line}");
    }
    out
}

#[cfg(test)]
mod tests;
