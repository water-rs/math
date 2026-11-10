//! The `Math` view.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::sync::Arc;
use core::cell::RefCell;

use nami::SignalExt as _;
use nami::signal::IntoComputed;
use peniko::{Blob, FontData};
use suiteki::Str;
use waterui_core::layout::Size;
use waterui_core::{Computed, Environment, View};
use waterui_graphics::canvas::{Canvas, Cx, DrawingCx, Font};
use waterui_graphics::color::{Color, ForegroundColor, WorkingColor};
use waterui_layout::frame::Frame;
use waterui_text::FontCollection;

use crate::ast::{MathItem, MathStyle};
use crate::font::MathFont;
use crate::layout::{Layouter, MathLayout};
use crate::{draw, latex, mathml, speech};

/// The math family used when the caller names none.
///
/// Only a face with an OpenType `MATH` table can set mathematics, so this is
/// not a stylistic default that something else could stand in for.
pub const DEFAULT_MATH_FAMILY: &str = "STIX Two Math";

/// Why a formula could not be prepared for drawing.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MathError {
    /// The source did not parse.
    #[error(transparent)]
    Latex(#[from] crate::latex::LatexError),
    /// No installed family by that name carries a `MATH` table.
    #[error(
        "no math font available: `{family}` is not installed, or carries no OpenType MATH table"
    )]
    NoFont {
        /// The family that was asked for.
        family: String,
    },
    /// The formula parsed but could not be laid out.
    #[error(transparent)]
    Layout(#[from] crate::layout::LayoutError),
}

/// A rendered mathematical formula.
///
/// The source is LaTeX. It is parsed into a semantic tree, laid out against the
/// OpenType `MATH` table of the chosen family, and drawn on a `Canvas`, so it
/// renders on whichever renderer draws the screen around it. A change of the
/// source typesets the formula again and re-records that one region of the
/// canvas.
#[derive(Debug, Clone)]
pub struct Math {
    source: Computed<Str>,
    style: MathStyle,
    font_size: f32,
    family: Str,
    color: Option<Color>,
}

impl Math {
    /// A formula from LaTeX source.
    ///
    /// The source takes a signal, so a formula bound to state updates without
    /// its subtree being rebuilt.
    #[must_use]
    pub fn new(source: impl IntoComputed<Str>) -> Self {
        Self {
            source: source.into_computed(),
            style: MathStyle::Text,
            font_size: 18.0,
            family: Str::from_static(DEFAULT_MATH_FAMILY),
            color: None,
        }
    }

    /// Sets the formula on its own line, in display style.
    #[must_use]
    pub const fn display(mut self) -> Self {
        self.style = MathStyle::Display;
        self
    }

    /// Sets the formula inline, in text style.
    #[must_use]
    pub const fn inline(mut self) -> Self {
        self.style = MathStyle::Text;
        self
    }

    /// Sets the em size the formula is set at.
    ///
    /// # Panics
    ///
    /// Panics if `size` is not finite and positive, which is a caller bug
    /// rather than bad input.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        assert!(
            size.is_finite() && size > 0.0,
            "math font size must be finite and positive, got {size}"
        );
        self.font_size = size;
        self
    }

    /// Sets the math family. It must carry an OpenType `MATH` table.
    #[must_use]
    pub fn font_family(mut self, family: impl Into<Str>) -> Self {
        self.family = family.into();
        self
    }

    /// Overrides the colour the formula is drawn in.
    #[must_use]
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// A formula prepared for drawing: the tree, its `MathML`, and its layout.
#[derive(Debug, Clone)]
pub struct PreparedMath {
    /// The semantic tree, kept because it is the accessibility representation.
    pub item: MathItem,
    /// The tree as `MathML`, for the accessibility payload.
    pub mathml: String,
    /// What to draw, positioned relative to the baseline origin.
    pub layout: MathLayout,
    /// Total advance width in pixels.
    pub width: f32,
    /// Height above the baseline.
    pub ascent: f32,
    /// Depth below the baseline.
    pub descent: f32,
}

impl PreparedMath {
    /// Total height.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.ascent + self.descent
    }
}

/// Parses and measures `source` against `font`.
///
/// # Errors
///
/// Returns [`MathError`] when the source does not parse or cannot be laid out.
pub fn prepare(
    source: &str,
    font: &FontData,
    font_size: f32,
    style: MathStyle,
) -> Result<PreparedMath, MathError> {
    let item = latex::parse(source)?;
    let math_font = MathFont::new(font.data.data(), font.index)
        .map_err(|error| MathError::Layout(crate::layout::LayoutError::Font(error)))?;
    let layouter = Layouter::new(&math_font)?;
    let layout = layouter.layout(&item, font_size, style)?;
    let mathml = mathml::to_mathml(&item, style);

    Ok(PreparedMath {
        item,
        mathml,
        width: layout.width,
        ascent: layout.ascent,
        descent: layout.descent,
        layout,
    })
}

