// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::testing::{Recorder, done, write_rows};
use serde_json::json;
use std::cell::RefCell;

fn unit(group: &str, kept: bool) -> Value {
    json!({"group": group, "surface": "x", "kept": kept})
}

fn judged(tool: &str, sample: &str, units: Value) -> Value {
    json!({"set": "commercial", "tool": tool, "aihb_cycle": "2026-10", "sample_id": sample, "units": units})
}

fn rewrite(sample: &str, source: &str, output: Value) -> Value {
    json!({"humanizer": "t", "sample_id": sample, "input": source, "output": output})
}

fn kinds(names: &[&str]) -> Kinds {
    names.iter().map(|n| (*n).to_owned()).collect()
}

/// The 2 downloads where `run` caches them for the current pins, for the tool 't'.
fn write_cache(cache: &Path, judgements: &[Value], rewrites: &[Value]) {
    write_rows(
        &cache.join(FACT_AUDIT_COMMIT).join("fact_units.jsonl"),
        judgements,
    );
    write_rows(
        &cache.join(AIHB_REVISION).join("2026-10").join("t.jsonl"),
        rewrites,
    );
}

fn offline(url: &str) -> Result<Vec<u8>, String> {
    panic!("no download expected: {url}")
}

#[test]
fn the_offline_stand_in_refuses_downloads() {
    assert!(std::panic::catch_unwind(|| offline("https://example.test")).is_err());
}

#[test]
fn flagged_reads_the_paper_kinds_and_meaning_words() {
    let report = "dropped number: $1,200\ndropped quotation: we will not proceed\n\
                  dropped meaning word: 'not' (1 before, 0 after)\ncheck name: Priya Nair\n\
                  added number: 4\ncheck added name: Jane Smith\nlength: 9 to 5 words (56% of the original)\n";
    assert_eq!(flagged(report), kinds(&["num", "quote", "meaning", "ent"]));
    assert_eq!(flagged("dropped quantity: 25 °C\n"), kinds(&["num"]));
    assert_eq!(
        flagged("dropped month: May 2026\ndropped url: https://example.com/p\n"),
        kinds(&["other"])
    );
    assert_eq!(flagged("dropped code: `refund --all`\n"), kinds(&["other"]));
    assert_eq!(
        flagged("No tracked items dropped; meaning not checked.\nadded number: 4\n"),
        kinds(&[])
    );
    assert_eq!(flagged("check added name: Jane Smith\n"), kinds(&[]));
}

#[test]
fn judged_documents_keep_only_the_cycle_commercial_rows() {
    let mut other_cycle = judged("a", "s03", json!([unit("num", false)]));
    other_cycle["aihb_cycle"] = json!("2026-09");
    let open = json!({"set": "open-12b", "tool": null, "aihb_cycle": "2026-10", "sample_id": "p1",
                      "units": [unit("num", false)]});
    let rows = [
        judged("a", "s01", json!([unit("num", false), unit("quote", true)])),
        judged("a", "s02", json!([])),
        other_cycle,
        open,
    ];
    let expected = BTreeMap::from([
        (("a".to_owned(), "s01".to_owned()), kinds(&["num"])),
        (("a".to_owned(), "s02".to_owned()), kinds(&[])),
    ]);
    assert_eq!(judged_documents(&rows).unwrap(), expected);
}

#[test]
fn judged_documents_refuse_a_repeat_or_a_non_boolean_kept() {
    let twice = [
        judged("a", "s01", json!([])),
        judged("a", "s01", json!([unit("num", false)])),
    ];
    assert_eq!(
        judged_documents(&twice).unwrap_err(),
        "judged twice: ('a', 's01')"
    );
    let fuzzy = [judged("a", "s01", json!([{"group": "num", "kept": "no"}]))];
    assert_eq!(
        judged_documents(&fuzzy).unwrap_err(),
        "a unit of ('a', 's01') has a non-boolean 'kept'"
    );
}

