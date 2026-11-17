// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Where the deprecated Node `mcp/server.mjs` finds `laconic-check` when `LACONIC_CHECK` is not
//! set: only in PATH's absolute entries, never in its working directory (which Node on Windows
//! searches first) or through a relative entry such as `.`. Tested only where `node` runs.

use std::ffi::{OsStr, OsString};
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

/// PATH holding `dirs` in order.
fn path(dirs: &[&Path]) -> OsString {
    std::env::join_paths(dirs).unwrap()
}

/// The folder holding the `laconic-check` this test runs.
fn binary_dir() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_laconic-check"))
        .parent()
        .unwrap()
}

/// Runs the server in `dir` with `LACONIC_CHECK` unset and PATH set to `path`, sends one `lint`
/// call and returns its reply.
fn lint_reply(node: &Path, dir: &Path, path: &OsStr) -> String {
    let mut server = Command::new(node);
    server.env_remove("LACONIC_CHECK");
    reply(server, dir, path)
}

/// Runs the server with `node`, a Node command holding the rest of its environment, in `dir`
/// with PATH set to `path`; sends one `lint` call and returns its reply.
fn reply(mut node: Command, dir: &Path, path: &OsStr) -> String {
    let mut child = node
        .arg(SERVER)
        .current_dir(dir)
        .env("PATH", path)
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
    let reply = lint_reply(&node, &dir, &path(&[Path::new("."), binary_dir()]));
    assert!(reply.contains("line 3:"), "{reply}");
    assert!(!reply.contains("isError"), "{reply}");
}

#[test]
fn bare_name_found_only_through_a_relative_entry_is_an_error() {
    let Some(node) = node() else { return };
    let dir = planted("path-relative");
    let reply = lint_reply(&node, &dir, &path(&[Path::new(".")]));
    assert!(reply.contains("\"isError\":true"), "{reply}");
    assert!(reply.contains("LACONIC_CHECK"), "{reply}");
    assert!(!reply.contains("planted"), "{reply}");
}

#[cfg(unix)]
#[test]
fn a_candidate_that_cannot_run_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let Some(node) = node() else { return };
    let dir = planted("path-not-executable");
    let file = dir.join("laconic-check");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let reply = lint_reply(&node, &dir, &path(&[&dir, binary_dir()]));
    assert!(reply.contains("line 3:"), "{reply}");
}

/// `link/../bin` is the `bin` beside the folder `link` points to, as the system resolves it.
#[cfg(unix)]
#[test]
fn a_path_entry_resolves_dot_dot_after_its_links() {
    let Some(node) = node() else { return };
    let dir = planted("path-dot-dot");
    let real = dir.join("real");
    std::fs::create_dir_all(real.join("pkg")).unwrap();
    std::fs::create_dir_all(real.join("bin")).unwrap();
    let _ = std::fs::remove_file(real.join("bin/laconic-check"));
    std::os::unix::fs::symlink(
        env!("CARGO_BIN_EXE_laconic-check"),
        real.join("bin/laconic-check"),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::rename(dir.join("laconic-check"), dir.join("bin/laconic-check")).unwrap();
    let _ = std::fs::remove_file(dir.join("link"));
    std::os::unix::fs::symlink(real.join("pkg"), dir.join("link")).unwrap();
    let reply = lint_reply(&node, &dir, dir.join("link/../bin").as_os_str());
    assert!(reply.contains("line 3:"), "{reply}");
    assert!(!reply.contains("planted"), "{reply}");
}

/// `LACONIC_CHECK` is the user's choice and is not searched for: a bare name runs as the system
/// finds it.
#[cfg(unix)]
#[test]
fn laconic_check_is_used_as_given() {
    let Some(node) = node() else { return };
    let dir = planted("path-given");
    let mut server = Command::new(node);
    server.env("LACONIC_CHECK", "laconic-check");
    let reply = reply(server, &dir, OsStr::new("."));
    assert!(reply.contains("planted"), "{reply}");
}

/// `\folder` names a folder on the working directory's drive.
#[cfg(windows)]
#[test]
fn a_drive_relative_entry_is_not_searched() {
    let Some(node) = node() else { return };
    let dir = planted("path-drive-relative");
    let full = dir.to_str().unwrap();
    assert_eq!(full.as_bytes()[1], b':', "{full}");
    let reply = lint_reply(&node, &dir, OsStr::new(&full[2..]));
    assert!(reply.contains("\"isError\":true"), "{reply}");
    assert!(reply.contains("LACONIC_CHECK"), "{reply}");
}

#[cfg(windows)]
#[test]
fn a_quoted_entry_is_searched() {
    let Some(node) = node() else { return };
    let dir = planted("path-quoted");
    let quoted = format!("\"{}\"", binary_dir().display());
    let reply = lint_reply(&node, &dir, OsStr::new(&quoted));
    assert!(reply.contains("line 3:"), "{reply}");
}

/// libuv also takes an entry in single quotes.
#[cfg(windows)]
#[test]
fn a_single_quoted_entry_is_searched() {
    let Some(node) = node() else { return };
    let dir = planted("path-single-quoted");
    let quoted = format!("'{}'", binary_dir().display());
    let reply = lint_reply(&node, &dir, OsStr::new(&quoted));
    assert!(reply.contains("line 3:"), "{reply}");
}