/// Finds an installed family that can set mathematics.
///
/// # Errors
///
/// Returns [`MathError::NoFont`] when no installed face by that name carries a
/// `MATH` table. There is deliberately no substitute: a face without the table
/// has no layout constants, so drawing with it would not be the same formula
/// set differently, it would be a formula with no geometry.
pub fn resolve_font(fonts: &FontCollection, family: &str) -> Result<FontData, MathError> {
    let no_font = || MathError::NoFont {
        family: String::from(family),
    };

    for face in fonts.faces(family) {
        let data = FontData::new(Blob::new(Arc::new(face.bytes())), face.index());
        if MathFont::new(data.data.data(), data.index).is_ok() {
            return Ok(data);
        }
    }
    Err(no_font())
}

/// A formula typeset against a math face, ready to draw.
///
/// [`Math`] typesets its source into one of these whenever the source changes
/// and draws it on its canvas. Code that wants a formula outside a view —
/// recording it into a `Drawing`, for instance — typesets one itself and calls
/// [`Typeset::record`].
#[derive(Debug, Clone)]
pub struct Typeset {
    prepared: PreparedMath,
    font: Font,
}

impl Typeset {
    /// Typesets `source` at `font_size` in `style`, set in `family` as the
    /// application's `fonts` provide it.
    ///
    /// # Errors
    ///
    /// Returns [`MathError`] when no installed face of `family` carries a
    /// `MATH` table, or the source does not parse or cannot be laid out.
    pub fn new(
        fonts: &FontCollection,
        family: &str,
        source: &str,
        font_size: f32,
        style: MathStyle,
    ) -> Result<Self, MathError> {
        let data = resolve_font(fonts, family)?;
        let font = Font::from_bytes(Arc::<[u8]>::from(data.data.data()), data.index);
        Self::with_font(&data, font, source, font_size, style)
    }

    fn with_font(
        data: &FontData,
        font: Font,
        source: &str,
        font_size: f32,
        style: MathStyle,
    ) -> Result<Self, MathError> {
        Ok(Self {
            prepared: prepare(source, data, font_size, style)?,
            font,
        })
    }

    /// The typeset formula: its tree, `MathML` and layout.
    #[must_use]
    pub const fn prepared(&self) -> &PreparedMath {
        &self.prepared
    }

    /// The box the formula occupies.
    ///
    /// A formula is text: it is exactly as wide as its advance and as tall as
    /// its ascent plus descent, and a container that names neither axis should
    /// get that rather than nothing. `None` when the formula is empty, which is
    /// not a size.
    #[must_use]
    pub fn size(&self) -> Option<Size> {
        let size = Size::new(self.prepared.width, self.prepared.height());
        (size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0)
            .then_some(size)
    }

    /// Draws the formula on `cx`, its top-left corner at the canvas origin,
    /// painted with `ink`. The ink is live: a theme change repaints the formula
    /// without recording it again.
    pub fn draw(&self, cx: &mut Cx, ink: &Computed<WorkingColor>) {
        draw::draw(
            &self.prepared.layout,
            cx,
            &self.font,
            ink,
            0.0,
            self.prepared.ascent,
        );
    }

    /// Records the formula into a `Drawing` being recorded, its top-left corner
    /// at the origin, painted with `ink`.
    pub fn record(&self, cx: &mut DrawingCx, ink: WorkingColor) {
        draw::draw(
            &self.prepared.layout,
            cx,
            &self.font,
            &ink,
            0.0,
            self.prepared.ascent,
        );
    }
}

/// How a view typesets its source: the family, size and style it is set in,
/// and the face and last formula it has resolved.
///
/// Measuring a formula *is* laying it out, and the view reads the typeset
/// formula for its canvas, its intrinsic size and nothing else, so the last one
/// is kept for as long as the source is unchanged.
struct Setter {
    /// The application's font collection, installed by the host and shared with
    /// every other component that shapes or typesets text. The math family is
    /// looked up in it; this never builds one of its own.
    fonts: FontCollection,
    family: Str,
    font_size: f32,
    style: MathStyle,
    cache: RefCell<SetterCache>,
}

#[derive(Default)]
struct SetterCache {
    /// The resolved face, which is one canvas font for the view's whole life:
    /// a font the canvas has not seen is a registration with the renderer.
    font: Option<(FontData, Font)>,
    /// The formula last typeset, or `None` when it could not be.
    typeset: Option<(Str, Option<Rc<Typeset>>)>,
}

