//! The gallery: every construct the crate lays out, rendered to PNG through
//! both offscreen engines.
//!
//! `OffscreenRenderer::new()` is the GPU pipeline a live surface uses and
//! `OffscreenRenderer::cpu()` is the CPU rasteriser; drawing the same
//! `Drawing` through both is also what exercises the engine-scoped font
//! registration — a registration made on one engine means nothing on the other,
//! and each renderer makes its own.
//!
//! The PNGs land in `WATERUI_MATH_GALLERY_DIR` when it is set and in the test
//! target's temporary directory otherwise, for review after the suite runs.

use std::path::{Path, PathBuf};

use waterui_core::layout::{Rect, Size};
use waterui_graphics::canvas::Drawing;
use waterui_graphics::color::WorkingColor;
use waterui_graphics::{OffscreenRenderer, OffscreenSize};
use waterui_math::ast::MathStyle;
use waterui_math::view::{DEFAULT_MATH_FAMILY, Typeset};
use waterui_text::FontCollection;

const GALLERY: &[(&str, &str)] = &[
    ("quadratic", r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"),
    ("summation", r"\sum_{k=0}^{n} k^2 = \frac{n(n+1)(2n+1)}{6}"),
    ("definite_integral", r"\int_0^\pi \sin(x)\,\mathrm{d}x = 2"),
    ("euler_identity", r"e^{i\pi} + 1 = 0"),
    ("pythagoras", r"a^2 + b^2 = c^2"),
    ("nested_radical", r"\sqrt{1 + \sqrt{1 + \sqrt{1 + x}}}"),
    ("binomial", r"\binom{n}{k} = \frac{n!}{k!(n-k)!}"),
    ("limit", r"\lim_{h \to 0} \frac{f(x+h) - f(x)}{h}"),
    (
        "continued_fraction",
        r"x = a_0 + \frac{1}{a_1 + \frac{1}{a_2}}",
    ),
    (
        "greek",
        r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta",
    ),
    ("exponents", r"x^{2n} + y^{2n} = z^{2n}"),
    (
        "stretchy_delimiters",
        r"\left( \frac{a}{b} \right) + \left[ c \right]",
    ),
    ("sub_sup", r"x_i^2 + y_j^2"),
    ("product", r"\prod_{i=1}^{n} i = n!"),
];

fn output_directory() -> PathBuf {
    let directory = std::env::var("WATERUI_MATH_GALLERY_DIR").map_or_else(
        |_| PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("waterui-math-gallery"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("could not create {}: {error}", directory.display()));
    directory
}

/// A formula on white, in a box of `size`: the gallery is reviewed as images,
/// and a transparent ground lets a hole in a glyph read as the absence it is.
fn on_white(typeset: &Typeset, size: Size) -> Drawing {
    Drawing::record(size, |cx| {
        cx.fill(Rect::from_size(size), WorkingColor::WHITE);
        typeset.record(cx, WorkingColor::BLACK);
    })
}

#[test]
fn renders_the_formula_gallery_on_both_engines() {
    let directory = output_directory();
    let gpu =
        OffscreenRenderer::new().expect("the formula gallery requires a GPU engine on this host");
    let cpu = OffscreenRenderer::cpu().expect("the formula gallery requires the CPU rasteriser");
    let pixels = OffscreenSize::try_from_pixels(560, 200).expect("the gallery size is nonzero");
    let size = Size::new(560.0, 200.0);

    let fonts = FontCollection::system();
    for &(name, source) in GALLERY {
        let typeset = Typeset::new(
            &fonts,
            DEFAULT_MATH_FAMILY,
            source,
            48.0,
            MathStyle::Display,
        )
        .unwrap_or_else(|error| panic!("could not typeset {source}: {error}"));
        // One drawing renders on both engines, each registering the math face
        // for itself.
        let drawing = on_white(&typeset, size);

        let image = gpu
            .render_drawing(&drawing, pixels)
            .unwrap_or_else(|error| panic!("could not render {source} on the GPU engine: {error}"));
        let path = directory.join(format!("{name}_gpu.png"));
        image
            .save_png(&path)
            .unwrap_or_else(|error| panic!("could not write {}: {error}", path.display()));
        println!("saved {}", path.display());

        let image = cpu
            .render_drawing(&drawing, pixels)
            .unwrap_or_else(|error| {
                panic!("could not render {source} on the CPU rasteriser: {error}")
            });
        let path = directory.join(format!("{name}_cpu.png"));
        image
            .save_png(&path)
            .unwrap_or_else(|error| panic!("could not write {}: {error}", path.display()));
        println!("saved {}", path.display());
    }
}

/// A PNG stays out of the repository: it is a render artifact, so the path is
/// under `target/` next to the binaries.
#[test]
fn gallery_output_directory_defaults_under_target() {
    // `WATERUI_MATH_GALLERY_DIR` is the override for a human reviewing images.
    if std::env::var("WATERUI_MATH_GALLERY_DIR").is_err() {
        assert!(output_directory().is_absolute());
    }
}

#[test]
fn gallery_directory_is_created_on_first_use() {
    let directory = output_directory();
    assert!(Path::new(&directory).is_dir());
}
