//! Drawing a laid-out formula on a canvas.
//!
//! A formula is glyph runs from the OpenType `MATH` face, outlines for stretched
//! delimiters and radical signs, and rules for fraction bars. All of it is
//! canvas vocabulary, so whichever renderer draws the screen around the formula
//! draws it, and the same commands record into a standalone `Drawing`.

use alloc::vec::Vec;

use kurbo::{BezPath, PathEl};
use nami::Computed;
use waterui_core::layout::{Affine2, Point};
use waterui_graphics::canvas::{Cx, DrawingCx, Font, Glyph, GlyphRun, Path};
use waterui_graphics::color::WorkingColor;

use crate::layout::{MathLayout, Placed};

/// What a formula is drawn on: a live [`Cx`], whose ink is a signal, or a
/// [`DrawingCx`], whose ink is a constant.
pub trait Surface {
    /// What the formula is painted with.
    type Ink;

    /// Fills `path` with `ink`.
    fn fill_path(&mut self, path: Path, ink: &Self::Ink);

    /// Draws `run` with `ink`.
    fn draw_glyphs(&mut self, run: GlyphRun, ink: &Self::Ink);

    /// Runs `body` with the origin moved to `(dx, dy)`.
    fn shifted(&mut self, dx: f32, dy: f32, body: impl FnOnce(&mut Self));
}

impl Surface for Cx {
    type Ink = Computed<WorkingColor>;

    fn fill_path(&mut self, path: Path, ink: &Self::Ink) {
        self.fill(path, ink.clone());
    }

    fn draw_glyphs(&mut self, run: GlyphRun, ink: &Self::Ink) {
        self.glyphs(run, ink.clone());
    }

    fn shifted(&mut self, dx: f32, dy: f32, body: impl FnOnce(&mut Self)) {
        self.transform(Affine2::translate(dx, dy), body);
    }
}

impl Surface for DrawingCx {
    type Ink = WorkingColor;

    fn fill_path(&mut self, path: Path, ink: &Self::Ink) {
        self.fill(path, *ink);
    }

    fn draw_glyphs(&mut self, run: GlyphRun, ink: &Self::Ink) {
        self.glyphs(&run, *ink);
    }

    fn shifted(&mut self, dx: f32, dy: f32, body: impl FnOnce(&mut Self)) {
        self.transform(Affine2::translate(dx, dy), body);
    }
}

/// Draws `layout` on `surface`, its baseline origin at `(x, y)`.
///
/// Items are emitted in layout order rather than grouped by kind, because a
/// rule drawn out of order would paint over a glyph that should sit on top of
/// it. Consecutive glyphs at the same size are still batched into one run,
/// which is what keeps a formula from becoming one draw call per character.
///
/// `font` is the math face the glyph ids index; `ink` is what the formula is
/// painted with.
pub fn draw<S: Surface>(
    layout: &MathLayout,
    surface: &mut S,
    font: &Font,
    ink: &S::Ink,
    x: f32,
    y: f32,
) {
    surface.shifted(x, y, |surface| {
        let mut pending: Vec<Glyph> = Vec::new();
        let mut pending_size = 0.0_f32;

        for item in &layout.items {
            match item {
                Placed::Glyph {
                    glyph,
                    x,
                    baseline,
                    size,
                } => {
                    // A run carries one em size, so a size change ends the run.
                    if !pending.is_empty() && (*size - pending_size).abs() > f32::EPSILON {
                        flush(surface, font, ink, pending_size, &mut pending);
                    }
                    pending_size = *size;
                    pending.push(Glyph {
                        id: u32::from(glyph.id.0),
                        x: *x,
                        y: *baseline,
                    });
                }
                Placed::Outline(outline) => {
                    flush(surface, font, ink, pending_size, &mut pending);
                    surface.fill_path(path(outline), ink);
                }
                Placed::Rule {
                    x,
                    y,
                    width,
                    height,
                } => {
                    flush(surface, font, ink, pending_size, &mut pending);
                    surface.fill_path(rule(*x, *y, *width, *height), ink);
                }
            }
        }

        flush(surface, font, ink, pending_size, &mut pending);
    });
}

fn flush<S: Surface>(
    surface: &mut S,
    font: &Font,
    ink: &S::Ink,
    size: f32,
    glyphs: &mut Vec<Glyph>,
) {
    if glyphs.is_empty() {
        return;
    }
    surface.draw_glyphs(
        GlyphRun::new(font.clone(), size, core::mem::take(glyphs)),
        ink,
    );
}

/// A filled rectangle: a fraction bar or a radical's overbar.
fn rule(x: f32, y: f32, width: f32, height: f32) -> Path {
    Path::builder()
        .move_to(Point::new(x, y))
        .line_to(Point::new(x + width, y))
        .line_to(Point::new(x + width, y + height))
        .line_to(Point::new(x, y + height))
        .close()
        .build()
}

/// The canvas path of a prebuilt outline.
#[expect(
    clippy::cast_possible_truncation,
    reason = "outlines are laid out in f32 points and widened to f64 by kurbo"
)]
fn path(outline: &BezPath) -> Path {
    let point = |point: kurbo::Point| Point::new(point.x as f32, point.y as f32);
    outline
        .elements()
        .iter()
        .fold(Path::builder(), |builder, element| match *element {
            PathEl::MoveTo(to) => builder.move_to(point(to)),
            PathEl::LineTo(to) => builder.line_to(point(to)),
            PathEl::QuadTo(control, to) => builder.quad_to(point(control), point(to)),
            PathEl::CurveTo(first, second, to) => {
                builder.cubic_to(point(first), point(second), point(to))
            }
            PathEl::ClosePath => builder.close(),
        })
        .build()
}
