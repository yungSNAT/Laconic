// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The OpenCode server plugin, mirroring the hook tests: the always-on flag gates injection,
//! and frontmatter stripping matches the hooks. V2 registers through `ctx.skill.transform` and
//! `ctx.command.transform`; V1 mutates the config object in its `config` hook. Both paths are
//! covered through `tests/opencode_plugin_driver.mjs`. Replaces `tests/test_opencode_plugin.py`.

use super::{Output, copy_tree, require, resolved, root, run, temp_dir, touch, write_text};
use crate::pi_check::TempDir;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Exercise both runtime APIs against the same fixtures.
const INJECTION_MODES: [&str; 2] = ["context", "v1-context"];
/// The two malformed command files: an unparsable header, and one never closed.
const MALFORMED: [&str; 2] = [
    "---\n{broken}\n---\nBody",
    "---\n{\"description\":\"unclosed\"}\nBody",
];

struct Fixture {
    _temp: TempDir,
    plugin_root: PathBuf,
    config_dir: PathBuf,
    node: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let node = require("node");
        let temp = temp_dir();
        let plugin_root = resolved(&temp.0).join("plugin");
        copy_tree(&root().join(".opencode"), &plugin_root.join(".opencode"));
        copy_tree(&root().join("skills"), &plugin_root.join("skills"));
        // The plugin reads its flag from $XDG_CONFIG_HOME/opencode/.laconic-always.
        let config_dir = temp.0.join("config");
        fs::create_dir_all(config_dir.join("opencode")).unwrap();
        Self {
            _temp: temp,
            plugin_root,
            config_dir,
            node,
        }
    }

    fn run_plugin(&self, mode: &str, config: Option<&Value>) -> Output {
        let mut command = Command::new(&self.node);
        command
            .arg(root().join("tests/opencode_plugin_driver.mjs"))
            .arg(self.plugin_root.join(".opencode/plugins/laconic.mjs"))
            .arg(mode)
            .env("XDG_CONFIG_HOME", &self.config_dir);
        if let Some(config) = config {
            command.arg(config.to_string());
        }
        run(&mut command, None, None)
    }

    /// The JSON the driver prints for `mode`, after checking it exited cleanly.
    fn json(&self, mode: &str, config: Option<&Value>) -> Value {
        let result = self.run_plugin(mode, config);
        result.assert_success();
        serde_json::from_str(&result.out())
            .unwrap_or_else(|e| panic!("{mode} printed no JSON ({e}): {}", result.out()))
    }

    fn opt_in(&self) {
        touch(&self.config_dir.join("opencode/.laconic-always"));
    }

    fn write_skill(&self, text: &str) {
        write_text(&self.plugin_root.join("skills/laconic/SKILL.md"), text);
    }

    fn command_file(&self) -> PathBuf {
        self.plugin_root.join(".opencode/command/laconic.md")
    }

    fn skills_path(&self) -> String {
        self.plugin_root.join("skills").display().to_string()
    }

    fn skill_path(&self, id: &str) -> String {
        self.plugin_root
            .join("skills")
            .join(id)
            .join("SKILL.md")
            .display()
            .to_string()
    }

    /// The `laconic` skill still registers after a broken command file.
    fn assert_skill_registered(&self) {
        assert_eq!(self.json("skill", None)[0]["id"], "laconic");
    }
}

/// The V1 config's `command` map, which must exist as in Python's `config["command"]`.
fn commands(config: &Value) -> &serde_json::Map<String, Value> {
    config["command"]
        .as_object()
        .unwrap_or_else(|| panic!("config has no command map: {config}"))
}

fn text(value: &Value) -> &str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{value} is not a string"))
}

// --- always-on injection, asserted through both APIs ---

#[test]
fn silent_without_opt_in_flag() {
    let fixture = Fixture::new();
    for mode in INJECTION_MODES {
        let result = fixture.run_plugin(mode, None);
        result.assert_success();
        assert_eq!(result.out(), "", "{mode}");
    }
}

