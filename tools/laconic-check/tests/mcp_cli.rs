// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The MCP servers over stdio: `laconic-check mcp` and the deprecated Node `mcp/server.mjs`,
//! which runs the binary once a call. The Node server is tested only where `node` runs.

use std::io::Write;
use std::process::{Command, Stdio};

use laconic_check::mcp::{Json, parse};

const BINARY: &str = env!("CARGO_BIN_EXE_laconic-check");

/// Each server's command: the binary, then the Node server when `node` runs.
fn servers() -> Vec<Command> {
    let mut binary = Command::new(BINARY);
    binary.arg("mcp");
    let mut servers = vec![binary];
    let node = Command::new("node").arg("--version").output();
    if node.is_ok_and(|out| out.status.success()) {
        let mut node = Command::new("node");
        node.arg(concat!(env!("CARGO_MANIFEST_DIR"), "/mcp/server.mjs"));
        servers.push(node);
    } else {
        eprintln!("Node MCP server not tested: `node --version` did not run");
    }
    servers
}

/// Sends the requests, one a line, to each server and returns each server's replies in order.
fn exchange(requests: &[&str]) -> Vec<Vec<Json>> {
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    servers()
        .into_iter()
        .map(|mut server| {
            let mut child = server
                .env("LACONIC_CHECK", BINARY)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            let out = child.wait_with_output().unwrap();
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(out.status.success(), "{server:?}: {stderr}");
            let stdout = String::from_utf8(out.stdout).unwrap();
            stdout
                .lines()
                .map(|line| parse(line).unwrap_or_else(|| panic!("not JSON: {line}")))
                .collect()
        })
        .collect()
}

fn member<'a>(value: &'a Json, key: &str) -> Option<&'a Json> {
    match value {
        Json::Object(members) => members.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn at<'a>(value: &'a Json, path: &[&str]) -> &'a Json {
    path.iter().fold(value, |value, key| {
        member(value, key).unwrap_or_else(|| panic!("no '{key}' in {value:?}"))
    })
}

fn string(value: &Json) -> &str {
    match value {
        Json::String(s) => s,
        other => panic!("not a string: {other:?}"),
    }
}

fn items(value: &Json) -> &[Json] {
    match value {
        Json::Array(items) => items,
        other => panic!("not an array: {other:?}"),
    }
}

/// Each server's result for one `tools/call`, which must not be a JSON-RPC error.
fn call(name: &str, arguments: &str) -> Vec<Json> {
    let request = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {{"name": "{name}", "arguments": {arguments}}}}}"#
    );
    exchange(&[&request])
        .into_iter()
        .map(|replies| {
            let [reply] = <[Json; 1]>::try_from(replies).unwrap();
            assert!(member(&reply, "error").is_none(), "{reply:?}");
            at(&reply, &["result"]).clone()
        })
        .collect()
}

/// The text of a result's first content item.
fn text(result: &Json) -> &str {
    string(at(&items(at(result, &["content"]))[0], &["text"]))
}

#[test]
fn initialise_then_list_tools() {
    for replies in exchange(&[
        r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}"#,
        r#"{"jsonrpc": "2.0", "method": "notifications/initialized"}"#,
        r#"{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}"#,
    ]) {
        let ids: Vec<&Json> = replies.iter().map(|reply| at(reply, &["id"])).collect();
        let want = [Json::Number("1".into()), Json::Number("2".into())];
        assert_eq!(
            ids,
            want.iter().collect::<Vec<_>>(),
            "a notification must get no reply"
        );
        let init = at(&replies[0], &["result"]);
        assert_eq!(string(at(init, &["serverInfo", "name"])), "laconic-check");
        assert_eq!(string(at(init, &["protocolVersion"])), "2025-06-18");
        let tools = items(at(&replies[1], &["result", "tools"]));
        let required = |name: &str| -> Vec<&str> {
            let tool = tools
                .iter()
                .find(|tool| string(at(tool, &["name"])) == name)
                .unwrap_or_else(|| panic!("no tool {name}"));
            items(at(tool, &["inputSchema", "required"]))
                .iter()
                .map(string)
                .collect()
        };
        assert_eq!(tools.len(), 2, "{tools:?}");
        assert_eq!(required("lint"), ["text"]);
        assert_eq!(required("preserve"), ["original", "edited"]);
    }
}

