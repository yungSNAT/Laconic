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
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::{Finding, RuleId};

/// The largest Markdown file whose headings `broken-link` reads.
const MAX_TARGET_BYTES: u64 = 1 << 20;

/// How many folder listings and file reads one workspace allows; past it, targets go unchecked.
const MAX_READS: usize = 10_000;

/// The folder `broken-link` resolves relative targets within. A target that leads outside it,
/// by `..` or through a symbolic link, is not checked.
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
            // A symbolic link may lead out of the workspace: resolve each step and stay inside.
            match here.join(part).canonicalize() {
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
        let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= MAX_TARGET_BYTES);
        let found = (small && self.spend())
            .then(|| std::fs::read_to_string(path).ok())
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
        let mut i = 0;
        while i < chars.len() {
            if in_code[i] || is_escaped(chars, i) {
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
                        let open = opening_bracket(chars, i);
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
        for r in &self.references {
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
        for t in &self.targets {
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

/// A reference label as CommonMark matches it: case-folded, with runs of whitespace as one
/// space and none at either end.
fn normalise(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn is_escaped(chars: &[char], i: usize) -> bool {
    chars[..i].iter().rev().take_while(|&&c| c == '\\').count() % 2 == 1
}

/// The `[start, end)` ranges of inline code spans: a run of backticks to the next run of the
/// same length.
fn code_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' || is_escaped(chars, i) {
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|&&c| c == '`').count();
        let mut k = i + run;
        let mut close = None;
        while k < chars.len() {
            if chars[k] == '`' {
                let other = chars[k..].iter().take_while(|&&c| c == '`').count();
                if other == run {
                    close = Some(k + run);
                    break;
                }
                k += other;
            } else {
                k += 1;
            }
        }
        match close {
            Some(end) => {
                spans.push((i, end));
                i = end;
            }
            None => i += run,
        }
    }
    spans
}

/// The `[` that opens the bracketed text closed at `close`, allowing nested brackets.
fn opening_bracket(chars: &[char], close: usize) -> Option<usize> {
    let mut depth = 0;
    for k in (0..close).rev() {
        match chars[k] {
            ']' if !is_escaped(chars, k) => depth += 1,
            '[' if !is_escaped(chars, k) => {
                if depth == 0 {
                    return Some(k);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// The destination of an inline link whose `(` ends just before `from`: its text, where it
/// starts and the index past the closing `)`.
fn inline_target(chars: &[char], from: usize) -> Option<(String, usize, usize)> {
    let mut i = from;
    while chars.get(i).is_some_and(|c| *c == ' ') {
        i += 1;
    }
    if chars.get(i) == Some(&'<') {
        let end = (i + 1..chars.len()).find(|&k| chars[k] == '>' && !is_escaped(chars, k))?;
        let close = (end..chars.len()).find(|&k| chars[k] == ')')?;
        return Some((chars[i + 1..end].iter().collect(), i + 1, close + 1));
    }
    let start = i;
    let mut depth = 0;
    while let Some(&c) = chars.get(i) {
        match c {
            '(' if !is_escaped(chars, i) => depth += 1,
            ')' if !is_escaped(chars, i) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            c if c.is_whitespace() => break,
            _ => {}
        }
        i += 1;
    }
    let close = (i..chars.len()).find(|&k| chars[k] == ')')?;
    Some((chars[start..i].iter().collect(), start, close + 1))
}

/// A reference definition, `[label]: target`, opening the line at `at` (up to 3 spaces in):
/// its label, target and the target's start.
fn definition(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    let indent = chars[at..]
        .iter()
        .take(4)
        .take_while(|&&c| c == ' ')
        .count();
    let open = at + indent;
    if indent > 3 || chars.get(open) != Some(&'[') || chars.get(open + 1) == Some(&'^') {
        return None;
    }
    let close = (open + 1..chars.len()).find(|&k| chars[k] == ']' && !is_escaped(chars, k))?;
    if chars.get(close + 1) != Some(&':') || close == open + 1 {
        return None;
    }
    let label: String = chars[open + 1..close].iter().collect();
    let mut start = close + 2;
    while chars.get(start).is_some_and(|c| *c == ' ') {
        start += 1;
    }
    let (start, end) = if chars.get(start) == Some(&'<') {
        let end = (start + 1..chars.len()).find(|&k| chars[k] == '>')?;
        (start + 1, end)
    } else {
        let end = (start..chars.len())
            .find(|&k| chars[k].is_whitespace())
            .unwrap_or(chars.len());
        (start, end)
    };
    (end > start).then(|| (label, chars[start..end].iter().collect(), start))
}

/// The quoted value of an HTML `href` or `src` attribute whose `=` is at `eq`: its text, where
/// it starts and the index past the closing quote.
fn attribute(chars: &[char], eq: usize) -> Option<(String, usize, usize)> {
    let name: String = chars[..eq]
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>()
        .to_lowercase();
    let len = if name.ends_with("href") {
        4
    } else if name.ends_with("src") {
        3
    } else {
        return None;
    };
    if eq < len + 1 || !chars[eq - len - 1].is_whitespace() {
        return None;
    }
    let quote = *chars.get(eq + 1).filter(|c| matches!(c, '"' | '\''))?;
    let end = (eq + 2..chars.len()).find(|&k| chars[k] == quote)?;
    Some((chars[eq + 2..end].iter().collect(), eq + 2, end + 1))
}

/// The decoded path and fragment of a target this rule checks, or `None` for one it never
/// checks: an absolute URL or path, a template placeholder or a target that does not decode.
fn split_target(target: &str) -> Option<(String, Option<String>)> {
    let target = target.trim();
    let scheme = target.split_once(':').is_some_and(|(s, _)| {
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    });
    if target.is_empty()
        || scheme
        || target.starts_with('/')
        || target.contains(['{', '\\', '<', '>'])
        || target.contains('$')
    {
        return None;
    }
    let (rest, fragment) = match target.split_once('#') {
        Some((rest, fragment)) => (rest, Some(decode(fragment)?)),
        None => (target, None),
    };
    let path = rest.split('?').next().unwrap_or("");
    let path = decode(path)?;
    if path.contains(':') {
        return None;
    }
    Some((path, fragment.filter(|f| !f.is_empty())))
}

/// `text` with its percent escapes decoded, or `None` when they do not form UTF-8.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Whether `fragment` names one of `anchors`, ignoring case and GitHub's `user-content-` prefix.
fn has_anchor(anchors: &HashSet<String>, fragment: &str) -> bool {
    let fragment = fragment.to_lowercase();
    let bare = fragment.strip_prefix("user-content-").unwrap_or(&fragment);
    bare == "top" || anchors.contains(bare)
}

/// The anchors of a Markdown document, in lower case: each heading's GitHub anchor (with `-1`,
/// `-2` for repeats), each `{#id}` attribute and each HTML `id` or `name` value. Code fences
/// and a leading frontmatter block hold none.
pub(super) fn anchors(text: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut heading = |title: &str, found: &mut HashSet<String>| {
        let (title, custom) = match title.trim_end().strip_suffix('}') {
            Some(rest) => match rest.rsplit_once("{#") {
                Some((title, id)) => (title, Some(id)),
                None => (title, None),
            },
            None => (title, None),
        };
        if let Some(id) = custom {
            found.insert(id.to_lowercase());
        }
        let slug = slug(title);
        let count = seen.entry(slug.clone()).or_insert(0);
        found.insert(if *count == 0 {
            slug.clone()
        } else {
            format!("{slug}-{count}")
        });
        *count += 1;
    };
    let mut lines = text.lines().peekable();
    if text.starts_with("---") && text.lines().skip(1).any(|l| l.trim_end() == "---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim_end() == "---" {
                break;
            }
        }
    }
    let mut fence: Option<&str> = None;
    let mut previous: Option<&str> = None;
    for line in lines {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(&trimmed[..3]);
            previous = None;
            continue;
        }
        collect_ids(trimmed, &mut found);
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        let atx = indent < 4
            && (1..=6).contains(&hashes)
            && trimmed[hashes..]
                .chars()
                .next()
                .is_none_or(|c| c == ' ' || c == '\t');
        let underline = indent < 4
            && !trimmed.is_empty()
            && (trimmed.trim_end().chars().all(|c| c == '=')
                || trimmed.trim_end().chars().all(|c| c == '-'));
        if atx {
            let title = trimmed[hashes..].trim().trim_end_matches('#');
            heading(title, &mut found);
            previous = None;
        } else if let (true, Some(title)) = (underline, previous) {
            heading(title, &mut found);
            previous = None;
        } else if trimmed.is_empty() {
            previous = None;
        } else if indent < 4 {
            previous = Some(trimmed);
        }
    }
    found
}

/// Records the values of `id="..."` and `name="..."` attributes in `line`.
fn collect_ids(line: &str, found: &mut HashSet<String>) {
    for key in ["id=", "name="] {
        let mut rest = line;
        while let Some(at) = rest.find(key) {
            let after = &rest[at + key.len()..];
            let before = rest[..at].chars().next_back();
            rest = after;
            if !before.is_some_and(char::is_whitespace) {
                continue;
            }
            if let Some(quote) = after.chars().next().filter(|c| matches!(c, '"' | '\''))
                && let Some(value) = after[1..].split(quote).next()
            {
                found.insert(value.to_lowercase());
            }
        }
    }
}

/// GitHub's anchor for a heading: its text without links' targets, HTML tags and punctuation
/// other than hyphens and underscores, in lower case, with each space a hyphen.
fn slug(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '<' => match (i..chars.len()).find(|&k| chars[k] == '>') {
                Some(end) => i = end + 1,
                None => i += 1,
            },
            ']' if chars.get(i + 1) == Some(&'(') => match inline_target(&chars, i + 2) {
                Some((_, _, end)) => i = end,
                None => i += 1,
            },
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests;
