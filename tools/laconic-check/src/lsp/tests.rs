// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

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

#[test]
fn a_project_rule_is_a_warning_with_its_name_and_no_reference_link() {
    // Under the crate's target folder: scratch that only this test writes.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("lsp-rules-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(PROJECT_FILE), "[rules]\nplain: utilise = use\n").unwrap();
    let init = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{}","capabilities":{{}}}}}}"#,
        file_uri(&root)
    );
    let (replies, _) = session(&[
        &init,
        &open(&file_uri(&root.join("a.md")), "markdown", "We utilise it."),
    ]);
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(published(&replies[1]), [row("project/plain", 0, 3, 4, 2)]);
    let diagnostic = at(&replies[1], "params.diagnostics.0");
    assert!(
        diagnostic.get("codeDescription").is_none(),
        "{diagnostic:?}"
    );
}

/// `initialize` for a client that applies versioned edits, offering `utf-8` when `utf8`.
fn init_edits(utf8: bool) -> String {
    let encodings = if utf8 {
        r#","general":{"positionEncodings":["utf-8"]}"#
    } else {
        ""
    };
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"capabilities":{{"workspace":{{"workspaceEdit":{{"documentChanges":true}}}}{encodings}}}}}}}"#
    )
}

/// A `codeAction` request `id` for `uri` from one line and character to another, for the
/// `only` kinds when given.
fn ask(id: usize, uri: &str, from: (usize, usize), to: (usize, usize), only: &[&str]) -> String {
    let position = |(line, character): (usize, usize)| {
        object(vec![
            ("line", number(line)),
            ("character", number(character)),
        ])
    };
    let mut context = vec![("diagnostics", Json::Array(Vec::new()))];
    if !only.is_empty() {
        context.push(("only", Json::Array(only.iter().map(|k| text(k)).collect())));
    }
    mcp::to_string(&object(vec![
        ("jsonrpc", text("2.0")),
        ("id", number(id)),
        ("method", text("textDocument/codeAction")),
        (
            "params",
            object(vec![
                ("textDocument", object(vec![("uri", text(uri))])),
                (
                    "range",
                    object(vec![("start", position(from)), ("end", position(to))]),
                ),
                ("context", object(context)),
            ]),
        ),
    ]))
}

fn n(json: &Json) -> usize {
    mcp::to_string(json).parse().unwrap()
}

/// The actions answering request `id`.
fn actions(replies: &[Json], id: usize) -> Vec<Json> {
    let reply = replies
        .iter()
        .find(|r| r.get("id").is_some_and(|i| n(i) == id))
        .unwrap_or_else(|| panic!("no reply {id} in {replies:?}"));
    match at(reply, "result") {
        Json::Array(actions) => actions.clone(),
        other => panic!("not a list of actions: {other:?}"),
    }
}

/// `body` after `action`'s edit, which must be one edit of `uri`, at `version` when given and
/// else a plain one; positions count UTF-8 bytes when `utf8`, else UTF-16 units, and may not
/// split a character or pass a line end.
fn apply(body: &str, action: &Json, uri: &str, version: Option<usize>, utf8: bool) -> String {
    assert_eq!(at(action, "kind").as_str(), Some("quickfix"));
    let edit = at(action, "edit");
    let edits = match version {
        Some(version) => {
            assert!(edit.get("changes").is_none(), "{edit:?}");
            let change = at(edit, "documentChanges.0");
            assert_eq!(at(change, "textDocument.uri").as_str(), Some(uri));
            assert_eq!(n(at(change, "textDocument.version")), version);
            at(change, "edits")
        }
        None => {
            assert!(edit.get("documentChanges").is_none(), "{edit:?}");
            // `at` would split the URI at its dots.
            at(edit, "changes")
                .get(uri)
                .expect("no edit of the document")
        }
    };
    let Json::Array(edits) = edits else {
        panic!("edits is not an array");
    };
    assert_eq!(edits.len(), 1, "{action:?}");
    let offset = |p: &Json| {
        let (line, character) = (n(at(p, "line")), n(at(p, "character")));
        let start: usize = body.split_inclusive('\n').take(line).map(str::len).sum();
        let mut units = 0;
        for (i, c) in body[start..].char_indices() {
            assert!(units <= character, "{line}:{character} splits a character");
            if units == character {
                return start + i;
            }
            assert!(
                !matches!(c, '\r' | '\n'),
                "{line}:{character} passes the line end"
            );
            units += if utf8 { c.len_utf8() } else { c.len_utf16() };
        }
        assert_eq!(units, character, "{line}:{character} passes the end");
        body.len()
    };
    let edit = &edits[0];
    let (from, to) = (
        offset(at(edit, "range.start")),
        offset(at(edit, "range.end")),
    );
    let new = at(edit, "newText").as_str().unwrap();
    format!("{}{new}{}", &body[..from], &body[to..])
}

