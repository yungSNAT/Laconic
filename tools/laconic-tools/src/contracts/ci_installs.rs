// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Offline guards for third-party CLI installation in load checks, pinned actions, Dependabot
//! coverage and the absence of pip installs. Replaces `tests/test_ci_installs.py`, with its
//! regular expressions unchanged.

use super::{re, read, read_json, root, truthy};
use crate::au_words::is_space;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const WORKFLOWS: [&str; 3] = [
    "plugin-load-check.yml",
    "pi-load-check.yml",
    "plugin-dist.yml",
];

/// `shlex.split` for a command without quotes, escapes or comments, which is all the guard
/// expects; anything else fails rather than being split differently from a shell.
fn split_command(command: &str) -> Vec<String> {
    assert!(
        !command.contains(['\'', '"', '\\', '#']),
        "quoting is not supported in an install command: {command}"
    );
    command
        .split(is_space)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Every line of workflow text that names pip, pip3, pip3.11, pipx and the like.
fn pip_lines(text: &str) -> Vec<&str> {
    re(r"(?m)^.*\bpip(?:x|[\d.]*)\b.*$")
        .find_iter(text)
        .map(|m| m.as_str())
        .collect()
}

/// The `npm ci` command in each load-check workflow, split into words.
fn install_commands() -> Vec<(&'static str, Vec<String>)> {
    let npm = re(r"(?m)^\s*(?:run:\s*)?(npm (?:ci|install) .+)$");
    WORKFLOWS
        .iter()
        .map(|name| {
            let text = read(&root().join(".github/workflows").join(name));
            let commands: Vec<&str> = npm
                .captures_iter(&text)
                .map(|c| c.get(1).unwrap().as_str())
                .collect();
            assert_eq!(commands.len(), 1, "{name}: {commands:?}");
            (*name, split_command(commands[0]))
        })
        .collect()
}

/// The workflow files, sorted by path, as `(name, text)`.
fn workflow_files() -> Vec<(String, String)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                read(&p),
            )
        })
        .collect()
}

/// A dotted version as integers, as `tuple(int(part) for part in version.split("."))`.
fn version_tuple(package: &Value) -> Vec<u64> {
    let version = package["version"].as_str().expect("a version string");
    version
        .split('.')
        .map(|part| {
            part.parse()
                .unwrap_or_else(|_| panic!("bad version {version}"))
        })
        .collect()
}

/// Packages in the Pi lockfile whose path ends with `node_modules/<name>`.
fn pi_packages(name: &str) -> Vec<Value> {
    let lock = read_json(&root().join("ci/pi/package-lock.json"));
    let suffix = format!("node_modules/{name}");
    let found: Vec<Value> = lock["packages"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(path, _)| path.ends_with(&suffix))
        .map(|(_, package)| package.clone())
        .collect();
    assert!(!found.is_empty(), "no {name} in the Pi lockfile");
    found
}

/// Dependabot's update blocks by ecosystem, as `dict(zip(pieces[::2], pieces[1::2]))` over
/// `re.split(...)[1:]`: a repeated ecosystem keeps its last block.
fn dependabot_blocks(config: &str) -> BTreeMap<String, String> {
    let header = re(r"(?m)^  - package-ecosystem: (\S+)$");
    let mut blocks = BTreeMap::new();
    let mut previous: Option<(String, usize)> = None;
    for captures in header.captures_iter(config) {
        let whole = captures.get(0).unwrap();
        if let Some((name, start)) = previous.take() {
            blocks.insert(name, config[start..whole.start()].to_owned());
        }
        previous = Some((captures[1].to_owned(), whole.end()));
    }
    if let Some((name, start)) = previous {
        blocks.insert(name, config[start..].to_owned());
    }
    blocks
}

fn assert_matches(pattern: &str, text: &str, context: &str) {
    assert!(
        re(pattern).is_match(text),
        "{context}: no match for {pattern}"
    );
}

#[test]
fn installed_cli_versions_are_exact() {
    let exact = re(r"^\d+\.\d+\.\d+$");
    for (name, command) in install_commands() {
        assert_eq!(command[1], "ci", "{name}");
        let prefix = command
            .iter()
            .position(|w| w == "--prefix")
            .expect("--prefix");
        let directory = root().join(&command[prefix + 1]);
        let manifest = read_json(&directory.join("package.json"));
        let lock = read_json(&directory.join("package-lock.json"));
        let dependencies = manifest["dependencies"].as_object().expect("dependencies");
        assert_eq!(dependencies.len(), 1, "{name}");
        assert_eq!(
            &manifest["dependencies"], &lock["packages"][""]["dependencies"],
            "{name}"
        );
        for (package, version) in dependencies {
            let version = version.as_str().expect("a version string");
            assert!(exact.is_match(version), "{name}: {package} {version}");
            let locked = &lock["packages"][format!("node_modules/{package}")]["version"];
            assert_eq!(locked.as_str(), Some(version), "{name}: {package}");
        }
        for (path, package) in lock["packages"].as_object().unwrap() {
            if !path.is_empty() {
                let integrity = package.get("integrity").expect("an integrity field");
                assert!(truthy(integrity), "{name}: {path}");
            }
        }
    }
}

