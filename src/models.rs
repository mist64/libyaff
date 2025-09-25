use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct YaffFont {
    pub yaff_version: Option<String>,
    pub name: Option<String>,
    pub family: Option<String>,
    pub subfamily: Option<String>,
    pub revision: Option<String>,
    pub point_size: Option<f32>,
    pub line_height: Option<i32>,
    pub style: Option<String>,
    pub weight: Option<String>,
    pub slant: Option<String>,
    pub setwidth: Option<String>,
    pub decoration: Option<String>,
    pub x_height: Option<i32>,
    pub cap_height: Option<i32>,
    pub ascent: Option<i32>,
    pub descent: Option<i32>,
    pub pixel_size: Option<i32>,
    pub leading: Option<i32>,
    pub raster_bounds: Option<(i32, i32, i32, i32)>,
    pub ink_bounds: Option<(i32, i32, i32, i32)>,
    pub raster_size: Option<(u32, u32)>,
    pub cell_size: Option<(u32, u32)>,
    pub bounding_box: Option<(u32, u32)>,
    pub average_width: Option<f32>,
    pub max_width: Option<i32>,
    pub cap_width: Option<i32>,
    pub digit_width: Option<i32>,
    pub spacing: Option<FontSpacing>,
    pub direction: Option<WritingDirection>,
    pub bold_smear: Option<i32>,
    pub italic_pitch: Option<(i32, i32)>,
    pub outline_thickness: Option<i32>,
    pub underline_thickness: Option<i32>,
    pub underline_descent: Option<i32>,
    pub strikethrough_thickness: Option<i32>,
    pub strikethrough_ascent: Option<i32>,
    pub superscript_size: Option<i32>,
    pub superscript_offset: Option<(i32, i32)>,
    pub subscript_size: Option<i32>,
    pub subscript_offset: Option<(i32, i32)>,
    pub small_cap_size: Option<i32>,
    pub word_space: Option<i32>,
    pub min_word_space: Option<i32>,
    pub max_word_space: Option<i32>,
    pub sentence_space: Option<i32>,
    pub author: Option<String>,
    pub foundry: Option<String>,
    pub copyright: Option<String>,
    pub notice: Option<String>,
    pub device: Option<String>,
    pub pixel_aspect: Option<(u32, u32)>,
    pub dpi: Option<(u32, u32)>,
    pub converter: Option<String>,
    pub source_name: Option<String>,
    pub source_format: Option<String>,
    pub history: Option<String>,
    pub encoding: Option<String>,
    pub default_char_label_raw: Option<String>,
    pub word_boundary_label_raw: Option<String>,
    pub global_left_bearing: Option<i32>,
    pub global_right_bearing: Option<i32>,
    pub global_shift_up: Option<i32>,
    pub glyphs: Vec<GlyphDefinition>,
}

/// A single glyph definition including bitmap data and typography metrics.
///
/// Each glyph can have multiple labels (Unicode codepoints, legacy encodings, or tags)
/// and includes bitmap pixel data plus optional bearing and kerning information.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GlyphDefinition {
    pub labels: Vec<Label>,
    pub bitmap: Bitmap,
    pub left_bearing: Option<i32>,
    pub right_bearing: Option<i32>,
    pub shift_up: Option<i32>,
    pub top_bearing: Option<i32>,
    pub bottom_bearing: Option<i32>,
    pub shift_left: Option<i32>,
    pub right_kerning: Option<HashMap<Label, f32>>,
    pub left_kerning: Option<HashMap<Label, f32>>,
    pub scalable_width: Option<f32>,
}

/// Glyph labeling system supporting Unicode, legacy codepoints, and custom tags.
///
/// YAFF supports multiple labeling schemes to accommodate different font encodings
/// and use cases, from modern Unicode fonts to legacy 8-bit character sets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Label {
    Unicode(Vec<u32>),   // Unicode code
    Codepoint(Vec<u16>), // encoding-specfic code
    Tag(String),         // tag like 'acircumflex'
    Anonymous,
}

/// Bitmap representation of a glyph as a 2D boolean array.
///
/// Pixels are stored row-by-row, with `true` representing foreground pixels
/// and `false` representing background pixels.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Bitmap {
    pub pixels: Vec<Vec<bool>>,
    pub width: usize,
    pub height: usize,
}

impl Bitmap {
    pub fn is_empty(&self) -> bool {
        self.width == 0 && self.height == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSpacing {
    Proportional,
    Monospace,
    CharacterCell,
    MultiCell,
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    InvalidSyntax {
        line: usize,
        message: String,
    },
    UnexpectedEndOfInput,
    SemanticError {
        line: usize,
        message: String,
    },
    InvalidPropertyValue {
        line: usize,
        property_key: String,
        value: String,
        expected_format: String,
    },
    UnsupportedFeature {
        line: usize,
        feature_name: String,
        message: String,
    },
    InconsistentGlyphLineLength {
        line: usize,
    },
    InvalidGlyphCharacter {
        line: usize,
        char_found: char,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(err) => write!(f, "IO error: {err}"),
            ParseError::InvalidSyntax { line, message } => {
                write!(f, "Invalid syntax at line {line}: {message}")
            }
            ParseError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            ParseError::UnsupportedFeature {
                line,
                feature_name,
                message,
            } => {
                write!(
                    f,
                    "Unsupported feature '{feature_name}' at line {line}: {message}"
                )
            }
            ParseError::SemanticError { line, message } => {
                write!(f, "Semantic error at line {line}: {message}")
            }
            ParseError::InvalidPropertyValue {
                line,
                property_key,
                value,
                expected_format,
            } => {
                write!(
                    f,
                    "Invalid property value for '{property_key}' at line {line}: '{value}'. Expected: {expected_format}"
                )
            }
            ParseError::InconsistentGlyphLineLength { line } => {
                write!(f, "Inconsistent glyph line length at line {line}")
            }
            ParseError::InvalidGlyphCharacter { line, char_found } => {
                write!(
                    f,
                    "Invalid glyph character '{char_found}' at line {line}"
                )
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::Io(err)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum LineType {
    KeyValue {
        key: String, // Empty string "" for anonymous ':'
        value_on_line: Option<String>,
    },
    BitmapLine {
        content: String, // Content after indent, e.g. ".@." or "-"
    },
    Continuation {
        content: String, // Content after indent
    },
    Unknown {
        // Typically for unindented lines that aren't KeyValue
        original_text: String,
    },
    #[default]
    Empty,
}