impl Setter {
    fn new(fonts: FontCollection, family: Str, font_size: f32, style: MathStyle) -> Self {
        Self {
            fonts,
            family,
            font_size,
            style,
            cache: RefCell::default(),
        }
    }

    /// The math face this formula is set in, resolved once and kept.
    ///
    /// `None` means no installed family by that name carries a `MATH` table, and
    /// there is deliberately no substitute — a face without the table has no
    /// layout constants, so the formula has no geometry at all.
    fn font(&self) -> Option<(FontData, Font)> {
        let mut cache = self.cache.borrow_mut();
        if cache.font.is_none() {
            match resolve_font(&self.fonts, self.family.as_str()) {
                Ok(data) => {
                    let font = Font::from_bytes(Arc::<[u8]>::from(data.data.data()), data.index);
                    cache.font = Some((data, font));
                }
                Err(error) => {
                    tracing::error!(%error, "math formula has no usable font");
                    return None;
                }
            }
        }
        cache.font.clone()
    }

    /// `source` typeset, or `None` when the face is missing or the source does
    /// not parse or lay out, which is reported.
    fn typeset(&self, source: &Str) -> Option<Rc<Typeset>> {
        if let Some((typeset, result)) = &self.cache.borrow().typeset
            && typeset == source
        {
            return result.clone();
        }
        let result = self.font().and_then(|(data, font)| {
            match Typeset::with_font(&data, font, source.as_str(), self.font_size, self.style) {
                Ok(typeset) => Some(Rc::new(typeset)),
                Err(error) => {
                    tracing::error!(%error, formula = %source, "could not typeset math formula");
                    None
                }
            }
        });
        self.cache.borrow_mut().typeset = Some((source.clone(), result.clone()));
        result
    }
}

impl View for Math {
    /// # Panics
    ///
    /// Panics when the host installed no [`FontCollection`] in the environment.
    /// A formula cannot be typeset without one, and building a second
    /// collection here is the per-view font enumeration this component was
    /// changed to stop doing.
    fn body(self, env: &Environment) -> impl View {
        let ink = self
            .color
            .clone()
            .map(|color| color.resolve(env))
            .or_else(|| {
                env.query::<ForegroundColor, Computed<WorkingColor>>()
                    .cloned()
            })
            .unwrap_or_else(|| Computed::constant(WorkingColor::BLACK));

        let setter = Rc::new(Setter::new(
            FontCollection::from_env(env),
            self.family,
            self.font_size,
            self.style,
        ));
        let typeset: Computed<Option<Rc<Typeset>>> = self
            .source
            .map(move |source| setter.typeset(&source))
            .computed();
        let size = typeset
            .map(|typeset| typeset.and_then(|typeset| typeset.size()))
            .computed();
        let style = self.style;
        let speech = self
            .source
            .map(move |source| accessibility_speech(&source, style).map(Str::from))
            .computed();

        // The canvas offers the spoken formula as its name, which an
        // application's own `.a11y_label(…)` overrides: attached here as
        // metadata it would be nearest-consumer and beat the application's
        // label instead of yielding to it.
        Frame::new(
            Canvas::new(move |cx| {
                // The closure runs once; a change of the source re-records
                // this one region, typeset again.
                cx.scope(typeset, move |cx, typeset: &Option<Rc<Typeset>>| {
                    if let Some(typeset) = typeset {
                        typeset.draw(cx, &ink);
                    }
                });
            })
            .intrinsic_size(size)
            .offered_label(speech),
        )
    }
}

