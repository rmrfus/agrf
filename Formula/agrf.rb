class Agrf < Formula
  desc "Numbers in, braille sparkline out"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.4.1.tar.gz"
  sha256 "3091e6a1245e665df2b73ffcafd7ef09ea24cba3935fe18d43876f71e5de5224"
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
