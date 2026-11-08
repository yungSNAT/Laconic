// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Package manifests for a laconic-check release, written from the release's SHA-256 files:
//! a Scoop manifest, a Homebrew formula, the 3 winget manifest files, the MCP bundle's
//! manifest.json and the MCP Registry's server.json. The release workflow attaches them to the
//! release; submitting them to a bucket, tap, winget-pkgs or the registry is a separate step.

use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const REPO: &str = "https://github.com/yungSNAT/Laconic";
const LICENSE: &str = "FSL-1.1-ALv2";
/// Homebrew allows 80 characters and the registry 100.
const SUMMARY: &str = "Flags Australian English and house-style candidates in Markdown prose";
const _: () = assert!(SUMMARY.len() <= 80);
const DESCRIPTION: &str = "Flags Australian English and house-style candidates in Markdown prose, \
                           and checks that an edit kept numbers, code, URLs and negations.";
const WINGET_ID: &str = "yungSNAT.laconic-check";
const WINGET_SCHEMA: &str = "1.28.0";
const REGISTRY_SCHEMA: &str =
    "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json";

const LINUX: &str = "laconic-check-x86_64-unknown-linux-musl.tar.gz";
const MACOS: &str = "laconic-check-aarch64-apple-darwin.tar.gz";
const WINDOWS: &str = "laconic-check-x86_64-pc-windows-msvc.zip";
const BUNDLE: &str = "laconic-check.mcpb";
/// Where the workflow unpacks each binary inside the bundle.
const BUNDLE_LINUX: &str = "server/linux-x64/laconic-check";
const BUNDLE_MACOS: &str = "server/darwin-arm64/laconic-check";
const BUNDLE_WINDOWS: &str = "server/win32-x64/laconic-check.exe";

fn url(version: &str, file: &str) -> String {
    format!("{REPO}/releases/download/laconic-check-v{version}/{file}")
}

/// A release version: 3 dot-separated numbers, since it goes into URLs and a Ruby formula.
fn check_version(version: &str) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err(format!("version {version:?} is not of the form 1.2.3"))
    }
}

/// The SHA-256 in `<dist>/<file>.sha256`, a `sha256sum` or `shasum` line naming `file`.
fn sha256(dist: &Path, file: &str) -> Result<String, String> {
    let path = dist.join(format!("{file}.sha256"));
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    match text.split_whitespace().collect::<Vec<_>>()[..] {
        [hash, name]
            if name.strip_prefix('*').unwrap_or(name) == file
                && hash.len() == 64
                && hash.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            Ok(hash.to_ascii_lowercase())
        }
        _ => Err(format!(
            "{} does not hold one SHA-256 for {file}",
            path.display()
        )),
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).expect("a JSON value serialises") + "\n"
}

/// The MCP bundle's manifest.json, written into the bundle before it is zipped.
fn mcpb_manifest(version: &str) -> String {
    let command = |path: &str| format!("${{__dirname}}/{path}");
    pretty(&json!({
        "manifest_version": "0.3",
        "name": "laconic-check",
        "version": version,
        "description": DESCRIPTION,
        "author": { "name": "yungSNAT", "url": "https://github.com/yungSNAT" },
        "homepage": REPO,
        "repository": { "type": "git", "url": REPO },
        "license": LICENSE,
        "server": {
            "type": "binary",
            "entry_point": BUNDLE_LINUX,
            "mcp_config": {
                "command": command(BUNDLE_LINUX),
                "args": ["mcp"],
                "platform_overrides": {
                    "darwin": { "command": command(BUNDLE_MACOS) },
                    "win32": { "command": command(BUNDLE_WINDOWS) }
                }
            }
        },
        "tools": [
            { "name": "lint", "description": "Flag Australian English and house-style candidates in Markdown prose." },
            { "name": "preserve", "description": "Report numbers, code, URLs and negations that an edit dropped." }
        ],
        // The macOS binary is for Apple silicon and the Linux one for x86_64.
        "compatibility": { "platforms": ["darwin", "win32", "linux"] }
    }))
}

