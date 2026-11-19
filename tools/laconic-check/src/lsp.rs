// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `laconic-check lsp`: a language server on standard input and output that publishes `lint`'s
//! findings as diagnostics for open Markdown documents. Messages use the Language Server
//! Protocol's base framing, a `Content-Length` header and a JSON-RPC body, read and written with
//! the JSON code in `mcp`, so the crate keeps no dependencies. It syncs whole documents and
//! offers each fix `lint` knows as a quick fix.

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Component, PathBuf};

use crate::lint::{self, Finding, RuleRef};
use crate::mcp::{self, Json, object, text};
use crate::sarif;
use crate::settings::{PROJECT_FILE, Settings};

/// The longest message body, in bytes; a longer one is skipped without being held.
const MAX_MESSAGE: usize = 16 << 20;
/// The longest header, all its lines together, in bytes.
const MAX_HEADER: usize = 1024;

#[derive(Default)]
struct Server {
    initialized: bool,
    shut_down: bool,
    utf8: bool,
    /// Whether the client takes a versioned edit, which it refuses once the document has
    /// changed. Without it, no fix is offered.
    versioned: bool,
    /// Whether the client checks an edit's version only when it is above 0 (Neovim), so only a
    /// document at a positive whole version gets a fix.
    zero_unchecked: bool,
    /// Workspace folders as paths, from `initialize`.
    folders: Vec<PathBuf>,
    /// Open Markdown documents by URI.
    open: HashMap<String, Document>,
}

/// An open document's text and the version the client gave it.
struct Document {
    version: Json,
    text: String,
}

/// Answers messages from `input` until `exit` or the end of input. True when `exit` came after
/// `shutdown`, as the protocol's exit status 0 requires.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> io::Result<bool> {
    let mut server = Server::default();
    loop {
        let Some(length) = header(&mut input)? else {
            return Ok(false);
        };
        let replies = if length > MAX_MESSAGE {
            let skipped = io::copy(&mut input.by_ref().take(length as u64), &mut io::sink())?;
            if skipped < length as u64 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            let message = format!("skipped a message of {length} bytes, over {MAX_MESSAGE}");
            vec![notify(
                "window/logMessage",
                object(vec![("type", number(1)), ("message", text(&message))]),
            )]
        } else {
            let mut body = vec![0; length];
            input.read_exact(&mut body)?;
            match server.handle(&String::from_utf8_lossy(&body)) {
                Step::Reply(replies) => replies,
                Step::Exit => return Ok(server.shut_down),
            }
        };
        for reply in replies {
            write!(output, "Content-Length: {}\r\n\r\n{reply}", reply.len())?;
        }
        output.flush()?;
    }
}

/// A message's `Content-Length`, or `None` at the end of input before a header starts.
fn header(input: &mut impl BufRead) -> io::Result<Option<usize>> {
    let invalid = |message: &str| io::Error::new(io::ErrorKind::InvalidData, message.to_owned());
    let mut length = None;
    let mut started = false;
    let mut block = input.take(MAX_HEADER as u64);
    loop {
        let mut line = Vec::new();
        block.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            return if started {
                Err(io::Error::from(io::ErrorKind::UnexpectedEof))
            } else {
                Ok(None)
            };
        }
        if line.last() != Some(&b'\n') {
            return Err(invalid("a header is too long or unfinished"));
        }
        started = true;
        let line = String::from_utf8_lossy(&line);
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            return length
                .map(Some)
                .ok_or_else(|| invalid("a message has no Content-Length"));
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            let value = value
                .trim()
                .parse()
                .map_err(|_| invalid("a bad Content-Length"))?;
            length = Some(value);
        }
    }
}

enum Step {
    Reply(Vec<String>),
    Exit,
}

impl Server {
    fn handle(&mut self, body: &str) -> Step {
        let Some(message) = mcp::parse(body) else {
            return Step::Reply(vec![mcp::reply(
                Json::Null,
                Err((-32700, "parse error".into())),
            )]);
        };
        let params = message.get("params");
        let Some(method) = message.get("method").and_then(Json::as_str) else {
            // A response to a request this server never sends.
            return Step::Reply(Vec::new());
        };
        if method == "exit" {
            return Step::Exit;
        }
        let replies = match message.get("id") {
            Some(id) => vec![mcp::reply(id.clone(), self.request(method, params))],
            None if self.initialized && !self.shut_down => self.notification(method, params),
            None => Vec::new(),
        };
        Step::Reply(replies)
    }

