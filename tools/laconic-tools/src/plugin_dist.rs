// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Build the plugin as the Claude plugin directory reads it. From an
//! organisation it has not verified, the directory accepts skills only under
//! `skills/` and only Markdown, text, JSON, YAML and image files: no hooks,
//! scripts or MCP server. So this build takes from the commit at HEAD only the
//! manifest, the listing icon, the skills, the licence files and a README for
//! the listing, and refuses anything the directory would refuse. The full
//! plugin stays on `main`; `.github/workflows/plugin-dist.yml` publishes this
//! build to the `claude-plugin` branch when the version
//! in `package.json` rises.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

mod icon;

const MANIFEST: &str = ".claude-plugin/plugin.json";
/// The directory takes this file as the listing icon, but only when the plugin
/// is first saved or submitted, so the build requires it.
const ICON: &str = ".claude-plugin/icon.png";
/// Each committed path or folder the build ships, and where it ships it.
const SOURCES: [(&str, &str); 6] = [
    (MANIFEST, MANIFEST),
    (ICON, ICON),
    ("skills", "skills"),
    ("LICENSE", "LICENSE"),
    ("NOTICE", "NOTICE"),
    ("docs/plugin-directory-readme.md", "README.md"),
];
/// The manifest keys the build ships: each describes the plugin and loads
/// nothing. Any other key fails the build, so a component added on `main` is
/// never shipped or dropped unnoticed. `version` is not among them: the
/// manifest on `main` declares none, and the build stamps `package.json`'s.
const MANIFEST_KEYS: [&str; 8] = [
    "$schema",
    "name",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
];
/// The folder in a skill for other hosts' metadata (Codex's `openai.yaml`,
/// Gemini's `gemini.toml`), which Claude Code does not read.
const OTHER_HOSTS: &str = "agents";
/// The directory holds a plugin with more than 512 files. Its validator
/// counted folders too on 13 November 2026, so this limit counts both.
const MAX_ENTRIES: usize = 512;
/// The directory holds a file larger than this.
const MAX_BYTES: usize = 256 * 1024;
const TEXT: [&str; 5] = ["md", "txt", "json", "yaml", "yml"];
/// Text files without an extension: the licence files at the root and in each skill.
const BARE_TEXT: [&str; 2] = ["LICENSE", "NOTICE"];
/// The file whose `version` every versioned manifest shares. The build stamps
/// it into the manifest, and the workflow publishes only when it rises.
const VERSION_SOURCE: &str = "package.json";

/// Files keyed by their path in the plugin folder.
type Files = BTreeMap<String, Vec<u8>>;

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

/// Where a committed file ships, or `None` for a skill file Claude Code does
/// not read: other hosts' metadata, or a file beside the skill folders.
fn shipped_path(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("skills/") {
        let mut parts = rest.split('/').skip(1);
        return match (parts.next(), parts.next()) {
            (None, _) | (Some(OTHER_HOSTS), Some(_)) => None,
            _ => Some(path.to_owned()),
        };
    }
    SOURCES
        .iter()
        .find(|(source, _)| *source == path)
        .map(|(_, destination)| (*destination).to_owned())
}

/// The shipped path and blob of each file in a `git ls-tree -r -z` listing.
/// Anything other than a regular, non-executable file is refused, as is a
/// missing source.
fn listed(listing: &str) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    for record in listing.split('\0').filter(|record| !record.is_empty()) {
        let (meta, path) = record
            .split_once('\t')
            .ok_or("unexpected git ls-tree output")?;
        let [mode, _, oid] = meta.split(' ').collect::<Vec<_>>()[..] else {
            return Err("unexpected git ls-tree output".into());
        };
        let Some(shipped) = shipped_path(path) else {
            continue;
        };
        if mode != "100644" {
            return Err(format!(
                "{path} must be a regular, non-executable file, not mode {mode}"
            ));
        }
        files.insert(shipped, oid.to_owned());
    }
    for (source, destination) in SOURCES {
        let folder = format!("{destination}/");
        if !files
            .keys()
            .any(|path| path == destination || path.starts_with(&folder))
        {
            return Err(format!("{source} is not committed at HEAD"));
        }
    }
    Ok(files)
}

