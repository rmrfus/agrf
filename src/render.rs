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
//!
//! The block charset draws the same pixel grid at one column by eight rows per
//! character, using U+2581..U+2588. Those glyphs can only express a fill from
//! the bottom, which is why point mode has no block rendering.

/// Pixels per output character, (width, height). One value occupies one pixel
/// column, so the first number is also how many values a character holds —
/// which is what a caller needs to size a sliding window.
pub const BRAILLE_CELL: (usize, usize) = (2, 4);
/// See [`BRAILLE_CELL`]. Blocks trade horizontal resolution for vertical.
pub const BLOCKS_CELL: (usize, usize) = (1, 8);

/// Bit contributed by the pixel at (row-in-cell dy, col-in-cell dx).
const BIT: [[u8; 2]; 4] = [
    [0x01, 0x08], // dots 1, 4
    [0x02, 0x10], // dots 2, 5
    [0x04, 0x20], // dots 3, 6
    [0x40, 0x80], // dots 7, 8
];

/// Lower bound of the Y axis.
#[derive(Debug, Clone, Copy)]
pub enum Floor {
    /// A fixed value. `Fixed(0.0)` is the default and keeps a bar's height
    /// proportional to its value.
    Fixed(f64),
    /// The window's own minimum, which spends the whole cell on the variation
    /// rather than on the distance from zero.
    Auto,
}

