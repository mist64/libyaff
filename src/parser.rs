use crate::models::*;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn from_str(s: &str) -> Result<YaffFont, ParseError> {
    let decoder = Decoder::new(s);
    decoder.parse_internal()
}

/// Classifies a single line from a YAFF file.
/// Returns a tuple of (line_type, indentation_level).
pub fn classify_line(line_str: &str) -> (LineType, usize) {
    let indent = line_str.chars().take_while(|&c| c.is_whitespace()).count();
    let content_after_indent = &line_str[indent..];
    let content_trimmed = content_after_indent.trim_end();

    if content_after_indent.is_empty() || content_after_indent.starts_with('#') {
        return (LineType::Empty, indent);
    }

    // ":"
    if content_trimmed == ":" {
        if indent == 0 {
            return (
                LineType::KeyValue {
                    key: "".to_string(), // Represent anonymous label key as empty string
                    value_on_line: None,
                },
                indent,
            );
        } else {
            return (
                LineType::Continuation {
                    content: ":".to_string(),
                },
                indent,
            );
        }
    }

    // "key:"
    if content_trimmed.ends_with(':') {
        let key = content_trimmed.trim_end_matches(':').trim_end().to_string();
        return (
            LineType::KeyValue {
                key,
                value_on_line: None,
            },
            indent,
        );
    }

    // "key: value"
    if let Some((key_raw, value_raw)) = content_after_indent.split_once(':') {
        let key = key_raw.trim_end().to_string();
        let value_trimmed_on_line = value_raw.trim();
        let value_on_line = if value_trimmed_on_line.is_empty() {
            None
        } else {
            Some(value_trimmed_on_line.to_string())
        };
        return (LineType::KeyValue { key, value_on_line }, indent);
    }

    // bitmap
    if content_trimmed == "-"
        || (!content_trimmed.is_empty() && content_trimmed.chars().all(|c| c == '.' || c == '@'))
    {
        if indent > 0 {
            return (
                LineType::BitmapLine {
                    content: content_after_indent.to_string(),
                },
                indent,
            );
        } else {
            return (
                LineType::Unknown {
                    original_text: line_str.to_string(),
                },
                indent,
            );
        }
    }

    if indent == 0 {
        (
            LineType::Unknown {
                original_text: line_str.to_string(),
            },
            indent,
        )
    } else {
        (
            LineType::Continuation {
                content: content_trimmed.to_string(),
            },
            indent,
        )
    }
}

// --- Regex definitions for individual label elements using std::sync::OnceLock ---
// For U+HEX or u+HEX
static RE_UNICODE_UPLUS_ELEMENT_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_unicode_uplus_element() -> &'static Regex {
    RE_UNICODE_UPLUS_ELEMENT_LOCK
        .get_or_init(|| Regex::new(r"^(?:u|U)\+([0-9a-fA-F]{1,6})$").unwrap())
}

// For 'char sequence'
static RE_UNICODE_CHAR_SEQ_ELEMENT_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_unicode_char_seq_element() -> &'static Regex {
    RE_UNICODE_CHAR_SEQ_ELEMENT_LOCK.get_or_init(|| Regex::new(r"^'(.*?)'$").unwrap())
}

// For "tag string" - this is for the whole key_str if it's a tag
static RE_TAG_LABEL_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_tag_label() -> &'static Regex {
    RE_TAG_LABEL_LOCK.get_or_init(|| Regex::new(r#"^"(.*?)"$"#).unwrap())
}

// For 0xHEX or 0XHEX
static RE_CODEPOINT_HEX_ELEMENT_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_codepoint_hex_element() -> &'static Regex {
    RE_CODEPOINT_HEX_ELEMENT_LOCK.get_or_init(|| Regex::new(r"^0[xX]([0-9a-fA-F]+)$").unwrap())
}

// For 0oOCT or 0OOCT
static RE_CODEPOINT_OCT_ELEMENT_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_codepoint_oct_element() -> &'static Regex {
    RE_CODEPOINT_OCT_ELEMENT_LOCK.get_or_init(|| Regex::new(r"^0[oO]([0-7]+)$").unwrap())
}

// For DECIMAL
static RE_CODEPOINT_DEC_ELEMENT_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_codepoint_dec_element() -> &'static Regex {
    RE_CODEPOINT_DEC_ELEMENT_LOCK.get_or_init(|| Regex::new(r"^([0-9]+)$").unwrap())
}

// Regex for deprecated unquoted tags (allows spaces, parentheses, and various characters, must start with letter or underscore)
static RE_UNQUOTED_TAG_LOCK: OnceLock<Regex> = OnceLock::new();
fn get_re_unquoted_tag() -> &'static Regex {
    RE_UNQUOTED_TAG_LOCK.get_or_init(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_./ ()-]*$").unwrap())
}

// Helper to parse a single Unicode element string (e.g., "u+0041" or "'A'")
fn parse_unicode_element(element_str: &str) -> Option<Vec<u32>> {
    if let Some(caps) = get_re_unicode_uplus_element().captures(element_str) {
        let hex_val = caps.get(1).unwrap().as_str();
        u32::from_str_radix(hex_val, 16).ok().map(|val| vec![val])
    } else if let Some(caps) = get_re_unicode_char_seq_element().captures(element_str) {
        let inner_str = caps.get(1).unwrap().as_str();
        Some(inner_str.chars().map(|c| c as u32).collect())
    } else {
        None
    }
}

// Helper to parse a sequence of Unicode elements (comma-separated)
fn parse_unicode_label_sequence(elements: &[&str]) -> Option<Label> {
    let mut unicode_values = Vec::new();
    for el_str in elements {
        if let Some(parsed_values) = parse_unicode_element(el_str) {
            unicode_values.extend(parsed_values);
        } else {
            return None; // Mixed types not allowed
        }
    }
    Some(Label::Unicode(unicode_values))
}

// Helper to parse a single Codepoint element string (e.g., "65", "0x41", "0o101")
fn parse_codepoint_element(element_str: &str) -> Option<u16> {
    let val_u32 = if let Some(caps) = get_re_codepoint_hex_element().captures(element_str) {
        u32::from_str_radix(caps.get(1).unwrap().as_str(), 16).ok()
    } else if let Some(caps) = get_re_codepoint_oct_element().captures(element_str) {
        u32::from_str_radix(caps.get(1).unwrap().as_str(), 8).ok()
    } else if get_re_codepoint_dec_element().is_match(element_str) {
        // Check is_match first for plain decimal
        element_str.parse::<u32>().ok()
    } else {
        None
    };

    match val_u32 {
        Some(v) if v <= 65535 => Some(v as u16),
        _ => None, // Parse failed or value out of u8 range
    }
}

// Helper to parse a sequence of Codepoint elements (comma-separated)
fn parse_codepoint_label_sequence(elements: &[&str]) -> Option<Label> {
    let mut codepoint_bytes = Vec::new();
    for el_str in elements {
        if let Some(byte_val) = parse_codepoint_element(el_str) {
            codepoint_bytes.push(byte_val);
        } else {
            return None; // Mixed types not allowed
        }
    }
    Some(Label::Codepoint(codepoint_bytes))
}

