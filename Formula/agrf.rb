class Agrf < Formula
  desc "Stream of int/float to unicode braille graphs"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.3.1.tar.gz"
  sha256 "116b2721ff9c5b689b1909ad7405ad5d40ccfd89c1458e343e2151ade58e5589"
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