/// The quick fixes for the whole of `body`, opened as Markdown after `init`: each one's title,
/// its diagnostic's code and the text its edit makes.
fn fixes_after(init: &str, body: &str, utf8: bool) -> Vec<(String, String, String)> {
    let uri = "untitled:fix.md";
    let end = (body.lines().count() + 1, 0);
    let (replies, _) = session(&[
        init,
        &open(uri, "markdown", body),
        &ask(7, uri, (0, 0), end, &[]),
    ]);
    actions(&replies, 7)
        .iter()
        .map(|a| {
            let title = at(a, "title").as_str().unwrap().to_owned();
            let code = at(a, "diagnostics.0.code").as_str().unwrap().to_owned();
            (title, code, apply(body, a, uri, Some(1), utf8))
        })
        .collect()
}

fn fixes(body: &str) -> Vec<(String, String, String)> {
    fixes_after(&init_edits(false), body, false)
}

/// The codes of the diagnostics published for `body`.
fn codes(body: &str) -> Vec<String> {
    let (replies, _) = session(&[INIT, &open("untitled:x.md", "markdown", body)]);
    published(&replies[1]).into_iter().map(|r| r.0).collect()
}

#[test]
fn the_server_offers_quick_fixes_and_no_other_kind() {
    let (replies, _) = session(&[INIT]);
    assert_eq!(
        mcp::to_string(at(&replies[0], "result.capabilities.codeActionProvider")),
        r#"{"codeActionKinds":["quickfix"]}"#
    );
}

#[test]
fn each_fix_replaces_its_finding_and_the_diagnostic_goes() {
    for (body, rule, title, fixed) in [
        (
            "The color is red.",
            "us-spelling",
            "Replace 'color' with 'colour'",
            "The colour is red.",
        ),
        (
            "Color is red.",
            "us-spelling",
            "Replace 'Color' with 'Colour'",
            "Colour is red.",
        ),
        (
            "COLOR is red.",
            "us-spelling",
            "Replace 'COLOR' with 'COLOUR'",
            "COLOUR is red.",
        ),
        (
            "It rose 5 percent.",
            "us-spelling",
            "Replace 'percent' with 'per cent'",
            "It rose 5 per cent.",
        ),
        (
            "It ran while we slept.",
            "word-choice",
            "Replace 'while' with 'whilst'",
            "It ran whilst we slept.",
        ),
        (
            "We use open-source code.",
            "hyphen",
            "Replace 'open-source' with 'open source'",
            "We use open source code.",
        ),
        (
            "Send an E-mail.",
            "hyphen",
            "Replace 'E-mail' with 'Email'",
            "Send an Email.",
        ),
        (
            "We ran five tests.",
            "number-word",
            "Replace 'five' with '5'",
            "We ran 5 tests.",
        ),
        (
            "Use a tool, e.g. a linter.",
            "latin",
            "Replace 'e.g.' with 'for example'",
            "Use a tool, for example a linter.",
        ),
        (
            "Use a tool, ie a linter.",
            "latin",
            "Replace 'ie' with 'that is'",
            "Use a tool, that is a linter.",
        ),
        (
            "We meet at 9am today.",
            "time",
            "Replace '9am' with '9 am'",
            "We meet at 9 am today.",
        ),
        (
            "We meet at 9.30PM today.",
            "time",
            "Replace '9.30PM' with '9:30 pm'",
            "We meet at 9:30 pm today.",
        ),
        (
            "We meet at 9 a.m. today.",
            "time",
            "Replace 'a.m.' with 'am'",
            "We meet at 9 am today.",
        ),
        (
            "We meet at 9.30 am today.",
            "time",
            "Replace '9.30' with '9:30'",
            "We meet at 9:30 am today.",
        ),
        (
            "It rose 5 % today.",
            "symbol-space",
            "Delete ' '",
            "It rose 5% today.",
        ),
        (
            "It costs $ 27.99 now.",
            "symbol-space",
            "Delete ' '",
            "It costs $27.99 now.",
        ),
        (
            "It shipped on 19th October 2026.",
            "date",
            "Replace '19th' with '19'",
            "It shipped on 19 October 2026.",
        ),
        (
            "It shipped on the 19th of October 2026.",
            "date",
            "Replace '19th of' with '19'",
            "It shipped on the 19 October 2026.",
        ),
        (
            "It shipped on 19 October, 2026.",
            "date",
            "Delete ','",
            "It shipped on 19 October 2026.",
        ),
        (
            "It shipped October 19 2026.",
            "date",
            "Replace 'October 19' with '19 October'",
            "It shipped 19 October 2026.",
        ),
        (
            "Music of the 1980's was loud.",
            "date",
            "Replace '1980's' with '1980s'",
            "Music of the 1980s was loud.",
        ),
        (
            "He said \"stop now\" and left.",
            "double-quotes",
            "Replace '\"stop now\"' with ''stop now''",
            "He said 'stop now' and left.",
        ),
        (
            "He said \u{201c}don't\u{201d} twice.",
            "double-quotes",
            "Replace '\u{201c}don't\u{201d}' with '\u{2018}don't\u{2019}'",
            "He said \u{2018}don't\u{2019} twice.",
        ),
        (
            "Approve\u{200b} it.",
            "residue",
            "Delete '\\u{200b}'",
            "Approve it.",
        ),
        (
            "Approve\u{202e} it.",
            "residue",
            "Delete '\\u{202e}'",
            "Approve it.",
        ),
        (
            "Go\u{e0041}\u{e0042} now.",
            "residue",
            "Delete '\\u{e0041}\\u{e0042}'",
            "Go now.",
        ),
    ] {
        let found = fixes(body);
        let offered: Vec<_> = found.iter().filter(|(_, code, _)| code == rule).collect();
        assert_eq!(offered.len(), 1, "{body}: {found:?}");
        assert_eq!(
            (offered[0].0.as_str(), offered[0].2.as_str()),
            (title, fixed),
            "{body}"
        );
        let after = codes(fixed);
        assert!(!after.iter().any(|c| c == rule), "{fixed}: {after:?}");
    }
}