#[test]
fn judged_documents_refuse_malformed_units() {
    for (units, message) in [
        (json!(""), "not a list"),
        (json!({}), "not a list"),
        (json!(["num"]), "not a list"),
        (json!([unit("date", false)]), "unknown group 'date'"),
        (json!([{"kept": false}]), "unknown group None"),
    ] {
        let error = judged_documents(&[judged("a", "s01", units)]).unwrap_err();
        assert!(error.contains(message), "{error}");
    }
    assert_eq!(
        judged_documents(&[json!({})]).unwrap_err(),
        "a row has no 'set'"
    );
    let unnamed =
        json!({"set": "commercial", "aihb_cycle": "2026-10", "tool": 3, "sample_id": "s"});
    assert!(
        judged_documents(&[unnamed])
            .unwrap_err()
            .starts_with("'tool' is not text")
    );
}

#[test]
fn read_jsonl_refuses_a_blank_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rows.jsonl");
    for text in ["{\"a\": 1}\n\n{\"a\": 2}\n", "{\"a\": 1}\n\n"] {
        fs::write(&path, text).unwrap();
        assert!(read_jsonl(&path).unwrap_err().ends_with("line 2 is blank"));
    }
    fs::write(&path, "{\"a\": 1}\r\n{\"a\": 2}").unwrap();
    assert_eq!(
        read_jsonl(&path).unwrap(),
        [json!({"a": 1}), json!({"a": 2})]
    );
}

#[test]
fn tally_counts_catches_and_failures_on_kept_documents() {
    let counts = tally(&[
        (kinds(&["num", "ent"]), true, kinds(&["num", "meaning"])),
        (kinds(&["quote"]), true, kinds(&["meaning"])),
        (kinds(&[]), false, kinds(&[])),
        (kinds(&[]), true, kinds(&["meaning"])),
        (kinds(&[]), true, kinds(&["quote", "meaning"])),
        (kinds(&[]), true, kinds(&["other", "meaning"])),
    ]);
    let c = |name: &str| count(&counts, name);
    assert_eq!(c("altered"), 2);
    assert_eq!(c("altered failed"), 2);
    assert_eq!((c("num altered"), c("num flagged")), (1, 1));
    assert_eq!((c("quote altered"), c("quote flagged")), (1, 0));
    assert_eq!((c("ent altered"), c("ent flagged")), (1, 0));
    assert_eq!(c("kept"), 4);
    assert_eq!(c("kept passed"), 1);
    assert_eq!(c("kept failed on meaning words only"), 1);
    assert_eq!(c("kept failed on numbers or quotations"), 1);
    assert_eq!(c("kept failed on months, code or URLs"), 1);
    let text = table(&counts);
    assert!(text.contains("| Any unit altered | 2 | 2 fail |"));
    assert!(text.contains("| Numbers altered | 1 | 1 report a dropped number or quantity |"));
    assert!(text.contains("| Names altered | 1 | 0 list a name to check |"));
    assert!(text.contains(
        "| Every unit kept | 4 | 1 pass; 1 fail only on meaning words; 1 fail on a month, inline code or URL; 1 fail on a number"
    ));
}

