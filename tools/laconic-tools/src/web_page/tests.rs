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
fn sha256_pads_at_each_block_boundary() {
    // Lengths beside the 56-byte padding limit and the 64-byte block, as Python's hashlib gives.
    for (n, digest) in [
        (
            55,
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
        ),
        (
            63,
            "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
        ),
        (
            64,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
        ),
        (
            65,
            "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0",
        ),
    ] {
        assert_eq!(hex(sha256("a".repeat(n).as_bytes())), digest, "{n}");
    }
}

#[test]
fn a_crlf_template_builds_the_page_an_lf_one_does() {
    // A browser reads CR LF in the page as LF, so a hash of CR LF text would block the script.
    let lf = "<meta content=\"script-src {{SCRIPT_HASH}}; style-src {{STYLE_HASH}}\">\n\
              <style>\np{}\n</style><script>\nconst w = \"{{WASM}}\";\n</script>\n";
    let crlf = lf.replace('\n', "\r\n");
    assert_eq!(build(&crlf, b"foo").unwrap(), build(lf, b"foo").unwrap());
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
