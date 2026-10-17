//! The CLI intro's frames as cells (Seamlessh `crates/cli/src/ui.rs`, `intro`):
//! dev-01 shaded in bits inside a ring with one packet orbiting, loose bits
//! beyond, every cell condensing out of grey noise from the centre outwards,
//! and the tagline condensing under it. Pure: the same clock always gives the
//! same frame, so the GPUI splash and the terminal intro agree.
//!
//! The numbers are the CLI's. Change them there first.

use super::mascot::{self, Pose};

/// How a cell is painted. Brightness carries the drawing; no hue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    Blank,
    /// A bit still in motion, or at rest outside the drawing: mid grey.
    Noise,
    /// A grey from 0 black to 1 white.
    Tone(f32),
}

pub type Cell = (char, Paint);

/// A repeatable 0..1 value for `n`: the same cell flickers the same way every run.
fn hash(n: f32) -> f32 {
    let x = n.sin() * 43_758.547;
    x - x.floor()
}

/// A bit for `n`: every moving cell is a 0 or a 1.
fn binary(n: f32) -> char {
    if hash(n) < 0.5 { '0' } else { '1' }
}

/// When cell `i` of `n` lands: the middle first, then outwards, each cell
/// late by its own hash. Every cell has landed by 0.56.
fn lands(i: usize, n: usize, seed: f32) -> f32 {
    let spread = if n > 1 {
        (i as f32 / (n - 1) as f32 - 0.5).abs() * 2.0
    } else {
        0.0
    };
    0.06 + spread * 0.28 + hash(i as f32 * 12.9898 + seed) * 0.22
}

/// `text` at `clock` seconds into condensing: landed cells as written in
/// `ink`, the rest as noise bits re-drawn 30 times a second.
pub fn condense(text: &str, clock: f32, seed: f32, ink: Paint) -> Vec<Cell> {
    let n = text.chars().count();
    let tick = (clock * 30.0).floor();
    text.chars()
        .enumerate()
        .map(|(i, ch)| {
            if clock >= lands(i, n, seed) {
                (ch, ink)
            } else {
                (binary(i as f32 * 3.7 + seed + tick * 0.71), Paint::Noise)
            }
        })
        .collect()
}

/// dev-01's width over its height in cells: about twice, as a cell is about
/// twice as tall as wide.
const ASPECT: f32 = 2.07;
/// The tallest drawing, in rows. Past this the shading costs more than it shows.
const ART_MAX: usize = 60;
/// How far the ring stands off the drawing, in cells across and rows down.
const RING_GAP: (f32, f32) = (4.0, 2.0);

/// The tagline under the drawing, as in the CLI.
pub const TAGLINE: &str = "Close the laptop. The work keeps going.";
/// When the tagline starts condensing, after the drawing begins.
pub const TAGLINE_DELAY: f32 = 0.25;
/// Every cell of the drawing has landed by now.
pub const LANDED: f32 = 1.75;
/// A still frame for Reduce Motion: landed, at rest, before the glance.
pub const STILL: f32 = 1.8;

type Shading = Vec<Vec<Option<f32>>>;

/// dev-01's faces, shaded once per size: at rest, glancing right, blinking.
pub struct Faces {
    size: (usize, usize),
    rest: Shading,
    look: Shading,
    shut: Shading,
}

impl Faces {
    pub fn new(size: (usize, usize)) -> Self {
        let (w, h) = size;
        Self {
            size,
            rest: mascot::shade(w, h, Pose::REST),
            look: mascot::shade(
                w,
                h,
                Pose {
                    look: 1.0,
                    open: 1.0,
                },
            ),
            shut: mascot::shade(
                w,
                h,
                Pose {
                    look: 0.0,
                    open: 0.15,
                },
            ),
        }
    }

    pub fn size(&self) -> (usize, usize) {
        self.size
    }

    /// Once dev-01 has landed it glances right, then blinks.
    fn at(&self, clock: f32) -> &[Vec<Option<f32>>] {
        if (1.9..2.25).contains(&clock) {
            &self.look
        } else if (2.45..2.6).contains(&clock) {
            &self.shut
        } else {
            &self.rest
        }
    }
}