#[test]
fn run_compares_each_pair_and_lists_the_differences() {
    let dir = tempfile::tempdir().unwrap();
    write_cache(
        dir.path(),
        &[
            judged("t", "s01", json!([unit("num", false)])),
            judged("t", "s02", json!([unit("num", true)])),
        ],
        &[
            rewrite("s01", "Pay $500.", json!("Pay $400.")),
            rewrite("s02", "It may rain on 3 May.", json!("Rain on 3 May.")),
        ],
    );
    let seen = RefCell::new(Vec::new());
    let check = |binary: &str, source: &str, rewrite: &str| {
        seen.borrow_mut()
            .push((binary.to_owned(), source.to_owned(), rewrite.to_owned()));
        Ok((
            true,
            kinds(if source.contains('$') {
                &["num"]
            } else {
                &["quote"]
            }),
        ))
    };
    let (counts, differences) = run("lc", dir.path(), &check, &offline).unwrap();
    assert_eq!(
        seen.into_inner(),
        [
            ("lc".into(), "Pay $500.".into(), "Pay $400.".into()),
            (
                "lc".into(),
                "It may rain on 3 May.".into(),
                "Rain on 3 May.".into()
            )
        ]
    );
    let expected: Counts = [
        ("altered", 1),
        ("altered failed", 1),
        ("num altered", 1),
        ("num flagged", 1),
        ("kept", 1),
        ("kept failed on numbers or quotations", 1),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    assert_eq!(counts, expected);
    assert_eq!(differences, ["t s02: missed [], extra ['quote']"]);
}

#[test]
fn run_lists_what_preserve_missed() {
    let dir = tempfile::tempdir().unwrap();
    write_cache(
        dir.path(),
        &[judged(
            "t",
            "s01",
            json!([unit("num", false), unit("ent", false)]),
        )],
        &[rewrite("s01", "Pay $500.", json!("Pay $400."))],
    );
    let check = |_: &str, _: &str, _: &str| Ok((false, kinds(&[])));
    let (_, differences) = run("lc", dir.path(), &check, &offline).unwrap();
    assert_eq!(differences, ["t s01: missed ['num'], extra []"]);
    let failing = |_: &str, _: &str, _: &str| Err("preserve exited 2: usage".to_owned());
    assert_eq!(
        run("lc", dir.path(), &failing, &offline).unwrap_err(),
        "preserve exited 2: usage"
    );
}

#[test]
fn run_refuses_a_judged_document_without_a_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    write_cache(dir.path(), &[judged("t", "s01", json!([]))], &[]);
    let check = |_: &str, _: &str, _: &str| Ok((false, kinds(&[])));
    assert_eq!(
        run("lc", dir.path(), &check, &offline).unwrap_err(),
        "no benchmark rewrite for [('t', 's01')]"
    );
}

#[test]
fn run_refuses_a_repeated_or_missing_rewrite() {
    let check = |_: &str, _: &str, _: &str| Ok((false, kinds(&[])));
    for (rows, message) in [
        (
            vec![
                rewrite("s01", "Pay $500.", json!("Pay $500.")),
                rewrite("s01", "Pay $500.", json!("Pay $500.")),
            ],
            "benchmark rewrite twice: ('t', 's01')",
        ),
        (
            vec![rewrite("s01", "Pay $500.", Value::Null)],
            "benchmark rewrite missing or not text for ('t', 's01')",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        write_cache(dir.path(), &[judged("t", "s01", json!([]))], &rows);
        assert_eq!(
            run("lc", dir.path(), &check, &offline).unwrap_err(),
            message
        );
    }
}

#[test]
fn run_reads_no_cache_from_another_pin() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path();
    // Where the downloads were cached before the cache held each pin apart.
    fs::write(
        cache.join("fact_units.jsonl"),
        judged("t", "s01", json!([unit("num", false)])).to_string(),
    )
    .unwrap();
    fs::create_dir(cache.join("2026-10")).unwrap();
    fs::write(
        cache.join("2026-10/t.jsonl"),
        rewrite("s01", "Pay $500.", json!("Pay $400.")).to_string(),
    )
    .unwrap();
    let pages = BTreeMap::from([
        (
            fact_audit_url(),
            judged("t", "s01", json!([unit("num", true)])).to_string(),
        ),
        (
            aihb_url("t"),
            rewrite("s01", "Pay $500.", json!("Pay $500.")).to_string(),
        ),
    ]);
    let opened = RefCell::new(Vec::new());
    let get = |url: &str| {
        opened.borrow_mut().push(url.to_owned());
        Ok(pages[url].clone().into_bytes())
    };
    let check = |_: &str, _: &str, _: &str| Ok((false, kinds(&[])));
    let (counts, _) = run("lc", cache, &check, &get).unwrap();
    let mut opened = opened.into_inner();
    opened.sort();
    assert_eq!(opened, pages.keys().cloned().collect::<Vec<_>>());
    assert_eq!(
        counts,
        Counts::from([("kept".into(), 1), ("kept passed".into(), 1)])
    );
}

