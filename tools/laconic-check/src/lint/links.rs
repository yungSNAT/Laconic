// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The `broken-anchor` and `broken-link` rules: link targets that lead nowhere, checked
//! offline. `broken-anchor` needs only the text: a `#fragment` naming no heading or id in the
//! file, and a full or collapsed reference link (`[text][label]`, `[text][]`) with no
//! definition. `broken-link` needs the file's place in a [`Workspace`]: a relative target that
//! names no file or folder, or names one in another case, and a fragment naming no heading in
//! another Markdown file. Targets with a scheme (`https:`, `mailto:`), protocol-relative and
//! root-relative targets are never checked, and nothing outside the workspace is read.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::{Finding, RuleId};

mod anchors;
mod syntax;
use anchors::*;
use syntax::*;

/// The largest Markdown file whose headings `broken-link` reads.
const MAX_TARGET_BYTES: u64 = 1 << 20;

/// How many folder listings and file reads one workspace allows; past it, targets go unchecked.
const MAX_READS: usize = 10_000;

/// The folder `broken-link` resolves relative targets within. A target that leads outside it
/// by `..`, or through any symbolic link or junction (inside the folder or not), is not
/// checked.
pub struct Workspace {
    root: PathBuf,
    listings: RefCell<HashMap<PathBuf, Option<Rc<Vec<OsString>>>>>,
    anchors: RefCell<HashMap<PathBuf, Option<Rc<HashSet<String>>>>>,
    reads: Cell<usize>,
}

/// Where a relative target leads.
#[derive(Debug, PartialEq)]
enum Resolved {
    /// Outside the workspace, or not checked (an odd target or the read budget spent).
    Unchecked,
    /// The component written, which names nothing in its folder.
    Missing(String),
    /// The component written, and the name the folder holds in another case.
    Case(String, String),
    Found(PathBuf),
}

impl Workspace {
    /// The workspace rooted at `root`, or `None` when `root` cannot be resolved.
    pub fn new(root: &Path) -> Option<Workspace> {
        Some(Workspace {
            root: root.canonicalize().ok()?,
            listings: RefCell::default(),
            anchors: RefCell::default(),
            reads: Cell::new(0),
        })
    }

    /// The folder holding `file`, resolved, if it lies inside the workspace.
    pub fn folder_of(&self, file: &Path) -> Option<PathBuf> {
        let folder = file.canonicalize().ok()?.parent()?.to_path_buf();
        folder.starts_with(&self.root).then_some(folder)
    }

    fn spend(&self) -> bool {
        let used = self.reads.get();
        self.reads.set(used + 1);
        used < MAX_READS
    }

    /// The names in `folder`, which must already be resolved and inside the workspace.
    fn listing(&self, folder: &Path) -> Option<Rc<Vec<OsString>>> {
        if let Some(cached) = self.listings.borrow().get(folder) {
            return cached.clone();
        }
        let names = if self.spend() {
            std::fs::read_dir(folder).ok().map(|entries| {
                Rc::new(
                    entries
                        .filter_map(|e| e.ok().map(|e| e.file_name()))
                        .collect(),
                )
            })
        } else {
            None
        };
        self.listings
            .borrow_mut()
            .insert(folder.to_path_buf(), names.clone());
        names
    }

