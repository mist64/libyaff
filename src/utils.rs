use crate::models::*;

/// Calculate the ascent value for a font based on its glyphs
pub fn calculate_ascent(font: &YaffFont) -> i32 {
    font.glyphs
        .iter()
        .map(|glyph| {
            let shift_up = glyph.shift_up.unwrap_or(0);
            let height = glyph.bitmap.height as i32;
            shift_up + height
        })
        .max()
        .unwrap_or(0)
}

/// Set the ascent value for a font based on its glyphs
pub fn set_ascent(font: &mut YaffFont) {
    font.ascent = Some(calculate_ascent(font));
}

/// Minimize the bounding boxes of all glyphs in a font
pub fn minimize_all_bounding_boxes(font: &mut YaffFont) {
    let ascent = font.ascent.unwrap_or(calculate_ascent(font));
    for glyph_def in font.glyphs.iter_mut() {
        minimize_glyph_bounding_box(glyph_def, ascent);
    }
}

/// Minimize the bounding box of a single glyph
pub fn minimize_glyph_bounding_box(glyph_def: &mut GlyphDefinition, ascent: i32) {
    if glyph_def.bitmap.pixels.is_empty() || glyph_def.bitmap.width == 0 {
        // If bitmap is already empty or has no width, ensure it's fully reset
        // and bearings reflect an empty glyph occupying no space.
        let original_width = glyph_def.bitmap.width as i32;
        glyph_def.bitmap.width = 0;
        glyph_def.bitmap.height = 0;
        glyph_def.bitmap.pixels.clear();

        // Adjust right_bearing to maintain original advance width
        if original_width > 0 {
            glyph_def.right_bearing = Some(glyph_def.right_bearing.unwrap_or(0) + original_width);
        }
        if glyph_def.top_bearing.is_some() {
            glyph_def.shift_up = glyph_def.top_bearing;
        }
        return;
    }

    let old_bitmap_height = glyph_def.bitmap.height;
    let old_bitmap_width = glyph_def.bitmap.width;

    let mut min_y: usize = old_bitmap_height;
    let mut max_y: usize = 0;
    let mut min_x: usize = old_bitmap_width;
    let mut max_x: usize = 0;
    let mut has_ink = false;

    for (y, row) in glyph_def.bitmap.pixels.iter().enumerate() {
        for (x, &pixel_is_set) in row.iter().enumerate() {
            if pixel_is_set {
                has_ink = true;
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }

    if !has_ink {
        // Glyph has a bitmap allocated but no inked pixels
        let original_width_i32 = glyph_def.bitmap.width as i32;
        glyph_def.bitmap.pixels.clear();
        glyph_def.bitmap.width = 0;
        glyph_def.bitmap.height = 0;

        // Adjust right_bearing to maintain original advance width
        glyph_def.right_bearing = Some(glyph_def.right_bearing.unwrap_or(0) + original_width_i32);

        if glyph_def.top_bearing.is_some() {
            glyph_def.shift_up = glyph_def.top_bearing;
        }
        return;
    }

    // Calculate new dimensions
    let new_bitmap_height = max_y - min_y + 1;
    let new_bitmap_width = max_x - min_x + 1;

    // Crop the bitmap pixels
    let new_pixels: Vec<Vec<bool>> = glyph_def.bitmap.pixels[min_y..=max_y]
        .iter()
        .map(|row| row[min_x..=max_x].to_vec())
        .collect();

    // Get original metrics, defaulting to 0 if None
    let old_left_bearing = glyph_def.left_bearing.unwrap_or(0);
    let old_right_bearing = glyph_def.right_bearing.unwrap_or(0);
    let old_shift_up = glyph_def.shift_up.unwrap_or(0);
    let old_top_bearing = glyph_def
        .top_bearing
        .unwrap_or_else(|| ascent - (old_shift_up + old_bitmap_height as i32));

    // Adjust metrics based on trimming
    glyph_def.left_bearing = Some(old_left_bearing + min_x as i32);
    glyph_def.top_bearing = Some(old_top_bearing + min_y as i32);
    glyph_def.shift_up = Some(glyph_def.top_bearing.unwrap() - new_bitmap_height as i32);

    // right_bearing: space from right edge of new bitmap to advance point
    let trimmed_from_right_of_content = (old_bitmap_width as i32 - 1) - max_x as i32;
    glyph_def.right_bearing = Some(old_right_bearing + trimmed_from_right_of_content);

    // Update bitmap
    glyph_def.bitmap.pixels = new_pixels;
    glyph_def.bitmap.width = new_bitmap_width;
    glyph_def.bitmap.height = new_bitmap_height;
}

/// Convert Codepoint labels to Unicode labels for ASCII range
///
/// For all glyphs that don't have a Unicode label, but do have a Codepoint label,
/// create a Unicode label from the Codepoint label. If there is no encoding,
/// assume ASCII, i.e. convert codepoints 0x20 to 0x7E, ignore the rest.
/// For other encodings, do nothing.
pub fn convert_codepoint_to_unicode_labels(font: &mut YaffFont) {
    match font.encoding.as_deref() {
        None | Some("ascii") => {
            for glyph in &mut font.glyphs {
                if glyph.labels.iter().any(|l| matches!(l, Label::Unicode(_))) {
                    // Already has a Unicode label, skip
                    continue;
                }
                if let Some(Label::Codepoint(codepoints)) = glyph
                    .labels
                    .iter()
                    .find(|l| matches!(l, Label::Codepoint(_)))
                {
                    let unicode_labels: Vec<Label> = codepoints
                        .iter()
                        .filter_map(|&cp| {
                            if (0x20..=0x7E).contains(&cp) {
                                Some(Label::Unicode(vec![cp as u32]))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !unicode_labels.is_empty() {
                        glyph.labels.extend(unicode_labels);
                    }
                }
            }
        }
        Some(_) => {
            println!("Warning: Unsupported encoding. Only ASCII is processed.");
        }
    }
}
