// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The always-on hooks in `hooks/`: each runtime stays silent without the opt-in flag, strips
//! frontmatter the same way, and the shared Claude and Codex launcher runs the Node hook.
//! Replaces `tests/test_always_on_hooks.py`.

use super::{Output, copy_tree, re, read_json, require, root, run, temp_dir, touch, write_text};
use crate::pi_check::{TempDir, which};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

struct Fixture {
    temp: TempDir,
    plugin_root: PathBuf,
    config_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = temp_dir();
        let plugin_root = temp.0.join("plugin with spaces");
        copy_tree(&root().join("hooks"), &plugin_root.join("hooks"));
        copy_tree(&root().join("skills"), &plugin_root.join("skills"));
        let config_dir = temp.0.join("claude config");
        fs::create_dir(&config_dir).unwrap();
        Self {
            temp,
            plugin_root,
            config_dir,
        }
    }

    fn skill(&self) -> PathBuf {
        self.plugin_root.join("skills/laconic/SKILL.md")
    }

    fn opt_in(&self) {
        touch(&self.config_dir.join(".laconic-always"));
    }

    /// Every hook runtime on PATH, by name, with the command that runs its hook.
    fn runtimes(&self) -> Vec<(&'static str, Vec<PathBuf>)> {
        let hooks = self.plugin_root.join("hooks");
        let mut runtimes = vec![];
        if let Some(node) = which("node") {
            runtimes.push(("node", vec![node, hooks.join("always-on.mjs")]));
        }
        if let Some(sh) = which("sh") {
            runtimes.push(("sh", vec![sh, hooks.join("always-on.sh")]));
        }
        if let Some(powershell) = which("pwsh").or_else(|| which("powershell")) {
            let mut command = vec![powershell];
            command
                .extend(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"].map(PathBuf::from));
            command.push(hooks.join("always-on.ps1"));
            runtimes.push(("powershell", command));
        }
        runtimes
    }

    fn run_hook(&self, command: &[PathBuf], timeout: Option<Duration>) -> Output {
        run(
            Command::new(&command[0])
                .args(&command[1..])
                .env("CLAUDE_CONFIG_DIR", &self.config_dir),
            None,
            timeout,
        )
    }

    /// Runs the SessionStart command from `hooks/hooks.json` through the shell, as Codex and
    /// Claude Code do, with the plugin root exported unless `export_root` is false.
    fn run_codex_hook(&self, plugin_root: Option<&Path>, export_root: bool) -> Output {
        let command = session_start_hook()["command"]
            .as_str()
            .expect("the hook command is a string")
            .to_owned();
        let mut shell = shell(&command);
        shell
            .env("CLAUDE_CONFIG_DIR", &self.config_dir)
            .env_remove("CLAUDE_PLUGIN_ROOT")
            .env_remove("PLUGIN_ROOT");
        if export_root {
            let plugin_root = plugin_root.unwrap_or(&self.plugin_root);
            shell
                .env("CLAUDE_PLUGIN_ROOT", plugin_root)
                .env("PLUGIN_ROOT", plugin_root);
        }
        let input = json!({
            "session_id": "test-session",
            "cwd": self.plugin_root.display().to_string(),
            "hook_event_name": "SessionStart",
            "source": "startup",
        });
        run(&mut shell, Some(&input.to_string()), None)
    }

    /// Asserts that every runtime succeeds silently on standard error and prints the same
    /// text, once separators and line endings are unified; returns that text by runtime.
    fn run_all(&self) -> BTreeMap<&'static str, String> {
        let mut outputs = BTreeMap::new();
        for (name, command) in self.runtimes() {
            let result = self.run_hook(&command, None);
            assert_eq!(result.code, Some(0), "{name}: {}", result.err());
            assert_eq!(result.err(), "", "{name}");
            outputs.insert(name, normalize(&result.out()));
        }
        outputs
    }
}

/// Python's `subprocess.run(command, shell=True)`.
fn shell(command: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Python runs `%ComSpec% /c "<command>"`, falling back to cmd.exe under SystemRoot,
        // and passes the command unquoted beyond that.
        let comspec = std::env::var_os("ComSpec")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("SystemRoot").map(|r| PathBuf::from(r).join("System32/cmd.exe"))
            })
            .expect("neither ComSpec nor SystemRoot is set");
        let mut shell = Command::new(comspec);
        shell.raw_arg(format!("/c \"{command}\""));
        shell
    }
    #[cfg(not(windows))]
    {
        let mut shell = Command::new("/bin/sh");
        shell.args(["-c", command]);
        shell
    }
}

fn session_start_hook() -> Value {
    read_json(&root().join("hooks/hooks.json"))["hooks"]["SessionStart"][0]["hooks"][0].clone()
}

/// The banner embeds the flag path. On Windows the sh runtime joins it with "/" while node and
/// PowerShell join with "\"; both name the same file, so unify separators (and newlines)
/// before comparing runtimes.
fn normalize(stdout: &str) -> String {
    stdout.replace("\r\n", "\n").replace('\\', "/")
}

fn assert_one_output(outputs: &BTreeMap<&str, String>) {
    let mut distinct: Vec<&String> = outputs.values().collect();
    distinct.dedup();
    assert_eq!(distinct.len(), 1, "runtimes disagree: {outputs:#?}");
}

#[test]
fn hook_is_silent_without_opt_in_flag() {
    let fixture = Fixture::new();
    let runtimes = fixture.runtimes();
    assert!(!runtimes.is_empty(), "no hook runtime is available");
    for (name, command) in runtimes {
        let result = fixture.run_hook(&command, None);
        assert_eq!(result.code, Some(0), "{name}");
        assert_eq!(result.out(), "", "{name}");
        assert_eq!(result.err(), "", "{name}");
    }
}