// Helper to handle deprecated unquoted tag format
fn handle_deprecated_unquoted_tag(
    single_element: &str,
    next_line_type: &LineType,
) -> Option<Label> {
    let should_treat_as_tag = matches!(
        next_line_type,
        LineType::BitmapLine { .. }
            | LineType::KeyValue {
                value_on_line: None,
                ..
            }
    );

    if should_treat_as_tag {
        println!(
            "Warning: Deprecated unquoted tag format detected: '{single_element}'. "
        );
        Some(Label::Tag(single_element.to_string()))
    } else {
        None
    }
}

/// Parses a key string (content before the final ':' of a label line)
/// into an `Option<Label>`.
/// Returns `None` if the string does not conform to any known label syntax.
pub fn parse_key_as_label(key_str: &str, next_line_type: &LineType) -> Option<Label> {
    let trimmed_key = key_str.trim();

    // Special case for a quoted comma so it doesn't get recognized as a sequence
    if trimmed_key == "','" {
        return Some(Label::Unicode(vec![0x2c]));
    }

    // Handle Anonymous Label (original line was just ":")
    if trimmed_key.is_empty() {
        return Some(Label::Anonymous);
    }

    // Handle double-quoted strings
    if let Some(caps) = get_re_tag_label().captures(trimmed_key) {
        let inner_content = caps.get(1).unwrap().as_str();

        return Some(Label::Tag(inner_content.to_string()));
    }

    // Handle potential multi-element labels (Unicode or Codepoint)
    let elements: Vec<&str> = trimmed_key.split(',').map(str::trim).collect();
    if elements.is_empty() || elements.iter().any(|e| e.is_empty()) {
        // This can happen if key_str was just "," or "elem1, ,elem2"
        return None;
    }

    // Determine label type from first element and parse accordingly
    let first_element = elements[0];

    // Try Unicode (Character) Label first
    if get_re_unicode_uplus_element().is_match(first_element)
        || get_re_unicode_char_seq_element().is_match(first_element)
    {
        return parse_unicode_label_sequence(&elements);
    }

    // Try Codepoint Label
    if get_re_codepoint_hex_element().is_match(first_element)
        || get_re_codepoint_oct_element().is_match(first_element)
        || get_re_codepoint_dec_element().is_match(first_element)
    {
        return parse_codepoint_label_sequence(&elements);
    }

    // Handle deprecated unquoted tags (single element only)
    if elements.len() == 1 && get_re_unquoted_tag().is_match(first_element) {
        return handle_deprecated_unquoted_tag(first_element, next_line_type);
    }

    None // Does not match any known label format (including previously matched unquoted tags)
}

fn parse_int_val(s: &str, line_num: usize, key_for_error: &str) -> Result<i32, ParseError> {
    s.trim()
        .parse::<i32>()
        .map_err(|_| ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "integer".to_string(),
        })
}

fn parse_float_val(s: &str, line_num: usize, key_for_error: &str) -> Result<f32, ParseError> {
    s.trim()
        .parse::<f32>()
        .map_err(|_| ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "float".to_string(),
        })
}

/// Parses a tuple of two i32 values from a string (e.g., for offsets)
fn parse_int_tuple_val(
    s: &str,
    line_num: usize,
    key_for_error: &str,
) -> Result<(i32, i32), ParseError> {
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',' || c == ':' || c == 'x')
        .map(|p| p.trim())
        .collect();
    if parts.len() == 2 {
        let x = parse_int_val(parts[0], line_num, key_for_error)?;
        let y = parse_int_val(parts[1], line_num, key_for_error)?;
        Ok((x, y))
    } else {
        Err(ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "x, y".to_string(),
        })
    }
}

/// Parases an unsigned integer value from a string
fn parse_uint_val(s: &str, line_num: usize, key_for_error: &str) -> Result<u32, ParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "unsigned integer".to_string(),
        })
}

/// Parses a tuple of two u32 values from a string (e.g., for sizes)
fn parse_uint_tuple_val(
    s: &str,
    line_num: usize,
    key_for_error: &str,
) -> Result<(u32, u32), ParseError> {
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',' || c == ':' || c == 'x')
        .map(|p| p.trim())
        .collect();
    if parts.len() == 2 {
        let x = parse_uint_val(parts[0], line_num, key_for_error)?;
        let y = parse_uint_val(parts[1], line_num, key_for_error)?;
        Ok((x, y))
    } else {
        Err(ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "X Y or X,Y or X:Y or XxY (two unsigned integers)".to_string(),
        })
    }
}

/// Parses a quad tuple of i32 values (e.g., for bounds) from a string
fn parse_i32_quad_tuple_val(
    s: &str,
    line_num: usize,
    key_for_error: &str,
) -> Result<(i32, i32, i32, i32), ParseError> {
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() == 4 {
        let p1 = parse_int_val(parts[0], line_num, key_for_error)?;
        let p2 = parse_int_val(parts[1], line_num, key_for_error)?;
        let p3 = parse_int_val(parts[2], line_num, key_for_error)?;
        let p4 = parse_int_val(parts[3], line_num, key_for_error)?;
        Ok((p1, p2, p3, p4))
    } else {
        Err(ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "N N N N (four integers, space or comma separated)".to_string(),
        })
    }
}

/// Parases the font spacing property value into FontSpacing enum
fn parse_font_spacing_val(
    s: &str,
    line_num: usize,
    key_for_error: &str,
) -> Result<FontSpacing, ParseError> {
    match s.trim().to_lowercase().as_str() {
        "proportional" => Ok(FontSpacing::Proportional),
        "monospace" => Ok(FontSpacing::Monospace),
        "character-cell" => Ok(FontSpacing::CharacterCell),
        "multi-cell" => Ok(FontSpacing::MultiCell),
        _ => Err(ParseError::InvalidPropertyValue {
            line: line_num,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "proportional | monospace | character-cell | multi-cell".to_string(),
        }),
    }
}

/// Parses a multi-line kerning map value into a HashMap<Label, f32>
fn parse_kerning_map_val(
    s: &str,
    line_num_start: usize,
    key_for_error: &str,
) -> Result<HashMap<Label, f32>, ParseError> {
    let mut map = HashMap::new();
    for (i, line_content) in s.lines().enumerate() {
        let current_line_num = line_num_start + i;
        let trimmed_line = line_content.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        let mut parts_iter = trimmed_line.rsplitn(2, char::is_whitespace);
        let val_str = parts_iter.next();
        let label_str = parts_iter.next();
        if let (Some(label_s), Some(val_s)) = (label_str, val_str) {
            if let Some(label) = parse_key_as_label(label_s, &LineType::default()) {
                let val = parse_float_val(val_s, current_line_num, key_for_error)?;
                map.insert(label, val);
                continue;
            }
        }
        return Err(ParseError::InvalidPropertyValue {
            line: current_line_num,
            property_key: key_for_error.to_string(),
            value: trimmed_line.to_string(),
            expected_format: "LABEL FLOAT_VALUE (e.g., '0x20 -0.99' or '\"tag\" 1.5')".to_string(),
        });
    }
    if map.is_empty() && !s.trim().is_empty() {
        return Err(ParseError::InvalidPropertyValue {
            line: line_num_start,
            property_key: key_for_error.to_string(),
            value: s.to_string(),
            expected_format: "One or more lines of: LABEL FLOAT_VALUE".to_string(),
        });
    }
    Ok(map)
}

