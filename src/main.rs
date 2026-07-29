use std::io::{self, Read, Write};

use clap::Parser;

mod render;
use render::Opts;

/// Stream of int/float -> unicode braille graph.
///
/// Reads whitespace-separated numbers from positional args (if any) or stdin
/// and draws them as a compact braille sparkline/barchart.
#[derive(Parser)]
#[command(version, about, long_about = None, allow_negative_numbers = true)]
struct Args {
    /// Values to plot; if omitted, read from stdin (whitespace-separated).
    values: Vec<String>,

    /// Output width in characters (holds width*2 values).
    #[arg(short, long, default_value_t = 20)]
    width: usize,

    /// Output height in characters (4 px per char).
    #[arg(short = 'H', long, default_value_t = 1)]
    height: usize,

    /// Y-axis maximum; values above are clamped. Default: auto (window max).
    #[arg(short, long)]
    max: Option<f64>,

    /// Point mode: draw only the marker at each value, no fill.
    #[arg(short, long)]
    point: bool,
}

fn main() {
    let args = Args::parse();

    // Bound the grid at the input boundary so render can't overflow usize or
    // attempt an absurd allocation on a fat-fingered -w/-H.
    const MAX_DIM: usize = 100_000;
    if args.width > MAX_DIM || args.height > MAX_DIM {
        eprintln!("agrf: --width/--height must be <= {MAX_DIM}");
        std::process::exit(2);
    }

    let input = if args.values.is_empty() {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("agrf: failed to read stdin: {e}");
            std::process::exit(1);
        }
        buf
    } else {
        args.values.join(" ")
    };

    let values = render::parse(&input);
    let opts = Opts {
        width: args.width,
        height: args.height,
        max: args.max,
        point: args.point,
    };

    // Write directly so a closed pipe (e.g. `agrf | head`) exits cleanly
    // instead of panicking — the print! macro unwraps the EPIPE write error.
    let out = render::render(&values, &opts);
    let mut stdout = io::stdout().lock();
    let res = stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush());
    if let Err(e) = res {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("agrf: {e}");
            std::process::exit(1);
        }
    }
}
