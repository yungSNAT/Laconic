class LaconicCheck < Formula
  desc "Flags Australian English and house-style candidates in Markdown prose"
  homepage "https://github.com/yungSNAT/Laconic"
  version "1.2.3"
  license "FSL-1.1-ALv2"

  on_macos do
    on_arm do
      url "https://github.com/yungSNAT/Laconic/releases/download/laconic-check-v1.2.3/laconic-check-aarch64-apple-darwin.tar.gz"
      sha256 "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/yungSNAT/Laconic/releases/download/laconic-check-v1.2.3/laconic-check-x86_64-unknown-linux-musl.tar.gz"
      sha256 "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    end
  end

  def install
    bin.install "laconic-check"
  end

  test do
    assert_match "us-spelling", pipe_output("#{bin}/laconic-check lint -", "The color.\n", 1)
  end
end
