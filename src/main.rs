use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::process::ExitCode;

use clap::Parser;

mod render;
use render::{Floor, Opts};

/// Stream of int/float -> unicode braille graph.
///
/// Reads whitespace-separated numbers from positional args (if any) or stdin
/// and draws them as a compact braille sparkline/barchart.
#[derive(Parser)]
#[command(version, about, long_about = None, allow_negative_numbers = true)]
struct Args {
    /// Values to plot; if omitted, read from stdin (whitespace-separated).
    values: Vec<String>,

    /// Output width in characters (holds width*2 values; width with blocks).
    #[arg(short, long, default_value_t = 20)]
    width: usize,

    /// Output height in characters (4 px per char; 8 with blocks).
    #[arg(short = 'H', long, default_value_t = 1)]
    height: usize,

    /// Y-axis minimum, or `auto` for the window's own minimum; values below
    /// are clamped.
    #[arg(long, default_value = "0", value_parser = parse_floor)]
    min: Floor,

    /// Y-axis maximum; values above are clamped. Default: auto (window max).
    #[arg(short, long)]
    max: Option<f64>,

    /// Point mode: draw only the marker at each value, no fill.
    #[arg(short, long)]
    point: bool,

    /// Glyphs to draw with: braille packs two values per character, blocks
    /// are the fallback where the font has no braille.
    #[arg(long, default_value = "braille", value_parser = parse_charset)]
    charset: Charset,

    /// Follow stdin: redraw after every line instead of waiting for EOF.
    #[arg(short, long)]
    follow: bool,
}

/// Which glyph family draws the graph.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Charset {
    Braille,
    Blocks,
}

/// `auto`, or any finite number. A non-finite floor is rejected rather than
/// accepted: it makes the span non-finite, and the graph would come out blank
/// with no indication of why.
fn parse_floor(s: &str) -> Result<Floor, String> {
    if s.eq_ignore_ascii_case("auto") {
        return Ok(Floor::Auto);
    }
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(Floor::Fixed(v)),
        _ => Err(format!("expected a finite number or `auto`, got `{s}`")),
    }
}

fn parse_charset(s: &str) -> Result<Charset, String> {
    match s {
        "braille" => Ok(Charset::Braille),
        "blocks" => Ok(Charset::Blocks),
        _ => Err(format!("expected `braille` or `blocks`, got `{s}`")),
    }
}

// ExitCode rather than process::exit: exit terminates on the spot and skips
// every destructor still on the stack — including the stdout lock held below.
fn main() -> ExitCode {
    let args = Args::parse();

    // Bound the grid at the input boundary so render can't overflow usize or
    // allocate absurdly. Capping the product covers both a huge single side and
    // a huge width*height together (either way ~4M cells is far past any tty).
    const MAX_CELLS: usize = 4_000_000;
    if args
        .width
        .checked_mul(args.height)
        .is_none_or(|c| c > MAX_CELLS)
    {
        eprintln!("agrf: width*height must be <= {MAX_CELLS}");
        // 2 is what clap exits with for a bad argument, and what the EXIT
        // STATUS section of the man page promises for one.
        return ExitCode::from(2);
    }

    // U+2581..U+2588 fill from the bottom and cannot place a lone marker part
    // way up a cell, so this combination has no honest rendering. Refusing it
    // beats drawing a bar and calling it a point.
    if args.point && args.charset == Charset::Blocks {
        eprintln!("agrf: --point needs braille; blocks can only fill from the bottom");
        return ExitCode::from(2);
    }

    // Following means reading forever, which positional values cannot do.
    // Silently ignoring the flag would leave the user waiting for a redraw
    // that is never coming.
    if args.follow && !args.values.is_empty() {
        eprintln!("agrf: --follow reads stdin; it cannot be combined with positional values");
        return ExitCode::from(2);
    }

    let opts = Opts {
        width: args.width,
        height: args.height,
        min: args.min,
        max: args.max,
        point: args.point,
    };

    if args.follow {
        return follow(&opts, args.charset);
    }

    let input = if args.values.is_empty() {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("agrf: failed to read stdin: {e}");
            return ExitCode::FAILURE;
        }
        buf
    } else {
        args.values.join(" ")
    };

    let values = render::parse(&input);

    // Write directly so a closed pipe (e.g. `agrf | head`) exits cleanly
    // instead of panicking — the print! macro unwraps the EPIPE write error.
    let out = draw(&values, &opts, args.charset);
    let mut stdout = io::stdout().lock();
    let res = stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush());
    if let Err(e) = res {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("agrf: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn draw(values: &[Option<f64>], opts: &Opts, charset: Charset) -> String {
    match charset {
        Charset::Braille => render::braille(values, opts),
        Charset::Blocks => render::blocks(values, opts),
    }
}

/// Redraw after every line of stdin, keeping only the values that still fit.
///
/// On a terminal each frame overwrites the last, by walking the cursor back up
/// `height` rows; anywhere else the frames are simply emitted one after
/// another, because a pipe has no cursor to move and a reader wants the
/// history. The trailing values are dropped as they scroll out of the window,
/// so following an endless stream does not grow the buffer.
fn follow(opts: &Opts, charset: Charset) -> ExitCode {
    let cell_w = match charset {
        Charset::Braille => render::BRAILLE_CELL.0,
        Charset::Blocks => render::BLOCKS_CELL.0,
    };
    let capacity = opts.width * cell_w;

    let mut values: Vec<Option<f64>> = Vec::new();
    let mut out = io::stdout().lock();
    // Decided once: the handle is the same for every frame, and asking per
    // frame would only cost syscalls.
    let overwrite = out.is_terminal();
    let mut drawn = false;

    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("agrf: failed to read stdin: {e}");
                return ExitCode::FAILURE;
            }
        };

        let batch = render::parse(&line);
        // A blank or non-numeric line adds nothing; redrawing the same frame
        // would just make a terminal flicker.
        if batch.is_empty() {
            continue;
        }
        values.extend(batch);
        if values.len() > capacity {
            values.drain(..values.len() - capacity);
        }

        let mut frame = String::new();
        if overwrite && drawn {
            // CUU: back to the first row of the frame just drawn. Every frame
            // is the same width, so the old one is fully covered — except when
            // the graph is wider than the terminal and the lines wrapped, which
            // no escape can repair without knowing the window size.
            frame.push_str(&format!("\x1b[{}A", opts.height));
        }
        frame.push_str(&draw(&values, opts, charset));

        let res = out.write_all(frame.as_bytes()).and_then(|()| out.flush());
        if let Err(e) = res {
            // Reader went away — the same clean exit as the one-shot path.
            if e.kind() == io::ErrorKind::BrokenPipe {
                return ExitCode::SUCCESS;
            }
            eprintln!("agrf: {e}");
            return ExitCode::FAILURE;
        }
        drawn = true;
    }
    ExitCode::SUCCESS
}