/// Reads a property value from the lines iterator, handling both single-line and multi-line values.
fn read_property_value_from_iter<'s, 'a: 's>(
    lines_iter: &'s mut LineIterator<'a>,
    initial_key_line_num: usize,
    initial_line_str: &'a str,
    key_for_error: &str,
    colon_pos_on_key_line: usize,
) -> Result<String, ParseError> {
    let key_line_val_part = initial_line_str[colon_pos_on_key_line + 1..].trim_start();
    if !key_line_val_part.is_empty() {
        if key_line_val_part.starts_with('"')
            && key_line_val_part.ends_with('"')
            && key_line_val_part.len() >= 2
        {
            Ok(key_line_val_part[1..key_line_val_part.len() - 1].to_string())
        } else {
            Ok(key_line_val_part.trim_end().to_string())
        }
    } else {
        let mut value_lines = Vec::new();
        let mut first_multiline = true;
        let mut expected_indent: Option<usize> = None;
        while let Some((_, line_str)) = lines_iter.peek() {
            let current_indent = line_str.chars().take_while(|&c| c.is_whitespace()).count();
            if first_multiline {
                if current_indent == 0 {
                    if initial_line_str[colon_pos_on_key_line + 1..]
                        .trim()
                        .is_empty()
                        && !initial_line_str[colon_pos_on_key_line + 1..].is_empty()
                    {
                        return Ok("".to_string());
                    }
                    return Err(ParseError::InvalidSyntax {
                        line: initial_key_line_num + 1,
                        message: format!(
                            "Property '{key_for_error}' expects an indented multi-line value or value on the same line."
                        ),
                    });
                }
                expected_indent = Some(current_indent);
                first_multiline = false;
            } else if current_indent < expected_indent.unwrap_or(current_indent + 1) {
                break;
            }
            let (_, consumed_line_str) = lines_iter.next().unwrap();
            let line_content = consumed_line_str.trim_start().trim_end();
            if line_content.starts_with('"')
                && line_content.ends_with('"')
                && line_content.len() >= 2
            {
                value_lines.push(line_content[1..line_content.len() - 1].to_string());
            } else {
                value_lines.push(line_content.to_string());
            }
        }
        if value_lines.is_empty() && key_line_val_part.is_empty() {
            Ok("".to_string())
        } else {
            Ok(value_lines.join("\n"))
        }
    }
}