    fn request(&mut self, method: &str, params: Option<&Json>) -> Result<Json, (i32, String)> {
        match method {
            _ if self.shut_down => Err((-32600, "the server is shutting down".into())),
            "initialize" if self.initialized => Err((-32600, "already initialised".into())),
            "initialize" => Ok(self.initialize(params)),
            _ if !self.initialized => Err((-32002, "the server is not initialised".into())),
            "shutdown" => {
                self.shut_down = true;
                Ok(Json::Null)
            }
            "textDocument/codeAction" => Ok(self.code_actions(params)),
            _ => Err((-32601, format!("method not found: {method}"))),
        }
    }

    fn initialize(&mut self, params: Option<&Json>) -> Json {
        let get = |path: &[&str]| path.iter().try_fold(params?, |value, key| value.get(key));
        self.utf8 = matches!(
            get(&["capabilities", "general", "positionEncodings"]),
            Some(Json::Array(offered)) if offered.iter().any(|e| e.as_str() == Some("utf-8"))
        );
        // Neovim 0.5 to 0.12 take a versioned edit without advertising it, but check its version
        // only when it is above 0, and open a document at version 0.
        let neovim = get(&["clientInfo", "name"]).and_then(Json::as_str) == Some("Neovim");
        let changes = get(&[
            "capabilities",
            "workspace",
            "workspaceEdit",
            "documentChanges",
        ]);
        self.versioned = changes == Some(&Json::Bool(true)) || (neovim && changes.is_none());
        self.zero_unchecked = neovim;
        let uris: Vec<&str> = match get(&["workspaceFolders"]) {
            Some(Json::Array(folders)) => folders
                .iter()
                .filter_map(|f| f.get("uri").and_then(Json::as_str))
                .collect(),
            _ => get(&["rootUri"])
                .and_then(Json::as_str)
                .into_iter()
                .collect(),
        };
        self.folders = uris
            .into_iter()
            .filter_map(|uri| file_path(uri, cfg!(windows)))
            .collect();
        self.initialized = true;
        object(vec![
            (
                "capabilities",
                object(vec![
                    (
                        "positionEncoding",
                        text(if self.utf8 { "utf-8" } else { "utf-16" }),
                    ),
                    (
                        "textDocumentSync",
                        object(vec![("openClose", Json::Bool(true)), ("change", number(1))]),
                    ),
                    (
                        "codeActionProvider",
                        object(vec![(
                            "codeActionKinds",
                            Json::Array(vec![text("quickfix")]),
                        )]),
                    ),
                ]),
            ),
            (
                "serverInfo",
                object(vec![
                    ("name", text("laconic-check")),
                    ("version", text(env!("CARGO_PKG_VERSION"))),
                ]),
            ),
        ])
    }

    fn notification(&mut self, method: &str, params: Option<&Json>) -> Vec<String> {
        let document = params.and_then(|p| p.get("textDocument"));
        let Some(uri) = document.and_then(|d| d.get("uri")).and_then(Json::as_str) else {
            return Vec::new();
        };
        let uri = uri.to_owned();
        let version = document
            .and_then(|d| d.get("version"))
            .cloned()
            .unwrap_or(Json::Null);
        let body = match method {
            "textDocument/didOpen" => {
                let markdown = document
                    .and_then(|d| d.get("languageId"))
                    .and_then(Json::as_str)
                    .is_some_and(|id| id.eq_ignore_ascii_case("markdown"));
                let Some(body) = document.and_then(|d| d.get("text")).and_then(Json::as_str) else {
                    return Vec::new();
                };
                if !markdown {
                    return Vec::new();
                }
                body
            }
            "textDocument/didChange" => {
                // Full sync: the last change holds the whole text.
                let changed = match params.and_then(|p| p.get("contentChanges")) {
                    Some(Json::Array(changes)) => changes.last(),
                    _ => None,
                };
                // A change with a range is part of a text, so the server no longer holds the
                // document's text and drops it, with its diagnostics.
                if changed.is_some_and(|c| c.get("range").is_some()) {
                    return match self.open.remove(&uri) {
                        Some(_) => vec![publish(&uri, Vec::new())],
                        None => Vec::new(),
                    };
                }
                match changed.and_then(|c| c.get("text")).and_then(Json::as_str) {
                    Some(body) if self.open.contains_key(&uri) => body,
                    _ => return Vec::new(),
                }
            }
            "textDocument/didClose" => {
                if self.open.remove(&uri).is_none() {
                    return Vec::new();
                }
                return vec![publish(&uri, Vec::new())];
            }
            _ => return Vec::new(),
        };
        let diagnostics = self.diagnostics(&uri, body);
        let text = body.to_owned();
        self.open.insert(uri.clone(), Document { version, text });
        vec![publish(&uri, diagnostics)]
    }

