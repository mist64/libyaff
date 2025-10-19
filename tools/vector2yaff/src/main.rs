use freetype::face::LoadFlag;
use freetype::Library;
use libyaff::{to_yaff_string, Bitmap, GlyphDefinition, Label, YaffFont};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::ops::RangeInclusive;
use std::path::Path;

/// Converts a 26.6 fixed-point value to an i32 (by shifting right by 6 bits).
fn convert_metric(value: i64) -> i32 {
    (value >> 6) as i32
}

/// Converts a font file (e.g., TTF, OTF) to a YaffFont struct.
///
/// # Arguments
/// * `font_path`: Path to the font file.
/// * `point_size`: Desired point size for rendering.
/// * `dpi`: DPI (dots per inch) for rendering. Common values are 72 or 96.
/// * `range`: Range of character codes to include in the font.
///
/// # Returns
/// * `YaffFont`: A YaffFont struct containing the font data.
pub fn convert_font_to_yaff(
    font_path: &str,
    point_size: f32,
    dpi: u32,
    range_vec: Vec<RangeInclusive<u32>>,
) -> Result<YaffFont, Box<dyn std::error::Error>> {
    let mut font = YaffFont::default();

    // Initialize FreeType and load the font face.
    let lib = Library::init()?;
    let face = lib.new_face(font_path, 0)?;
    face.set_char_size(0, (point_size * 64.0) as isize, dpi, dpi)?;

    // Global Font Properties
    font.yaff_version = Some("1.0.3".to_string());

    let family_name_opt = face.family_name();
    let style_name_opt = face.style_name();

    if let Some(family) = &family_name_opt {
        font.family = Some(family.clone());
        if let Some(style) = &style_name_opt {
            font.name = Some(format!("{family} {style} {point_size}pt"));
            font.style = Some(style.clone());
        } else {
            font.name = Some(format!("{family} {point_size}pt"));
        }
    } else if let Some(style) = &style_name_opt {
        font.name = Some(format!("{point_size}pt {style}"));
        font.subfamily = Some(style.clone());
        font.style = Some(style.clone());
    } else {
        font.name = Some(format!("{point_size}pt Font"));
    }
    font.point_size = Some(point_size);

    if let Some(metrics) = face.size_metrics() {
        font.ascent = Some(convert_metric(metrics.ascender));
        font.descent = Some(-convert_metric(metrics.descender));
        font.line_height = Some(convert_metric(metrics.height));
        font.max_width = Some(convert_metric(metrics.max_advance));
        // pixel_size = ascent + descent
        if let (Some(asc), Some(desc)) = (font.ascent, font.descent) {
            font.pixel_size = Some(asc + desc);
            if let Some(lh) = font.line_height {
                font.leading = Some(lh - (asc + desc));
            }
        }
    }

    font.converter = Some(format!(
        "{} {} (using freetype-rs)",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ));

    let font_file_path = Path::new(font_path);
    if let Some(file_name) = font_file_path.file_name().and_then(|s| s.to_str()) {
        font.source_name = Some(file_name.to_string());
    }

    let ext = font_file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let format_str = match ext.to_lowercase().as_str() {
        "ttf" | "ttc" => "TrueType",
        "otf" | "otc" => "OpenType",
        "pfa" | "pfb" => "Type 1",
        "cff" => "CFF",
        "woff" => "WOFF",
        "woff2" => "WOFF2",
        "fnt" | "fon" => "Windows FNT/FON",
        "pcf" => "X11 PCF",
        "bdf" => "BDF",
        "pfr" => "PFR",
        _ => "Unknown",
    };
    font.source_format = Some(format_str.to_string());
    font.dpi = Some((dpi, dpi));

    for char_code_val in range_vec.into_iter().flatten() {
        // Load glyph with monochrome rendering.
        if face
            .load_char(
                char_code_val as usize,
                LoadFlag::RENDER | LoadFlag::TARGET_MONO,
            )
            .is_err()
        {
            continue;
        }
        let ft_glyph = face.glyph();
        let ft_bitmap = ft_glyph.bitmap();

        let mut glyph_def = GlyphDefinition::default();

        // Label
        if let Some(char_val) = std::char::from_u32(char_code_val) {
            // For printable ASCII, use the character itself in a single-quoted Unicode label element
            glyph_def.labels.push(Label::Unicode(vec![char_val as u32]));
        } else {
            // Otherwise, use the Unicode codepoint
            glyph_def.labels.push(Label::Unicode(vec![char_code_val]));
        }

        // Bitmap
        let width = ft_bitmap.width() as usize;
        let rows = ft_bitmap.rows() as usize;
        let pitch = ft_bitmap.pitch().unsigned_abs() as usize;
        let buffer = ft_bitmap.buffer();

        let mut pixels_matrix: Vec<Vec<bool>> = Vec::new();
        if width > 0 && rows > 0 {
            for y in 0..rows {
                let mut pixel_row = Vec::with_capacity(width);
                for x in 0..width {
                    let byte_index = y * pitch + (x / 8);
                    let bit_index = 7 - (x % 8); // MSB first
                    let pixel_on = (buffer[byte_index] >> bit_index) & 1;
                    pixel_row.push(pixel_on == 1);
                }
                pixels_matrix.push(pixel_row);
            }
        }
        glyph_def.bitmap = if pixels_matrix == vec![vec![false]] {
            // Single blank pixel -> empty bitmap
            Bitmap {
                pixels: vec![vec![]],
                width: 0,
                height: 0,
            }
        } else {
            Bitmap {
                pixels: pixels_matrix,
                width,
                height: rows,
            }
        };

        // Per-glyph Metrics
        let ft_metrics = ft_glyph.metrics();
        let advance_width = convert_metric(ft_glyph.advance().x);
        let hori_bearing_x = convert_metric(ft_metrics.horiBearingX);
        let glyph_width = convert_metric(ft_glyph.metrics().width);

        glyph_def.left_bearing = Some(hori_bearing_x + glyph_width - width as i32);
        glyph_def.right_bearing =
            Some(advance_width - (glyph_def.left_bearing.unwrap() + width as i32));

        // append_metric(&mut yaff_content, "top-bearing", ascender - bitmap_top);
        let bitmap_top = ft_glyph.bitmap_top();
        glyph_def.top_bearing = Some(font.ascent.unwrap() - bitmap_top);
        
        // Calculate shift-up for compatibility: baseline to raster bottom edge
        let bitmap_bottom = bitmap_top - rows as i32;
        glyph_def.shift_up = Some(bitmap_bottom);

        font.glyphs.push(glyph_def);
    }

    // Extract kerning information if available
    if face.has_kerning() {
        // Collect all character codes we've processed
        let char_codes: Vec<u32> = font
            .glyphs
            .iter()
            .filter_map(|g| {
                // Extract character code from Unicode label
                if let Some(Label::Unicode(codes)) = g.labels.first() {
                    codes.first().copied()
                } else {
                    None
                }
            })
            .collect();

        // Build a map for right_kerning data
        let mut kerning_map: HashMap<u32, HashMap<Label, f32>> = HashMap::new();

        // Check kerning between all pairs
        for &left_char in &char_codes {
            for &right_char in &char_codes {
                if let Some(left_index) = face.get_char_index(left_char as usize) {
                    if let Some(right_index) = face.get_char_index(right_char as usize) {
                        if let Ok(kerning) = face.get_kerning(
                            left_index,
                            right_index,
                            freetype::face::KerningMode::KerningDefault,
                        ) {
                            let kern_x = convert_metric(kerning.x);
                            if kern_x != 0 {
                                let right_label = Label::Unicode(vec![right_char]);

                                kerning_map
                                    .entry(left_char)
                                    .or_default()
                                    .insert(right_label, kern_x as f32);
                            }
                        }
                    }
                }
            }
        }

        // Apply kerning data to glyphs
        for glyph in &mut font.glyphs {
            if let Some(Label::Unicode(codes)) = glyph.labels.first() {
                if let Some(&char_code) = codes.first() {
                    if let Some(kern_pairs) = kerning_map.get(&char_code) {
                        glyph.right_kerning = Some(kern_pairs.clone());
                    }
                }
            }
        }
    }

    Ok(font)
}