/// The shipped files at HEAD, as stored in Git, at the paths they ship to.
fn committed(root: &Path) -> Result<Files, String> {
    let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", "HEAD", "--"];
    args.extend(SOURCES.map(|(source, _)| source));
    let listing = String::from_utf8(git(root, &args)?)
        .map_err(|_| "git ls-tree listed a path that is not UTF-8")?;
    listed(&listing)?
        .into_iter()
        .map(|(path, oid)| Ok((path, git(root, &["cat-file", "blob", &oid])?)))
        .collect()
}

/// The plugin version in `package.json`, which must be `X.Y.Z` without
/// leading zeros: the workflow compares versions with `sort -V`, which orders
/// a pre-release above its release and 0.5.00 above 0.5.0.
fn package_version(bytes: &[u8]) -> Result<String, String> {
    let package: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("{VERSION_SOURCE}: {e}"))?;
    let version = package["version"].as_str().unwrap_or_default();
    let plain = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    };
    if version.split('.').count() != 3 || !version.split('.').all(plain) {
        return Err(format!(
            "{VERSION_SOURCE} version {version:?} must be X.Y.Z without leading zeros"
        ));
    }
    Ok(version.to_owned())
}

/// The manifest the directory reads: `main`'s, refused if it declares anything
/// beyond `MANIFEST_KEYS`.
/// `version` is the one the build stamps, which `main`'s manifest leaves out.
fn manifest(bytes: &[u8], version: &str) -> Result<Vec<u8>, String> {
    let mut manifest: Map<String, Value> =
        serde_json::from_slice(bytes).map_err(|e| format!("{MANIFEST}: {e}"))?;
    if let Some(key) = manifest
        .keys()
        .find(|key| !MANIFEST_KEYS.contains(&key.as_str()))
    {
        return Err(format!(
            "{MANIFEST} key {key} is not one the directory build ships"
        ));
    }
    manifest.insert("version".to_owned(), Value::from(version));
    let mut out = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    out.push(b'\n');
    Ok(out)
}

/// The front matter of a skill file: the lines between its opening and
/// closing `---`, or nothing when it has none.
fn front_matter(text: &str) -> &str {
    text.strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .map_or("", |(matter, _)| matter)
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Refuse a file the directory would refuse from an unverified organisation,
/// and say whether it is Markdown, whose links `check` then resolves.
fn accept(path: &str, bytes: &[u8]) -> Result<bool, String> {
    let parts: Vec<&str> = path.split('/').collect();
    let name = parts[parts.len() - 1];
    let in_skill = parts.len() >= 3 && parts[0] == "skills";
    if !in_skill && ![MANIFEST, ICON, "README.md", "LICENSE", "NOTICE"].contains(&path) {
        return Err(format!(
            "{path} is outside skills/ and is not the manifest, icon, README or a licence file"
        ));
    }
    if path == ICON {
        return icon::check(bytes).map(|()| false);
    }
    if path != MANIFEST && parts.iter().any(|part| part.starts_with('.')) {
        return Err(format!("{path} is a hidden file or in a hidden folder"));
    }
    if name == "SKILL.md" && parts.len() != 3 {
        return Err(format!("{path} is a skill outside skills/<name>/SKILL.md"));
    }
    let kind = extension(name);
    if !TEXT.contains(&kind.as_str()) && !BARE_TEXT.contains(&name) {
        return Err(format!(
            "{path} is not Markdown, text, JSON or YAML, and the only image the build ships is {ICON}"
        ));
    }
    if bytes.len() > MAX_BYTES {
        return Err(format!(
            "{path} has {} bytes; the directory accepts up to {MAX_BYTES}",
            bytes.len()
        ));
    }
    let Some(text) = std::str::from_utf8(bytes)
        .ok()
        .filter(|text| !text.contains('\0'))
    else {
        return Err(format!("{path} must be UTF-8 text without NUL bytes"));
    };
    // Claude Code runs a skill's front matter hooks and its !`command` lines.
    if kind == "md" && text.contains("!`") {
        return Err(format!(
            "{path} has a !` command, which the plugin would run"
        ));
    }
    if name == "SKILL.md"
        && front_matter(&text.replace("\r\n", "\n"))
            .lines()
            .any(|line| line.starts_with("hooks:"))
    {
        return Err(format!("{path} declares hooks in its front matter"));
    }
    if bytes.starts_with(b"#!") {
        return Err(format!("{path} starts with #!, so it reads as a script"));
    }
    Ok(kind == "md")
}

/// The Markdown link targets in `text`, outside fenced code blocks.
fn targets(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
        } else if !fenced {
            for (at, _) in line.match_indices("](") {
                let rest = &line[at + 2..];
                let end = rest.find([')', ' ', '\t']).unwrap_or(rest.len());
                found.push(&rest[..end]);
            }
        }
    }
    found
}

