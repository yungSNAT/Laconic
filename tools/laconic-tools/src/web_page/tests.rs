// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::root;

fn hex(digest: [u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn sha256_matches_the_fips_examples() {
    for (text, digest) in [
        (
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ] {
        assert_eq!(hex(sha256(text.as_bytes())), digest, "{text:?}");
    }
}

#[test]
fn base64_pads_as_rfc_4648_does() {
    for (text, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(text.as_bytes()), encoded);
    }
}

#[test]
fn the_policy_hashes_the_script_and_style_as_written() {
    let template = "<meta content=\"script-src {{SCRIPT_HASH}}; style-src {{STYLE_HASH}}\">\
                    <style>p{}</style><script>const w = \"{{WASM}}\";</script>";
    let page = build(template, b"foo").unwrap();
    assert!(
        page.contains("<script>const w = \"Zm9v\";</script>"),
        "{page}"
    );
    assert!(
        page.contains(&format!("script-src {}", hash("const w = \"Zm9v\";"))),
        "{page}"
    );
    assert!(
        page.contains(&format!("style-src {}\"", hash("p{}"))),
        "{page}"
    );
    // The browsers' worked example of a script hash.
    assert_eq!(
        hash("alert('Hello, world.');"),
        "'sha256-qznLcsROx4GACP2dm0UCKCzCG+HiZ1guq6ZZDob/Tng='"
    );
}

#[test]
fn a_template_without_exactly_one_of_each_marker_is_an_error() {
    let error = build("<style></style><script></script>", b"").unwrap_err();
    assert!(error.contains("0 of {{WASM}}"), "{error}");
    let error = build("{{WASM}}<script></script><script></script>", b"").unwrap_err();
    assert!(error.contains("exactly one <script>"), "{error}");
}

#[test]
fn the_repository_page_builds() {
    let template = fs::read_to_string(root().join(TEMPLATE)).unwrap();
    let page = build(&template, b"\0asm").unwrap();
    assert!(page.contains("default-src 'none'"));
    assert!(!page.contains("{{"), "a marker is left");
}
