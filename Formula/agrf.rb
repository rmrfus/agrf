class Agrf < Formula
  desc "Numbers in, braille sparkline out"
  homepage "https://github.com/rmrfus/agrf"
  url "https://github.com/rmrfus/agrf/archive/refs/tags/v0.4.0.tar.gz"
  sha256 "7d9d10c7e24736606fc10de40581acbb5f6a31850749f135455f42e3d994531e"
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