/// The login page's shimmer on a cell of brightness `t` at `diag`, 0 to 1
/// from the top left to the bottom right: each cell breathes on its own
/// phase, a slow band of light crosses on the diagonal, and a few glint.
fn sparkle(t: f32, seed: f32, diag: f32, clock: f32) -> f32 {
    let phase = hash(seed + 7.1) * std::f32::consts::TAU;
    if hash(seed + 3.3) > 0.97 && (clock * 3.1 + phase * 5.0).sin() > 0.9 {
        return 1.0;
    }
    let sweep = (clock * 0.55) % 1.8 - 0.4;
    let band = (-((diag - sweep) / 0.09).powi(2)).exp() * 0.28 * (t * 3.0).min(1.0);
    let breathe = 0.07 * (clock * 2.4 + phase).sin();
    (t + breathe + band).clamp(0.0, 1.0)
}

/// Rows the splash takes for a drawing `h` rows tall.
pub fn splash_rows(h: usize) -> usize {
    (2.0 * (h as f32 / 2.0 + RING_GAP.1)).ceil() as usize + 1
}

/// The largest drawing whose splash fits `cols` by `rows` cells, or `None`
/// when that is too small to read.
pub fn fit(cols: usize, rows: usize) -> Option<(usize, usize)> {
    let by_rows = rows.checked_sub(splash_rows(0))?;
    let by_cols = ((cols as f32 - 2.0 * RING_GAP.0 - 2.0) / ASPECT).max(0.0) as usize;
    let h = by_rows.min(by_cols).min(ART_MAX);
    (h >= 10).then(|| ((h as f32 * ASPECT).round() as usize, h))
}

/// The settled splash at `clock`, `cols` wide: dev-01 shaded in bits, a ring
/// of bits with one packet orbiting, and loose bits thinning out beyond.
fn splash(face: &[Vec<Option<f32>>], cols: usize, clock: f32) -> Vec<Vec<Cell>> {
    let h = face.len();
    let w = face.first().map_or(0, Vec::len);
    let (rx, ry) = (w as f32 / 2.0 + RING_GAP.0, h as f32 / 2.0 + RING_GAP.1);
    let rows = splash_rows(h);
    let (cx, cy) = ((cols as f32 - 1.0) / 2.0, (rows as f32 - 1.0) / 2.0);
    let (x0, y0) = (
        (cx - w as f32 / 2.0).round() as isize,
        (cy - h as f32 / 2.0).round() as isize,
    );
    // Loose and body bits turn over every quarter second, not every frame.
    let turn = (clock * 4.0).floor() * 5.3;
    let orbit = clock * std::f32::consts::TAU * 0.55;
    // The ring is plotted, not tested, so it is one cell wide all the way round.
    let mut ring = vec![vec![false; cols]; rows];
    let steps = (8.0 * (rx + ry)) as usize;
    for k in 0..steps {
        let a = k as f32 / steps as f32 * std::f32::consts::TAU;
        let (x, y) = ((cx + rx * a.cos()).round(), (cy + ry * a.sin()).round());
        if x >= 0.0
            && y >= 0.0
            && let Some(cell) = ring.get_mut(y as usize).and_then(|r| r.get_mut(x as usize))
        {
            *cell = true;
        }
    }
    (0..rows)
        .map(|y| {
            (0..cols)
                .map(|x| {
                    let seed = x as f32 * 3.1 + y as f32 * 17.7;
                    let diag = (x as f32 / cols as f32 + y as f32 / rows as f32) / 2.0;
                    let lit = |t: f32| Paint::Tone(sparkle(t, seed, diag, clock));
                    let tone = usize::try_from(y as isize - y0)
                        .ok()
                        .zip(usize::try_from(x as isize - x0).ok())
                        .and_then(|(r, c)| {
                            face.get(r).and_then(|row| row.get(c)).copied().flatten()
                        });
                    if let Some(t) = tone {
                        return match t {
                            // Pupils and anything as dark stay holes.
                            t if t < 0.1 => (' ', Paint::Blank),
                            // The dim body's bits turn over, so it breathes.
                            t if t < 0.35 => (binary(seed + turn), lit(t)),
                            t => (binary(seed), lit(t)),
                        };
                    }
                    if ring[y][x] {
                        let a = ((y as f32 - cy) / ry).atan2((x as f32 - cx) / rx);
                        // How far behind the packet this cell is, 0 to one turn.
                        let behind = (orbit - a).rem_euclid(std::f32::consts::TAU);
                        let t = 0.2 + 0.78 * (-behind / 0.45).exp();
                        return (binary(seed + 2.2), lit(t));
                    }
                    let reach = ((x as f32 - cx) / rx).powi(2) + ((y as f32 - cy) / ry).powi(2);
                    if reach > 1.0 && hash(seed) < 0.24 / reach {
                        (binary(seed + turn), lit(0.16 + 0.14 * hash(seed + 1.3)))
                    } else {
                        (' ', Paint::Blank)
                    }
                })
                .collect()
        })
        .collect()
}