#[test]
fn injects_ruleset_when_opted_in() {
    let fixture = Fixture::new();
    fixture.opt_in();
    for mode in INJECTION_MODES {
        let result = fixture.run_plugin(mode, None);
        result.assert_success();
        assert!(
            result.out().contains("LACONIC MODE ACTIVE (always-on)"),
            "{mode}"
        );
        assert!(
            result.out().contains("Answer first, then justify"),
            "{mode}"
        );
    }
}

#[test]
fn strips_frontmatter_with_trailing_whitespace() {
    let fixture = Fixture::new();
    fixture.write_skill("---   \nname: fixture\n--- \t\nFixture body.\n");
    fixture.opt_in();
    for mode in INJECTION_MODES {
        let result = fixture.run_plugin(mode, None);
        result.assert_success();
        assert!(!result.out().contains("name: fixture"), "{mode}");
        assert!(result.out().contains("\n\nFixture body."), "{mode}");
    }
}

#[test]
fn keeps_content_when_frontmatter_is_unclosed() {
    let fixture = Fixture::new();
    fixture.write_skill("---\nname: fixture\nFixture body, fence never closed.\n");
    fixture.opt_in();
    for mode in INJECTION_MODES {
        let result = fixture.run_plugin(mode, None);
        result.assert_success();
        assert!(
            result.out().contains("Fixture body, fence never closed."),
            "{mode}"
        );
    }
}

#[test]
fn both_apis_share_the_same_injected_text() {
    let fixture = Fixture::new();
    fixture.opt_in();
    let v2 = fixture.run_plugin("context", None).out();
    let v1 = fixture.run_plugin("v1-context", None).out();
    assert_eq!(v2, v1);
}

// --- V2 registration ---

#[test]
fn setup_registers_the_slash_command() {
    // Regression test for #140: a global install (plugin loaded from a path outside any
    // checkout, no project-scope .opencode/command/ directory in play) must still get
    // /laconic, because OpenCode's skill-sourced commands are not surfaced in the TUI's `/`
    // menu.
    let fixture = Fixture::new();
    let command = &fixture.json("command", None)[0];
    assert_eq!(command["name"], "laconic");
    assert!(text(&command["description"]).contains("Australian English"));
    assert!(text(&command["template"]).contains("stop laconic"));
}

#[test]
fn setup_registers_the_skill() {
    let fixture = Fixture::new();
    let skill = &fixture.json("skill", None)[0];
    assert_eq!(skill["id"], "laconic");
    assert_eq!(text(&skill["path"]), fixture.skill_path("laconic"));
    assert!(text(&skill["description"]).contains("Australian English"));
    assert!(!text(&skill["content"]).contains("---"));
}

#[test]
fn setup_registers_the_prose_skill_with_its_routing_limits() {
    let fixture = Fixture::new();
    let skills = fixture.json("skill", None);
    let prose = skills
        .as_array()
        .unwrap()
        .iter()
        .rfind(|skill| skill["id"] == "laconic-prose")
        .expect("laconic-prose is registered");
    assert_eq!(text(&prose["path"]), fixture.skill_path("laconic-prose"));
    assert!(
        text(&prose["description"]).contains("small edits need only the compact Laconic rules")
    );
    assert!(!text(&prose["content"]).contains("---"));
}

#[test]
fn missing_command_keeps_skill_registration() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.command_file()).unwrap();
    assert_eq!(fixture.json("command", None), json!([]));
    fixture.assert_skill_registered();
}

#[test]
fn malformed_command_keeps_skill_registration() {
    let fixture = Fixture::new();
    for malformed in MALFORMED {
        write_text(&fixture.command_file(), malformed);
        assert_eq!(fixture.json("command", None), json!([]), "{malformed}");
        fixture.assert_skill_registered();
    }
}

// --- V1 config registration ---

