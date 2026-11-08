// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The Pi and OMP package entries. Replaces `tests/test_omp_package.py`.

use super::{read, read_json, root};
use serde_json::json;

#[test]
fn omp_and_pi_extension_manifests_are_declared() {
    let package = read_json(&root().join("package.json"));
    let extension = json!(["./extensions/laconic.ts"]);
    assert_eq!(package["omp"], json!({"extensions": extension}));
    assert_eq!(
        package["pi"],
        json!({"extensions": extension, "skills": ["./skills"]})
    );
}

#[test]
fn skill_remains_explicitly_invoked() {
    let skill = read(&root().join("skills/laconic/SKILL.md"));
    let frontmatter = skill
        .split("---\n")
        .nth(1)
        .expect("SKILL.md has frontmatter");
    assert!(frontmatter.contains("name: laconic"));
    assert!(frontmatter.contains("disable-model-invocation: true"));
}
