//! Copied from Seamlessh `crates/cli/src/mascot.rs` so the desktop splash draws
//! the same dev-01 as the CLI intro. Keep the two in step until the CLI moves
//! onto this workspace.
//!
//! dev-01, the site's mascot (`site/src/components/landing/Peeker.astro`),
//! rasterised into terminal cells. Pure geometry and shading: how bright
//! each cell is. Which glyph carries that brightness is the intro's business.

type Point = (f32, f32);

/// The drawing's own coordinates, copied from the site's SVG (viewBox
/// 0 0 160 168) with relative commands made absolute.
const BODY: &str = "M28 86 C22 58 38 36 56 42 C62 22 84 18 96 38 C110 20 136 28 132 52 \
                    C150 60 150 96 132 112 C118 128 42 130 28 110 C16 98 18 90 28 86 Z";
const STACHE: &str = "M50 106 C58 100 68 104 76 108 C84 104 94 100 102 106 \
                      C96 112 84 114 76 110 C68 114 56 112 50 106 Z";
const LIMBS: [&str; 6] = [
    "M60 124 L56 148",
    "M48 148 L62 148",
    "M100 124 L104 148",
    "M98 148 L112 148",
    "M32 96 C20 100 16 112 20 118",
    "M128 96 C140 100 144 112 140 118",
];
const PUFFS: [&str; 2] = [
    "M124 96 C128 90 126 84 130 80",
    "M130 94 C136 88 134 82 138 78",
];
/// Eye whites and pupils: (cx, cy, rx, ry).
const WHITES: [(f32, f32, f32, f32); 2] = [(58.0, 74.0, 17.0, 24.0), (100.0, 74.0, 17.0, 24.0)];
const PUPILS: [(f32, f32, f32, f32); 2] = [(64.0, 74.0, 6.2, 13.0), (106.0, 74.0, 6.2, 13.0)];
/// The cigar, its band and its ash tip, all turned -18 degrees about (98, 108).
const CIGAR: (f32, f32, f32, f32) = (96.0, 105.0, 28.0, 6.5);
const BAND: (f32, f32, f32, f32) = (114.0, 105.0, 5.0, 6.5);
const TIP: (f32, f32, f32) = (125.5, 108.25, 2.0);
const CIGAR_TURN: (f32, f32, f32) = (98.0, 108.0, -18.0);
/// Stroke widths from the site's CSS, in drawing units.
const EDGE_WIDTH: f32 = 3.2;
const LIMB_WIDTH: f32 = 3.0;
const PUFF_WIDTH: f32 = 1.6;
/// The part of the viewBox the drawing occupies.
const FRAME: (f32, f32, f32, f32) = (14.0, 16.0, 152.0, 152.0);

/// The site's fills as brightness, 0 black to 1 white: `--mark`, the cigar's
/// #d6d6d6, band #8a8a8a, tip #5c5c5c, smoke #a3a3a3, body #141414.
const MARK: f32 = 0.96;
const CIGAR_TONE: f32 = 0.84;
const BAND_TONE: f32 = 0.54;
const TIP_TONE: f32 = 0.36;
const PUFF_TONE: f32 = 0.64;
const PUPIL_TONE: f32 = 0.08;

/// How dev-01 holds its face for one frame. As on the site: a glance moves
/// the pupils up to 4 units sideways, and a blink squashes each eye to 15%
/// of its height about its centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// -1 looks left, 1 looks right.
    pub look: f32,
    /// 1 open, 0.15 shut.
    pub open: f32,
}

impl Pose {
    pub const REST: Self = Self {
        look: 0.0,
        open: 1.0,
    };
}

/// Add the points `command` draws with `numbers` to `points`.
fn draw(command: char, numbers: &[f32], points: &mut Vec<Point>) {
    match command {
        'M' | 'L' => points.extend(numbers.as_chunks::<2>().0.iter().map(|&[x, y]| (x, y))),
        'C' => {
            for c in numbers.as_chunks::<6>().0 {
                let start = points.last().copied().unwrap_or((c[0], c[1]));
                for i in 1..=16 {
                    let t = i as f32 / 16.0;
                    let u = 1.0 - t;
                    let (a, b, e, f) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    points.push((
                        a * start.0 + b * c[0] + e * c[2] + f * c[4],
                        a * start.1 + b * c[1] + e * c[3] + f * c[5],
                    ));
                }
            }
        }
        _ => {}
    }
}