fn scoop(version: &str, windows: &str) -> String {
    pretty(&json!({
        "version": version,
        "description": DESCRIPTION,
        "homepage": REPO,
        "license": LICENSE,
        "architecture": { "64bit": { "url": url(version, WINDOWS), "hash": windows } },
        "bin": "laconic-check.exe"
    }))
}

fn homebrew(version: &str, macos: &str, linux: &str) -> String {
    format!(
        r##"class LaconicCheck < Formula
  desc "{SUMMARY}"
  homepage "{REPO}"
  version "{version}"
  license "{LICENSE}"

  on_macos do
    on_arm do
      url "{macos_url}"
      sha256 "{macos}"
    end
  end

  on_linux do
    on_intel do
      url "{linux_url}"
      sha256 "{linux}"
    end
  end

  def install
    bin.install "laconic-check"
  end

  test do
    assert_match "us-spelling", pipe_output("#{{bin}}/laconic-check lint -", "The color.\n", 1)
  end
end
"##,
        macos_url = url(version, MACOS),
        linux_url = url(version, LINUX),
    )
}

/// The winget version, installer and default locale manifests, by file name.
fn winget(version: &str, windows: &str) -> [(String, String); 3] {
    let head = |kind: &str| {
        format!(
            "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{WINGET_SCHEMA}.schema.json\n\n\
             PackageIdentifier: {WINGET_ID}\nPackageVersion: {version}\n"
        )
    };
    let tail = |kind: &str| format!("ManifestType: {kind}\nManifestVersion: {WINGET_SCHEMA}\n");
    [
        (
            format!("{WINGET_ID}.yaml"),
            format!(
                "{}DefaultLocale: en-AU\n{}",
                head("version"),
                tail("version")
            ),
        ),
        (
            format!("{WINGET_ID}.installer.yaml"),
            format!(
                "{}InstallerType: zip\nNestedInstallerType: portable\nNestedInstallerFiles:\n\
                 - RelativeFilePath: laconic-check.exe\n  PortableCommandAlias: laconic-check\n\
                 Installers:\n- Architecture: x64\n  InstallerUrl: {}\n  InstallerSha256: {}\n{}",
                head("installer"),
                url(version, WINDOWS),
                windows.to_ascii_uppercase(),
                tail("installer")
            ),
        ),
        (
            format!("{WINGET_ID}.locale.en-AU.yaml"),
            format!(
                "{}PackageLocale: en-AU\nPublisher: yungSNAT\nPublisherUrl: https://github.com/yungSNAT\n\
                 PackageName: laconic-check\nPackageUrl: {REPO}\nLicense: {LICENSE}\n\
                 LicenseUrl: {REPO}/blob/main/LICENSE\nShortDescription: {DESCRIPTION}\n\
                 Tags:\n- australian-english\n- linter\n- markdown\n- mcp\n- prose\n\
                 ReleaseNotesUrl: {REPO}/releases/tag/laconic-check-v{version}\n{}",
                head("defaultLocale"),
                tail("defaultLocale")
            ),
        ),
    ]
}

fn server_json(version: &str, bundle: &str) -> String {
    pretty(&json!({
        "$schema": REGISTRY_SCHEMA,
        "name": "io.github.yungSNAT/laconic-check",
        "description": SUMMARY,
        "repository": { "url": REPO, "source": "github" },
        "version": version,
        "packages": [{
            "registryType": "mcpb",
            "identifier": url(version, BUNDLE),
            "version": version,
            "fileSha256": bundle,
            "transport": { "type": "stdio" }
        }]
    }))
}

/// Writes manifest.json into the unpacked bundle at `dir`.
pub fn write_mcpb_manifest(version: &str, dir: &Path) -> Result<String, String> {
    check_version(version)?;
    let path = dir.join("manifest.json");
    fs::write(&path, mcpb_manifest(version))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(format!("wrote {}", path.display()))
}