#[test]
fn runtimes_strip_frontmatter_with_trailing_whitespace() {
    let fixture = Fixture::new();
    write_text(
        &fixture.skill(),
        "---   \nname: fixture\n--- \t\nFixture body.\n",
    );
    fixture.opt_in();
    let outputs = fixture.run_all();
    for (name, output) in &outputs {
        assert!(!output.contains("name: fixture"), "{name}: {output}");
        assert!(output.contains("\n\nFixture body.\n"), "{name}: {output}");
    }
    assert_one_output(&outputs);
}

#[test]
fn node_preserves_internal_newlines_without_backtracking() {
    let fixture = Fixture::new();
    let node = require("node");
    let body = format!("Croft Manor.\n{}End.", "\n".repeat(200_000));
    fs::write(fixture.skill(), format!("{body}\r\n\n")).unwrap();
    fixture.opt_in();
    let result = fixture.run_hook(
        &[node, fixture.plugin_root.join("hooks/always-on.mjs")],
        Some(Duration::from_secs(5)),
    );
    result.assert_success();
    assert!(result.out().ends_with(&format!("\n\n{body}\n")));
}

#[test]
fn node_trims_only_complete_trailing_line_endings() {
    let fixture = Fixture::new();
    let node = require("node");
    fixture.opt_in();
    for (suffix, expected) in [
        ("\r\n\n", ""),
        ("\r\r\n", "\r"),
        ("\n\r", "\n\r"),
        (" \t", " \t"),
        ("\u{2028}", "\u{2028}"),
    ] {
        fs::write(fixture.skill(), format!("Body.{suffix}")).unwrap();
        let result = fixture.run_hook(
            &[
                node.clone(),
                fixture.plugin_root.join("hooks/always-on.mjs"),
            ],
            Some(Duration::from_secs(5)),
        );
        result.assert_success();
        // Compare bytes: text mode would turn a kept lone CR into LF.
        let tail = format!("Body.{expected}\n");
        assert!(
            result.stdout.ends_with(tail.as_bytes()),
            "suffix {suffix:?}: {:?}",
            result.out()
        );
    }
}

#[test]
fn runtimes_keep_content_when_frontmatter_is_unclosed() {
    // An opening --- with no closing delimiter is not frontmatter. Keeping the whole file
    // beats injecting a banner that promises "the ruleset below" followed by nothing.
    let fixture = Fixture::new();
    write_text(
        &fixture.skill(),
        "---\nname: fixture\nFixture body, fence never closed.\n",
    );
    fixture.opt_in();
    let outputs = fixture.run_all();
    for (name, output) in &outputs {
        assert!(
            output.contains("Fixture body, fence never closed."),
            "{name}: {output}"
        );
    }
    assert_one_output(&outputs);
}

#[test]
fn codex_command_runs_the_hook_instead_of_parsing_session_json() {
    let fixture = Fixture::new();
    fixture.opt_in();
    let result = fixture.run_codex_hook(None, true);
    result.assert_success();
    assert_eq!(result.err(), "");
    assert!(result.out().contains("LACONIC MODE ACTIVE (always-on)"));
}

#[test]
fn codex_command_is_silent_without_opt_in_flag() {
    let fixture = Fixture::new();
    let result = fixture.run_codex_hook(None, true);
    result.assert_success();
    assert_eq!(result.err(), "");
    assert_eq!(result.out(), "");
}

#[test]
fn codex_command_swallows_missing_plugin_errors() {
    let fixture = Fixture::new();
    let missing = fixture.plugin_root.join("missing plugin");
    let result = fixture.run_codex_hook(Some(&missing), true);
    result.assert_success();
    assert_eq!(result.err(), "");
    assert_eq!(result.out(), "");
}

#[test]
fn codex_command_keeps_shell_characters_in_an_exported_path() {
    let fixture = Fixture::new();
    fixture.opt_in();
    let hostile_root = fixture.temp.0.join("plugin $HOME %TEMP% `echo` & (test)");
    copy_tree(&fixture.plugin_root, &hostile_root);
    let result = fixture.run_codex_hook(Some(&hostile_root), true);
    result.assert_success();
    assert!(result.out().contains("LACONIC MODE ACTIVE"));
}

#[test]
fn codex_command_warns_when_opted_in_but_given_no_root() {
    let fixture = Fixture::new();
    fixture.opt_in();
    let result = fixture.run_codex_hook(None, false);
    result.assert_success();
    assert_eq!(result.out(), "");
    assert!(result.err().contains("CLAUDE_PLUGIN_ROOT"));
}

#[test]
fn codex_command_is_silent_without_root_or_opt_in_flag() {
    let fixture = Fixture::new();
    let result = fixture.run_codex_hook(None, false);
    result.assert_success();
    assert_eq!(result.out(), "");
    assert_eq!(result.err(), "");
}

#[test]
fn hook_uses_a_shared_claude_and_codex_launcher() {
    let hook = session_start_hook();
    assert!(hook.get("args").is_none());
    let command = hook["command"].as_str().unwrap();
    assert!(
        re(r#"^node(?: --input-type=module)? -e ""#).is_match(command),
        "{command}"
    );
    for wanted in [
        "process.env.CLAUDE_PLUGIN_ROOT",
        "process.env.PLUGIN_ROOT",
        "await import",
        ".catch",
    ] {
        assert!(command.contains(wanted), "missing {wanted}");
    }
    assert!(!command.contains("${CLAUDE_PLUGIN_ROOT}"));
}
