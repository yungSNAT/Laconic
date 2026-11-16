// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
const SHUTDOWN: &str = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
const EXIT: &str = r#"{"jsonrpc":"2.0","method":"exit"}"#;

fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

fn frames(mut output: &[u8]) -> Vec<Json> {
    let mut replies = Vec::new();
    while let Some(length) = header(&mut output).unwrap() {
        let (body, rest) = output.split_at(length);
        replies.push(mcp::parse(std::str::from_utf8(body).unwrap()).unwrap());
        output = rest;
    }
    replies
}

/// The replies to `messages`, and whether `serve` ended as after `shutdown` and `exit`.
fn session(messages: &[&str]) -> (Vec<Json>, bool) {
    let input: String = messages.iter().map(|m| frame(m)).collect();
    let mut output = Vec::new();
    let clean = serve(input.as_bytes(), &mut output).unwrap();
    (frames(&output), clean)
}

/// A member by a dotted path; a number indexes an array.
fn at<'a>(json: &'a Json, path: &str) -> &'a Json {
    path.split('.')
        .fold(json, |value, key| match (value, key.parse::<usize>()) {
            (Json::Array(items), Ok(index)) => &items[index],
            _ => value
                .get(key)
                .unwrap_or_else(|| panic!("no {path} in {}", mcp::to_string(json))),
        })
}

fn notice(method: &str, params: Vec<(&str, Json)>) -> String {
    notify(method, object(params))
}

fn open(uri: &str, language: &str, body: &str) -> String {
    let document = object(vec![
        ("uri", text(uri)),
        ("languageId", text(language)),
        ("version", number(1)),
        ("text", text(body)),
    ]);
    notice("textDocument/didOpen", vec![("textDocument", document)])
}

fn change(uri: &str, body: &str) -> String {
    let changes = Json::Array(vec![object(vec![("text", text(body))])]);
    let document = object(vec![("uri", text(uri)), ("version", number(2))]);
    notice(
        "textDocument/didChange",
        vec![("textDocument", document), ("contentChanges", changes)],
    )
}

fn close(uri: &str) -> String {
    let document = object(vec![("uri", text(uri))]);
    notice("textDocument/didClose", vec![("textDocument", document)])
}

/// Each diagnostic of a `publishDiagnostics` notification as its code, line, start, end and
/// severity, sorted.
fn published(reply: &Json) -> Vec<(String, String, String, String, String)> {
    assert_eq!(
        at(reply, "method").as_str(),
        Some("textDocument/publishDiagnostics")
    );
    let Json::Array(diagnostics) = at(reply, "params.diagnostics") else {
        panic!("diagnostics is not an array");
    };
    let mut found: Vec<_> = diagnostics
        .iter()
        .map(|d| {
            let n = |path| mcp::to_string(at(d, path));
            let code = d
                .get("code")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_owned();
            (
                code,
                n("range.start.line"),
                n("range.start.character"),
                n("range.end.character"),
                n("severity"),
            )
        })
        .collect();
    found.sort();
    found
}

fn row(
    code: &str,
    line: usize,
    start: usize,
    end: usize,
    severity: usize,
) -> (String, String, String, String, String) {
    let n = |n: usize| n.to_string();
    (code.into(), n(line), n(start), n(end), n(severity))
}

#[test]
fn a_session_publishes_findings_as_documents_open_change_and_close() {
    let uri = "untitled:a.md";
    let (replies, clean) = session(&[
        INIT,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
        &open(uri, "markdown", "# Title\nThe color \u{2014} red.\n"),
        &change(uri, "The colour \u{2014} red."),
        &close(uri),
        SHUTDOWN,
        EXIT,
    ]);
    assert!(clean);
    assert_eq!(replies.len(), 5, "{replies:?}");
    let init = &replies[0];
    assert_eq!(mcp::to_string(at(init, "id")), "1");
    assert_eq!(
        at(init, "result.capabilities.positionEncoding").as_str(),
        Some("utf-16")
    );
    assert_eq!(
        mcp::to_string(at(init, "result.capabilities.textDocumentSync")),
        r#"{"openClose":true,"change":1}"#
    );
    assert_eq!(
        at(init, "result.serverInfo.name").as_str(),
        Some("laconic-check")
    );
    assert_eq!(
        published(&replies[1]),
        [row("em-dash", 1, 10, 11, 2), row("us-spelling", 1, 4, 5, 2)]
    );
    let first = at(&replies[1], "params.diagnostics.0");
    let code = at(first, "code").as_str().unwrap();
    assert_eq!(
        at(first, "codeDescription.href").as_str().unwrap(),
        format!("{}#{code}", lint::RULES_URL)
    );
    assert_eq!(at(first, "source").as_str(), Some("laconic-check"));
    assert_eq!(at(&replies[1], "params.uri").as_str(), Some(uri));
    assert_eq!(published(&replies[2]), [row("em-dash", 0, 11, 12, 2)]);
    assert!(published(&replies[3]).is_empty());
    assert_eq!(
        mcp::to_string(&replies[4]),
        r#"{"jsonrpc":"2.0","id":2,"result":null}"#
    );
}