    /// The diagnostics of the text of the document at `uri`; or one error for a project file
    /// that cannot be read.
    fn diagnostics(&self, uri: &str, body: &str) -> Vec<Json> {
        match self.findings(uri, body) {
            Ok(found) => found.iter().map(to_diagnostic).collect(),
            Err(message) => vec![diagnostic(0, 0, 0, 1, None, &message)],
        }
    }

    /// `lint`'s findings in `body` that the project file's settings keep, each with the start
    /// and end of its diagnostic in the client's code units.
    fn findings(&self, uri: &str, body: &str) -> Result<Vec<(Finding, usize, usize)>, String> {
        let settings = self.settings(uri)?;
        let lines: Vec<&str> = body.lines().collect();
        let width = self.width();
        let found = lint::lint_counting(body, &settings.rules).0;
        Ok(found
            .into_iter()
            .filter_map(|finding| {
                let line = lines
                    .get(finding.line.checked_sub(1)?)
                    .copied()
                    .unwrap_or("");
                if !settings.keeps(&finding, line) {
                    return None;
                }
                let (start, end) = sarif::span(line, finding.column, width);
                Some((finding, start, end))
            })
            .collect())
    }

    fn width(&self) -> fn(char) -> usize {
        if self.utf8 {
            char::len_utf8
        } else {
            char::len_utf16
        }
    }

    /// A quick fix for each finding with a fix whose diagnostic meets the requested range,
    /// computed from the text the server holds, never from the diagnostics the client sends,
    /// and edited at that text's version, so a client refuses the edit once the text changes.
    fn code_actions(&self, params: Option<&Json>) -> Json {
        let get = |path: &[&str]| path.iter().try_fold(params?, |value, key| value.get(key));
        let quickfix = match get(&["context", "only"]) {
            Some(Json::Array(kinds)) => kinds
                .iter()
                .any(|k| matches!(k.as_str(), Some("quickfix" | ""))),
            _ => true,
        };
        let uri = get(&["textDocument", "uri"]).and_then(Json::as_str);
        let range = get(&["range", "start"])
            .and_then(position)
            .zip(get(&["range", "end"]).and_then(position));
        let document = uri
            .and_then(|uri| self.open.get(uri))
            .filter(|d| match &d.version {
                Json::Number(n) => {
                    quickfix
                        && self.versioned
                        && (!self.zero_unchecked || n.parse::<u64>().is_ok_and(|v| v > 0))
                }
                _ => false,
            })
            // A lone CR ends a line for the client but not for `lint`, so an edit would land on
            // the wrong line.
            .filter(|d| !d.text.split("\r\n").any(|part| part.contains('\r')));
        let (Some(uri), Some((from, to)), Some(document)) = (uri, range, document) else {
            return Json::Array(Vec::new());
        };
        let Ok(found) = self.findings(uri, &document.text) else {
            return Json::Array(Vec::new());
        };
        let lines: Vec<&str> = document.text.lines().collect();
        let width = self.width();
        let actions = found
            .iter()
            .filter(|(finding, start, end)| {
                let line = finding.line - 1;
                (line, *start) <= to && from <= (line, *end)
            })
            .filter_map(|placed| {
                let (finding, _, _) = placed;
                let fix = finding.fix.as_ref()?;
                let line = lines.get(finding.line - 1)?;
                let start = sarif::span(line, fix.column, width).0;
                let end = sarif::span(line, fix.column + fix.old.chars().count(), width).0;
                let at = |character| {
                    object(vec![
                        ("line", number(finding.line - 1)),
                        ("character", number(character)),
                    ])
                };
                let edit = object(vec![
                    (
                        "range",
                        object(vec![("start", at(start)), ("end", at(end))]),
                    ),
                    ("newText", text(&fix.new)),
                ]);
                let version = object(vec![
                    ("uri", text(uri)),
                    ("version", document.version.clone()),
                ]);
                let change = object(vec![
                    ("textDocument", version),
                    ("edits", Json::Array(vec![edit])),
                ]);
                let title = if fix.new.is_empty() {
                    format!("Delete '{}'", fix.old.escape_debug())
                } else {
                    format!("Replace '{}' with '{}'", fix.old, fix.new)
                };
                Some(object(vec![
                    ("title", text(&title)),
                    ("kind", text("quickfix")),
                    ("diagnostics", Json::Array(vec![to_diagnostic(placed)])),
                    (
                        "edit",
                        object(vec![("documentChanges", Json::Array(vec![change]))]),
                    ),
                ]))
            })
            .collect();
        Json::Array(actions)
    }