    /// Follows the decoded relative `path` from `folder`, one name at a time, comparing each
    /// name exactly so a target that only matches in another case is reported.
    fn resolve(&self, folder: &Path, path: &str) -> Resolved {
        let Ok(relative) = folder.strip_prefix(&self.root) else {
            return Resolved::Unchecked;
        };
        let mut parts: Vec<String> = relative
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    if parts.pop().is_none() {
                        return Resolved::Unchecked;
                    }
                }
                _ => parts.push(part.to_owned()),
            }
        }
        let mut here = self.root.clone();
        for part in &parts {
            let Some(names) = self.listing(&here) else {
                return Resolved::Unchecked;
            };
            if !names.iter().any(|n| n == part.as_str()) {
                let lower = part.to_lowercase();
                return match names
                    .iter()
                    .find(|n| n.to_string_lossy().to_lowercase() == lower)
                {
                    Some(n) => Resolved::Case(part.clone(), n.to_string_lossy().into_owned()),
                    None => Resolved::Missing(part.clone()),
                };
            }
            // A symbolic link or junction may lead out of the workspace, and following it to
            // find out would touch what lies outside, so a target through one is not checked.
            let next = here.join(part);
            let plain = std::fs::symlink_metadata(&next).is_ok_and(|m| !m.file_type().is_symlink());
            if !plain {
                return Resolved::Unchecked;
            }
            match next.canonicalize() {
                Ok(next) if next.starts_with(&self.root) => here = next,
                _ => return Resolved::Unchecked,
            }
        }
        Resolved::Found(here)
    }

    /// The anchors of the Markdown file at the resolved `path`, or `None` when it is too large,
    /// unreadable or past the read budget.
    fn anchors_of(&self, path: &Path) -> Option<Rc<HashSet<String>>> {
        if let Some(cached) = self.anchors.borrow().get(path) {
            return cached.clone();
        }
        // Only a regular file is opened: opening a FIFO waits for a writer, and a device may
        // never end. The read stays capped should the file grow after the check.
        let small =
            std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() <= MAX_TARGET_BYTES);
        let found = (small && self.spend())
            .then(|| {
                let mut text = String::new();
                std::fs::File::open(path)
                    .and_then(|f| f.take(MAX_TARGET_BYTES + 1).read_to_string(&mut text))
                    .ok()
                    .filter(|&n| n as u64 <= MAX_TARGET_BYTES)
                    .map(|_| text)
            })
            .flatten()
            .map(|text| Rc::new(anchors(&text)));
        self.anchors
            .borrow_mut()
            .insert(path.to_path_buf(), found.clone());
        found
    }
}

/// A link target and where it starts.
struct Target {
    text: String,
    line: usize,
    column: usize,
}

/// The links of one document, gathered paragraph by paragraph.
#[derive(Default)]
pub(super) struct Links {
    targets: Vec<Target>,
    /// Full and collapsed reference links: the label and where the link starts.
    references: Vec<Target>,
    /// Normalised labels of reference definitions.
    defined: HashSet<String>,
}