/// The path part of a target that names a file in the plugin, or `None` for
/// a URL, an anchor or an empty target.
fn local(target: &str) -> Option<&str> {
    let path = target.split('#').next().unwrap_or_default();
    (!path.is_empty() && !target.contains(':')).then_some(path)
}

/// `path` relative to the folder `base`, with `.` and `..` resolved, or
/// `None` when it climbs out of the plugin.
fn resolve(base: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = base.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// Every folder that holds a file, without the plugin folder itself.
fn folders(files: &Files) -> BTreeSet<String> {
    files
        .keys()
        .flat_map(|path| path.match_indices('/').map(|(at, _)| path[..at].to_owned()))
        .collect()
}

/// Refuse a plugin the directory would refuse, or one whose Markdown links a
/// file it does not ship, and return its count of files and folders.
fn check(files: &Files) -> Result<usize, String> {
    let folders = folders(files);
    let entries = files.len() + folders.len();
    if entries >= MAX_ENTRIES {
        return Err(format!(
            "the plugin has {entries} files and folders; the directory needs fewer than {MAX_ENTRIES}"
        ));
    }
    for (path, bytes) in files {
        if !accept(path, bytes)? {
            continue;
        }
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let base = path.rsplit_once('/').map_or("", |(folder, _)| folder);
        for target in targets(text) {
            let Some(target_path) = local(target) else {
                continue;
            };
            let found = resolve(base, target_path)
                .is_some_and(|found| files.contains_key(&found) || folders.contains(&found));
            if !found {
                return Err(format!(
                    "{path} links to {target}, which the plugin does not ship"
                ));
            }
        }
    }
    Ok(entries)
}

/// Write the plugin at HEAD of `root` into the new folder `destination`.
pub fn build(root: &Path, destination: &Path) -> Result<String, String> {
    let mut files = committed(root)?;
    let package = git(
        root,
        &["cat-file", "blob", &format!("HEAD:{VERSION_SOURCE}")],
    )?;
    let generated = manifest(&files[MANIFEST], &package_version(&package)?)?;
    files.insert(MANIFEST.to_owned(), generated);
    let entries = check(&files)?;
    fs::create_dir(destination)
        .map_err(|e| format!("cannot create {}: {e}", destination.display()))?;
    for (path, bytes) in &files {
        let target = destination.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("cannot create {path}: {e}"))?;
        }
        fs::write(&target, bytes).map_err(|e| format!("cannot write {path}: {e}"))?;
    }
    Ok(format!(
        "built {} files ({entries} files and folders) in {}",
        files.len(),
        destination.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::icon::tests::png;
    use super::*;
    use crate::pi_check::TempDir;

    fn plugin(extra: &[(&str, &str)]) -> Files {
        let mut files = Files::new();
        for (path, text) in [
            (MANIFEST, "{\"name\": \"laconic\"}\n"),
            ("README.md", "See [the skill](skills/laconic/SKILL.md).\n"),
            ("LICENSE", "licence\n"),
            ("NOTICE", "notice\n"),
            ("skills/laconic/SKILL.md", "---\nname: laconic\n---\n"),
            ("skills/laconic/LICENSE", "licence\n"),
            ("skills/laconic/references/a.yaml", "a: 1\n"),
        ]
        .iter()
        .chain(extra)
        {
            files.insert((*path).to_owned(), text.as_bytes().to_vec());
        }
        files.insert(ICON.to_owned(), png(1024, 1024));
        files
    }

    fn refused(path: &str, text: &str) -> String {
        check(&plugin(&[(path, text)])).expect_err(path)
    }

    #[test]
    fn only_the_listing_icon_ships_as_an_image() {
        for (path, reason) in [
            ("icon.png", "outside skills/"),
            (".claude-plugin/logo.png", "outside skills/"),
            ("assets/icon.png", "outside skills/"),
            ("skills/laconic/icon.png", "the only image the build ships"),
        ] {
            let mut files = plugin(&[]);
            files.insert(path.to_owned(), png(1024, 1024));
            let error = check(&files).unwrap_err();
            assert!(error.contains(reason), "{path}: {error}");
        }
        let mut oblong = plugin(&[]);
        oblong.insert(ICON.to_owned(), png(1024, 1000));
        assert!(check(&oblong).unwrap_err().contains("square"));
    }

    #[test]
    fn the_repository_builds_a_skills_only_plugin() {
        let temp = TempDir::new("laconic-plugin-dist-").unwrap();
        let out = temp.0.join("plugin");
        build(&crate::root(), &out).unwrap();
        for path in [
            MANIFEST,
            ICON,
            "README.md",
            "LICENSE",
            "NOTICE",
            "skills/laconic/SKILL.md",
            "skills/laconic-prose/SKILL.md",
        ] {
            assert!(out.join(path).is_file(), "{path}");
        }
        assert_eq!(
            fs::read(out.join(ICON)).unwrap(),
            fs::read(crate::root().join(ICON)).unwrap()
        );
        for path in ["hooks", "tools", ".cursor", "skills/laconic/agents"] {
            assert!(!out.join(path).exists(), "{path}");
        }
        let json =
            |path: &Path| -> Value { serde_json::from_slice(&fs::read(path).unwrap()).unwrap() };
        let version = json(&out.join(MANIFEST))["version"].clone();
        assert!(version.is_string());
        assert_eq!(
            version,
            json(&crate::root().join(VERSION_SOURCE))["version"]
        );
        assert!(build(&crate::root(), &out).is_err(), "an existing folder");
    }

    #[test]
    fn a_skills_only_plugin_passes() {
        assert_eq!(check(&plugin(&[])), Ok(12));
    }

    #[test]
    fn disallowed_paths_are_refused() {
        for path in [
            "hooks/hooks.json",
            "tools/laconic-check/mcp/server.mjs",
            ".mcp.json",
            "skills/README.md",
            ".cursor/skills/laconic/SKILL.md",
            "skills/laconic/.claude/settings.json",
            "skills/laconic/.DS_Store",
            "skills/laconic/references/SKILL.md",
        ] {
            refused(path, "{}\n");
        }
    }

    #[test]
    fn disallowed_file_types_are_refused() {
        for path in [
            "skills/laconic/scripts/run.sh",
            "skills/laconic/scripts/check.py",
            "skills/laconic/server.mjs",
            "skills/laconic/agents/gemini.toml",
            "skills/laconic/icon.png",
            "skills/laconic/bin/tool",
        ] {
            assert!(refused(path, "x\n").contains("not Markdown"), "{path}");
        }
    }

    #[test]
    fn unreadable_large_or_script_text_is_refused() {
        let large = "x".repeat(MAX_BYTES + 1);
        assert!(refused("skills/laconic/big.md", &large).contains("bytes"));
        assert!(refused("skills/laconic/nul.md", "a\0b").contains("NUL"));
        assert!(refused("skills/laconic/run.md", "#!/bin/sh\n").contains("#!"));
        let mut binary = plugin(&[]);
        binary.insert("skills/laconic/bad.txt".into(), vec![0xff, 0xfe]);
        assert!(check(&binary).unwrap_err().contains("UTF-8"));
    }

    #[test]
    fn skills_that_run_commands_are_refused() {
        let bang = refused("skills/laconic/more.md", "Status: !`git status`\n");
        assert!(bang.contains("!`"), "{bang}");
        let hooks = "---\nname: laconic-x\nhooks:\n  Stop: []\n---\nBody\n";
        assert!(refused("skills/laconic-x/SKILL.md", hooks).contains("hooks"));
        let crlf = hooks.replace('\n', "\r\n");
        assert!(refused("skills/laconic-x/SKILL.md", &crlf).contains("hooks"));
        let body = "---\nname: laconic-x\n---\nhooks: are fine in the body\n";
        assert!(check(&plugin(&[("skills/laconic-x/SKILL.md", body)])).is_ok());
    }

    #[test]
    fn only_regular_files_from_every_source_are_listed() {
        let record = |mode: &str, path: &str| format!("{mode} blob 0a1b\t{path}\0");
        let sources: String = [
            MANIFEST,
            ICON,
            "skills/laconic/SKILL.md",
            "LICENSE",
            "NOTICE",
            "docs/plugin-directory-readme.md",
        ]
        .iter()
        .map(|path| record("100644", path))
        .collect();
        let files = listed(&sources).unwrap();
        assert_eq!(files.len(), SOURCES.len());
        assert_eq!(files["README.md"], "0a1b");
        for mode in ["100755", "120000", "160000"] {
            let listing = sources.clone() + &record(mode, "skills/laconic/x.md");
            assert!(listed(&listing).unwrap_err().contains(mode), "{mode}");
        }
        // Other hosts' metadata is left out before its mode is read.
        assert!(listed(&(sources.clone() + &record("120000", "skills/laconic/agents/a"))).is_ok());
        let without_notice = sources.replace(&record("100644", "NOTICE"), "");
        let error = listed(&without_notice).unwrap_err();
        assert!(error.contains("NOTICE is not committed"), "{error}");
        let without_icon = sources.replace(&record("100644", ICON), "");
        let error = listed(&without_icon).unwrap_err();
        assert!(error.contains("icon.png is not committed"), "{error}");
    }

    #[test]
    fn links_must_stay_in_the_plugin() {
        for link in [
            "[r](../../rules/laconic.md)",
            "[x](missing.md#top)",
            "[o](../../../a)",
        ] {
            assert!(
                refused("skills/laconic/more.md", link).contains("does not ship"),
                "{link}"
            );
        }
        let fine = "[u](https://example.com) [a](#use) [s](SKILL.md)\n```\n[f](gone.md)\n```\n";
        assert!(check(&plugin(&[("skills/laconic/more.md", fine)])).is_ok());
    }

    #[test]
    fn too_many_entries_are_refused() {
        let names: Vec<String> = (0..MAX_ENTRIES)
            .map(|n| format!("skills/laconic/references/{n}.md"))
            .collect();
        let extra: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "x\n")).collect();
        assert!(
            check(&plugin(&extra))
                .unwrap_err()
                .contains("files and folders")
        );
    }

    #[test]
    fn the_version_must_be_plain() {
        assert_eq!(
            package_version(br#"{"name": "x", "version": "0.5.10"}"#),
            Ok("0.5.10".to_owned())
        );
        for version in [
            r#""0.5""#,
            r#""0.5.0.1""#,
            r#""0.5.0-rc.1""#,
            r#""0.5.00""#,
            r#""01.2.3""#,
            r#""v1.2.3""#,
            r#"" 1.2.3""#,
            r#""1..3""#,
            r#""""#,
            "1",
            "null",
        ] {
            let package = format!(r#"{{"version": {version}}}"#);
            assert!(package_version(package.as_bytes()).is_err(), "{version}");
        }
        for package in ["{}", "[]", "not json"] {
            assert!(package_version(package.as_bytes()).is_err(), "{package}");
        }
    }

    #[test]
    fn the_manifest_ships_only_descriptive_keys() {
        let shipped = manifest(
            br#"{"name": "laconic", "author": {"name": "yungSNAT"}}"#,
            "1.2.3",
        )
        .unwrap();
        let shipped: Value = serde_json::from_slice(&shipped).unwrap();
        assert_eq!(shipped["author"]["name"], "yungSNAT");
        assert_eq!(shipped["version"], "1.2.3");
        for key in ["hooks", "mcpServers", "commands", "skills", "version"] {
            let text = format!(r#"{{"name": "laconic", "{key}": "./x"}}"#);
            assert!(
                manifest(text.as_bytes(), "1.2.3")
                    .unwrap_err()
                    .contains(key),
                "{key}"
            );
        }
    }

    #[test]
    fn other_hosts_metadata_and_loose_files_are_left_out() {
        assert_eq!(shipped_path("skills/laconic/agents/gemini.toml"), None);
        assert_eq!(shipped_path("skills/plugin.json"), None);
        assert_eq!(
            shipped_path("skills/laconic/SKILL.md").as_deref(),
            Some("skills/laconic/SKILL.md")
        );
        assert_eq!(
            shipped_path("skills/laconic/agents").as_deref(),
            Some("skills/laconic/agents")
        );
        assert_eq!(
            shipped_path("docs/plugin-directory-readme.md").as_deref(),
            Some("README.md")
        );
    }
}