/// Writes the Scoop, Homebrew, winget and registry manifests into `out` from the SHA-256 files
/// of the 3 archives and the bundle in `dist`.
pub fn write_manifests(version: &str, dist: &Path, out: &Path) -> Result<String, String> {
    check_version(version)?;
    let windows = sha256(dist, WINDOWS)?;
    let mut files = vec![
        ("laconic-check.json".to_owned(), scoop(version, &windows)),
        (
            "laconic-check.rb".to_owned(),
            homebrew(version, &sha256(dist, MACOS)?, &sha256(dist, LINUX)?),
        ),
        (
            "server.json".to_owned(),
            server_json(version, &sha256(dist, BUNDLE)?),
        ),
    ];
    files.extend(winget(version, &windows));
    fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    for (name, text) in &files {
        fs::write(out.join(name), text).map_err(|e| format!("cannot write {name}: {e}"))?;
    }
    Ok(format!(
        "wrote {} manifests to {}",
        files.len(),
        out.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const FIXTURE: &str = "tests/packages";
    const REGENERATE: &str =
        "cargo run -- package-manifests 1.2.3 tests/packages/dist tests/packages/expected";

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("laconic-packages-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The committed manifests are the generator's output for the fixture's fake hashes, so a
    /// reviewer sees exactly what a release attaches.
    #[test]
    fn manifests_match_the_committed_examples() {
        let out = temp("golden");
        write_manifests("1.2.3", &fixture().join("dist"), &out).unwrap();
        let mut expected: Vec<_> = fs::read_dir(fixture().join("expected"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        expected.sort();
        let mut written: Vec<_> = fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        written.sort();
        assert_eq!(written, expected, "run {REGENERATE}");
        for name in written {
            let want = fs::read_to_string(fixture().join("expected").join(&name)).unwrap();
            let got = fs::read_to_string(out.join(&name)).unwrap();
            assert_eq!(
                got,
                want.replace("\r\n", "\n"),
                "{name:?} differs; run {REGENERATE}"
            );
        }
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn mcpb_manifest_runs_each_platforms_binary() {
        let manifest: Value = serde_json::from_str(&mcpb_manifest("1.2.3")).unwrap();
        let config = &manifest["server"]["mcp_config"];
        assert_eq!(
            config["command"],
            "${__dirname}/server/linux-x64/laconic-check"
        );
        assert_eq!(config["args"], json!(["mcp"]));
        assert_eq!(
            config["platform_overrides"]["win32"]["command"],
            "${__dirname}/server/win32-x64/laconic-check.exe"
        );
        assert_eq!(manifest["version"], "1.2.3");
    }

    #[test]
    fn checksum_files_must_name_their_file_and_hold_one_hash() {
        let dist = temp("sha");
        let hash = "AB".repeat(32);
        for (text, ok) in [
            (format!("{hash}  {WINDOWS}\n"), true),
            (format!("{hash} *{WINDOWS}\n"), true),
            (format!("{hash}  {LINUX}\n"), false),
            (format!("{}  {WINDOWS}\n", &hash[1..]), false),
            (format!("{}g  {WINDOWS}\n", &hash[1..]), false),
            (format!("{hash}  {WINDOWS}\n{hash}  {WINDOWS}\n"), false),
        ] {
            fs::write(dist.join(format!("{WINDOWS}.sha256")), &text).unwrap();
            let result = sha256(&dist, WINDOWS);
            assert_eq!(result.is_ok(), ok, "{text:?}: {result:?}");
            if ok {
                assert_eq!(result.unwrap(), "ab".repeat(32));
            }
        }
        assert!(sha256(&dist, MACOS).unwrap_err().contains("cannot read"));
        fs::remove_dir_all(&dist).unwrap();
    }

    #[test]
    fn versions_must_be_3_numbers() {
        assert!(check_version("0.3.0").is_ok());
        for bad in [
            "",
            "0.3",
            "0.3.0.1",
            "v0.3.0",
            "0.3.0-rc1",
            "0..3",
            "0.3.\"x",
        ] {
            assert!(check_version(bad).is_err(), "{bad}");
        }
        let unused = std::env::temp_dir().join("laconic-packages-unused");
        assert!(write_manifests("0.3", &fixture().join("dist"), &unused).is_err());
        assert!(!unused.exists());
    }

    #[test]
    fn the_description_is_a_plain_yaml_scalar() {
        assert!(!DESCRIPTION.contains(": ") && !DESCRIPTION.contains(" #"));
    }
}
