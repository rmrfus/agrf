//! Pure rendering: numeric tokens -> braille pixel grid -> string.
//!
//! A braille cell (U+2800 base) is 2 px wide x 4 px tall. Dot numbering:
//!
//! ```text
//!   1 4      dy=0 (top)
//!   2 5      dy=1
//!   3 6      dy=2
//!   7 8      dy=3 (bottom)
//! ```
//!
//! The bit for each (dy, dx) — verified against real glyphs.

/// Bit contributed by the pixel at (row-in-cell dy, col-in-cell dx).
const BIT: [[u8; 2]; 4] = [
    [0x01, 0x08], // dots 1, 4
    [0x02, 0x10], // dots 2, 5
    [0x04, 0x20], // dots 3, 6
    [0x40, 0x80], // dots 7, 8
];

/// Render options. `width`/`height` are in characters.
pub struct Opts {
    pub width: usize,
    pub height: usize,
    pub max: Option<f64>,
    pub point: bool,
}

/// Split input on whitespace and parse each token as f64.
/// Non-numeric or non-finite (nan/inf) tokens become `None` — a gap that
/// still occupies its x-column but draws nothing.
pub fn parse(input: &str) -> Vec<Option<f64>> {
    input
        .split_whitespace()
        .map(|tok| tok.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect()
}

/// Render values into a braille graph string (one `\n`-terminated line per
/// character row, top row first).
pub fn render(values: &[Option<f64>], opts: &Opts) -> String {
    let rows_px = opts.height * 4;
    let cols_px = opts.width * 2;

    // Degenerate size: still emit the requested number of (empty) lines.
    if rows_px == 0 {
        return "\n".repeat(opts.height);
    }

    // Keep only the last `cols_px` values; they sit left-aligned, blank right.
    let window: &[Option<f64>] = if values.len() > cols_px {
        &values[values.len() - cols_px..]
    } else {
        values
    };

    // Y scale: explicit --max, else the window's max (negatives count as 0).
    // Clamped to >= 0 so a degenerate scale just yields an empty/baseline graph.
    let ymax = opts.max.unwrap_or_else(|| {
        window
            .iter()
            .filter_map(|v| v.map(|x| x.max(0.0)))
            .fold(0.0, f64::max)
    });
    let ymax = if ymax.is_finite() && ymax > 0.0 {
        ymax
    } else {
        0.0
    };

    // Pixel grid, row 0 = top.
    let mut grid = vec![vec![false; cols_px]; rows_px];

    for (c, cell) in window.iter().enumerate() {
        let v = match cell {
            Some(v) => *v,
            None => continue, // gap: leave the column empty
        };
        // Negatives clamp to 0; anything above ymax clamps to ymax.
        let frac = if ymax > 0.0 {
            v.clamp(0.0, ymax) / ymax
        } else {
            0.0
        };

        if opts.point {
            // Single marker: 0 -> bottom pixel row, ymax -> top pixel row.
            let up = (frac * (rows_px - 1) as f64).round() as usize;
            let row = rows_px - 1 - up.min(rows_px - 1);
            grid[row][c] = true;
        } else {
            // Bar: fill from the bottom. Any positive value shows >= 1 pixel.
            let mut px = (frac * rows_px as f64).round() as usize;
            if ymax > 0.0 && v > 0.0 && px == 0 {
                px = 1;
            }
            for k in 0..px.min(rows_px) {
                grid[rows_px - 1 - k][c] = true;
            }
        }
    }

    // Pixel grid -> braille characters.
    let mut out = String::with_capacity(opts.height * (opts.width + 1));
    for cr in 0..opts.height {
        for cc in 0..opts.width {
            let mut bits = 0u8;
            for dy in 0..4 {
                for dx in 0..2 {
                    if grid[cr * 4 + dy][cc * 2 + dx] {
                        bits |= BIT[dy][dx];
                    }
                }
            }
            // 0x2800..=0x28FF is always a valid char; unwrap can't fail.
            out.push(char::from_u32(0x2800 + bits as u32).unwrap());
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(width: usize, height: usize, max: Option<f64>, point: bool) -> Opts {
        Opts {
            width,
            height,
            max,
            point,
        }
    }

    #[test]
    fn parse_swallows_non_numeric_and_non_finite() {
        let v = parse("1 2 x -3 nan inf 4.5");
        assert_eq!(
            v,
            vec![
                Some(1.0),
                Some(2.0),
                None,
                Some(-3.0),
                None,
                None,
                Some(4.5)
            ]
        );
    }

    #[test]
    fn full_bars_fill_the_cell() {
        // Two full-height bars in one char -> all 8 dots.
        let out = render(&[Some(1.0), Some(1.0)], &o(1, 1, Some(1.0), false));
        assert_eq!(out, "⣿\n");
    }

    #[test]
    fn half_bar_fills_bottom_two_rows() {
        // value 2 of max 4 -> 2 px of 4 -> lower half.
        let out = render(&[Some(2.0), Some(2.0)], &o(1, 1, Some(4.0), false));
        assert_eq!(out, "⣤\n");
    }

    #[test]
    fn tiny_value_shows_one_pixel() {
        // 1 of 100 rounds to 0 px, but min-1px keeps it visible (bottom row).
        let out = render(&[Some(1.0), Some(1.0)], &o(1, 1, Some(100.0), false));
        assert_eq!(out, "⣀\n");
    }

    #[test]
    fn exact_zero_stays_empty_in_bar_mode() {
        let out = render(&[Some(0.0), Some(0.0)], &o(1, 1, Some(100.0), false));
        assert_eq!(out, "⠀\n"); // U+2800, blank braille
    }

    #[test]
    fn negatives_clamp_to_zero() {
        let out = render(&[Some(-5.0), Some(-5.0)], &o(1, 1, Some(10.0), false));
        assert_eq!(out, "⠀\n");
    }

    #[test]
    fn window_keeps_last_values() {
        // cap = width*2 = 2; only the last two (both max) are drawn.
        let out = render(
            &[Some(0.0), Some(0.0), Some(1.0), Some(1.0)],
            &o(1, 1, Some(1.0), false),
        );
        assert_eq!(out, "⣿\n");
    }

    #[test]
    fn underfill_is_left_aligned_blank_right() {
        // One full value in a 2-char graph -> left char left-column filled,
        // rest blank.
        let out = render(&[Some(1.0)], &o(2, 1, Some(1.0), false));
        assert_eq!(out, "⡇⠀\n");
    }

    #[test]
    fn non_numeric_leaves_a_gap() {
        // col0 full, col1 gap (None), col2 full -> ⡇ then ⡇ (gap = right px of
        // first char stays empty).
        let out = render(&[Some(1.0), None, Some(1.0)], &o(2, 1, Some(1.0), false));
        assert_eq!(out, "⡇⡇\n");
    }

    #[test]
    fn point_zero_is_bottom_max_is_top() {
        let bottom = render(&[Some(0.0), Some(0.0)], &o(1, 1, Some(1.0), true));
        assert_eq!(bottom, "⣀\n"); // dots 7,8
        let top = render(&[Some(1.0), Some(1.0)], &o(1, 1, Some(1.0), true));
        assert_eq!(top, "⠉\n"); // dots 1,4
    }

    #[test]
    fn empty_input_yields_blank_grid() {
        let out = render(&[], &o(3, 1, None, false));
        assert_eq!(out, "⠀⠀⠀\n");
    }
}
