class Agrf < Formula
  desc "Stream of int/float to unicode braille graphs"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0a7df7a97066bc6dd7b35a1836670904f43414d10714e936573bc55d5cae457c"
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
