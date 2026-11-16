// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

#[test]
fn fetch_downloads_once_into_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub/file.jsonl");
    let opened = RefCell::new(0);
    let get = |_: &str| {
        *opened.borrow_mut() += 1;
        Ok(b"{}\n".to_vec())
    };
    assert_eq!(fetch("https://example.com/f", &path, &get).unwrap(), path);
    fetch("https://example.com/f", &path, &get).unwrap();
    assert_eq!(opened.into_inner(), 1);
    assert_eq!(fs::read_to_string(&path).unwrap(), "{}\n");
}

#[test]
fn fetch_refuses_a_scheme_other_than_http() {
    let dir = tempfile::tempdir().unwrap();
    for url in ["file:///etc/hosts", "ftp://example.com/f", "example.com/f"] {
        assert_eq!(
            fetch(url, &dir.path().join("file.jsonl"), &offline).unwrap_err(),
            format!("not an http or https URL: {url}")
        );
    }
}

#[test]
fn an_interrupted_download_is_fetched_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub/file.jsonl");
    let interrupted = |_: &str| Err("interrupted".to_owned());
    assert_eq!(
        fetch("https://example.com/f", &path, &interrupted).unwrap_err(),
        "interrupted"
    );
    assert!(!path.exists());
    // A torn earlier attempt left its partial file; the next download replaces it.
    fs::write(dir.path().join("sub/file.jsonl.partial"), "{\"a\"").unwrap();
    fetch("https://example.com/f", &path, &|_| {
        Ok(b"{\"a\": 1}\n".to_vec())
    })
    .unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\": 1}\n");
    let names: Vec<String> = fs::read_dir(dir.path().join("sub"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["file.jsonl"]);
}

#[test]
fn http_get_reads_a_whole_body_and_reports_a_failed_status() {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = "x".repeat(11 * 1024 * 1024);
    let served = body.clone();
    let server = std::thread::spawn(move || {
        for (status, content) in [("200 OK", served.as_str()), ("404 Not Found", "")] {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{content}",
                content.len()
            )
            .unwrap();
        }
    });
    let url = format!("http://{address}/f");
    assert_eq!(
        http_get(&url).unwrap().len(),
        body.len(),
        "beyond ureq's 10 MB default"
    );
    assert!(http_get(&url).unwrap_err().starts_with(&url));
    server.join().unwrap();
}

#[test]
fn http_get_times_out_when_the_server_stalls() {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    // Part of the headers, then headers and part of the body; each followed by silence.
    for sent in [
        "HTTP/1.1 200 OK\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nxx",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (finished, wait) = std::sync::mpsc::channel::<()>();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            stream.write_all(sent.as_bytes()).unwrap();
            stream.flush().unwrap();
            let _ = wait.recv();
        });
        let url = format!("http://{address}/f");
        let started = std::time::Instant::now();
        let error = get_with_timeout(&url, Duration::from_millis(200)).unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(10), "{sent:?}");
        assert!(error.starts_with(&url), "{error}");
        drop(finished);
        server.join().unwrap();
    }
}
