# agrf — conventions

Whitespace-separated numbers on stdin or argv, a braille sparkline on stdout.
One binary, one dependency (clap), no config file. Meant to sit at the end of a
pipe in a status bar or a `watch` loop.

- build: `nix develop --command cargo build --release --locked`
- test: `nix develop --command cargo test --locked`
- lint: `nix develop --command cargo clippy --all-targets --locked -- -D warnings`
- man: `nix develop --command groff -man -Tutf8 -ww -z man/man1/agrf.1`

Install the hook once per clone: `git config core.hooksPath hooks`. It lints
the staged tree, not the working tree, in `target/pre-commit/` with a target
dir of its own — see the note in the hook about why that path must be fixed
and separate.

## Layout

- `render.rs` — everything: tokens -> pixel grid -> string. `pixels` builds the
  grid at whatever cell size the charset wants; `braille` and `blocks` are the
  two conversions out of it. Pure, no I/O, so the whole drawing contract is
  testable from `#[cfg(test)]`.
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
- **The pre-commit hook builds in a fixed directory with its own
  `CARGO_TARGET_DIR`.** `env!("CARGO_BIN_EXE_agrf")` in `tests/` is resolved at
  compile time, so building the suite under `mktemp -d` bakes in a path that is
  deleted seconds later; sharing the working tree's `target/` then serves that
  dead binary to the next `cargo test`, which fails with a bare `No such file
  or directory` and no hint of where it came from.
- **`--follow` redraws in place only on a terminal.** Down a pipe the cursor
  escapes would land in the reader's data, so the frames are emitted plainly
  instead. The check is `IsTerminal` on stdout, decided once rather than per
  frame.
- **The follow buffer is trimmed to the window on every line.** Dropping values
  that have scrolled off is not tidiness — it is the difference between a tool
  that can watch a stream for a week and one that is an OOM with a countdown.
- **The block glyphs can only fill from the bottom, so `--point` has no block
  rendering.** U+2581..U+2588 are eighths measured up from the baseline; there
  is no glyph for a lone mark part way up a cell. The combination is refused at
  the argument boundary rather than approximated, because the nearest drawable
  thing is a bar, and a bar is the one shape point mode exists to avoid.
- **An auto floor over a window with no finite values is degenerate, not an
  error.** The fold seeds at `+inf`, so the span comes out non-finite and lands
  in the same blank-graph path as having no data — which is why that path is
  tested rather than guarded twice.
- **`rust-version` in Cargo.toml and `toolchain:` in the msrv CI job move in
  the same commit.** Nothing checks that they agree. Raise the floor alone and
  the job keeps passing on the old toolchain — it still exists and still builds
  — so it goes on verifying a floor the crate no longer claims, which is the
  one failure that job exists to prevent.
- **`width * height` is capped at the argument boundary, before render.** The
  grid is allocated eagerly, so an unchecked product is an OOM (or a `usize`
  overflow) driven straight from argv.
