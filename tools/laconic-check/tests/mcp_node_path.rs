// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Where the deprecated Node `mcp/server.mjs` finds `laconic-check` when `LACONIC_CHECK` is not
//! set: only in PATH's absolute entries, never in its working directory (which Node on Windows
//! searches first) or through a relative entry such as `.`. Tested only where `node` runs.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SERVER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/mcp/server.mjs");
const LINT: &str = r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "lint", "arguments": {"text": "Clean line.\n\nIt worked — mostly.\n"}}}"#;

/// Node's absolute path, or `None` where `node` does not run.
fn node() -> Option<PathBuf> {
    let out = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .ok()
        .filter(|out| out.status.success());
    if out.is_none() {
        eprintln!("Node MCP server's PATH search not tested: `node` did not run");
    }
    out.map(|out| PathBuf::from(String::from_utf8(out.stdout).unwrap().trim()))
}

/// A fresh directory holding a planted `laconic-check`: a script that prints 'planted' on Unix
/// and a file that cannot run on Windows, so a server that runs it does not lint.
fn planted(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("laconic-check-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("laconic-check{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&file, "#!/bin/sh\necho planted\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    dir
}

/// Runs the server in `dir` with `LACONIC_CHECK` unset and PATH set to `path`, sends one `lint`
/// call and returns its reply.
fn lint_reply(node: &Path, dir: &Path, path: &[&Path]) -> String {
    let mut child = Command::new(node)
        .arg(SERVER)
        .current_dir(dir)
        .env_remove("LACONIC_CHECK")
        .env("PATH", std::env::join_paths(path).unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(format!("{LINT}\n").as_bytes()).unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn bare_name_runs_the_binary_on_an_absolute_path_entry() {
    let Some(node) = node() else { return };
    let dir = planted("path-search");
    let binary = Path::new(env!("CARGO_BIN_EXE_laconic-check"));
    let reply = lint_reply(&node, &dir, &[Path::new("."), binary.parent().unwrap()]);
    assert!(reply.contains("line 3:"), "{reply}");
    assert!(!reply.contains("isError"), "{reply}");
}

#[test]
fn bare_name_found_only_through_a_relative_entry_is_an_error() {
    let Some(node) = node() else { return };
    let dir = planted("path-relative");
    let reply = lint_reply(&node, &dir, &[Path::new(".")]);
    assert!(reply.contains("\"isError\":true"), "{reply}");
    assert!(reply.contains("LACONIC_CHECK"), "{reply}");
    assert!(!reply.contains("planted"), "{reply}");
}
