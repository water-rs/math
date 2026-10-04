//! Recording a laid-out formula through the `Draw` contract.
//!
//! Nothing here knows which render target is underneath. That is the point:
//! the same commands render on whichever target the host records them for —
//! the GPU engine a live surface draws through, the CPU rasteriser an
//! offscreen render goes through, or a target that is not Cherenkov's at all.

use alloc::vec::Vec;

use nami::Computed;
use waterui_graphics::draw::kurbo::{Affine, Rect};
use waterui_graphics::draw::{
    Draw, Fixed, FontId, Glyph, GlyphRun, GlyphStyle, Recorder, WorkingColor,
};

use crate::layout::{MathLayout, Placed};

/// Records `layout` into `recorder`, its baseline origin at `(x, y)`.
///
/// Items are emitted in layout order rather than grouped by kind, because a
/// rule drawn out of order would paint over a glyph that should sit on top of
/// it. Consecutive glyphs at the same size are still batched into one run,
/// which is what keeps a formula from becoming one draw call per character.
///
/// `font` is the id [`RecordingResources::name`][resources] returned for the
/// math face in this recording; `paint` is the colour the formula is drawn
/// in, bound so a theme change repaints without another recording.
///
/// [resources]: waterui_graphics::RecordingResources::name
pub fn draw(
    layout: &MathLayout,
    recorder: &mut Recorder,
    font: FontId,
    paint: &Computed<WorkingColor>,
    x: f32,
    y: f32,
) {
    recorder.transform(
        Fixed(Affine::translate((f64::from(x), f64::from(y)))),
        |recorder| {
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
                        // A run carries one em size, so a size change ends the
                        // run.
                        if !pending.is_empty() && (*size - pending_size).abs() > f32::EPSILON {
                            flush(recorder, font, paint, pending_size, &mut pending);
                        }
                        pending_size = *size;
                        pending.push(Glyph {
                            id: u32::from(glyph.id.0),
                            x: *x,
                            y: *baseline,
                            transform: None,
                        });
                    }
                    Placed::Outline(outline) => {
                        flush(recorder, font, paint, pending_size, &mut pending);
                        recorder.fill(Fixed(outline.clone()), paint.clone());
                    }
                    Placed::Rule {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        flush(recorder, font, paint, pending_size, &mut pending);
                        recorder.fill(
                            Fixed(Rect::new(
                                f64::from(*x),
                                f64::from(*y),
                                f64::from(*x + *width),
                                f64::from(*y + *height),
                            )),
                            paint.clone(),
                        );
                    }
                }
            }

            flush(recorder, font, paint, pending_size, &mut pending);
        },
    );
}

fn flush(
    recorder: &mut Recorder,
    font: FontId,
    paint: &Computed<WorkingColor>,
    size: f32,
    glyphs: &mut Vec<Glyph>,
) {
    if glyphs.is_empty() {
        return;
    }
    recorder.glyphs(
        Fixed(GlyphRun {
            font,
            size,
            coords: Vec::new().into(),
            glyphs: core::mem::take(glyphs).into(),
            style: GlyphStyle::Fill,
        }),
        paint.clone(),
    );
}
