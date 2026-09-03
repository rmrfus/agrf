use std::io::{self, Read, Write};
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
    let opts = Opts {
        width: args.width,
        height: args.height,
        min: args.min,
        max: args.max,
        point: args.point,
    };

    // Write directly so a closed pipe (e.g. `agrf | head`) exits cleanly
    // instead of panicking — the print! macro unwraps the EPIPE write error.
    let out = match args.charset {
        Charset::Braille => render::braille(&values, &opts),
        Charset::Blocks => render::blocks(&values, &opts),
    };
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