#[test]
fn dependency_lifecycle_scripts_are_disabled() {
    for (name, command) in install_commands() {
        assert!(command.iter().any(|w| w == "--ignore-scripts"), "{name}");
    }
}

#[test]
fn pi_transport_excludes_the_known_vulnerable_range() {
    // GHSA-4qv8-2p9j-2685, GHSA-f72v-6w45-q472 and GHSA-xjj6-f33q-929c.
    for package in pi_packages("undici") {
        let version = version_tuple(&package);
        assert!(
            !(vec![8, 0, 0] <= version && version < vec![8, 12, 2]),
            "{}",
            package["version"]
        );
    }
}

#[test]
fn pi_glob_matcher_excludes_the_known_vulnerable_range() {
    // GHSA-g3hv-75wg-m249: minimatch before 11.0.1.
    for package in pi_packages("minimatch") {
        assert!(
            version_tuple(&package) >= vec![11, 0, 1],
            "{}",
            package["version"]
        );
    }
}

#[test]
fn load_checks_do_not_retain_checkout_credentials() {
    for name in WORKFLOWS {
        let text = read(&root().join(".github/workflows").join(name));
        assert!(text.contains("permissions:\n  contents: read\n"), "{name}");
        let checkouts: Vec<&str> = text.split("- uses: actions/checkout@").skip(1).collect();
        assert!(!checkouts.is_empty(), "{name}");
        for checkout in checkouts {
            let step = checkout.split("\n      - ").next().unwrap();
            assert!(
                step.contains("persist-credentials: false"),
                "{name}: {step}"
            );
        }
    }
}

#[test]
fn workflow_actions_are_pinned_to_full_commit_shas() {
    let uses = re(r"(?m)^\s*(?:- )?uses:\s*(.+?)\s*$");
    let pinned = re(r"^[\w./-]+@[0-9a-f]{40} # v\d");
    for (name, text) in workflow_files() {
        for captures in uses.captures_iter(&text) {
            let target = &captures[1];
            // A local action, such as `./` for this repository's action.yml, is pinned by the
            // commit the workflow runs on.
            if target.starts_with("./") {
                continue;
            }
            assert!(pinned.is_match(target), "{name}: {target}");
        }
    }
}

#[test]
fn dependabot_covers_actions_and_every_ci_lockfile() {
    // There is no YAML parser here, so match each line exactly, block by block.
    let config = read(&root().join(".github/dependabot.yml"));
    assert_matches(r"(?m)^version: 2$", &config, "dependabot.yml");
    assert_matches(r"(?m)^updates:$", &config, "dependabot.yml");
    let blocks = dependabot_blocks(&config);
    let ecosystems: Vec<&str> = blocks.keys().map(String::as_str).collect();
    assert_eq!(ecosystems, ["cargo", "github-actions", "npm"]);
    for (ecosystem, block) in &blocks {
        assert_matches(
            r"(?m)^    schedule:\n      interval: \w+$",
            block,
            ecosystem,
        );
    }
    assert_matches(
        r"(?m)^    directory: /$",
        &blocks["github-actions"],
        "github-actions",
    );
    let mut lockfiles: Vec<String> = fs::read_dir(root().join("ci"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.join("package-lock.json").is_file())
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    lockfiles.sort();
    assert!(!lockfiles.is_empty());
    for directory in lockfiles {
        let pattern = format!(r"(?m)^      - /ci/{}$", regex::escape(&directory));
        assert_matches(&pattern, &blocks["npm"], &directory);
    }
    // Every Cargo lockfile that lists a registry package; laconic-check's lists none.
    let mut crates = Vec::new();
    for entry in fs::read_dir(root().join("tools")).unwrap() {
        let dir = entry.unwrap().path();
        for lock in [dir.join("Cargo.lock"), dir.join("fuzz").join("Cargo.lock")] {
            if fs::read_to_string(&lock).is_ok_and(|text| text.contains("\nsource = \"registry+")) {
                let crate_dir = lock.parent().unwrap().strip_prefix(root()).unwrap();
                crates.push(crate_dir.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    assert!(!crates.is_empty());
    for directory in crates {
        let pattern = format!(r"(?m)^      - /{}$", regex::escape(&directory));
        assert_matches(&pattern, &blocks["cargo"], &directory);
    }
}

#[test]
fn no_workflow_installs_with_pip() {
    // No workflow needs a Python package, so any line naming pip, however it is spelt,
    // chained, continued or folded, fails until it is reviewed: an install must then be
    // pinned by hash and covered by Dependabot.
    let workflows = workflow_files();
    assert!(workflows.iter().any(|(name, _)| name == "tests.yml"));
    for (name, text) in workflows {
        assert_eq!(pip_lines(&text), Vec::<&str>::new(), "{name}");
    }
    for text in [
        "run: pip3.11 install foo",
        "run: python -m pip --no-input install foo",
        "run: >\n  python -m pip\n  install foo",
        "run: uv pip install foo",
        "run: pipx install foo",
    ] {
        assert!(!pip_lines(text).is_empty(), "{text}");
    }
}
