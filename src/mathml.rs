//! Publishing the semantic tree as `MathML`.
//!
//! A formula drawn through [`crate::scene`] reaches the screen as filled paths
//! and glyph runs. To a screen reader that is a picture with no content, which
//! is why the tree is kept after layout rather than consumed by it: `MathML` is
//! what assistive technology actually reads, and this is where it comes from.
//!
//! The markup is written through an XML writer rather than assembled from
//! strings, so tags balance and content is escaped by construction — a formula
//! containing `<` or `&` is ordinary, not an edge case.

use alloc::string::String;
use alloc::vec::Vec;

use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use crate::ast::{MathItem, MathStyle, MathVariant};

/// What the writer is told when it refuses a byte it cannot refuse.
///
/// The buffer is a [`Vec<u8>`], whose [`std::io::Write`] impl has no failure
/// mode, so every `io::Result` threaded through this module is `Ok` by
/// construction. Saying so once here is what keeps [`to_mathml`] total.
const WRITE_CANNOT_FAIL: &str = "writing MathML into an in-memory buffer cannot fail";

/// Renders `item` as a `MathML` `<math>` element.
///
/// `style` sets the `display` attribute, which is the difference between a
/// formula announced as set on its own line and one announced inline.
///
/// This is total: a tree that parsed always has markup. The buffer is a
/// [`Vec<u8>`] that never refuses a write, and every byte put into it comes
/// either from `&str` content or from the escaper's ASCII entities, so the
/// bytes are always UTF-8. A `Result` here would be an error no caller could
/// act on, and the caller that had one discarded it and published an empty
/// accessibility payload instead.
///
/// # Panics
///
/// Panics if the buffer refuses a write or if the bytes it collected are not
/// UTF-8 — neither of which the paragraph above leaves reachable. The
/// assertion is what makes the function total; it is not a failure mode a
/// caller can encounter or handle.
#[must_use]
pub fn to_mathml(item: &MathItem, style: MathStyle) -> String {
    let mut writer = Writer::new(Vec::new());

    let mut root = BytesStart::new("math");
    root.push_attribute(("xmlns", "http://www.w3.org/1998/Math/MathML"));
    root.push_attribute((
        "display",
        if style.is_display() {
            "block"
        } else {
            "inline"
        },
    ));
    write(&mut writer, Event::Start(root));
    element(&mut writer, item, None);
    write(&mut writer, Event::End(BytesEnd::new("math")));

    String::from_utf8(writer.into_inner()).expect("MathML is written from UTF-8 text")
}

fn write(writer: &mut Writer<Vec<u8>>, event: Event<'_>) {
    writer.write_event(event).expect(WRITE_CANNOT_FAIL);
}

fn leaf(writer: &mut Writer<Vec<u8>>, tag: &str, text: &str, variant: Option<MathVariant>) {
    let mut start = BytesStart::new(tag);
    if let Some(variant) = variant {
        start.push_attribute(("mathvariant", variant.mathml()));
    }
    write(writer, Event::Start(start));
    write(writer, Event::Text(BytesText::new(text)));
    write(writer, Event::End(BytesEnd::new(tag)));
}

fn wrap(
    writer: &mut Writer<Vec<u8>>,
    tag: &str,
    children: &[&MathItem],
    variant: Option<MathVariant>,
) {
    write(writer, Event::Start(BytesStart::new(tag)));
    for child in children {
        element(writer, child, variant);
    }
    write(writer, Event::End(BytesEnd::new(tag)));
}

fn element(writer: &mut Writer<Vec<u8>>, item: &MathItem, variant: Option<MathVariant>) {
    match item {
        MathItem::Ident(text) => leaf(writer, "mi", text.as_str(), variant),
        MathItem::Number(text) => leaf(writer, "mn", text.as_str(), variant),
        MathItem::Text(text) => leaf(writer, "mtext", text.as_str(), variant),
        MathItem::Operator(operator) => leaf(writer, "mo", operator.glyph.as_str(), variant),
        MathItem::Space(em) => {
            let mut space = BytesStart::new("mspace");
            let width = alloc::format!("{em}em");
            space.push_attribute(("width", width.as_str()));
            write(writer, Event::Empty(space));
        }
        MathItem::Styled {
            variant: styled,
            body,
        } => element(writer, body, Some(*styled)),
        MathItem::Row(items) => {
            let children: Vec<&MathItem> = items.iter().collect();
            wrap(writer, "mrow", &children, variant);
        }
        MathItem::Fraction {
            numerator,
            denominator,
        } => wrap(writer, "mfrac", &[numerator, denominator], variant),
        MathItem::Radical { radicand, degree } => match degree {
            None => wrap(writer, "msqrt", &[radicand], variant),
            Some(degree) => wrap(writer, "mroot", &[radicand, degree], variant),
        },
        MathItem::Scripts { base, sub, sup } => match (sub, sup) {
            (Some(sub), Some(sup)) => wrap(writer, "msubsup", &[base, sub, sup], variant),
            (Some(sub), None) => wrap(writer, "msub", &[base, sub], variant),
            (None, Some(sup)) => wrap(writer, "msup", &[base, sup], variant),
            (None, None) => element(writer, base, variant),
        },
        MathItem::Fenced { open, body, close } => {
            write(writer, Event::Start(BytesStart::new("mrow")));
            if let Some(open) = open {
                leaf(writer, "mo", open.glyph.as_str(), variant);
            }
            element(writer, body, variant);
            if let Some(close) = close {
                leaf(writer, "mo", close.glyph.as_str(), variant);
            }
            write(writer, Event::End(BytesEnd::new("mrow")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex;

    fn mathml(source: &str) -> String {
        let item = latex::parse(source).unwrap_or_else(|error| panic!("`{source}`: {error}"));
        to_mathml(&item, MathStyle::Text)
    }

    #[test]
    fn a_fraction_is_an_mfrac() {
        let markup = mathml(r"\frac{a}{b}");
        assert!(markup.contains("<mfrac>"), "got {markup}");
        assert!(markup.contains("</mfrac>"), "got {markup}");
    }

    #[test]
    fn a_square_root_is_an_msqrt_and_a_cube_root_an_mroot() {
        assert!(mathml(r"\sqrt{x}").contains("<msqrt>"));
        assert!(mathml(r"\sqrt[3]{x}").contains("<mroot>"));
    }

    #[test]
    fn scripts_choose_the_matching_element() {
        assert!(mathml("x^2").contains("<msup>"));
        assert!(mathml("x_i").contains("<msub>"));
        assert!(mathml("x_i^2").contains("<msubsup>"));
    }

    #[test]
    fn the_root_declares_the_mathml_namespace() {
        let markup = mathml("x");
        assert!(
            markup.contains("http://www.w3.org/1998/Math/MathML"),
            "assistive technology keys off the namespace: {markup}"
        );
    }

    #[test]
    fn display_mode_reaches_the_markup() {
        let item = latex::parse(r"\frac{a}{b}").expect("parses");
        let block = to_mathml(&item, MathStyle::Display);
        let inline = to_mathml(&item, MathStyle::Text);
        assert!(block.contains("display=\"block\""), "got {block}");
        assert!(inline.contains("display=\"inline\""), "got {inline}");
    }

    /// A relation like `<` must not corrupt the markup. This is exactly what
    /// hand-assembled XML gets wrong.
    #[test]
    fn markup_special_characters_are_escaped() {
        let markup = mathml("a<b");
        assert!(
            !markup.contains("<mo><</mo>"),
            "a raw `<` would make the document unparseable: {markup}"
        );
        assert!(markup.contains("&lt;"), "expected an escaped `<`: {markup}");
    }
}
