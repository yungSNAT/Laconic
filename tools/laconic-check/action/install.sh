#!/usr/bin/env bash
# Downloads the laconic-check release binary for this runner, checks it against the release's
# SHA-256 file and puts it on the PATH. Used by the composite Action in action.yml.
set -euo pipefail
case "$VERSION" in
  '' | *[!0-9.]*)
    echo "::error::version must be a release number such as 0.3.0, not '$VERSION'"
    exit 2
    ;;
esac
case "$RUNNER_OS/$RUNNER_ARCH" in
  Linux/X64) target=x86_64-unknown-linux-musl ext=tar.gz ;;
  Windows/X64) target=x86_64-pc-windows-msvc ext=zip ;;
  macOS/ARM64) target=aarch64-apple-darwin ext=tar.gz ;;
  *)
    echo "::error::laconic-check has no release binary for $RUNNER_OS on $RUNNER_ARCH"
    exit 2
    ;;
esac
name="laconic-check-$target.$ext"
url="https://github.com/yungSNAT/Laconic/releases/download/laconic-check-v$VERSION"
dir="$RUNNER_TEMP/laconic-check-$VERSION"
mkdir -p "$dir"
cd "$dir"
curl -fsSL --retry 3 -o "$name" "$url/$name"
curl -fsSL --retry 3 -o "$name.sha256" "$url/$name.sha256"
expected=$(cut -d ' ' -f 1 "$name.sha256")
actual=$({ sha256sum "$name" 2>/dev/null || shasum -a 256 "$name"; } | cut -d ' ' -f 1)
if [ "$expected" != "$actual" ]; then
  echo "::error::$name does not match its published SHA-256"
  exit 1
fi
if [ "$ext" = zip ]; then unzip -oq "$name"; else tar -xzf "$name"; fi
echo "$dir" >> "$GITHUB_PATH"