#[test]
fn advisory_rules_are_hints_and_others_warnings() {
    let body = "It may possibly rain.";
    let (replies, _) = session(&[INIT, &open("untitled:b", "Markdown", body)]);
    assert_eq!(published(&replies[1]), [row("stacked-hedge", 0, 3, 4, 3)]);
}

#[test]
fn positions_count_in_the_encoding_the_client_offers() {
    let body = "\u{1F600} color";
    let (replies, _) = session(&[INIT, &open("untitled:c", "markdown", body)]);
    assert_eq!(published(&replies[1]), [row("us-spelling", 0, 3, 4, 2)]);
    let utf8 = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"general":{"positionEncodings":["utf-8","utf-16"]}}}}"#;
    let (replies, _) = session(&[utf8, &open("untitled:c", "markdown", body)]);
    assert_eq!(
        at(&replies[0], "result.capabilities.positionEncoding").as_str(),
        Some("utf-8")
    );
    assert_eq!(published(&replies[1]), [row("us-spelling", 0, 5, 6, 2)]);
}

#[test]
fn requests_out_of_order_or_unknown_get_errors_and_others_nothing() {
    let error = |reply: &Json| mcp::to_string(at(reply, "error.code"));
    let (replies, clean) = session(&[
        SHUTDOWN,
        &open("untitled:d", "markdown", "color"),
        INIT,
        INIT,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{}}"#,
        "{",
        r#"{"jsonrpc":"2.0","id":9,"result":null}"#,
        &open("untitled:d", "plaintext", "color"),
        &change("untitled:d", "color"),
        &close("untitled:d"),
        r#"{"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":1}}"#,
        SHUTDOWN,
        r#"{"jsonrpc":"2.0","id":4,"method":"initialize","params":{}}"#,
        &open("untitled:e", "markdown", "color"),
        EXIT,
    ]);
    assert!(clean);
    let codes: Vec<String> = replies
        .iter()
        .map(|r| r.get("error").map_or_else(|| "ok".into(), |_| error(r)))
        .collect();
    assert_eq!(
        codes,
        ["-32002", "ok", "-32600", "-32601", "-32700", "ok", "-32600"]
    );
    assert_eq!(mcp::to_string(at(&replies[4], "id")), "null");
}

#[test]
fn exit_status_says_whether_shutdown_came_first() {
    assert!(!session(&[INIT, EXIT]).1);
    assert!(!session(&[INIT]).1);
    let (replies, clean) = session(&[SHUTDOWN, EXIT]);
    assert_eq!(replies.len(), 1);
    assert!(!clean);
}

#[test]
fn framing_skips_a_message_too_long_and_rejects_a_broken_header() {
    let mut input = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE + 1).into_bytes();
    input.resize(input.len() + MAX_MESSAGE + 1, b' ');
    input.extend(format!("content-length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{INIT}", INIT.len()).bytes());
    let mut output = Vec::new();
    assert!(!serve(&input[..], &mut output).unwrap());
    let replies = frames(&output);
    assert_eq!(
        at(&replies[0], "method").as_str(),
        Some("window/logMessage")
    );
    assert!(
        at(&replies[0], "params.message")
            .as_str()
            .unwrap()
            .starts_with("skipped a message of")
    );
    assert_eq!(mcp::to_string(at(&replies[1], "id")), "1");

    let long = format!("X-Long: {}\r\n\r\n", "a".repeat(MAX_HEADER));
    // Short lines that add up to more than a header may hold.
    let many = format!(
        "{}Content-Length: 2\r\n\r\n{{}}",
        "X: y\r\n".repeat(MAX_HEADER / 6)
    );
    let truncated = format!("Content-Length: {}\r\n\r\n{{}}", MAX_MESSAGE + 1);
    for (input, kind) in [
        (many, io::ErrorKind::InvalidData),
        (truncated, io::ErrorKind::UnexpectedEof),
        (
            "Content-Type: x\r\n\r\n{}".to_owned(),
            io::ErrorKind::InvalidData,
        ),
        (
            "Content-Length: x\r\n\r\n".to_owned(),
            io::ErrorKind::InvalidData,
        ),
        (long, io::ErrorKind::InvalidData),
        (
            "Content-Length: 10\r\n\r\n{}".to_owned(),
            io::ErrorKind::UnexpectedEof,
        ),
        (
            "Content-Length: 2\r\n".to_owned(),
            io::ErrorKind::UnexpectedEof,
        ),
    ] {
        let error = serve(input.as_bytes(), io::sink()).unwrap_err();
        assert_eq!(error.kind(), kind, "{input:?}");
    }
}