#[test]
fn v1_config_hook_registers_the_slash_command() {
    let fixture = Fixture::new();
    let config = fixture.json("v1-config", None);
    let command = &config["command"]["laconic"];
    assert!(text(&command["description"]).contains("Australian English"));
    assert!(text(&command["template"]).contains("stop laconic"));
}

#[test]
fn v1_config_hook_registers_the_skills_path() {
    let fixture = Fixture::new();
    let config = fixture.json("v1-config", None);
    let paths = config["skills"]["paths"].as_array().unwrap();
    assert!(paths.contains(&json!(fixture.skills_path())), "{paths:?}");
}

#[test]
fn v1_config_preserves_existing_command() {
    let fixture = Fixture::new();
    let custom = json!({"description": "User command", "template": "Keep this", "agent": "plan"});
    let config = fixture.json("v1-config", Some(&json!({"command": {"laconic": custom}})));
    assert_eq!(config["command"]["laconic"], custom);
}

#[test]
fn v1_repeated_config_does_not_duplicate_skill_paths() {
    let fixture = Fixture::new();
    let config = fixture.json("v1-config", None);
    assert_eq!(config["skills"]["paths"], json!([fixture.skills_path()]));
}

#[test]
fn v1_command_preserves_metadata_and_trims_template() {
    let fixture = Fixture::new();
    // Python's json.dumps of the metadata, byte for byte.
    let metadata = r#"{"description": "ADHD: \"focus\"\nnext line", "agent": "plan", "model": "fixture/model", "subagent": true}"#;
    fs::write(
        fixture.command_file(),
        format!("---  \r\n{metadata}\r\n--- \t\r\n\r\nUse the skill.\r\n\r\n"),
    )
    .unwrap();
    let mut expected: Value = serde_json::from_str(metadata).unwrap();
    expected["template"] = json!("Use the skill.");
    let config = fixture.json("v1-config", None);
    assert_eq!(config["command"]["laconic"], expected);
}

#[test]
fn v1_missing_command_keeps_skill_discovery() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.command_file()).unwrap();
    let config = fixture.json("v1-config", None);
    assert!(!commands(&config).contains_key("laconic"));
    assert_eq!(config["skills"]["paths"], json!([fixture.skills_path()]));
}

#[test]
fn v1_malformed_command_does_not_leak_frontmatter_into_prompt() {
    let fixture = Fixture::new();
    for malformed in MALFORMED {
        write_text(&fixture.command_file(), malformed);
        let config = fixture.json("v1-config", None);
        assert!(!commands(&config).contains_key("laconic"), "{malformed}");
        assert_eq!(config["skills"]["paths"], json!([fixture.skills_path()]));
    }
}

#[test]
fn js_mirror_reexports_the_implementation() {
    // Discovery uses .js; re-exporting must preserve the module identity.
    let fixture = Fixture::new();
    let mirror = root().join(".opencode/plugins/laconic.js");
    let implementation = root().join(".opencode/plugins/laconic.mjs");
    assert!(mirror.is_file(), "missing V2 auto-discovery mirror");
    assert!(implementation.is_file());
    // Node turns the paths into file URLs, as Python's Path.as_uri did.
    let script = "import { pathToFileURL } from 'node:url';\
        const load = async (path) => (await import(pathToFileURL(path).href)).default;\
        const a = await load(process.argv[1]);\
        const b = await load(process.argv[2]);\
        process.stdout.write(JSON.stringify({\
        same: a === b,\
        id: a.id, keys: Object.keys(a).sort()}));";
    let result = run(
        Command::new(&fixture.node)
            .args(["--input-type=module", "-e", script])
            .arg(&mirror)
            .arg(&implementation)
            .env("XDG_CONFIG_HOME", &fixture.config_dir),
        None,
        None,
    );
    result.assert_success();
    let export: Value = serde_json::from_str(&result.out()).unwrap();
    assert_eq!(
        export["same"], true,
        "mirror does not re-export the implementation"
    );
    assert_eq!(export["id"], "laconic");
    assert_eq!(export["keys"], json!(["id", "server", "setup"]));
}