/// A path of absolute `M`, `L`, `C` and `Z` commands as a polyline, curves
/// cut into 16 straight pieces.
fn polyline(d: &str) -> Vec<Point> {
    let spaced: String = d
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                format!(" {c} ")
            } else {
                c.to_string()
            }
        })
        .collect();
    let mut points = Vec::new();
    let mut numbers = Vec::new();
    let mut command = ' ';
    for word in spaced.split_whitespace() {
        if let Ok(n) = word.parse::<f32>() {
            numbers.push(n);
        } else {
            draw(command, &numbers, &mut points);
            numbers.clear();
            command = word.chars().next().unwrap_or(' ');
        }
    }
    draw(command, &numbers, &mut points);
    points
}

/// A polyline with its bounding box, so far points skip the segment walk.
struct Line {
    points: Vec<Point>,
    closed: bool,
    min: Point,
    max: Point,
}

impl Line {
    fn new(d: &str, closed: bool) -> Self {
        let points = polyline(d);
        let min = points
            .iter()
            .fold((f32::MAX, f32::MAX), |m, p| (m.0.min(p.0), m.1.min(p.1)));
        let max = points
            .iter()
            .fold((f32::MIN, f32::MIN), |m, p| (m.0.max(p.0), m.1.max(p.1)));
        Self {
            points,
            closed,
            min,
            max,
        }
    }

    /// Whether `p` is within `reach` of the line.
    fn near(&self, p: Point, reach: f32) -> bool {
        if p.0 < self.min.0 - reach
            || p.1 < self.min.1 - reach
            || p.0 > self.max.0 + reach
            || p.1 > self.max.1 + reach
        {
            return false;
        }
        let n = self.points.len();
        let segments = if self.closed { n } else { n.saturating_sub(1) };
        (0..segments).any(|i| {
            let (a, b) = (self.points[i], self.points[(i + 1) % n]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = dx * dx + dy * dy;
            let t = if len > 0.0 {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2) <= reach * reach
        })
    }

    /// Even-odd test against the closed line.
    fn encloses(&self, p: Point) -> bool {
        let Some(&last) = self.points.last() else {
            return false;
        };
        let mut odd = false;
        let mut b = last;
        for &a in &self.points {
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
                odd = !odd;
            }
            b = a;
        }
        odd
    }
}

fn in_ellipse(p: Point, (cx, cy, rx, ry): (f32, f32, f32, f32)) -> bool {
    ((p.0 - cx) / rx).powi(2) + ((p.1 - cy) / ry).powi(2) <= 1.0
}

fn in_rect(p: Point, (x, y, w, h): (f32, f32, f32, f32)) -> bool {
    (x..=x + w).contains(&p.0) && (y..=y + h).contains(&p.1)
}

/// `p` in the cigar's own unturned frame.
fn unturn(p: Point) -> Point {
    let (ox, oy, deg) = CIGAR_TURN;
    let (sin, cos) = (-deg).to_radians().sin_cos();
    let (dx, dy) = (p.0 - ox, p.1 - oy);
    (ox + dx * cos - dy * sin, oy + dx * sin + dy * cos)
}

struct Drawing {
    body: Line,
    stache: Line,
    limbs: Vec<Line>,
    puffs: Vec<Line>,
}

impl Drawing {
    fn new() -> Self {
        Self {
            body: Line::new(BODY, true),
            stache: Line::new(STACHE, true),
            limbs: LIMBS.iter().map(|d| Line::new(d, false)).collect(),
            puffs: PUFFS.iter().map(|d| Line::new(d, false)).collect(),
        }
    }

    /// The brightness at `p`, or `None` off the drawing. Tested top layer
    /// first, in the SVG's paint order reversed: moustache, cigar and smoke,
    /// eyes, body and its outline, limbs.
    fn tone(&self, p: Point, pose: Pose) -> Option<f32> {
        if self.stache.encloses(p) {
            return Some(MARK);
        }
        let c = unturn(p);
        if (c.0 - TIP.0).powi(2) + (c.1 - TIP.1).powi(2) <= TIP.2 * TIP.2 {
            return Some(TIP_TONE);
        }
        if in_rect(c, BAND) {
            return Some(BAND_TONE);
        }
        if in_rect(c, CIGAR) {
            return Some(CIGAR_TONE);
        }
        if self.puffs.iter().any(|l| l.near(p, PUFF_WIDTH / 2.0)) {
            return Some(PUFF_TONE);
        }
        for (&(wx, wy, wrx, wry), &(px, py, prx, pry)) in WHITES.iter().zip(&PUPILS) {
            let white = (wx, wy, wrx, wry * pose.open);
            if !in_ellipse(p, white) {
                continue;
            }
            let (px, py) = (px + 4.0 * pose.look, wy + (py - wy) * pose.open);
            if in_ellipse(p, (px, py, prx, pry * pose.open)) {
                // A catchlight high on the pupil, toward the light.
                let glint = (px - 2.2, py - 6.0 * pose.open, 1.7, 2.6 * pose.open);
                return Some(if in_ellipse(p, glint) {
                    MARK
                } else {
                    PUPIL_TONE
                });
            }
            // The white rounds away from the light at its lower edge.
            let fall = ((p.1 - wy) / wry).clamp(0.0, 1.0);
            return Some(MARK - 0.12 * fall * fall);
        }
        if self.body.near(p, EDGE_WIDTH / 2.0) {
            return Some(MARK);
        }
        if self.body.encloses(p) {
            // A dim volume: the black body catches light from the upper left.
            let reach = ((p.0 - 52.0).powi(2) + (p.1 - 44.0).powi(2)).sqrt() / 110.0;
            return Some(0.12 + 0.18 * (1.0 - reach).clamp(0.0, 1.0));
        }
        if self.limbs.iter().any(|l| l.near(p, LIMB_WIDTH / 2.0)) {
            return Some(MARK);
        }
        None
    }
}

