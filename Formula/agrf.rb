class Agrf < Formula
  desc "Stream of int/float to unicode braille graphs"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "7f0d69b39bcb67d3321dbbf0381f1ff1672b9c9382019bf9b8b8be6f086f7be7"
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