// --- Setter Functions ---
fn set_global_property(
    font: &mut YaffFont,
    key: String,
    value_str: String, // Renamed from value to value_str for clarity
    line_num: usize,
) -> Result<(), ParseError> {
    match key.as_str() {
        "yaff" => font.yaff_version = Some(value_str),
        "name" => font.name = Some(value_str),
        "family" => font.family = Some(value_str),
        "subfamily" => font.subfamily = Some(value_str),
        "revision" => font.revision = Some(value_str),
        "point-size" => font.point_size = Some(parse_float_val(&value_str, line_num, &key)?),
        "line-height" => font.line_height = Some(parse_int_val(&value_str, line_num, &key)?),
        "style" => font.style = Some(value_str),
        "weight" => font.weight = Some(value_str),
        "slant" => font.slant = Some(value_str),
        "setwidth" => font.setwidth = Some(value_str),
        "decoration" => font.decoration = Some(value_str),
        "x-height" => font.x_height = Some(parse_int_val(&value_str, line_num, &key)?),
        "cap-height" => font.cap_height = Some(parse_int_val(&value_str, line_num, &key)?),
        "ascent" => font.ascent = Some(parse_int_val(&value_str, line_num, &key)?),
        "descent" => font.descent = Some(parse_int_val(&value_str, line_num, &key)?),
        "pixel-size" => font.pixel_size = Some(parse_int_val(&value_str, line_num, &key)?),
        "leading" => font.leading = Some(parse_int_val(&value_str, line_num, &key)?),
        "raster-bounds" => {
            font.raster_bounds = Some(parse_i32_quad_tuple_val(&value_str, line_num, &key)?)
        }
        "ink-bounds" => {
            font.ink_bounds = Some(parse_i32_quad_tuple_val(&value_str, line_num, &key)?)
        }
        "raster-size" => font.raster_size = Some(parse_uint_tuple_val(&value_str, line_num, &key)?),
        "cell-size" => font.cell_size = Some(parse_uint_tuple_val(&value_str, line_num, &key)?),
        "bounding-box" => {
            font.bounding_box = Some(parse_uint_tuple_val(&value_str, line_num, &key)?)
        }
        "average-width" => font.average_width = Some(parse_float_val(&value_str, line_num, &key)?),
        "max-width" => font.max_width = Some(parse_int_val(&value_str, line_num, &key)?),
        "cap-width" => font.cap_width = Some(parse_int_val(&value_str, line_num, &key)?),
        "digit-width" => font.digit_width = Some(parse_int_val(&value_str, line_num, &key)?),
        "spacing" => font.spacing = Some(parse_font_spacing_val(&value_str, line_num, &key)?),
        "direction" => match value_str.to_lowercase().as_str() {
            "left-to-right" => font.direction = Some(WritingDirection::LeftToRight),
            "right-to-left" => font.direction = Some(WritingDirection::RightToLeft),
            "top-to-bottom" => {
                return Err(ParseError::UnsupportedFeature {
                    line: line_num,
                    feature_name: key.clone(),
                    message: "Direction 'top-to-bottom' (vertical writing) is not supported."
                        .to_string(),
                });
            }
            _ => {
                return Err(ParseError::InvalidPropertyValue {
                    line: line_num,
                    property_key: key.clone(),
                    value: value_str,
                    expected_format: "left-to-right | right-to-left".to_string(),
                });
            }
        },
        "bold-smear" => font.bold_smear = Some(parse_int_val(&value_str, line_num, &key)?),
        "italic-pitch" => {
            font.italic_pitch = Some(parse_int_tuple_val(&value_str, line_num, &key)?)
        }
        "outline-thickness" => {
            font.outline_thickness = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "underline-thickness" => {
            font.underline_thickness = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "underline-descent" => {
            font.underline_descent = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "strikethrough-thickness" => {
            font.strikethrough_thickness = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "strikethrough-ascent" => {
            font.strikethrough_ascent = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "superscript-size" => {
            font.superscript_size = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "superscript-offset" => {
            font.superscript_offset = Some(parse_int_tuple_val(&value_str, line_num, &key)?)
        }
        "subscript-size" => font.subscript_size = Some(parse_int_val(&value_str, line_num, &key)?),
        "subscript-offset" => {
            font.subscript_offset = Some(parse_int_tuple_val(&value_str, line_num, &key)?)
        }
        "small-cap-size" => font.small_cap_size = Some(parse_int_val(&value_str, line_num, &key)?),
        "word-space" => font.word_space = Some(parse_int_val(&value_str, line_num, &key)?),
        "min-word-space" => font.min_word_space = Some(parse_int_val(&value_str, line_num, &key)?),
        "max-word-space" => font.max_word_space = Some(parse_int_val(&value_str, line_num, &key)?),
        "sentence-space" => font.sentence_space = Some(parse_int_val(&value_str, line_num, &key)?),
        "author" => font.author = Some(value_str),
        "foundry" => font.foundry = Some(value_str),
        "copyright" => font.copyright = Some(value_str),
        "notice" => font.notice = Some(value_str),
        "device" => font.device = Some(value_str),
        "pixel-aspect" => {
            font.pixel_aspect = Some(parse_uint_tuple_val(&value_str, line_num, &key)?)
        }
        "dpi" => font.dpi = Some(parse_uint_tuple_val(&value_str, line_num, &key)?),
        "converter" => font.converter = Some(value_str),
        "source-name" => font.source_name = Some(value_str),
        "source-format" => font.source_format = Some(value_str),
        "history" => font.history = Some(value_str),
        "encoding" => font.encoding = Some(value_str),
        "default-char" => font.default_char_label_raw = Some(value_str),
        "word-boundary" => font.word_boundary_label_raw = Some(value_str),
        "left-bearing" => {
            font.global_left_bearing = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "right-bearing" => {
            font.global_right_bearing = Some(parse_int_val(&value_str, line_num, &key)?)
        }
        "shift-up" => font.global_shift_up = Some(parse_int_val(&value_str, line_num, &key)?),

        "line-width" => {
            eprintln!(
                "Warning: Unsupported vertical metric property '{key}' on line {line_num}. This property is ignored."
            );
        }
        "left-extent" | "right-extent" => {
            return Err(ParseError::UnsupportedFeature {
                line: line_num,
                feature_name: key.clone(),
                message: "Vertical metric property is not supported.".to_string(),
            });
        }
        "levels" => {
            return Err(ParseError::UnsupportedFeature {
                line: line_num,
                feature_name: key.clone(),
                message: "Greyscale 'levels' property is not supported.".to_string(),
            });
        }
        _ => {
            println!("Warning: Unknown global property: {key}");
        }
    }
    Ok(())
}

fn set_glyph_property(
    glyph: &mut GlyphDefinition,
    key: String,
    value: String, // Using `value` as per the snippet provided in the query
    line_num: usize,
) -> Result<(), ParseError> {
    match key.as_str() {
        // Standard per-glyph properties
        "left-bearing" => glyph.left_bearing = Some(parse_int_val(&value, line_num, &key)?),
        "right-bearing" => glyph.right_bearing = Some(parse_int_val(&value, line_num, &key)?),
        "shift-up" => glyph.shift_up = Some(parse_int_val(&value, line_num, &key)?),
        "top-bearing" => glyph.top_bearing = Some(parse_int_val(&value, line_num, &key)?),
        "bottom-bearing" => glyph.bottom_bearing = Some(parse_int_val(&value, line_num, &key)?),
        "shift-left" => glyph.shift_left = Some(parse_int_val(&value, line_num, &key)?),
        "scalable-width" => glyph.scalable_width = Some(parse_float_val(&value, line_num, &key)?),
        "right-kerning" => {
            glyph.right_kerning = Some(parse_kerning_map_val(&value, line_num, &key)?)
        }
        "left-kerning" => glyph.left_kerning = Some(parse_kerning_map_val(&value, line_num, &key)?),

        // Deprecated per-glyph properties
        "offset" => {
            // Value is an "x y" pair
            let parts: Vec<&str> = value.split_whitespace().collect();
            if parts.len() == 2 {
                glyph.left_bearing = Some(parse_int_val(parts[0], line_num, &key)?);
                glyph.shift_up = Some(parse_int_val(parts[1], line_num, &key)?);
            } else {
                return Err(ParseError::InvalidPropertyValue {
                    line: line_num,
                    property_key: key.to_string(),
                    value,
                    expected_format: "X Y (two integers for x and y offset)".to_string(),
                });
            }
        }
        "tracking" => {
            // Equal to right-bearing
            glyph.right_bearing = Some(parse_int_val(&value, line_num, &key)?);
        }
        "kern-to" => {
            // Equal to right-kerning
            glyph.right_kerning = Some(parse_kerning_map_val(&value, line_num, &key)?);
        }

        // Unsupported vertical metrics (per-glyph)
        "scalable-height" => {
            return Err(ParseError::UnsupportedFeature {
                line: line_num,
                feature_name: key.clone(),
                message: "Vertical metric per-glyph property is not supported.".to_string(),
            });
        }
        _ => {
            println!(
                "Warning: Line {line_num}: Unknown per-glyph property '{key}' with value '{value}', discarding."
            );
        }
    }
    Ok(())
}

// --- Line Iterator with Tracking ---
struct LineIterator<'a> {
    inner: std::iter::Peekable<std::str::Lines<'a>>,
    current_line_number: usize,
}

impl<'a> LineIterator<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            inner: s.lines().peekable(),
            current_line_number: 0,
        }
    }

    fn next(&mut self) -> Option<(usize, &'a str)> {
        self.inner.next().map(|line| {
            self.current_line_number += 1;
            (self.current_line_number, line)
        })
    }

    fn peek(&mut self) -> Option<(usize, &'a str)> {
        self.inner
            .peek()
            .map(|&line| (self.current_line_number + 1, line))
    }

    fn current_line_number(&self) -> usize {
        self.current_line_number
    }
}

// --- Parser State Machine ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    GlobalProps,        // A: global properties
    AccumulatingLabels, // B: label(s) encountered, expecting more labels or bitmap start
    ParsingBitmap,      // C: bitmap line(s)
    ParsingGlyphProps,  // D: glyph properties
}

// Parser State Transitions:
//
// A (GlobalProps) → B (AccumulatingLabels):
//   - When first glyph label is encountered at indent 0
//
// B (AccumulatingLabels) → C (ParsingBitmap):
//   - When bitmap data starts (indented non-empty content or "-")
//
// C (ParsingBitmap) → D (ParsingGlyphProps):
//   - When glyph properties start (indented key-value pairs after bitmap)
//   - Or when bitmap ends and transitions to next glyph/end
//
// C (ParsingBitmap) → B (AccumulatingLabels):
//   - When new glyph labels start at indent 0 (commits current glyph)
//
// D (ParsingGlyphProps) → B (AccumulatingLabels):
//   - When new glyph labels start at indent 0 (commits current glyph)

struct Decoder<'a> {
    lines_iter: LineIterator<'a>,
    font: YaffFont,
    state: ParserState,

    // State-specific data
    pending_labels: Vec<Label>,
    current_glyph_being_built: Option<GlyphDefinition>, // Holds glyph during C and D states
    current_glyph_bitmap_lines: Vec<String>,
    current_glyph_expected_indent: Option<usize>,
}

impl<'a> Decoder<'a> {
    fn new(s: &'a str) -> Self {
        let content = s.strip_prefix('\u{FEFF}').unwrap_or(s);
        Decoder {
            lines_iter: LineIterator::new(content),
            font: YaffFont::new(),
            state: ParserState::GlobalProps,
            pending_labels: Vec::new(),
            current_glyph_being_built: None,
            current_glyph_bitmap_lines: Vec::new(),
            current_glyph_expected_indent: None,
        }
    }

    fn next_significant_line(&mut self) -> Option<(usize, LineType, usize, &'a str)> {
        while let Some((line_num, line_str)) = self.lines_iter.peek() {
            let (line_type, indent) = classify_line(line_str);
            match line_type {
                LineType::Empty => {
                    // Comment or empty line, consume and continue
                    self.lines_iter.next();
                }
                _ => {
                    // Found a significant line
                    return Some((line_num, line_type, indent, line_str));
                }
            }
        }
        None
    }

    fn consume_line(&mut self) {
        self.lines_iter.next();
    }

    fn commit_current_glyph(&mut self) {
        if let Some(glyph) = self.current_glyph_being_built.take() {
            self.font.glyphs.push(glyph);
        }
        self.current_glyph_bitmap_lines.clear();
        self.current_glyph_expected_indent = None;
        // pending_labels should have been cleared when glyph construction started
    }

    fn parse_internal(mut self) -> Result<YaffFont, ParseError> {
        // This is way more complicated than it should be. The spec requires this complexity. :(
        while let Some((line_num, line_type, indent, line_str_raw)) = self.next_significant_line() {
            // The line_str_raw is the peeked line. We consume it only if the current state handles it.
            // If a state transitions without consuming, the next state will re-evaluate this same line.

            match self.state {
                ParserState::GlobalProps => {
                    match line_type {
                        LineType::KeyValue {
                            key: key_str,
                            value_on_line,
                        } if indent == 0 => {
                            self.consume_line();
                            let value_on_line2 = value_on_line.clone();
                            let next_line_type = self
                                .lines_iter
                                .peek()
                                .map(|(_, s)| classify_line(s).0)
                                .unwrap_or_default();
                            let label = parse_key_as_label(&key_str, &next_line_type);
                            if value_on_line2.is_none() {
                                if let Some(new_label) = label {
                                    // It's a label
                                    if !self.pending_labels.contains(&new_label) {
                                        self.pending_labels.push(new_label);
                                    }
                                    self.state = ParserState::AccumulatingLabels;
                                } else {
                                    // It's a global property with no value on the same line
                                    let value_to_set = read_property_value_from_iter(
                                        &mut self.lines_iter,
                                        line_num,
                                        line_str_raw,
                                        &key_str,
                                        line_str_raw.find(':').unwrap_or(line_str_raw.len()),
                                    )?;
                                    if value_to_set.is_empty() {
                                        return Err(ParseError::InvalidSyntax {
                                            line: line_num,
                                            message: format!(
                                                "Multi-line property '{key_str}' expects at least one continuation line."
                                            ),
                                        });
                                    }
                                    set_global_property(
                                        &mut self.font,
                                        key_str,
                                        value_to_set,
                                        line_num,
                                    )?;
                                }
                            } else {
                                // It's a global property with value on the same line
                                let value_to_set = value_on_line.unwrap();
                                set_global_property(
                                    &mut self.font,
                                    key_str,
                                    value_to_set,
                                    line_num,
                                )?;
                            }
                        }
                        _ => {
                            return Err(ParseError::InvalidSyntax {
                                line: line_num,
                                message: "Expected unindented global property or first label."
                                    .to_string(),
                            });
                        }
                    }
                }
                ParserState::AccumulatingLabels => match line_type {
                    LineType::KeyValue {
                        key: key_str,
                        value_on_line,
                    } if indent == 0 => {
                        self.consume_line();
                        let next_line_type = self
                            .lines_iter
                            .peek()
                            .map(|(_, s)| classify_line(s).0)
                            .unwrap_or_default();
                        let label = parse_key_as_label(&key_str, &next_line_type);
                        if value_on_line.is_none() {
                            if let Some(new_label) = label {
                                if !self.pending_labels.contains(&new_label) {
                                    self.pending_labels.push(new_label);
                                }
                            } else {
                                return Err(ParseError::SemanticError { line: line_num, message: "Expected label, but found property-like line after glyphs started.".to_string()});
                            }
                        } else {
                            return Err(ParseError::SemanticError { line: line_num, message: "Expected label, but found property-like line after glyphs started.".to_string()});
                        }
                    }
                    LineType::BitmapLine { content } => {
                        self.consume_line();
                        if self.pending_labels.is_empty() {
                            return Err(ParseError::SemanticError {
                                line: line_num,
                                message: "Bitmap data found without preceding labels.".to_string(),
                            });
                        }
                        self.current_glyph_expected_indent = Some(indent);
                        if content == "-" {
                            let glyph = GlyphDefinition {
                                labels: std::mem::take(&mut self.pending_labels),
                                bitmap: Bitmap::default(),
                                ..Default::default()
                            };
                            self.current_glyph_being_built = Some(glyph);
                            self.state = ParserState::ParsingGlyphProps;
                        } else {
                            self.current_glyph_bitmap_lines.push(content);
                            self.state = ParserState::ParsingBitmap;
                        }
                    }
                    _ => {
                        return Err(ParseError::InvalidSyntax {
                            line: line_num,
                            message: "Expected more labels or bitmap data.".to_string(),
                        });
                    }
                },
                ParserState::ParsingBitmap => {
                    let expected_indent = self.current_glyph_expected_indent.unwrap();
                    match line_type {
                        LineType::BitmapLine { content } if indent == expected_indent => {
                            self.consume_line();
                            self.current_glyph_bitmap_lines.push(content);
                        }
                        LineType::KeyValue {
                            key: key_str,
                            value_on_line,
                        } if indent == expected_indent => {
                            // End of bitmap, start of per-glyph properties
                            let bitmap = self.finalize_bitmap(line_num)?;
                            let mut glyph = GlyphDefinition {
                                labels: std::mem::take(&mut self.pending_labels),
                                bitmap,
                                ..Default::default()
                            };
                            // Apply this first property
                            self.consume_line();
                            let value_on_line2 = value_on_line.clone();
                            let value_to_set = if let Some(val_on_line) = value_on_line {
                                val_on_line
                            } else {
                                read_property_value_from_iter(
                                    &mut self.lines_iter,
                                    line_num,
                                    line_str_raw,
                                    &key_str,
                                    line_str_raw.find(':').unwrap_or(line_str_raw.len()),
                                )?
                            };
                            if value_to_set.is_empty() && value_on_line2.is_none() {
                                return Err(ParseError::InvalidSyntax {
                                    line: line_num,
                                    message: format!(
                                        "Multi-line glyph property '{key_str}' expects at least one continuation line."
                                    ),
                                });
                            }
                            set_glyph_property(&mut glyph, key_str, value_to_set, line_num)?;
                            self.current_glyph_being_built = Some(glyph);
                            self.state = ParserState::ParsingGlyphProps;
                        }
                        LineType::KeyValue { key: _, .. } if indent == 0 => {
                            // New label
                            let bitmap = self.finalize_bitmap(line_num)?;
                            let bitmap2 = bitmap.clone();
                            let glyph_to_commit = GlyphDefinition {
                                labels: std::mem::take(&mut self.pending_labels),
                                bitmap,
                                ..Default::default()
                            };
                            if let Some(mut existing_glyph) = self.current_glyph_being_built.take()
                            {
                                existing_glyph.bitmap = bitmap2;
                                self.font.glyphs.push(existing_glyph);
                            } else {
                                self.font.glyphs.push(glyph_to_commit);
                            }

                            self.state = ParserState::AccumulatingLabels;
                        }
                        _ => {
                            // Any other line type or indent mismatch ends bitmap
                            let bitmap = self.finalize_bitmap(line_num)?;
                            let glyph_to_commit = GlyphDefinition {
                                labels: std::mem::take(&mut self.pending_labels),
                                bitmap,
                                ..Default::default()
                            };
                            self.current_glyph_being_built = Some(glyph_to_commit);
                            self.state = ParserState::ParsingGlyphProps;
                        }
                    }
                }
                ParserState::ParsingGlyphProps => {
                    let expected_indent = self.current_glyph_expected_indent.unwrap();
                    match line_type {
                        LineType::KeyValue {
                            key: key_str,
                            value_on_line,
                        } if indent == expected_indent => {
                            self.consume_line();
                            let glyph =
                                self.current_glyph_being_built.as_mut().ok_or_else(|| {
                                    ParseError::SemanticError {
                                        line: line_num,
                                        message:
                                            "Internal error: no glyph being built for properties"
                                                .to_string(),
                                    }
                                })?;
                            let value_on_line2 = value_on_line.clone();
                            let value_to_set = if let Some(val_on_line) = value_on_line {
                                val_on_line
                            } else {
                                read_property_value_from_iter(
                                    &mut self.lines_iter,
                                    line_num,
                                    line_str_raw,
                                    &key_str,
                                    line_str_raw.find(':').unwrap_or(line_str_raw.len()),
                                )?
                            };
                            if value_to_set.is_empty() && value_on_line2.is_none() {
                                return Err(ParseError::InvalidSyntax {
                                    line: line_num,
                                    message: format!(
                                        "Multi-line glyph property '{key_str}' expects at least one continuation line."
                                    ),
                                });
                            }
                            set_glyph_property(glyph, key_str, value_to_set, line_num)?;
                        }
                        LineType::KeyValue { key: _, .. } if indent == 0 => {
                            // New label
                            self.commit_current_glyph();
                            self.state = ParserState::AccumulatingLabels;
                        }
                        _ => {
                            self.commit_current_glyph();
                            self.state = ParserState::AccumulatingLabels;
                        }
                    }
                }
            }
        }

        match self.state {
            ParserState::GlobalProps => {}
            ParserState::AccumulatingLabels => {
                if !self.pending_labels.is_empty() {
                    return Err(ParseError::UnexpectedEndOfInput);
                }
            }
            ParserState::ParsingBitmap => {
                if !self.current_glyph_bitmap_lines.is_empty() {
                    let bitmap = self.finalize_bitmap(self.lines_iter.current_line_number() + 1)?;
                    let glyph_to_commit = GlyphDefinition {
                        labels: std::mem::take(&mut self.pending_labels),
                        bitmap,
                        ..Default::default()
                    };
                    self.font.glyphs.push(glyph_to_commit);
                } else if !self.pending_labels.is_empty() {
                    return Err(ParseError::UnexpectedEndOfInput);
                }
            }
            ParserState::ParsingGlyphProps => {
                if let Some(glyph) = self.current_glyph_being_built.take() {
                    self.font.glyphs.push(glyph);
                }
            }
        }
        Ok(self.font)
    }

    fn finalize_bitmap(&mut self, error_line_num: usize) -> Result<Bitmap, ParseError> {
        let lines_data = std::mem::take(&mut self.current_glyph_bitmap_lines);
        if lines_data.is_empty() {
            if self
                .current_glyph_being_built
                .as_ref()
                .is_none_or(|g| g.bitmap.is_empty())
            {
                return Err(ParseError::InvalidSyntax {
                    line: error_line_num,
                    message: "Missing or invalid bitmap data after labels.".to_string(),
                });
            }
            return Ok(Bitmap::default());
        }

        let expected_width = if lines_data[0].is_empty() {
            0
        } else {
            lines_data[0].len()
        };
        let mut pixels_matrix: Vec<Vec<bool>> = Vec::new();

        for (i, line_data) in lines_data.iter().enumerate() {
            let line_data = if line_data.len() < expected_width {
                // We are okay with shorter lines, we pad with '.'
                let line_num = error_line_num - lines_data.len() + i;
                println!(
                    "Line {line_num}: Bitmap line is shorter than expected width."
                );
                let mut line_data = line_data.to_string();
                line_data.push_str(&".".repeat(expected_width - line_data.len()));
                line_data
            } else if line_data.len() > expected_width {
                return Err(ParseError::InconsistentGlyphLineLength {
                    line: error_line_num - lines_data.len() + i,
                });
            } else {
                line_data.to_string()
            };
            let mut pixel_row = Vec::new();
            for char_c in line_data.chars() {
                match char_c {
                    '.' => pixel_row.push(false),
                    '@' => pixel_row.push(true),
                    _ => {
                        return Err(ParseError::InvalidGlyphCharacter {
                            line: error_line_num - lines_data.len() + i,
                            char_found: char_c,
                        });
                    }
                }
            }
            pixels_matrix.push(pixel_row);
        }
        Ok(Bitmap {
            pixels: pixels_matrix,
            width: expected_width,
            height: lines_data.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_line_trailing_spaces_unindented() {
        // Label with trailing spaces
        match classify_line("label:   ") {
            (
                LineType::KeyValue {
                    key,
                    value_on_line: None,
                },
                0,
            ) => {
                assert_eq!(key, "label");
            }
            _ => panic!("Expected KeyValue with 'label' key for 'label:   '"),
        }
    }

    #[test]
    fn test_classify_line_trailing_spaces_indented() {
        // Indented colon with trailing spaces
        match classify_line("    :   ") {
            (LineType::Continuation { content }, 4) => {
                assert_eq!(content, ":");
            }
            _ => panic!("Expected Continuation with ':' content for '    :   '"),
        }
    }

    #[test]
    fn test_classify_ignores_empty_and_comment_lines() {
        assert_eq!(classify_line(""), (LineType::Empty, 0));
        assert_eq!(classify_line("    "), (LineType::Empty, 4));
        assert_eq!(classify_line("# This is a comment"), (LineType::Empty, 0));
        assert_eq!(classify_line("  # Indented comment"), (LineType::Empty, 2));
        assert_eq!(classify_line("#"), (LineType::Empty, 0));
    }

    #[test]
    fn test_classify_toplevel_key_value_with_value() {
        assert_eq!(
            classify_line("name: My Font"),
            (
                LineType::KeyValue {
                    key: "name".to_string(),
                    value_on_line: Some("My Font".to_string()),
                },
                0
            )
        );
        assert_eq!(
            classify_line("key  :  value with spaces  "),
            (
                LineType::KeyValue {
                    key: "key".to_string(),
                    value_on_line: Some("value with spaces".to_string()),
                },
                0
            )
        );
    }

    #[test]
    fn test_classify_toplevel_key_value_without_value_on_line() {
        assert_eq!(
            classify_line("notice:"),
            (
                LineType::KeyValue {
                    key: "notice".to_string(),
                    value_on_line: None,
                },
                0
            )
        );
        assert_eq!(
            classify_line("u+0041:  "),
            (
                LineType::KeyValue {
                    key: "u+0041".to_string(),
                    value_on_line: None,
                },
                0
            )
        );
    }

    #[test]
    fn test_classify_toplevel_anonymous_glyph_label() {
        assert_eq!(
            classify_line(":"),
            (
                LineType::KeyValue {
                    key: "".to_string(), // Key is empty string for anonymous
                    value_on_line: None,
                },
                0
            )
        );
    }

    #[test]
    fn test_classify_indented_bitmap_line() {
        assert_eq!(
            classify_line("  .@..@"),
            (
                LineType::BitmapLine {
                    content: ".@..@".to_string(),
                },
                2
            )
        );
        assert_eq!(
            classify_line("    -"),
            (
                LineType::BitmapLine {
                    content: "-".to_string(),
                },
                4
            )
        );
        assert_eq!(
            classify_line("  @@.  "),
            (
                LineType::BitmapLine {
                    content: "@@.  ".to_string(),
                },
                2
            )
        );
    }

    #[test]
    fn test_classify_indented_key_value() {
        assert_eq!(
            classify_line("    key: value"),
            (
                LineType::KeyValue {
                    key: "key".to_string(),
                    value_on_line: Some("value".to_string()),
                },
                4
            )
        );
        assert_eq!(
            classify_line("  another-key:"),
            (
                LineType::KeyValue {
                    key: "another-key".to_string(),
                    value_on_line: None,
                },
                2
            )
        );
    }

    #[test]
    fn test_classify_indented_continuation() {
        assert_eq!(
            classify_line("  This is a multi-line value"),
            (
                LineType::Continuation {
                    content: "This is a multi-line value".to_string(),
                },
                2
            )
        );
        assert_eq!(
            classify_line("  .@.X"),
            (
                LineType::Continuation {
                    content: ".@.X".to_string(),
                },
                2
            )
        );
        assert_eq!(
            classify_line("  :"),
            (
                LineType::Continuation {
                    content: ":".to_string(),
                },
                2
            )
        );
    }

    #[test]
    fn test_classify_unknown_line() {
        assert_eq!(
            classify_line("Just some text without a colon"),
            (
                LineType::Unknown {
                    original_text: "Just some text without a colon".to_string()
                },
                0
            )
        );
    }

    #[test]
    fn test_distinguish_indented_types() {
        assert_eq!(
            classify_line("  ..."),
            (
                LineType::BitmapLine {
                    content: "...".to_string(),
                },
                2
            )
        );
        assert_eq!(
            classify_line("  prop: val"),
            (
                LineType::KeyValue {
                    key: "prop".to_string(),
                    value_on_line: Some("val".to_string()),
                },
                2
            )
        );
        assert_eq!(
            classify_line("  prop val"),
            (
                LineType::Continuation {
                    content: "prop val".to_string(),
                },
                2
            )
        );
        assert_eq!(
            classify_line("  . @ ."),
            (
                LineType::Continuation {
                    content: ". @ .".to_string(),
                },
                2
            )
        );
    }

    #[test]
    fn test_anonymous_label() {
        assert_eq!(
            parse_key_as_label("", &LineType::default()),
            Some(Label::Anonymous)
        );
        assert_eq!(
            parse_key_as_label("   ", &LineType::default()),
            Some(Label::Anonymous)
        );
    }

    #[test]
    fn test_tag_label() {
        assert_eq!(
            parse_key_as_label("\"simple_tag\"", &LineType::default()),
            Some(Label::Tag("simple_tag".to_string()))
        );
        assert_eq!(
            parse_key_as_label("\"tag with spaces\"", &LineType::default()),
            Some(Label::Tag("tag with spaces".to_string()))
        );
        assert_eq!(
            parse_key_as_label("\"\"", &LineType::default()),
            Some(Label::Tag("".to_string()))
        );
    }

    #[test]
    fn test_unicode_label_single_uplus() {
        assert_eq!(
            parse_key_as_label("u+0041", &LineType::default()),
            Some(Label::Unicode(vec![0x0041]))
        );
        assert_eq!(
            parse_key_as_label("U+1F600", &LineType::default()),
            Some(Label::Unicode(vec![0x1F600]))
        );
    }

    #[test]
    fn test_unicode_label_single_quoted_char_seq() {
        assert_eq!(
            parse_key_as_label("'A'", &LineType::default()),
            Some(Label::Unicode(vec![0x0041]))
        );
        assert_eq!(
            parse_key_as_label("'Hello'", &LineType::default()),
            Some(Label::Unicode(vec![
                'H' as u32, 'e' as u32, 'l' as u32, 'l' as u32, 'o' as u32
            ]))
        );
        assert_eq!(
            parse_key_as_label("''", &LineType::default()),
            Some(Label::Unicode(vec![]))
        );

        // Test quoted comma as Unicode label
        assert_eq!(
            parse_key_as_label("','", &LineType::default()),
            Some(Label::Unicode(vec![0x2c]))
        );
    }

    #[test]
    fn test_unicode_label_multi_element() {
        assert_eq!(
            parse_key_as_label("u+0041, 'B', U+0043", &LineType::default()),
            Some(Label::Unicode(vec![0x41, 'B' as u32, 0x43]))
        );
        assert_eq!(
            parse_key_as_label(" 'Foo', u+0020, 'Bar' ", &LineType::default()),
            Some(Label::Unicode(vec![
                'F' as u32, 'o' as u32, 'o' as u32, 0x20, 'B' as u32, 'a' as u32, 'r' as u32
            ]))
        );

        assert_eq!(
            parse_key_as_label("u+00ae, u+f87f", &LineType::default()),
            Some(Label::Unicode(vec![0x00ae, 0xf87f]))
        );
    }

    #[test]
    fn test_codepoint_label_single_element() {
        assert_eq!(
            parse_key_as_label("65", &LineType::default()),
            Some(Label::Codepoint(vec![65]))
        );
        assert_eq!(
            parse_key_as_label("0x41", &LineType::default()),
            Some(Label::Codepoint(vec![0x41]))
        );
        assert_eq!(
            parse_key_as_label("0o101", &LineType::default()),
            Some(Label::Codepoint(vec![65]))
        );
    }

    #[test]
    fn test_codepoint_label_multi_element() {
        assert_eq!(
            parse_key_as_label("65, 0x42, 0o72", &LineType::default()),
            Some(Label::Codepoint(vec![65, 0x42, 0o72]))
        );
        assert_eq!(
            parse_key_as_label(" 1,2 , 3 ", &LineType::default()),
            Some(Label::Codepoint(vec![1, 2, 3]))
        );

        assert_eq!(
            parse_key_as_label("0x24, 0x5d", &LineType::default()),
            Some(Label::Codepoint(vec![0x24, 0x5d]))
        );
    }

    #[test]
    fn test_quotes() {
        assert_eq!(
            parse_key_as_label("'''", &LineType::default()),
            Some(Label::Unicode(vec!['\'' as u32]))
        );
        assert_eq!(
            parse_key_as_label("\"\"\"", &LineType::default()),
            Some(Label::Tag("\"".to_string()))
        );
    }

    #[test]
    fn test_invalid_labels() {
        assert_eq!(parse_key_as_label("u+GHIJ", &LineType::default()), None); // Invalid hex in u+
        assert_eq!(parse_key_as_label("'A", &LineType::default()), None); // Mismatched quote
        assert_eq!(parse_key_as_label("\"Tag", &LineType::default()), None); // Mismatched double quote
        assert_eq!(parse_key_as_label("0xFG", &LineType::default()), None); // Invalid hex in 0x
        assert_eq!(parse_key_as_label("0o88", &LineType::default()), None); // Invalid octal
        assert_eq!(parse_key_as_label("12A", &LineType::default()), None); // Invalid decimal
        assert_eq!(parse_key_as_label("65536", &LineType::default()), None); // Codepoint out of range
        assert_eq!(parse_key_as_label("u+0041, 65", &LineType::default()), None); // Mixed Unicode and Codepoint elements
        assert_eq!(parse_key_as_label("65, u+0041", &LineType::default()), None); // Mixed Codepoint and Unicode elements
        assert_eq!(parse_key_as_label(",u+0041", &LineType::default()), None); // Leading comma / empty element
        assert_eq!(parse_key_as_label("u+0041,", &LineType::default()), None); // Trailing comma / empty element
        assert_eq!(
            parse_key_as_label("u+0041, ,u+0042", &LineType::default()),
            None
        ); // Empty element in middle
    }

    #[test]
    fn test_not_a_label() {
        assert_eq!(
            parse_key_as_label("simple_property_key", &LineType::default()),
            None
        );
        assert_eq!(
            parse_key_as_label("key with spaces", &LineType::default()),
            None
        );
    }

    #[test]
    fn test_unquoted_tag_with_dash_next_line() {
        let next_line = "    -";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(
            parse_key_as_label("mytag", &next_line_type),
            Some(Label::Tag("mytag".to_string()))
        );
    }

    #[test]
    fn test_unquoted_tag_with_dots_at_next_line() {
        let next_line = "    ....";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(
            parse_key_as_label("anothertag", &next_line_type),
            Some(Label::Tag("anothertag".to_string()))
        );

        let next_line = "  @@@@";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(
            parse_key_as_label("tag_with_at", &next_line_type),
            Some(Label::Tag("tag_with_at".to_string()))
        );

        let next_line = "    .@.@";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(
            parse_key_as_label("mixed_tag", &next_line_type),
            Some(Label::Tag("mixed_tag".to_string()))
        );
    }

    #[test]
    fn test_unquoted_tag_with_colon_next_line() {
        let next_line = "property:";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(
            parse_key_as_label("header_tag", &next_line_type),
            Some(Label::Tag("header_tag".to_string()))
        );

        let next_line = "some_key:";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(
            parse_key_as_label("config_tag", &next_line_type),
            Some(Label::Tag("config_tag".to_string()))
        );
    }

    #[test]
    fn test_unquoted_not_tag_cases() {
        // Should NOT be treated as tag - has a value after the colon
        let next_line = "some_key: value";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(parse_key_as_label("not_tag", &next_line_type), None);

        // Should NOT be treated as tag - no next line context
        assert_eq!(parse_key_as_label("simple_tag", &LineType::default()), None);

        // Should NOT be treated as tag - next line doesn't match patterns
        let next_line = "some regular content";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(parse_key_as_label("not_a_tag", &next_line_type), None);

        let next_line = "123 numbers";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(parse_key_as_label("also_not_tag", &next_line_type), None);

        // Should NOT be treated as tag - empty next line
        let next_line = "";
        let next_line_type = classify_line(next_line).0;
        assert_eq!(parse_key_as_label("empty_next", &next_line_type), None);
    }

    #[test]
    fn test_kerning_parsing() {
        // Test single-line kerning
        let yaff_single_kerning = r#"
'A':
    .@..
    @@..
    right-kerning: 'B' -0.5
        "#;

        let font = from_str(yaff_single_kerning).unwrap();
        assert_eq!(font.glyphs.len(), 1);

        let glyph = &font.glyphs[0];
        assert!(glyph.right_kerning.is_some());
        let kerning_map = glyph.right_kerning.as_ref().unwrap();
        assert_eq!(kerning_map.len(), 1);
        assert_eq!(kerning_map[&Label::Unicode(vec!['B' as u32])], -0.5);

        // Test multi-line kerning table
        let yaff_multi_kerning = r#"
'A':
    .@..
    @@..
    right-kerning:
        'B' -0.5
        u+0043 1.2
        0x44 -0.8
        "#;

        let font = from_str(yaff_multi_kerning).unwrap();
        assert_eq!(font.glyphs.len(), 1);

        let glyph = &font.glyphs[0];
        assert!(glyph.right_kerning.is_some());
        let kerning_map = glyph.right_kerning.as_ref().unwrap();
        assert_eq!(kerning_map.len(), 3);
        assert_eq!(kerning_map[&Label::Unicode(vec!['B' as u32])], -0.5);
        assert_eq!(kerning_map[&Label::Unicode(vec![0x0043])], 1.2);
        assert_eq!(kerning_map[&Label::Codepoint(vec![0x44])], -0.8);

        // Test left-kerning
        let yaff_left_kerning = r#"
'A':
    .@..
    @@..
    left-kerning:
        'Z' 0.3
        "#;

        let font = from_str(yaff_left_kerning).unwrap();
        let glyph = &font.glyphs[0];
        assert!(glyph.left_kerning.is_some());
        let kerning_map = glyph.left_kerning.as_ref().unwrap();
        assert_eq!(kerning_map.len(), 1);
        assert_eq!(kerning_map[&Label::Unicode(vec!['Z' as u32])], 0.3);

        // Test deprecated kern-to (should map to right-kerning)
        let yaff_kern_to = r#"
'A':
    .@..
    @@..
    kern-to:
        'Y' 2.5
        "#;

        let font = from_str(yaff_kern_to).unwrap();
        let glyph = &font.glyphs[0];
        assert!(glyph.right_kerning.is_some());
        assert!(glyph.left_kerning.is_none());
        let kerning_map = glyph.right_kerning.as_ref().unwrap();
        assert_eq!(kerning_map.len(), 1);
        assert_eq!(kerning_map[&Label::Unicode(vec!['Y' as u32])], 2.5);

        // Test kerning with tag labels
        let yaff_tag_kerning = r#"
'A':
    .@..
    @@..
    right-kerning:
        "space" -1.0
        "#;

        let font = from_str(yaff_tag_kerning).unwrap();
        let glyph = &font.glyphs[0];
        assert!(glyph.right_kerning.is_some());
        let kerning_map = glyph.right_kerning.as_ref().unwrap();
        assert_eq!(kerning_map.len(), 1);
        assert_eq!(kerning_map[&Label::Tag("space".to_string())], -1.0);
    }
}