/// Samples per cell, across and down: a cell is about twice as tall as wide.
const SAMPLES: (usize, usize) = (3, 6);

/// dev-01 in `cols` by `rows` cells, each the mean brightness of a grid of
/// samples so edges fall off in steps, or `None` where the drawing covers
/// less than a third of the cell.
pub fn shade(cols: usize, rows: usize, pose: Pose) -> Vec<Vec<Option<f32>>> {
    let drawing = Drawing::new();
    let (x0, y0, x1, y1) = FRAME;
    let scale = ((x1 - x0) / cols as f32, (y1 - y0) / rows as f32);
    let n = (SAMPLES.0 * SAMPLES.1) as f32;
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    let (mut sum, mut covered) = (0.0, 0.0);
                    for sy in 0..SAMPLES.1 {
                        for sx in 0..SAMPLES.0 {
                            let p = (
                                x0 + (c as f32 + (sx as f32 + 0.5) / SAMPLES.0 as f32) * scale.0,
                                y0 + (r as f32 + (sy as f32 + 0.5) / SAMPLES.1 as f32) * scale.1,
                            );
                            if let Some(t) = drawing.tone(p, pose) {
                                sum += t;
                                covered += 1.0;
                            }
                        }
                    }
                    (covered / n >= 0.34).then_some(sum / n)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_read_with_every_curve_flattened() {
        assert_eq!(polyline(LIMBS[0]), vec![(60.0, 124.0), (56.0, 148.0)]);
        assert_eq!(polyline(BODY).len(), 1 + 6 * 16);
    }

    #[test]
    fn the_layers_meet_in_the_sites_order() {
        let d = Drawing::new();
        let rest = Pose::REST;
        assert_eq!(d.tone((76.0, 108.0), rest), Some(MARK), "moustache");
        assert_eq!(d.tone((64.0, 78.0), rest), Some(PUPIL_TONE), "pupil");
        assert_eq!(d.tone((61.8, 68.0), rest), Some(MARK), "catchlight");
        assert_eq!(d.tone((105.0, 106.0), rest), Some(CIGAR_TONE), "cigar");
        assert_eq!(d.tone((10.0, 10.0), rest), None, "ground");
        let body = d.tone((80.0, 50.0), rest).unwrap_or(0.0);
        assert!(body > 0.1 && body < 0.35, "{body}");
    }

    #[test]
    fn a_blink_shuts_the_eyes_and_a_glance_moves_the_pupils() {
        let d = Drawing::new();
        let shut = Pose {
            look: 0.0,
            open: 0.15,
        };
        assert_eq!(d.tone((64.0, 74.0), shut), Some(PUPIL_TONE));
        assert_ne!(d.tone((58.0, 58.0), shut), d.tone((58.0, 58.0), Pose::REST));
        let right = Pose {
            look: 1.0,
            open: 1.0,
        };
        assert_eq!(d.tone((70.5, 74.0), right), Some(PUPIL_TONE));
        assert_ne!(d.tone((70.5, 74.0), Pose::REST), Some(PUPIL_TONE));
    }

    #[test]
    fn the_shaded_drawing_has_dark_pupils_in_bright_eyes() {
        let cells = shade(58, 28, Pose::REST);
        assert_eq!(cells.len(), 28);
        assert!(cells.iter().all(|r| r.len() == 58));
        let tones: Vec<f32> = cells.iter().flatten().flatten().copied().collect();
        assert!(tones.iter().any(|&t| t > 0.9));
        assert!(tones.iter().any(|&t| t < 0.15));
        // Partly covered cells give the in-between steps.
        assert!(tones.iter().any(|&t| (0.4..0.7).contains(&t)));
    }
}
