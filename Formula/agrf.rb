class Agrf < Formula
  desc "Numbers in, braille sparkline out"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.3.2.tar.gz"
  sha256 "1822fa19a5b732455bb2f70541faf3f55c7eb3606eacdeba9430219ea4b0d2a2"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    man1.install "man/man1/agrf.1"
  end

  test do
    # two max-value columns fill a whole braille cell
    assert_equal "⣿", pipe_output("#{bin}/agrf -w 1 -m 1", "1 1\n").strip
  end
end