#[test]
fn rules_without_one_known_replacement_offer_nothing() {
    for body in [
        "It failed \u{2014} twice.",
        "We need red, green, and blue.",
        "We met on Wednesday 22 October 2026.",
        "It may possibly rain.",
        "Hope this helps.",
        "It's not a tool, it's a platform.",
        // A joiner inside an emoji sequence.
        "Go \u{1F468}\u{200d}\u{1F469} now.",
        // The full stop may also end the sentence.
        "Start at 9 a.m. Then rest.",
        "Read a book, e.g. Dune.",
    ] {
        assert!(!codes(body).is_empty(), "{body}");
        let found = fixes(body);
        assert!(found.is_empty(), "{body}: {found:?}");
    }
}

#[test]
fn fixes_count_positions_in_the_encoding_the_client_offers() {
    // 'İ' lower-cases to 2 characters, which once moved every later column on its line.
    let body = "\u{130}stanbul \u{1F600} Caf\u{e9} color, \u{1F600}\u{1F600} five.\n";
    for utf8 in [false, true] {
        let texts: Vec<String> = fixes_after(&init_edits(utf8), body, utf8)
            .into_iter()
            .map(|f| f.2)
            .collect();
        assert_eq!(
            texts,
            [
                "\u{130}stanbul \u{1F600} Caf\u{e9} colour, \u{1F600}\u{1F600} five.\n",
                "\u{130}stanbul \u{1F600} Caf\u{e9} color, \u{1F600}\u{1F600} 5.\n",
            ],
            "utf8 {utf8}"
        );
    }
}

#[test]
fn fixes_keep_crlf_line_ends() {
    let texts: Vec<String> = fixes("The color\r\nis five.\r\n")
        .into_iter()
        .map(|f| f.2)
        .collect();
    assert_eq!(
        texts,
        ["The colour\r\nis five.\r\n", "The color\r\nis 5.\r\n"]
    );
}

#[test]
fn fixes_follow_the_latest_text_at_its_version() {
    let uri = "untitled:s.md";
    let init = init_edits(false);
    let (replies, _) = session(&[
        &init,
        &open(uri, "markdown", "color"),
        &change(uri, "The red color"),
        // Where version 1 had its finding, version 2 has none.
        &ask(7, uri, (0, 0), (0, 1), &[]),
        &ask(8, uri, (0, 8), (0, 13), &[]),
        &close(uri),
        &ask(9, uri, (0, 0), (0, 13), &[]),
    ]);
    assert!(actions(&replies, 7).is_empty());
    let found = actions(&replies, 8);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        apply("The red color", &found[0], uri, Some(2), false),
        "The red colour"
    );
    assert!(actions(&replies, 9).is_empty());

    // A plain edit for a client that does not say it takes versioned ones, or for a document
    // the client gave no version.
    let (replies, _) = session(&[
        INIT,
        &open(uri, "markdown", "color"),
        &ask(7, uri, (0, 0), (0, 5), &[]),
    ]);
    let found = actions(&replies, 7);
    assert_eq!(apply("color", &found[0], uri, None, false), "colour");
    let unversioned = notice(
        "textDocument/didOpen",
        vec![(
            "textDocument",
            object(vec![
                ("uri", text(uri)),
                ("languageId", text("markdown")),
                ("text", text("color")),
            ]),
        )],
    );
    let (replies, _) = session(&[&init, &unversioned, &ask(7, uri, (0, 0), (0, 5), &[])]);
    let found = actions(&replies, 7);
    assert_eq!(apply("color", &found[0], uri, None, false), "colour");
}