impl Links {
    /// Records the links in `chars`, the raw text of a paragraph or line; `at` maps a 0-based
    /// index in it to its line and 1-based column.
    pub(super) fn collect(&mut self, chars: &[char], at: impl Fn(usize) -> (usize, usize)) {
        // Lint joins a paragraph's lines with a space; the scans stop at a line break where
        // CommonMark does, so each such space is read as one.
        let chars: Vec<char> = (0..chars.len())
            .map(|k| match chars[k] {
                ' ' if k + 1 < chars.len() && at(k + 1).0 != at(k).0 => '\n',
                c => c,
            })
            .collect();
        let chars = &chars[..];
        let mut in_code = vec![false; chars.len()];
        for (start, end) in code_spans(chars) {
            in_code[start..end].fill(true);
        }
        let push = |list: &mut Vec<Target>, text: String, i: usize| {
            let (line, column) = at(i);
            list.push(Target { text, line, column });
        };
        // A definition, `[label]: target`, opens a line; it is recognised at the paragraph's
        // start and after each line break.
        let mut line_start = 0;
        while line_start < chars.len() {
            if let Some((label, target, start)) = definition(chars, line_start) {
                self.defined.insert(normalise(&label));
                push(&mut self.targets, target, start);
            }
            let (line, _) = at(line_start);
            line_start = (line_start..chars.len())
                .find(|&i| at(i).0 != line)
                .unwrap_or(chars.len());
        }
        let openers = opening_brackets(chars);
        let mut i = 0;
        while i < chars.len() {
            // Only `]` and `=` start anything, so only they are tested for an escape: testing
            // every character walks a run of backslashes back from each of its members.
            if in_code[i] || (matches!(chars[i], ']' | '=') && is_escaped(chars, i)) {
                i += 1;
                continue;
            }
            match chars[i] {
                ']' if chars.get(i + 1) == Some(&'(') => {
                    if let Some((target, start, end)) = inline_target(chars, i + 2) {
                        push(&mut self.targets, target, start);
                        i = end;
                        continue;
                    }
                }
                ']' if chars.get(i + 1) == Some(&'[') => {
                    if let Some(close) = (i + 2..chars.len()).find(|&k| chars[k] == ']') {
                        let label: String = chars[i + 2..close].iter().collect();
                        let open = openers[i];
                        let followed = matches!(chars.get(close + 1), Some('(' | ':'));
                        if let (Some(open), false) = (open, followed) {
                            let label = if label.trim().is_empty() {
                                chars[open + 1..i].iter().collect()
                            } else {
                                label
                            };
                            if !label.starts_with('^') && !label.contains('[') {
                                push(&mut self.references, label, open);
                            }
                            i = close + 1;
                            continue;
                        }
                    }
                }
                '=' => {
                    if let Some((target, start, end)) = attribute(chars, i) {
                        push(&mut self.targets, target, start);
                        i = end;
                        continue;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// The findings for `text`, the whole document: `broken-anchor` always, and `broken-link`
    /// when `place` gives the workspace and the folder of the file `text` came from.
    pub(super) fn findings(&self, text: &str, place: Option<(&Workspace, &Path)>) -> Vec<Finding> {
        let mut found = Vec::new();
        let own = anchors(text);
        // A link inside an HTML comment renders nothing, so it leads nowhere and is not read.
        // The comments come in order without overlapping, so only the last one starting at or
        // before a link can hold it.
        let comments = comment_spans(text);
        let live = |t: &&Target| {
            let at = (t.line, t.column);
            let before = comments.partition_point(|&(start, _)| start <= at);
            before == 0 || comments[before - 1].1 < at
        };
        for r in self.references.iter().filter(live) {
            if !self.defined.contains(&normalise(&r.text)) {
                found.push(finding(
                    r,
                    RuleId::BrokenAnchor,
                    format!(
                        "'[{}]' has no definition such as '[{}]: target' in this file",
                        r.text.trim(),
                        r.text.trim()
                    ),
                ));
            }
        }
        for t in self.targets.iter().filter(live) {
            let Some((path, fragment)) = split_target(&t.text) else {
                continue;
            };
            if path.is_empty() {
                if let Some(fragment) = fragment.filter(|f| !has_anchor(&own, f)) {
                    found.push(finding(
                        t,
                        RuleId::BrokenAnchor,
                        format!("'#{fragment}' matches no heading or id in this file"),
                    ));
                }
                continue;
            }
            let Some((workspace, folder)) = place else {
                continue;
            };
            let message = match workspace.resolve(folder, &path) {
                Resolved::Unchecked => continue,
                Resolved::Missing(part) if part == path.trim_end_matches('/') => {
                    format!("'{}': no such file or folder", t.text)
                }
                Resolved::Missing(part) => {
                    format!("'{}': no file or folder named '{part}'", t.text)
                }
                Resolved::Case(written, actual) => format!(
                    "'{}': '{written}' is '{actual}' on disk; a case-sensitive host will not find it",
                    t.text
                ),
                Resolved::Found(file) => {
                    let Some(fragment) = fragment else { continue };
                    let markdown = file.extension().is_some_and(|e| {
                        e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown")
                    });
                    if !markdown {
                        continue;
                    }
                    match workspace.anchors_of(&file) {
                        Some(set) if !has_anchor(&set, &fragment) => format!(
                            "'{}': '#{fragment}' matches no heading or id in that file",
                            t.text
                        ),
                        _ => continue,
                    }
                }
            };
            found.push(finding(t, RuleId::BrokenLink, message));
        }
        found
    }
}

fn finding(target: &Target, rule: RuleId, message: String) -> Finding {
    Finding {
        line: target.line,
        column: target.column,
        rule: rule.into(),
        message,
        fix: None,
    }
}

#[cfg(test)]
mod tests;