/// One row of the splash at `clock`: cells start scattered and condense from
/// the centre outwards, each on its own hash, flickering as bits until they
/// land. Every cell has landed by [`LANDED`].
fn land_row(cells: &[Cell], row: usize, rows: usize, clock: f32) -> Vec<Cell> {
    let n = cells.len().max(1) as f32;
    let tick = (clock * 16.0).floor();
    let down = ((row as f32 + 0.5) / rows.max(1) as f32 - 0.5).abs() * 2.0;
    cells
        .iter()
        .enumerate()
        .map(|(x, &cell)| {
            let tile = x as f32 * 1.7 + row as f32 * 31.3;
            let across = ((x as f32 + 0.5) / n - 0.5).abs() * 2.0;
            let spread = across.max(down);
            if clock >= 0.7 + spread * 0.6 + hash(tile) * 0.45 {
                cell
            } else if cell.0 == ' ' && hash(tile + 5.3) > 0.35 {
                (' ', Paint::Blank)
            } else {
                (binary(tile + tick * 1.31), Paint::Noise)
            }
        })
        .collect()
}

/// The drawing's rows at `clock`, `cols` wide.
pub fn frame(faces: &Faces, cols: usize, clock: f32) -> Vec<Vec<Cell>> {
    let settled = splash(faces.at(clock), cols, clock);
    let rows = settled.len();
    settled
        .iter()
        .enumerate()
        .map(|(row, cells)| land_row(cells, row, rows, clock))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_the_same_every_time() {
        let faces = Faces::new((40, 20));
        assert_eq!(frame(&faces, 80, 0.9), frame(&faces, 80, 0.9));
    }

    #[test]
    fn every_cell_has_landed_by_the_landed_clock() {
        let faces = Faces::new((40, 20));
        let early = frame(&faces, 80, 0.2);
        assert!(early.iter().flatten().any(|c| c.1 == Paint::Noise));
        let late = frame(&faces, 80, LANDED);
        assert!(late.iter().flatten().all(|c| c.1 != Paint::Noise));
    }

    #[test]
    fn the_frame_spans_the_columns_and_the_ring() {
        let faces = Faces::new((40, 20));
        let rows = frame(&faces, 90, STILL);
        assert_eq!(rows.len(), splash_rows(20));
        assert!(rows.iter().all(|r| r.len() == 90));
    }

    #[test]
    fn the_tagline_reads_once_condensed() {
        let text: String = condense(TAGLINE, 0.6, 1.0, Paint::Tone(0.5))
            .iter()
            .map(|c| c.0)
            .collect();
        assert_eq!(text, TAGLINE);
        assert!(
            condense(TAGLINE, 0.0, 1.0, Paint::Tone(0.5))
                .iter()
                .any(|c| c.1 == Paint::Noise)
        );
    }

    #[test]
    fn a_small_window_shows_no_drawing() {
        assert_eq!(fit(30, 12), None);
        let (w, h) = fit(200, 50).unwrap_or_default();
        assert!(h >= 10 && splash_rows(h) <= 50 && w + 10 <= 200);
    }
}