fn file_uri(path: &std::path::Path) -> String {
    let path = path.display().to_string().replace('\\', "/");
    format!(
        "file://{}{path}",
        if path.starts_with('/') { "" } else { "/" }
    )
}

#[test]
fn the_deepest_workspace_folder_with_a_project_file_sets_the_rules() {
    let root = std::env::temp_dir().join(format!("laconic-check-lsp-{}", std::process::id()));
    let inner = root.join("inner");
    fs::create_dir_all(&inner).unwrap();
    fs::write(root.join(PROJECT_FILE), "disable = us-spelling\n").unwrap();
    fs::write(inner.join(PROJECT_FILE), "only = nothing-rule\n").unwrap();
    let folder =
        |path: &std::path::Path| object(vec![("uri", text(&file_uri(path))), ("name", text("w"))]);
    let init = mcp::to_string(&object(vec![
        ("jsonrpc", text("2.0")),
        ("id", number(1)),
        ("method", text("initialize")),
        (
            "params",
            object(vec![
                ("capabilities", object(vec![])),
                (
                    "workspaceFolders",
                    Json::Array(vec![folder(&root), folder(&inner)]),
                ),
            ]),
        ),
    ]));
    let body = "The color \u{2014} red.";
    let outside = "untitled:x";
    let (replies, _) = session(&[
        &init,
        &open(&file_uri(&root.join("a.md")), "markdown", body),
        &open(&file_uri(&inner.join("b.md")), "markdown", body),
        &open(outside, "markdown", body),
    ]);
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(published(&replies[1]), [row("em-dash", 0, 10, 11, 2)]);
    assert_eq!(published(&replies[2]), [row("", 0, 0, 0, 1)]);
    let message = at(&replies[2], "params.diagnostics.0.message")
        .as_str()
        .unwrap();
    assert!(
        message.contains(":1: unknown rule 'nothing-rule'"),
        "{message}"
    );
    assert_eq!(published(&replies[3]).len(), 2);

    let root_uri = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///nowhere","capabilities":{}}}"#;
    let (replies, _) = session(&[root_uri, &open("file:///nowhere/a.md", "markdown", body)]);
    assert_eq!(published(&replies[1]).len(), 2);
}

#[test]
fn file_uris_become_local_paths_and_others_none() {
    let path = |uri, windows| file_path(uri, windows).map(|p| p.display().to_string());
    assert_eq!(
        path("file:///home/a%20b/x.md", false).as_deref(),
        Some("/home/a b/x.md")
    );
    assert_eq!(
        path("file://localhost/home/x.md", false).as_deref(),
        Some("/home/x.md")
    );
    assert_eq!(
        path("file:///c%3A/Users/x.md", true).as_deref(),
        Some("C:/Users/x.md")
    );
    assert_eq!(path("file:///d:/x.md", true).as_deref(), Some("D:/x.md"));
    for (uri, windows) in [
        ("file://server/share/x.md", false),
        ("untitled:x", false),
        ("file:///home/../etc/x.md", false),
        ("file:///home/%zz", false),
        ("file:///home/%2", false),
        ("file:///home/%ff", false),
        ("file:///x.md", true),
        ("file:///c%3A/a/../b", true),
        ("file:///C:relative/x.md", true),
        ("file:///1:/x.md", true),
        ("file:///C:", true),
    ] {
        assert_eq!(path(uri, windows), None, "{uri}");
    }
}