#[test]
fn preserve_reads_the_report_and_refuses_a_usage_error() {
    let recorder = Recorder::new();
    let dropped = |command: &[String], input: &Input| {
        recorder.record(command, input);
        let files: Vec<String> = command[command.len() - 2..]
            .iter()
            .map(|p| fs::read_to_string(p).unwrap())
            .collect();
        assert_eq!(files, ["Growth was -3%.", "Growth was 3%."]);
        done(1, "dropped number: -3%\n", "")
    };
    assert_eq!(
        preserve("lc", "Growth was -3%.", "Growth was 3%.", false, &dropped).unwrap(),
        (true, kinds(&["num"]))
    );
    preserve("lc", "Growth was -3%.", "Growth was 3%.", true, &dropped).unwrap();
    let calls = recorder.calls();
    assert_eq!(calls[0].command[..2], ["lc", "preserve"]);
    assert_ne!(calls[0].command[2], "--loose");
    assert_eq!(calls[1].command[..3], ["lc", "preserve", "--loose"]);
    assert!(calls[0].inherited);
    let broken = |_: &[String], _: &Input| done(2, "", "usage\n");
    assert_eq!(
        preserve("lc", "a", "b", false, &broken).unwrap_err(),
        "preserve exited 2: usage"
    );
    let passed = |_: &[String], _: &Input| done(0, "No tracked items dropped.\n", "");
    assert_eq!(
        preserve("lc", "a", "a", false, &passed).unwrap(),
        (false, kinds(&[]))
    );
}

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
fn http_get_times_out_when_the_body_stalls() {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
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
        // Headers and part of the body, then silence until the client gives up.
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nxx").unwrap();
        stream.flush().unwrap();
        let _ = wait.recv();
    });
    let url = format!("http://{address}/f");
    let started = std::time::Instant::now();
    let error = get_with_timeout(&url, Duration::from_millis(200)).unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(error.starts_with(&url), "{error}");
    drop(finished);
    server.join().unwrap();
}

#[test]
fn report_prints_the_table_and_the_differences() {
    let counts = tally(&[(kinds(&["num"]), true, kinds(&["num"]))]);
    let out = report(&counts, &["t s01: missed ['num'], extra []".to_owned()]);
    assert!(out.starts_with("Judged documents: 1\n\n"));
    assert!(out.contains("| Any unit altered | 1 | 1 fail |"));
    assert!(
        out.ends_with("differ on numbers or quotations: 1\n- t s01: missed ['num'], extra []\n")
    );
}

#[cfg(unix)]
#[test]
fn which_finds_executables_and_refuses_other_files() {
    let dir = tempfile::tempdir().unwrap();
    let program = crate::testing::script(dir.path(), "laconic-check", "exit 0");
    let plain = dir.path().join("laconic-check.txt");
    fs::write(&plain, "not a program").unwrap();
    assert_eq!(which(program.to_str().unwrap()), Some(program));
    assert_eq!(which(plain.to_str().unwrap()), None);
    assert_eq!(which(dir.path().join("missing").to_str().unwrap()), None);
    assert_eq!(which(dir.path().to_str().unwrap()), None);
    assert!(which("sh").is_some());
    assert_eq!(which("no-such-program-xyz"), None);
}

#[cfg(unix)]
#[test]
fn exec_runs_preserve_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let binary = crate::testing::script(
        dir.path(),
        "laconic-check",
        "if [ \"$2\" = --loose ]; then echo 'dropped meaning word: not'; exit 1; fi; echo 'dropped number: 4'; exit 1",
    );
    let binary = binary.to_str().unwrap();
    assert_eq!(
        preserve(binary, "a 4", "a", false, &exec).unwrap(),
        (true, kinds(&["num"]))
    );
    assert_eq!(
        preserve(binary, "not", "", true, &exec).unwrap(),
        (true, kinds(&["meaning"]))
    );
    let missing = preserve("/no/such/laconic-check", "a", "b", false, &exec).unwrap_err();
    assert!(
        missing.starts_with("cannot run /no/such/laconic-check: "),
        "{missing}"
    );
}

#[test]
fn the_pins_are_full_commits() {
    for pin in [AIHB_REVISION, FACT_AUDIT_COMMIT] {
        assert_eq!(pin.len(), 40);
        assert!(
            pin.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
    }
}
