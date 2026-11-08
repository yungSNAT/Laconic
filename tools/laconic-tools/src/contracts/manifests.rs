// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Every plugin manifest declares one version and CHANGELOG.md names it. Replaces
//! `tests/test_manifest_versions.py`.

use super::{re, read, read_json, root};
use serde_json::Value;
use std::collections::BTreeSet;

const VERSIONED: [&str; 6] = [
    ".claude-plugin/plugin.json",
    ".codex-plugin/plugin.json",
    "gemini-extension.json",
    "qwen-extension.json",
    "kimi.plugin.json",
    "package.json",
];
/// These declare no version today; if one ever does, it must agree too.
const OPTIONAL: [&str; 3] = [
    ".claude-plugin/marketplace.json",
    ".agents/plugins/marketplace.json",
    "plugin.json",
];

/// Each declared version by manifest (and plugin entry), in declaration order.
fn declared_versions() -> Vec<(String, Value)> {
    let mut versions = vec![];
    for name in VERSIONED.iter().chain(&OPTIONAL) {
        let manifest = read_json(&root().join(name));
        if let Some(version) = manifest.get("version") {
            versions.push(((*name).to_owned(), version.clone()));
        }
        for plugin in manifest
            .get("plugins")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(version) = plugin.get("version") {
                versions.push((
                    format!("{name} ({})", shown(&plugin["name"])),
                    version.clone(),
                ));
            }
        }
    }
    versions
}

/// A value as Python's f-string shows it: a string as itself, anything else as JSON.
fn shown(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

#[test]
fn declared_versions_agree() {
    let versions = declared_versions();
    let distinct: BTreeSet<String> = versions.iter().map(|(_, v)| v.to_string()).collect();
    assert_eq!(distinct.len(), 1, "{versions:?}");
}

#[test]
fn versioned_manifests_declare_a_version() {
    let versions = declared_versions();
    for name in VERSIONED {
        assert!(versions.iter().any(|(n, _)| n == name), "{name}");
    }
}

#[test]
fn changelog_names_the_current_versions() {
    let changelog = read(&root().join("CHANGELOG.md"));
    let versions = declared_versions();
    let plugin = shown(&versions.first().expect("a declared version").1);
    assert!(
        changelog.contains(&format!("## [plugin {plugin}] - ")),
        "{plugin}"
    );
    let cargo = read(&root().join("tools/laconic-check/Cargo.toml"));
    let checker = &re(r#"(?m)^version = "([^"]+)""#)
        .captures(&cargo)
        .expect("laconic-check declares a version")[1];
    assert!(
        changelog.contains(&format!("## [laconic-check {checker}] - ")),
        "{checker}"
    );
}
