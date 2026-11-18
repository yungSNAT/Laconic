// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `laconic-check lsp`: a language server on standard input and output that publishes `lint`'s
//! findings as diagnostics for open Markdown documents. Messages use the Language Server
//! Protocol's base framing, a `Content-Length` header and a JSON-RPC body, read and written with
//! the JSON code in `mcp`, so the crate keeps no dependencies. It syncs whole documents and
//! offers nothing beyond diagnostics.

use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Component, PathBuf};

use crate::lint::{self, RuleRef};
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
    /// Workspace folders as paths, from `initialize`.
    folders: Vec<PathBuf>,
    /// The URIs of open Markdown documents. Their text is not kept, as full sync sends it whole
    /// with each change.
    open: HashSet<String>,
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
            _ => Err((-32601, format!("method not found: {method}"))),
        }
    }

    fn initialize(&mut self, params: Option<&Json>) -> Json {
        let get = |path: &[&str]| path.iter().try_fold(params?, |value, key| value.get(key));
        self.utf8 = matches!(
            get(&["capabilities", "general", "positionEncodings"]),
            Some(Json::Array(offered)) if offered.iter().any(|e| e.as_str() == Some("utf-8"))
        );
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
                self.open.insert(uri.clone());
                body
            }
            "textDocument/didChange" => {
                // Full sync: the last change holds the whole text.
                let changed = match params.and_then(|p| p.get("contentChanges")) {
                    Some(Json::Array(changes)) => changes.last(),
                    _ => None,
                };
                match changed.and_then(|c| c.get("text")).and_then(Json::as_str) {
                    Some(body) if self.open.contains(&uri) => body,
                    _ => return Vec::new(),
                }
            }
            "textDocument/didClose" => {
                if !self.open.remove(&uri) {
                    return Vec::new();
                }
                return vec![publish(&uri, Vec::new())];
            }
            _ => return Vec::new(),
        };
        vec![publish(&uri, self.diagnostics(&uri, body))]
    }

    /// `lint`'s findings in the text of the document at `uri`, after the project file's
    /// settings; or one error for a project file that cannot be read.
    fn diagnostics(&self, uri: &str, body: &str) -> Vec<Json> {
        let settings = match self.settings(uri) {
            Ok(settings) => settings,
            Err(message) => return vec![diagnostic(0, 0, 0, 1, None, &message)],
        };
        let lines: Vec<&str> = body.lines().collect();
        let width = if self.utf8 {
            char::len_utf8
        } else {
            char::len_utf16
        };
        lint::lint_counting(body, &settings.rules)
            .0
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
                let severity = if finding.rule.advisory() { 3 } else { 2 };
                Some(diagnostic(
                    finding.line - 1,
                    start,
                    end,
                    severity,
                    Some(&finding.rule),
                    &finding.message,
                ))
            })
            .collect()
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
