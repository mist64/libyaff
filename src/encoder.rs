use crate::models::*;
use std::collections::HashMap;
use std::fmt::Write;

const GLYPH_ITEM_INDENT: &str = "  ";

fn format_label_to_string(label: &Label) -> String {
    match label {
        Label::Unicode(values) => {
            if values.is_empty() {
                "''".to_string()
            } else {
                values
                    .iter()
                    .map(|&v| match std::char::from_u32(v) {
                        Some(c) if c.is_ascii() && !c.is_control() && c != '\'' => {
                            format!("'{c}'")
                        }
                        _ => format!("u+{v:04X}"),
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            }
        }
        Label::Codepoint(values) => {
            // Assuming u16 now
            if values.is_empty() {
                "".to_string()
            } else {
                values
                    .iter()
                    .map(|v| format!("0x{v:X}"))
                    .collect::<Vec<String>>()
                    .join(", ")
            }
        }
        Label::Tag(tag_str) => format!("\"{tag_str}\""),
        Label::Anonymous => "".to_string(),
    }
}

fn append_property<T, F>(buffer: &mut String, key: &str, value_opt: &Option<T>, formatter: F)
where
    F: Fn(&T) -> String,
{
    if let Some(value) = value_opt {
        let formatted_value = formatter(value);
        if formatted_value.contains('\n') {
            writeln!(buffer, "{key}:").unwrap();
            for line in formatted_value.lines() {
                writeln!(buffer, "{GLYPH_ITEM_INDENT}{line}").unwrap();
            }
        } else if formatted_value.is_empty() && key != "yaff" {
            writeln!(buffer, "{key}:").unwrap();
        } else if !formatted_value.is_empty() {
            writeln!(buffer, "{key}: {formatted_value}").unwrap();
        }
    }
}

fn append_string(buffer: &mut String, key: &str, value_opt: &Option<String>) {
    append_property(buffer, key, value_opt, |s| s.clone());
}

fn append_i32(buffer: &mut String, key: &str, value_opt: &Option<i32>) {
    append_property(buffer, key, value_opt, |v| v.to_string());
}

fn append_f32(buffer: &mut String, key: &str, value_opt: &Option<f32>) {
    append_property(buffer, key, value_opt, |v| format!("{v}"));
}

fn append_i32_tuple(buffer: &mut String, key: &str, value_opt: &Option<(i32, i32)>) {
    append_property(buffer, key, value_opt, |(v1, v2)| format!("{v1} {v2}"));
}

fn append_u32_tuple(buffer: &mut String, key: &str, value_opt: &Option<(u32, u32)>) {
    append_property(buffer, key, value_opt, |(v1, v2)| format!("{v1} {v2}"));
}

fn append_i32_quad_tuple(buffer: &mut String, key: &str, value_opt: &Option<(i32, i32, i32, i32)>) {
    append_property(buffer, key, value_opt, |(v1, v2, v3, v4)| {
        format!("{v1} {v2} {v3} {v4}")
    });
}

fn append_font_spacing(buffer: &mut String, key: &str, value_opt: &Option<FontSpacing>) {
    append_property(buffer, key, value_opt, |v| match v {
        FontSpacing::Proportional => "proportional".to_string(),
        FontSpacing::Monospace => "monospace".to_string(),
        FontSpacing::CharacterCell => "character-cell".to_string(),
        FontSpacing::MultiCell => "multi-cell".to_string(),
    });
}

fn append_writing_direction(buffer: &mut String, key: &str, value_opt: &Option<WritingDirection>) {
    append_property(buffer, key, value_opt, |v| match v {
        WritingDirection::LeftToRight => "left-to-right".to_string(),
        WritingDirection::RightToLeft => "right-to-left".to_string(),
    });
}

fn format_kerning_map_to_string(map: &HashMap<Label, f32>) -> String {
    let mut s = String::new();
    let mut sorted_pairs: Vec<(&Label, &f32)> = map.iter().collect();
    sorted_pairs.sort_by(|a, b| {
        // Sort by label string representation
        let a_str = format_label_to_string(a.0);
        let b_str = format_label_to_string(b.0);
        a_str.cmp(&b_str)
    });

    for (i, (label, value)) in sorted_pairs.iter().enumerate() {
        if i > 0 {
            s.push('\n');
        }
        let label_str = format_label_to_string(label);
        write!(s, "{GLYPH_ITEM_INDENT}{label_str} {value:.2}").unwrap();
    }
    s
}

pub fn to_yaff_string(font: &YaffFont) -> String {
    let mut buffer = String::new();
    let mut has_written_any_global_prop = false;

    let ensure_newline_for_global = |buf: &mut String, written_flag: &mut bool| {
        if !*written_flag && !buf.is_empty() && !buf.ends_with('\n') {
            buf.push('\n');
        }
        *written_flag = true;
    };

    macro_rules! append_global {
        ($key:expr, $field:expr, $writer:ident) => {
            if $field.is_some() {
                ensure_newline_for_global(&mut buffer, &mut has_written_any_global_prop);
                $writer(&mut buffer, $key, $field);
                has_written_any_global_prop = true;
            }
        };
        ($key:expr, $field:expr, $writer:ident, $formatter:expr) => {
            if $field.is_some() {
                ensure_newline_for_global(&mut buffer, &mut has_written_any_global_prop);
                append_property(&mut buffer, $key, $field, $formatter);
                has_written_any_global_prop = true;
            }
        };
    }

    // --- Encode Global Properties ---
    append_global!("yaff", &font.yaff_version, append_string);
    append_global!("name", &font.name, append_string);
    append_global!("family", &font.family, append_string);
    append_global!("subfamily", &font.subfamily, append_string);
    append_global!("revision", &font.revision, append_string);
    append_global!("point-size", &font.point_size, append_f32);
    append_global!("line-height", &font.line_height, append_i32);
    append_global!("style", &font.style, append_string);
    append_global!("weight", &font.weight, append_string);
    append_global!("slant", &font.slant, append_string);
    append_global!("setwidth", &font.setwidth, append_string);
    append_global!("decoration", &font.decoration, append_string);
    append_global!("x-height", &font.x_height, append_i32);
    append_global!("cap-height", &font.cap_height, append_i32);
    append_global!("ascent", &font.ascent, append_i32);
    append_global!("descent", &font.descent, append_i32);
    append_global!("pixel-size", &font.pixel_size, append_i32);
    append_global!("leading", &font.leading, append_i32);
    append_global!("raster-bounds", &font.raster_bounds, append_i32_quad_tuple);
    append_global!("ink-bounds", &font.ink_bounds, append_i32_quad_tuple);
    append_global!("raster-size", &font.raster_size, append_u32_tuple);
    append_global!("cell-size", &font.cell_size, append_u32_tuple);
    append_global!("bounding-box", &font.bounding_box, append_u32_tuple);
    append_global!("average-width", &font.average_width, append_f32);
    append_global!("max-width", &font.max_width, append_i32);
    append_global!("cap-width", &font.cap_width, append_i32);
    append_global!("digit-width", &font.digit_width, append_i32);
    append_global!("spacing", &font.spacing, append_font_spacing);
    append_global!("direction", &font.direction, append_writing_direction);
    append_global!("bold-smear", &font.bold_smear, append_i32);
    append_global!("italic-pitch", &font.italic_pitch, append_i32_tuple);
    append_global!("outline-thickness", &font.outline_thickness, append_i32);
    append_global!("underline-thickness", &font.underline_thickness, append_i32);
    append_global!("underline-descent", &font.underline_descent, append_i32);
    append_global!(
        "strikethrough-thickness",
        &font.strikethrough_thickness,
        append_i32
    );
    append_global!(
        "strikethrough-ascent",
        &font.strikethrough_ascent,
        append_i32
    );
    append_global!("superscript-size", &font.superscript_size, append_i32);
    append_global!(
        "superscript-offset",
        &font.superscript_offset,
        append_i32_tuple
    );
    append_global!("subscript-size", &font.subscript_size, append_i32);
    append_global!("subscript-offset", &font.subscript_offset, append_i32_tuple);
    append_global!("small-cap-size", &font.small_cap_size, append_i32);
    append_global!("word-space", &font.word_space, append_i32);
    append_global!("min-word-space", &font.min_word_space, append_i32);
    append_global!("max-word-space", &font.max_word_space, append_i32);
    append_global!("sentence-space", &font.sentence_space, append_i32);
    append_global!("author", &font.author, append_string);
    append_global!("foundry", &font.foundry, append_string);
    append_global!("copyright", &font.copyright, append_string);
    append_global!("notice", &font.notice, append_string);
    append_global!("device", &font.device, append_string);
    append_global!("pixel-aspect", &font.pixel_aspect, append_u32_tuple);
    append_global!("dpi", &font.dpi, append_u32_tuple);
    append_global!("converter", &font.converter, append_string);
    append_global!("source-name", &font.source_name, append_string);
    append_global!("source-format", &font.source_format, append_string);
    append_global!("history", &font.history, append_string);
    append_global!("encoding", &font.encoding, append_string);
    append_global!("default-char", &font.default_char_label_raw, append_string);
    append_global!(
        "word-boundary",
        &font.word_boundary_label_raw,
        append_string
    );
    append_global!("left-bearing", &font.global_left_bearing, append_i32);
    append_global!("right-bearing", &font.global_right_bearing, append_i32);
    append_global!("shift-up", &font.global_shift_up, append_i32);

    // --- Encode Glyphs ---
    if !font.glyphs.is_empty() {
        if has_written_any_global_prop {
            while !buffer.ends_with("\n\n") {
                buffer.push('\n');
            }
        } else if !buffer.is_empty() && !buffer.ends_with('\n') {
            buffer.push('\n');
        }

        for (glyph_index, glyph) in font.glyphs.iter().enumerate() {
            if glyph_index > 0 {
                buffer.push('\n'); // Blank line between glyphs
            }

            // Labels
            if glyph.labels.is_empty() && !glyph.bitmap.is_empty() {
                writeln!(buffer, ":").unwrap();
            } else {
                for label in &glyph.labels {
                    let label_str = format_label_to_string(label);
                    writeln!(buffer, "{label_str}:").unwrap();
                }
            }

            // Bitmap
            if glyph.bitmap.is_empty() {
                writeln!(buffer, "{GLYPH_ITEM_INDENT}-").unwrap();
            } else {
                for row in &glyph.bitmap.pixels {
                    write!(buffer, "{GLYPH_ITEM_INDENT}").unwrap();
                    for &pixel_is_set in row {
                        buffer.push(if pixel_is_set { '@' } else { '.' });
                    }
                    buffer.push('\n');
                }
            }

            // Per-Glyph Properties
            let mut per_glyph_props_s = String::new();
            let mut has_any_per_glyph_prop = false;

            if glyph.left_bearing.is_some() {
                append_i32(&mut per_glyph_props_s, "left-bearing", &glyph.left_bearing);
                has_any_per_glyph_prop = true;
            }
            if glyph.right_bearing.is_some() {
                append_i32(
                    &mut per_glyph_props_s,
                    "right-bearing",
                    &glyph.right_bearing,
                );
                has_any_per_glyph_prop = true;
            }
            if glyph.shift_up.is_some() {
                append_i32(&mut per_glyph_props_s, "shift-up", &glyph.shift_up);
                has_any_per_glyph_prop = true;
            }
            if glyph.top_bearing.is_some() {
                append_i32(&mut per_glyph_props_s, "top-bearing", &glyph.top_bearing);
                has_any_per_glyph_prop = true;
            }
            if glyph.bottom_bearing.is_some() {
                append_i32(
                    &mut per_glyph_props_s,
                    "bottom-bearing",
                    &glyph.bottom_bearing,
                );
                has_any_per_glyph_prop = true;
            }
            if glyph.shift_left.is_some() {
                append_i32(&mut per_glyph_props_s, "shift-left", &glyph.shift_left);
                has_any_per_glyph_prop = true;
            }
            if glyph.scalable_width.is_some() {
                append_f32(
                    &mut per_glyph_props_s,
                    "scalable-width",
                    &glyph.scalable_width,
                );
                has_any_per_glyph_prop = true;
            }

            if let Some(map) = &glyph.right_kerning {
                if !map.is_empty() {
                    let formatted_map = format_kerning_map_to_string(map);
                    if !formatted_map.is_empty() {
                        writeln!(per_glyph_props_s, "right-kerning:\n{formatted_map}").unwrap();
                        has_any_per_glyph_prop = true;
                    }
                }
            }
            if let Some(map) = &glyph.left_kerning {
                if !map.is_empty() {
                    let formatted_map = format_kerning_map_to_string(map);
                    if !formatted_map.is_empty() {
                        writeln!(per_glyph_props_s, "left-kerning:\n{formatted_map}").unwrap();
                        has_any_per_glyph_prop = true;
                    }
                }
            }

            if has_any_per_glyph_prop {
                while !buffer.ends_with("\n\n") {
                    buffer.push('\n');
                }
                for line in per_glyph_props_s.trim_end_matches('\n').lines() {
                    writeln!(buffer, "{GLYPH_ITEM_INDENT}{line}").unwrap();
                }
            }
        }
    }
    buffer.trim_end_matches('\n').to_string() + "\n" // Ensure single trailing newline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_basic_font() {
        let mut font = YaffFont::new();
        font.name = Some("Basic Font".to_string());
        font.ascent = Some(8);

        let yaff_output = to_yaff_string(&font);
        assert!(yaff_output.contains("name: Basic Font"));
        assert!(yaff_output.contains("ascent: 8"));
    }
}
