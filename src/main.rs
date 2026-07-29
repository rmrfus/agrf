use std::io::{self, Read};

use clap::Parser;

mod render;
use render::Opts;

/// Stream of int/float -> unicode braille graph.
///
/// Reads whitespace-separated numbers from positional args (if any) or stdin
/// and draws them as a compact braille sparkline/barchart.
#[derive(Parser)]
#[command(version, about, long_about = None)]
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

    print!("{}", render::render(&values, &opts));
}
