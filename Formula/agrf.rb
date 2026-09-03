class Agrf < Formula
  desc "Numbers in, braille sparkline out"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.3.3.tar.gz"
  sha256 "92af963af9a4c0c8fda61a7a7790b38920e8e6c55e13d8c6f1b601ff19ec5ec6"
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
