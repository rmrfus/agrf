//! End-to-end checks for the behaviour that only exists in `main`: the follow
//! loop and the argument combinations it refuses. Everything about the drawing
//! itself is unit-tested next to `render`.
//!
//! No `#![allow]` at the top: the clippy test exemptions follow the `#[test]`
//! marker, so they cover these functions. The helper below is not a test
//! function, which is why it returns `Result` instead of unwrapping.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

/// Run the binary with `args`, feed it `stdin`, and collect everything.
/// stdout is a pipe here, never a terminal, which is the case the tests below
/// depend on.
fn run(args: &[&str], stdin: &str) -> io::Result<Run> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agrf"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut pipe = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("child stdin was not piped"))?;
    pipe.write_all(stdin.as_bytes())?;
    drop(pipe); // EOF, or the child follows forever

    let out = child.wait_with_output()?;
    Ok(Run {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code(),
    })
}

#[test]
fn follow_draws_a_frame_per_line_rather_than_one_at_eof() -> io::Result<()> {
    let got = run(&["-f", "-w", "4", "-m", "8"], "1\n4\n8\n")?;
    assert_eq!(got.code, Some(0), "stderr: {}", got.stderr);
    // Three input lines, three frames, each one character row tall.
    assert_eq!(got.stdout.lines().count(), 3, "output: {:?}", got.stdout);
    Ok(())
}

#[test]
fn follow_adds_each_line_to_the_running_window() -> io::Result<()> {
    let got = run(&["-f", "-w", "1", "-m", "8"], "0\n8\n")?;
    let frames: Vec<&str> = got.stdout.lines().collect();
    // One char holds two values: first frame has only the zero, second gains a
    // full-height bar in the right-hand column.
    assert_eq!(frames, vec!["⠀", "⢸"], "output: {:?}", got.stdout);
    Ok(())
}

#[test]
fn follow_forgets_values_that_scroll_out_of_the_window() -> io::Result<()> {
    // Far more values than the window holds; the last frame must show the tail,
    // not an accumulation of everything seen.
    let feed: String = (1..=200).map(|n| format!("{n}\n")).collect();
    let got = run(&["-f", "-w", "1", "-m", "200"], &feed)?;
    let last = got.stdout.lines().last().unwrap_or_default();
    assert_eq!(last, "⣿", "output tail: {:?}", last);
    Ok(())
}

#[test]
fn piped_follow_output_carries_no_cursor_escapes() -> io::Result<()> {
    // Redrawing in place is for a terminal. Down a pipe the frames are plain
    // lines, or anything reading them gets escape sequences in its data.
    let got = run(&["-f", "-w", "4"], "1\n2\n3\n")?;
    assert!(!got.stdout.contains('\u{1b}'), "output: {:?}", got.stdout);
    Ok(())
}

#[test]
fn follow_ignores_a_line_with_no_numbers_in_it() -> io::Result<()> {
    let got = run(&["-f", "-w", "4", "-m", "8"], "1\n\n\n8\n")?;
    // Two numeric lines, so two frames: blank lines must not redraw.
    assert_eq!(got.stdout.lines().count(), 2, "output: {:?}", got.stdout);
    Ok(())
}

#[test]
fn follow_with_positional_values_is_rejected_not_ignored() -> io::Result<()> {
    let got = run(&["-f", "1", "2", "3"], "")?;
    assert_eq!(got.code, Some(2));
    assert!(got.stderr.contains("--follow"), "stderr: {}", got.stderr);
    Ok(())
}

#[test]
fn point_mode_with_blocks_is_rejected_not_approximated() -> io::Result<()> {
    let got = run(&["-p", "--charset", "blocks", "1", "2"], "")?;
    assert_eq!(got.code, Some(2));
    assert!(got.stderr.contains("braille"), "stderr: {}", got.stderr);
    Ok(())
}

#[test]
fn non_finite_max_is_rejected_not_drawn_blank() -> io::Result<()> {
    // A non-finite ceiling makes the span non-finite and the graph comes out
    // blank with no indication of why — the same reason --min rejects one.
    // The `=` form: `-inf` through a space never reaches the parser, clap
    // reads it as short flags first (exit 2 either way, but a different
    // message).
    for max in ["--max=inf", "--max=-inf", "--max=NaN"] {
        let got = run(&[max, "1", "2"], "")?;
        assert_eq!(got.code, Some(2), "max={max}");
        assert!(
            got.stderr.contains("finite"),
            "max={max} stderr: {}",
            got.stderr
        );
    }
    Ok(())
}

#[test]
fn zero_width_or_height_is_rejected_not_drawn_empty() -> io::Result<()> {
    // A zero-sized graph draws nothing with a success exit, which reads as
    // "no data" rather than "bad arguments".
    for args in [vec!["-w", "0"], vec!["-H", "0"]] {
        let got = run(&args, "1 2 3\n")?;
        assert_eq!(got.code, Some(2), "args={args:?}");
        assert!(
            got.stderr.contains(">= 1"),
            "args={args:?} stderr: {}",
            got.stderr
        );
    }
    Ok(())
}

#[test]
fn a_closed_output_pipe_is_not_an_error() -> io::Result<()> {
    // `agrf | head` is normal usage; the documented exit status says 0. The
    // output has to exceed the pipe buffer (~64 KB) so the child actually
    // blocks on the write instead of finishing into the buffer before the read
    // end below is closed — a small frame would pass without ever seeing EPIPE.
    let mut child = Command::new(env!("CARGO_BIN_EXE_agrf"))
        .args(["-w", "1000", "-H", "100", "-m", "8"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut pipe = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("child stdin was not piped"))?;
    // 2000 values fill the 1000-char window; the frame itself is ~300 KB.
    pipe.write_all("8 ".repeat(2000).as_bytes())?;
    drop(pipe); // EOF

    // Reader goes away while the child is still writing: every further write
    // fails with EPIPE, which must be a clean exit rather than a panic.
    drop(child.stdout.take());
    let status = child.wait()?;

    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        err.read_to_string(&mut stderr)?;
    }
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
    assert!(!stderr.contains("panicked"), "stderr: {stderr}");
    Ok(())
}
