# agrf — conventions

Whitespace-separated numbers on stdin or argv, a braille sparkline on stdout.
One binary, one dependency (clap), no config file. Meant to sit at the end of a
pipe in a status bar or a `watch` loop.

- build: `nix develop --command cargo build --release --locked`
- test: `nix develop --command cargo test --locked`
- lint: `nix develop --command cargo clippy --all-targets --locked -- -D warnings`
- man: `nix develop --command groff -man -Tutf8 -ww -z man/man1/agrf.1`

Install the hook once per clone: `git config core.hooksPath hooks`. It lints
the staged tree, not the working tree.

## Layout

- `render.rs` — everything: tokens -> pixel grid -> string. Pure, no I/O, so
  the whole drawing contract is testable from `#[cfg(test)]`.
- `main.rs` — argument parsing, stdin, and the write.

## Non-negotiables

- **Braille dots 7 and 8 are bits 0x40 and 0x80, not a continuation of the
  1..6 pattern.** The 6-dot cell came first and the 8-dot extension was bolted
  on above the existing bits, so a `BIT` table filled in by pattern-matching
  renders plausible-looking garbage rather than failing. The table is verified
  against real glyphs, and the tests assert on literal braille characters for
  exactly that reason.
- **Any value above the floor draws at least one pixel.** Rounding alone maps
  1-of-100 to zero pixels, which is indistinguishable from an all-zero series —
  the one case a monitoring sparkline exists to catch.
- **A value at or below `--min` draws nothing.** That is what keeps "nothing"
  distinguishable from "a little", and it is why the one-pixel floor above is
  conditioned on `v > ymin` rather than applied to everything.
- **A non-numeric or non-finite token becomes a gap that still occupies its
  column.** Dropping it instead would shift every later column left and quietly
  misalign the whole graph against time — a silently wrong picture, which is
  worse than a hole in it.
- **The window keeps the *last* `2*width` values, drawn left to right.** A
  short series is left-aligned with blanks on the right; a long one loses its
  head, not its tail. Anything else means a live stream stops updating once it
  fills the width.
- **Output goes through `write_all` + `flush`, with `BrokenPipe` as a clean
  exit.** `agrf | head` is normal usage, and the `print!` family unwraps the
  EPIPE write error into a panic.
- **`rust-version` in Cargo.toml and `toolchain:` in the msrv CI job move in
  the same commit.** Nothing checks that they agree. Raise the floor alone and
  the job keeps passing on the old toolchain — it still exists and still builds
  — so it goes on verifying a floor the crate no longer claims, which is the
  one failure that job exists to prevent.
- **`width * height` is capped at the argument boundary, before render.** The
  grid is allocated eagerly, so an unchecked product is an OOM (or a `usize`
  overflow) driven straight from argv.