// Logic for parsing ranges from command line
fn parse_num(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex number '{s}': {e}"))
    } else {
        s.parse::<u32>()
            .map_err(|e| format!("Invalid decimal number '{s}': {e}"))
    }
}

fn parse_ranges(input: &str) -> Result<Vec<RangeInclusive<u32>>, String> {
    input
        .split(',')
        .map(|part| {
            let bounds: Vec<&str> = part.split('-').collect();
            match bounds.len() {
                1 => {
                    let n = parse_num(bounds[0])?;
                    Ok(n..=n)
                }
                2 => {
                    let start = parse_num(bounds[0])?;
                    let end = parse_num(bounds[1])?;
                    if start > end {
                        Err(format!("Start {start} greater than end {end}"))
                    } else {
                        Ok(start..=end)
                    }
                }
                _ => Err(format!("Invalid range format: {part}")),
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut dpi_value: u32 = 72;
    let mut positional = vec![];

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dpi" => {
                if i + 1 >= args.len() {
                    eprintln!("Expected argument after --dpi");
                    std::process::exit(1);
                }
                dpi_value = args[i + 1].parse().expect("Invalid DPI value");
                i += 2;
            }
            _ => {
                positional.push(args[i].clone());
                i += 1;
            }
        }
    }

    if positional.len() != 4 {
        eprintln!(
            "Usage: {} [--dpi <DPI>] <TTF_PATH> <POINT_SIZE> <RANGE> <OUTPUT_YAFF>\n\
             Example: {} --dpi 96 font.ttf 12 0x20-0x7E,0x20AC output.yaff",
            args[0], args[0],
        );
        std::process::exit(1);
    }

    let font_path = &positional[0];
    let point_size: f32 = positional[1].parse().expect("Invalid point size");
    let ranges = parse_ranges(&positional[2])?;
    let output_file_path = &positional[3];

    match convert_font_to_yaff(font_path, point_size, dpi_value, ranges) {
        Ok(font) => {
            let yaff_content = to_yaff_string(&font);
            let mut file = File::create(output_file_path)?;
            file.write_all(yaff_content.as_bytes())?;
            println!("YAFF file '{output_file_path}' created successfully.");
        }
        Err(e) => {
            eprintln!("Error converting font: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