/// MCP 2026-07-28: no handshake; each request names its version in `_meta`.
#[test]
fn stateless_discover_list_and_call() {
    let meta = r#""_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientCapabilities": {}}"#;
    let discover = format!(
        r#"{{"jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": {{{meta}}}}}"#
    );
    let list =
        format!(r#"{{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {{{meta}}}}}"#);
    let call = format!(
        r#"{{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {{{meta}, "name": "lint", "arguments": {{"text": "It worked.\n"}}}}}}"#
    );
    let unsupported = r#"{"jsonrpc": "2.0", "id": 4, "method": "tools/list", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "1900-01-01", "io.modelcontextprotocol/clientCapabilities": {}}}}"#;
    let no_capabilities = r#"{"jsonrpc": "2.0", "id": 5, "method": "tools/list", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28"}}}"#;
    for replies in exchange(&[&discover, &list, &call, unsupported, no_capabilities]) {
        let [discovered, listed, called, unsupported, no_capabilities] =
            <[Json; 5]>::try_from(replies).unwrap();
        let discovered = at(&discovered, &["result"]);
        assert_eq!(string(at(discovered, &["resultType"])), "complete");
        let versions: Vec<&str> = items(at(discovered, &["supportedVersions"]))
            .iter()
            .map(string)
            .collect();
        assert_eq!(versions, ["2026-07-28"]);
        assert_eq!(
            string(at(
                discovered,
                &["_meta", "io.modelcontextprotocol/serverInfo", "name"]
            )),
            "laconic-check"
        );
        assert_eq!(string(at(discovered, &["cacheScope"])), "public");
        let listed = at(&listed, &["result"]);
        assert_eq!(string(at(listed, &["resultType"])), "complete");
        assert_eq!(items(at(listed, &["tools"])).len(), 2);
        assert!(matches!(at(listed, &["ttlMs"]), Json::Number(_)));
        let called = at(&called, &["result"]);
        assert_eq!(string(at(called, &["resultType"])), "complete");
        assert_eq!(text(called), "No findings.");
        assert_eq!(
            at(&unsupported, &["error", "code"]),
            &Json::Number("-32022".into())
        );
        assert_eq!(
            string(at(&unsupported, &["error", "data", "requested"])),
            "1900-01-01"
        );
        assert_eq!(
            at(&no_capabilities, &["error", "code"]),
            &Json::Number("-32602".into())
        );
    }
}

#[test]
fn lint_reports_a_finding_by_line() {
    for result in call(
        "lint",
        r#"{"text": "Clean line.\n\nIt worked — mostly.\n"}"#,
    ) {
        assert!(member(&result, "isError").is_none(), "{result:?}");
        assert!(
            text(&result)
                .lines()
                .any(|line| line.starts_with("line 3:")),
            "{result:?}"
        );
    }
}

#[test]
fn lint_reports_clean_text() {
    for result in call("lint", r#"{"text": "It worked.\n"}"#) {
        assert_eq!(text(&result), "No findings.");
    }
}

#[test]
fn preserve_reports_a_dropped_number() {
    let arguments = r#"{"original": "Keep 42 items.\n", "edited": "Keep items.\n"}"#;
    for result in call("preserve", arguments) {
        assert!(member(&result, "isError").is_none(), "{result:?}");
        assert!(text(&result).contains("42"), "{result:?}");
    }
}

#[test]
fn tool_errors_are_reported_in_the_result() {
    for result in call("lint", r#"{"text": 1}"#) {
        assert_eq!(member(&result, "isError"), Some(&Json::Bool(true)));
    }
}
