class Agrf < Formula
  desc "Stream of int/float to unicode braille graphs"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.3.0.tar.gz"
  sha256 "4074e2f798f8970c06fc2bb666339ae7adf0775529a9d408ae9621313232090f"
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
