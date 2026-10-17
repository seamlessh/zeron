//! Seamlessh's boot splash: the CLI intro (dev-01 condensing out of bits
//! inside its orbiting ring, the tagline under it) painted on a monospace
//! cell grid over the opaque window background. Zeron's splash lifecycle is
//! unchanged: the shell shows this while the engine connects and plays
//! `splash-out` when it is ready.

mod field;
mod mascot;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Font, FontWeight, Hsla, IntoElement, ParentElement, Pixels,
    SharedString, Styled, TextAlign, TextRun, Window, canvas, div, point, px,
};

use crate::motion;
use crate::theme::Theme;
use field::{Cell, Faces, Paint};

pub use field::{LANDED, STILL};

/// Bits are small: the drawing is made of many cells, not read as text.
const BIT_SIZE: f32 = 10.0;
const TAGLINE_SIZE: f32 = 12.0;
/// The drawing takes at most this share of the window's height.
const ART_SHARE: f32 = 0.55;
/// The CLI's grey for a bit in motion, #5c5c5c over black.
const NOISE_ALPHA: f32 = 0x5c as f32 / 255.0;

thread_local! {
    /// Shading dev-01 samples every cell several times; do it once per size.
    static FACES: RefCell<Option<Rc<Faces>>> = const { RefCell::new(None) };
}

fn faces(size: (usize, usize)) -> Rc<Faces> {
    FACES.with(|slot| {
        let mut slot = slot.borrow_mut();
        match slot.as_ref() {
            Some(faces) if faces.size() == size => faces.clone(),
            _ => {
                let faces = Rc::new(Faces::new(size));
                *slot = Some(faces.clone());
                faces
            }
        }
    })
}

/// The CLI's 24-step grey ramp, as the theme's text over its background.
/// Returns the colour and whether the cell is bold, or `None` for a blank.
fn ink(text: Hsla, paint: Paint) -> Option<(u8, Hsla, bool)> {
    match paint {
        Paint::Blank => None,
        Paint::Noise => Some((u8::MAX, text.opacity(NOISE_ALPHA), false)),
        Paint::Tone(t) => {
            let level = (t.clamp(0.0, 1.0) * 23.0).round() as u8;
            let alpha = (8.0 + 10.0 * f32::from(level)) / 255.0;
            Some((level, text.opacity(alpha), t > 0.85))
        }
    }
}

/// One row of cells as a shaped line: runs merge neighbours of the same grey,
/// so a row is a handful of runs rather than one per cell.
fn paint_row(
    cells: &[Cell],
    origin: gpui::Point<Pixels>,
    size: Pixels,
    line_height: Pixels,
    (regular, bold): (&Font, &Font),
    text: Hsla,
    window: &mut Window,
    cx: &mut App,
) {
    let mut line = String::with_capacity(cells.len());
    let mut runs: Vec<(Option<u8>, TextRun)> = Vec::new();
    for &(glyph, paint) in cells {
        let (key, color, heavy) = match ink(text, paint) {
            Some((level, color, heavy)) => (Some(level), color, heavy),
            None => (None, gpui::transparent_black(), false),
        };
        let glyph = if key.is_none() { ' ' } else { glyph };
        line.push(glyph);
        let len = glyph.len_utf8();
        match runs.last_mut() {
            Some((last, run)) if *last == key => run.len += len,
            _ => runs.push((
                key,
                TextRun {
                    len,
                    font: if heavy { bold.clone() } else { regular.clone() },
                    color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                },
            )),
        }
    }
    let runs: Vec<TextRun> = runs.into_iter().map(|(_, run)| run).collect();
    let shaped = window
        .text_system()
        .shape_line(SharedString::from(line), size, &runs, None);
    let _ = shaped.paint(origin, line_height, TextAlign::Left, None, window, cx);
}

fn paint(
    bounds: Bounds<Pixels>,
    clock: f32,
    family: SharedString,
    text: Hsla,
    window: &mut Window,
    cx: &mut App,
) {
    let regular = gpui::font(family);
    let bold = Font {
        weight: FontWeight::BOLD,
        ..regular.clone()
    };
    let cell_width = |size: f32| {
        let id = window.text_system().resolve_font(&regular);
        window
            .text_system()
            .ch_width(id, px(size))
            .map_or(size * 0.6, f32::from)
    };
    // Cells twice as tall as wide, as the CLI's drawing assumes.
    let (bit_w, tag_w) = (cell_width(BIT_SIZE), cell_width(TAGLINE_SIZE));
    let bit_h = bit_w * 2.0;
    let tag_h = TAGLINE_SIZE * 1.5;
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let cols = (width / bit_w).floor() as usize;
    let art = field::fit(cols, (height * ART_SHARE / bit_h).floor() as usize);
    let rows = art.map_or(Vec::new(), |size| field::frame(&faces(size), cols, clock));

    let gap = if rows.is_empty() { 0.0 } else { bit_h * 2.0 };
    let block = rows.len() as f32 * bit_h + gap + tag_h;
    let top = f32::from(bounds.origin.y) + ((height - block) / 2.0).max(0.0);
    let left = f32::from(bounds.origin.x) + (width - cols as f32 * bit_w) / 2.0;
    for (row, cells) in rows.iter().enumerate() {
        let origin = point(px(left), px(top + row as f32 * bit_h));
        paint_row(
            cells,
            origin,
            px(BIT_SIZE),
            px(bit_h),
            (&regular, &bold),
            text,
            window,
            cx,
        );
    }

    let tagline = field::condense(
        field::TAGLINE,
        clock - field::TAGLINE_DELAY,
        1.0,
        Paint::Tone(0.55),
    );
    let tag_left = f32::from(bounds.origin.x) + (width - tagline.len() as f32 * tag_w) / 2.0;
    let tag_top = top + rows.len() as f32 * bit_h + gap;
    paint_row(
        &tagline,
        point(px(tag_left), px(tag_top)),
        px(TAGLINE_SIZE),
        px(tag_h),
        (&regular, &bold),
        text,
        window,
        cx,
    );
}

/// The full-window splash at `clock` seconds since launch. While `fading` it
/// plays Zeron's `splash-out`; the shell removes it after that has run.
pub fn overlay(theme: &Theme, fading: bool, clock: f32, cx: &mut App) -> AnyElement {
    let family = crate::typography::code_effective_family_name(cx);
    let text = theme.text;
    let content = div()
        .absolute()
        .inset_0()
        // Opaque, as every Seamlessh surface is; Zeron's splash was frosted.
        .bg(theme.bg)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, cx| paint(bounds, clock, family, text, window, cx),
            )
            .absolute()
            .inset_0(),
        );
    if fading {
        motion::splash_out("boot-splash-out", content).into_any_element()
    } else {
        content.into_any_element()
    }
}