#[test]
fn a_request_gets_the_fixes_of_the_diagnostics_it_touches_and_kinds_it_asks_for() {
    let uri = "untitled:r.md";
    let (replies, _) = session(&[
        &init_edits(false),
        &open(uri, "markdown", "The color is five."),
        &ask(7, uri, (0, 4), (0, 4), &[]),
        &ask(8, uri, (0, 13), (0, 13), &["quickfix"]),
        &ask(9, uri, (0, 0), (0, 18), &["source.fixAll"]),
        &ask(10, uri, (0, 0), (0, 18), &["refactor", "quickfix"]),
    ]);
    let titles = |id| -> Vec<String> {
        actions(&replies, id)
            .iter()
            .map(|a| at(a, "title").as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(titles(7), ["Replace 'color' with 'colour'"]);
    assert_eq!(titles(8), ["Replace 'five' with '5'"]);
    assert!(titles(9).is_empty());
    assert_eq!(titles(10).len(), 2);
    let found = actions(&replies, 7);
    assert_eq!(
        mcp::to_string(at(&found[0], "diagnostics.0.range")),
        r#"{"start":{"line":0,"character":4},"end":{"line":0,"character":5}}"#
    );
}

#[test]
fn protected_and_wrapped_text_gets_no_fix() {
    let fixed = |body| -> Vec<String> { fixes(body).into_iter().map(|f| f.2).collect() };
    assert_eq!(
        fixed("The `color` and 'color' and [color](https://x.org/color) and color.\n"),
        ["The `color` and 'color' and [color](https://x.org/color) and colour.\n"]
    );
    assert_eq!(
        fixed("Say `\"stop\"` and \"stop\" now.\n"),
        ["Say `\"stop\"` and 'stop' now.\n"]
    );
    // The swap would leave the inner quotation in the same marks as the outer.
    let nested = "He said \"it's 'fine'\" today.\n";
    assert!(codes(nested).contains(&"double-quotes".to_owned()));
    assert!(fixed(nested).is_empty());
    // A finding whose text runs across a line wrap.
    let wrapped = "It shipped October\n19 2026.\n";
    assert!(codes(wrapped).contains(&"date".to_owned()));
    assert!(fixed(wrapped).is_empty());
    // An alert's body is prose, read with its marker as a space.
    assert_eq!(
        fixed("> [!NOTE]\n> The color is red.\n"),
        ["> [!NOTE]\n> The colour is red.\n"]
    );
}

#[test]
fn overlapping_fixes_are_separate_actions_each_applied_alone() {
    // Under the crate's target folder: scratch that only this test writes.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("lsp-fixes-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(PROJECT_FILE), "[rules]\nplain: color = hue\n").unwrap();
    let init = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{}","capabilities":{{"workspace":{{"workspaceEdit":{{"documentChanges":true}}}}}}}}}}"#,
        file_uri(&root)
    );
    let uri = file_uri(&root.join("a.md"));
    let body = "The color is red.";
    let (replies, _) = session(&[
        &init,
        &open(&uri, "markdown", body),
        &ask(7, &uri, (0, 0), (0, 17), &[]),
    ]);
    let found = actions(&replies, 7);
    let applied: Vec<(String, String)> = found
        .iter()
        .map(|a| {
            let code = at(a, "diagnostics.0.code").as_str().unwrap().to_owned();
            (code, apply(body, a, &uri, Some(1), false))
        })
        .collect();
    let after = |text: &str| {
        let (replies, _) = session(&[&init, &open(&uri, "markdown", text)]);
        published(&replies[1])
    };
    let hue = after("The hue is red.");
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        applied,
        [
            ("us-spelling".to_owned(), "The colour is red.".to_owned()),
            ("project/plain".to_owned(), "The hue is red.".to_owned()),
        ]
    );
    assert!(hue.is_empty(), "{hue:?}");
}
