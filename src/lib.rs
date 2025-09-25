//! # LibYAFF: Yet Another Font Format Library
//!
//! A Rust library for parsing, manipulating, and generating bitmap fonts in the YAFF format.
//!
//! ## Features
//!
//! - **Complete YAFF format support**: Parse and generate YAFF 1.0.x format files
//! - **Unicode and legacy encoding**: Support for Unicode, codepoint, and tag-based glyph labeling
//! - **Advanced typography**: Kerning, bearing adjustments, and font metrics
//! - **Robust parsing**: Handles format variations and provides detailed error messages
//! - **Memory efficient**: Optimized for embedded and resource-constrained environments
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "parsing")]
//! # {
//! use libyaff::{YaffFont, to_yaff_string};
//!
//! // Load a YAFF font from file
//! let font = YaffFont::from_path("my_font.yaff")?;
//! println!("Loaded font: {}", font.name.as_ref().unwrap_or(&"".to_string()));
//!
//! // Access font metrics
//! if let Some(ascent) = font.ascent {
//!     println!("Font ascent: {}", ascent);
//! }
//!
//! // Convert back to YAFF format
//! let yaff_content = to_yaff_string(&font);
//! std::fs::write("output.yaff", yaff_content)?;
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! ## Font Structure
//!
//! A YAFF font consists of:
//! - **Metadata**: Font name, family, size, style information
//! - **Metrics**: Ascent, descent, line height, kerning data
//! - **Glyphs**: Individual character bitmaps with labels and metrics
//!
//! ## Error Handling
//!
//! All parsing operations return `Result<T, ParseError>` with detailed error information
//! including line numbers and context for debugging malformed YAFF files.

#[cfg(feature = "encoding")]
mod encoder;
mod models;
#[cfg(feature = "parsing")]
mod parser;
mod utils;

#[cfg(feature = "encoding")]
pub use crate::encoder::to_yaff_string;
pub use crate::models::*;
#[cfg(feature = "parsing")]
pub use crate::parser::{classify_line, parse_key_as_label};
pub use crate::utils::{
    calculate_ascent, convert_codepoint_to_unicode_labels, minimize_all_bounding_boxes,
    minimize_glyph_bounding_box, set_ascent,
};
#[cfg(feature = "parsing")]
use std::fs::File;
#[cfg(feature = "parsing")]
use std::io::BufReader;
#[cfg(feature = "parsing")]
use std::path::Path;

impl YaffFont {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "parsing")]
use std::str::FromStr;

#[cfg(feature = "parsing")]
impl FromStr for YaffFont {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::from_str(s)
    }
}

#[cfg(feature = "parsing")]
impl YaffFont {

    pub fn from_reader<R: std::io::Read>(mut reader: R) -> Result<YaffFont, ParseError> {
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;
        Self::from_str(&buffer)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, ParseError> {
        let file = File::open(path).map_err(ParseError::Io)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_creation_and_defaults() {
        let font = YaffFont::new();
        assert!(font.name.is_none());
        assert!(font.glyphs.is_empty());
        assert_eq!(font.yaff_version, None);
    }

    #[test]
    fn test_font_property_setting() {
        let mut font = YaffFont::new();
        font.name = Some("Test Font".to_string());
        font.ascent = Some(10);
        font.descent = Some(-2);

        assert_eq!(font.name, Some("Test Font".to_string()));
        assert_eq!(font.ascent, Some(10));
        assert_eq!(font.descent, Some(-2));
    }

    #[test]
    fn test_glyph_creation() {
        let glyph = GlyphDefinition {
            labels: vec![Label::Unicode(vec![65])], // 'A'
            bitmap: Bitmap::default(),
            left_bearing: Some(-1),
            right_bearing: Some(2),
            ..Default::default()
        };

        assert_eq!(glyph.labels.len(), 1);
        assert_eq!(glyph.left_bearing, Some(-1));
        assert_eq!(glyph.right_bearing, Some(2));
        assert!(glyph.bitmap.pixels.is_empty());
    }

    #[test]
    fn test_label_variants() {
        let unicode_label = Label::Unicode(vec![65, 66]); // "AB"
        let codepoint_label = Label::Codepoint(vec![65, 66]);
        let tag_label = Label::Tag("my-tag".to_string());
        let anonymous_label = Label::Anonymous;

        assert!(matches!(unicode_label, Label::Unicode(_)));
        assert!(matches!(codepoint_label, Label::Codepoint(_)));
        assert!(matches!(tag_label, Label::Tag(_)));
        assert!(matches!(anonymous_label, Label::Anonymous));
    }

    #[test]
    fn test_bitmap_creation() {
        let bitmap = Bitmap {
            pixels: vec![vec![true, false, true], vec![false, true, false]],
            width: 3,
            height: 2,
        };

        assert_eq!(bitmap.width, 3);
        assert_eq!(bitmap.height, 2);
        assert_eq!(bitmap.pixels.len(), 2);
        assert_eq!(bitmap.pixels[0].len(), 3);
    }
}