/// What the formula's accessibility node says.
///
/// A source that does not parse has no semantic tree and therefore no markup to
/// speak from, and it does not draw either — so the only content that exists is
/// the LaTeX the author wrote, and that is what the node says. The parse failure
/// is reported: a formula that silently reads as its own source is a formula
/// that silently did not render.
///
/// A formula that parsed but that `MathCAT` cannot speak leaves the node
/// unnamed: content that cannot say anything true about itself. There is
/// deliberately nothing generic to say in its place — "math formula" tells a
/// listener nothing and would hide the failure — so the error is logged with
/// the formula that produced it.
fn accessibility_speech(source: &Str, style: MathStyle) -> Option<String> {
    let item = match latex::parse(source.as_str()) {
        Ok(item) => item,
        Err(error) => {
            tracing::error!(
                %error,
                formula = %source,
                "formula does not parse; its source is all the accessibility node can say"
            );
            return Some(String::from(source.as_str()));
        }
    };

    match speech::speak(&mathml::to_mathml(&item, style)) {
        Ok(spoken) => Some(spoken),
        Err(error) => {
            tracing::error!(
                %error,
                formula = %source,
                "formula cannot be spoken; its accessibility node stays unnamed"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use suiteki::Str;
    use waterui_core::{Environment, View};
    use waterui_text::FontCollection;

    use super::{
        DEFAULT_MATH_FAMILY, Math, Setter, Typeset, accessibility_speech, prepare, resolve_font,
    };
    use crate::ast::MathStyle;
    use crate::{latex, mathml, speech};

    /// A formula built where no host installed a collection says so, instead of
    /// enumerating the system's fonts on its own — which is what it used to do,
    /// once per view.
    #[test]
    #[should_panic(expected = "no font collection is installed in the environment")]
    fn a_formula_without_a_font_collection_names_the_host() {
        let _ = Math::new(Str::from_static("x")).body(&Environment::new());
    }

    /// The node carries the formula spoken as a sentence, not the markup.
    ///
    /// It used to carry the `MathML` itself, which a screen reader reads out as
    /// tag names. This pins that the view publishes exactly what the speech
    /// engine says about the same formula.
    #[test]
    fn the_accessibility_node_carries_speech_rather_than_markup() {
        let source = Str::from_static(r"\frac{a}{b}");
        let markup = mathml::to_mathml(
            &latex::parse(source.as_str()).expect("the fixture formula parses"),
            MathStyle::Text,
        );
        let expected = speech::speak(&markup).expect("the fixture formula speaks");

        assert_eq!(
            accessibility_speech(&source, MathStyle::Text).as_deref(),
            Some(expected.as_str())
        );
        assert!(
            !expected.contains('<'),
            "the node must carry a sentence, not markup, got `{expected}`"
        );
    }

    /// A source that does not parse has no tree and therefore no markup, and it
    /// does not draw either. The one thing that does exist is what the author
    /// wrote, so that is what the node says — never an empty payload.
    #[test]
    fn a_source_that_does_not_parse_still_says_something() {
        let source = Str::from_static(r"\begin{matrix}a & b\\c & d\end{matrix}");
        assert!(
            latex::parse(source.as_str()).is_err(),
            "this fixture exists to be refused by the parser"
        );

        assert_eq!(
            accessibility_speech(&source, MathStyle::Text).as_deref(),
            Some(source.as_str()),
            "the node must carry the source when there is nothing to speak from"
        );
    }

    /// `\mathrm{d}` is the differential the integral gallery formula needs:
    /// the command must parse, lay out, and reach the markup upright — an
    /// unsupported construct there leaves the whole formula unrenderable.
    #[test]
    fn a_mathrm_group_lays_out_the_integral() {
        let font = resolve_font(&FontCollection::system(), DEFAULT_MATH_FAMILY)
            .expect("the math face resolves");
        let prepared = prepare(r"\int_0^1 x\,\mathrm{d}x", &font, 48.0, MathStyle::Display)
            .expect("an integral with an upright differential parses and lays out");

        assert!(prepared.width > 0.0, "the laid-out formula has ink");
        assert!(
            prepared.mathml.contains("mathvariant=\"normal\""),
            "the differential is upright in the markup: {}",
            prepared.mathml
        );
    }

    /// The box a formula measures follows the source: typesetting it again is
    /// what changes the canvas' intrinsic size, and an unchanged source is not
    /// typeset twice.
    #[test]
    fn the_typeset_follows_the_source() {
        let setter = Setter::new(
            FontCollection::system(),
            Str::from_static(DEFAULT_MATH_FAMILY),
            18.0,
            MathStyle::Text,
        );
        let narrow = setter
            .typeset(&Str::from_static("x"))
            .expect("a letter typesets");
        let again = setter
            .typeset(&Str::from_static("x"))
            .expect("the same source is kept");
        assert!(
            alloc::rc::Rc::ptr_eq(&narrow, &again),
            "an unchanged source is not typeset again"
        );

        let wide = setter
            .typeset(&Str::from_static(r"x + y + z"))
            .expect("a sum typesets");
        let (narrow, wide) = (
            narrow.size().expect("a letter has a box"),
            wide.size().expect("a sum has a box"),
        );
        assert!(wide.width > narrow.width, "{wide:?} is wider than {narrow:?}");
    }

    /// A formula whose source does not parse draws nothing and has no size,
    /// without taking the view down.
    #[test]
    fn a_source_that_does_not_parse_has_no_typeset() {
        let setter = Setter::new(
            FontCollection::system(),
            Str::from_static(DEFAULT_MATH_FAMILY),
            18.0,
            MathStyle::Text,
        );
        assert!(
            setter
                .typeset(&Str::from_static(r"\begin{matrix}a\end{matrix}"))
                .is_none()
        );
        let result = Typeset::new(
            &FontCollection::system(),
            DEFAULT_MATH_FAMILY,
            r"\begin{matrix}a\end{matrix}",
            18.0,
            MathStyle::Text,
        );
        assert!(result.is_err());
    }
}