    /// The settings of the project file in the deepest workspace folder holding the document;
    /// the defaults when no folder holds it or the folder has no project file.
    fn settings(&self, uri: &str) -> Result<Settings, String> {
        let Some(path) = file_path(uri, cfg!(windows)) else {
            return Ok(Settings::default());
        };
        let folder = self
            .folders
            .iter()
            .filter(|folder| path.starts_with(folder))
            .max_by_key(|folder| folder.components().count());
        let Some(file) = folder.map(|folder| folder.join(PROJECT_FILE)) else {
            return Ok(Settings::default());
        };
        if !file.is_file() {
            return Ok(Settings::default());
        }
        let shown = file.display().to_string();
        let body = fs::read_to_string(&file).map_err(|e| format!("{shown}: {e}"))?;
        Settings::parse(&shown, &body)
    }
}

fn number(n: usize) -> Json {
    Json::Number(n.to_string())
}

/// A position's line and character.
fn position(json: &Json) -> Option<(usize, usize)> {
    let field = |key| match json.get(key)? {
        Json::Number(n) => n.parse().ok(),
        _ => None,
    };
    Some((field("line")?, field("character")?))
}

fn to_diagnostic((finding, start, end): &(Finding, usize, usize)) -> Json {
    let severity = if finding.rule.advisory() { 3 } else { 2 };
    diagnostic(
        finding.line - 1,
        *start,
        *end,
        severity,
        Some(&finding.rule),
        &finding.message,
    )
}

fn diagnostic(
    line: usize,
    start: usize,
    end: usize,
    severity: usize,
    rule: Option<&RuleRef>,
    message: &str,
) -> Json {
    let position = |character| {
        object(vec![
            ("line", number(line)),
            ("character", number(character)),
        ])
    };
    let mut members = vec![
        (
            "range",
            object(vec![("start", position(start)), ("end", position(end))]),
        ),
        ("severity", number(severity)),
        ("source", text("laconic-check")),
        ("message", text(message)),
    ];
    if let Some(rule) = rule {
        members.push(("code", text(rule.name())));
        // A project file's rule has no entry in the rule reference.
        if let Some(href) = rule.url() {
            members.push(("codeDescription", object(vec![("href", text(&href))])));
        }
    }
    object(members)
}

fn notify(method: &str, params: Json) -> String {
    mcp::to_string(&object(vec![
        ("jsonrpc", text("2.0")),
        ("method", text(method)),
        ("params", params),
    ]))
}

fn publish(uri: &str, diagnostics: Vec<Json>) -> String {
    notify(
        "textDocument/publishDiagnostics",
        object(vec![
            ("uri", text(uri)),
            ("diagnostics", Json::Array(diagnostics)),
        ]),
    )
}

/// The local path of a `file` URI with an empty or `localhost` authority; `None` for any
/// other, or for a path with a `..` part, which could name a folder outside the one it seems
/// to be in. On Windows, `/c%3A/x` is `C:/x`, its drive letter in capitals.
fn file_path(uri: &str, windows: bool) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let mut bytes = Vec::with_capacity(rest.len());
    let mut raw = rest.bytes();
    while let Some(b) = raw.next() {
        if b == b'%' {
            let hex = [raw.next()?, raw.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    let mut path = String::from_utf8(bytes).ok()?;
    if windows {
        // Only an absolute path from a drive's root: `C:x` is relative to C:'s current folder.
        let drive = path.strip_prefix('/')?;
        let [letter, b':', b'/', ..] = *drive.as_bytes() else {
            return None;
        };
        if !letter.is_ascii_alphabetic() {
            return None;
        }
        path = format!("{}{}", char::from(letter.to_ascii_uppercase()), &drive[1..]);
    }
    let path = PathBuf::from(path);
    (!path.components().any(|c| c == Component::ParentDir)).then_some(path)
}

#[cfg(test)]
mod tests;