/// Render options. `width`/`height` are in characters.
pub struct Opts {
    pub width: usize,
    pub height: usize,
    pub min: Floor,
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

/// Build the pixel grid, `cell_w` x `cell_h` pixels per output character.
/// Row 0 is the top. Returns an empty grid when no rows were asked for.
fn pixels(values: &[Option<f64>], opts: &Opts, cell_w: usize, cell_h: usize) -> Vec<Vec<bool>> {
    let rows_px = opts.height * cell_h;
    let cols_px = opts.width * cell_w;
    if rows_px == 0 {
        return Vec::new();
    }

    // Keep only the last `cols_px` values; they sit left-aligned, blank right.
    let window: &[Option<f64>] = if values.len() > cols_px {
        &values[values.len() - cols_px..]
    } else {
        values
    };

    // Y range [ymin, ymax]: ymax defaults to the window's peak. Values outside
    // the range clamp to it. A non-positive or non-finite span (no data, or
    // min >= max) is degenerate -> empty/baseline graph. An auto floor over an
    // empty window yields +inf, which lands in that same degenerate case.
    let ymin = match opts.min {
        Floor::Fixed(v) => v,
        Floor::Auto => window
            .iter()
            .filter_map(|v| *v)
            .fold(f64::INFINITY, f64::min),
    };
    let ymax = opts.max.unwrap_or_else(|| {
        window
            .iter()
            .filter_map(|v| *v)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let span = ymax - ymin;
    let scaled = span.is_finite() && span > 0.0;

    let mut grid = vec![vec![false; cols_px]; rows_px];

    for (c, cell) in window.iter().enumerate() {
        // gap: leave the column empty
        let Some(v) = *cell else { continue };
        // Clamp into [ymin, ymax] and map to a 0..1 fraction of the range.
        let frac = if scaled {
            (v.clamp(ymin, ymax) - ymin) / span
        } else {
            0.0
        };

        if opts.point {
            // Single marker: ymin -> bottom pixel row, ymax -> top pixel row.
            let up = (frac * (rows_px - 1) as f64).round() as usize;
            let row = rows_px - 1 - up.min(rows_px - 1);
            grid[row][c] = true;
        } else {
            // Bar: fill from the bottom. Any value above the floor shows >= 1 px.
            let mut px = (frac * rows_px as f64).round() as usize;
            if scaled && v > ymin && px == 0 {
                px = 1;
            }
            for k in 0..px.min(rows_px) {
                grid[rows_px - 1 - k][c] = true;
            }
        }
    }
    grid
}

/// Render values as braille (one `\n`-terminated line per character row, top
/// row first). 2 px wide x 4 px tall per character, so a graph `width`
/// characters across holds `2 * width` values.
pub fn braille(values: &[Option<f64>], opts: &Opts) -> String {
    let grid = pixels(values, opts, BRAILLE_CELL.0, BRAILLE_CELL.1);
    if grid.is_empty() {
        return "\n".repeat(opts.height);
    }

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
            // bits is a u8, so the sum stays inside 0x2800..=0x28FF, which is
            // entirely assigned and holds no surrogate — from_u32 cannot
            // return None here. #[expect] rather than #[allow] so the day this
            // stops being an unwrap, clippy says the attribute is now dead.
            #[expect(clippy::unwrap_used, reason = "value is a proven-valid scalar")]
            out.push(char::from_u32(0x2800 + bits as u32).unwrap());
        }
        out.push('\n');
    }
    out
}

/// Render values as block eighths — the fallback for a terminal whose font has
/// no braille. 1 px wide x 8 px tall per character, so a graph `width`
/// characters across holds `width` values: half the horizontal resolution of
/// braille for twice the vertical.
///
/// Bar mode only. U+2581..U+2588 can express a fill from the bottom and
/// nothing else, so a lone marker has no glyph — `point` is rejected before
/// this is called.
pub fn blocks(values: &[Option<f64>], opts: &Opts) -> String {
    let grid = pixels(values, opts, BLOCKS_CELL.0, BLOCKS_CELL.1);
    if grid.is_empty() {
        return "\n".repeat(opts.height);
    }

    let mut out = String::with_capacity(opts.height * (opts.width + 1));
    for cr in 0..opts.height {
        // The eight pixel rows this character row is made of. The glyph is a
        // fill from the bottom, so a column's level is just how many of them
        // are set — the bar is contiguous by construction.
        let cell = &grid[cr * 8..(cr + 1) * 8];
        for cc in 0..opts.width {
            let filled = cell.iter().filter(|row| row[cc]).count();
            if filled == 0 {
                // A space, not U+2588's empty sibling: there isn't one. The
                // braille blank (U+2800) is a glyph, this column is nothing.
                out.push(' ');
            } else {
                // 1..=8 eighths -> U+2581..=U+2588, all assigned, so from_u32
                // cannot return None. See the note in `braille` above.
                #[expect(clippy::unwrap_used, reason = "value is a proven-valid scalar")]
                out.push(char::from_u32(0x2580 + filled as u32).unwrap());
            }
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
            min: Floor::Fixed(0.0),
            max,
            point,
        }
    }

    fn band(min: f64, max: f64) -> Opts {
        Opts {
            width: 1,
            height: 1,
            min: Floor::Fixed(min),
            max: Some(max),
            point: false,
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
        let out = braille(&[Some(1.0), Some(1.0)], &o(1, 1, Some(1.0), false));
        assert_eq!(out, "⣿\n");
    }

    #[test]
    fn half_bar_fills_bottom_two_rows() {
        // value 2 of max 4 -> 2 px of 4 -> lower half.
        let out = braille(&[Some(2.0), Some(2.0)], &o(1, 1, Some(4.0), false));
        assert_eq!(out, "⣤\n");
    }

    #[test]
    fn tiny_value_shows_one_pixel() {
        // 1 of 100 rounds to 0 px, but min-1px keeps it visible (bottom row).
        let out = braille(&[Some(1.0), Some(1.0)], &o(1, 1, Some(100.0), false));
        assert_eq!(out, "⣀\n");
    }

    #[test]
    fn exact_zero_stays_empty_in_bar_mode() {
        let out = braille(&[Some(0.0), Some(0.0)], &o(1, 1, Some(100.0), false));
        assert_eq!(out, "⠀\n"); // U+2800, blank braille
    }

    #[test]
    fn negatives_clamp_to_zero() {
        let out = braille(&[Some(-5.0), Some(-5.0)], &o(1, 1, Some(10.0), false));
        assert_eq!(out, "⠀\n");
    }

    #[test]
    fn window_keeps_last_values() {
        // cap = width*2 = 2; only the last two (both max) are drawn.
        let out = braille(
            &[Some(0.0), Some(0.0), Some(1.0), Some(1.0)],
            &o(1, 1, Some(1.0), false),
        );
        assert_eq!(out, "⣿\n");
    }

    #[test]
    fn underfill_is_left_aligned_blank_right() {
        // One full value in a 2-char graph -> left char left-column filled,
        // rest blank.
        let out = braille(&[Some(1.0)], &o(2, 1, Some(1.0), false));
        assert_eq!(out, "⡇⠀\n");
    }

    #[test]
    fn non_numeric_leaves_a_gap() {
        // col0 full, col1 gap (None), col2 full -> ⡇ then ⡇ (gap = right px of
        // first char stays empty).
        let out = braille(&[Some(1.0), None, Some(1.0)], &o(2, 1, Some(1.0), false));
        assert_eq!(out, "⡇⡇\n");
    }

    #[test]
    fn point_zero_is_bottom_max_is_top() {
        let bottom = braille(&[Some(0.0), Some(0.0)], &o(1, 1, Some(1.0), true));
        assert_eq!(bottom, "⣀\n"); // dots 7,8
        let top = braille(&[Some(1.0), Some(1.0)], &o(1, 1, Some(1.0), true));
        assert_eq!(top, "⠉\n"); // dots 1,4
    }

    #[test]
    fn empty_input_yields_blank_grid() {
        let out = braille(&[], &o(3, 1, None, false));
        assert_eq!(out, "⠀⠀⠀\n");
    }

    #[test]
    fn min_shifts_the_floor() {
        // Range [40, 60]: 40 sits on the floor (empty), 50 half, 60 full.
        assert_eq!(braille(&[Some(40.0), Some(40.0)], &band(40.0, 60.0)), "⠀\n");
        assert_eq!(braille(&[Some(50.0), Some(50.0)], &band(40.0, 60.0)), "⣤\n");
        assert_eq!(braille(&[Some(60.0), Some(60.0)], &band(40.0, 60.0)), "⣿\n");
    }

    fn auto_floor(width: usize) -> Opts {
        Opts {
            width,
            height: 1,
            min: Floor::Auto,
            max: None,
            point: false,
        }
    }

    #[test]
    fn auto_floor_starts_the_scale_at_the_window_minimum() {
        // 40..60 with a zero floor spends the cell on the distance from zero;
        // with an auto floor the same series uses the full height.
        // Floor 0: 40 of 60 is 3 px of 4, 60 is 4 px -> both columns tall.
        let fixed = braille(&[Some(40.0), Some(60.0)], &o(1, 1, None, false));
        assert_eq!(fixed, "⣾\n");
        // Floor 40: the first value sits on the floor and vanishes, the second
        // gets the whole cell.
        let auto = braille(&[Some(40.0), Some(60.0)], &auto_floor(1));
        assert_eq!(auto, "⢸\n");
    }

    #[test]
    fn auto_floor_on_a_flat_series_draws_nothing() {
        // Every value equals the floor, so the span is zero: there is no
        // variation to show, and a full cell would be a lie.
        let out = braille(&[Some(7.0), Some(7.0)], &auto_floor(1));
        assert_eq!(out, "⠀\n");
    }

    #[test]
    fn auto_floor_over_an_empty_window_stays_blank() {
        // The fold seeds at +inf with nothing to reduce, so the span is
        // non-finite — the same degenerate path as having no data at all.
        let out = braille(&[], &auto_floor(2));
        assert_eq!(out, "⠀⠀\n");
    }

    #[test]
    fn blocks_fill_one_eighth_per_level() {
        for (value, expect) in [(0.0, " "), (1.0, "▁"), (4.0, "▄"), (8.0, "█")] {
            let out = blocks(&[Some(value)], &o(1, 1, Some(8.0), false));
            assert_eq!(out, format!("{expect}\n"), "value {value}");
        }
    }

    #[test]
    fn blocks_hold_one_value_per_character_not_two() {
        // Braille packs two columns into a cell; blocks are one column wide,
        // so the same width holds half as many values and the window keeps
        // the last two rather than the last four.
        let out = blocks(
            &[Some(8.0), Some(8.0), Some(0.0), Some(4.0)],
            &o(2, 1, Some(8.0), false),
        );
        assert_eq!(out, " ▄\n");
    }

    #[test]
    fn below_min_clamps_to_floor() {
        // 30 is below the floor of 40 -> clamps to the floor -> empty.
        assert_eq!(braille(&[Some(30.0), Some(30.0)], &band(40.0, 60.0)), "⠀\n");
    }
}
