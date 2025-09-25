# LibYAFF: Yet Another Font Format Library

[![Crates.io](https://img.shields.io/crates/v/libyaff.svg)](https://crates.io/crates/libyaff)
[![Docs.rs](https://docs.rs/libyaff/badge.svg)](https://docs.rs/libyaff)

A Rust library for parsing, manipulating, and generating bitmap fonts in the [YAFF format](https://github.com/robhagemans/monobit/blob/master/YAFF.md). YAFF is a human-readable, line-based format for describing bitmap fonts, supporting kerning and multiple labels for each glyph.

**Includes a handy tool to convert vector fonts (TTF, OTF, and more) to YAFF format** - see the [Tools](#tools) section below.

## Features

- **Complete YAFF format support**: Parse and generate YAFF 1.0.x format files.
- **Unicode and legacy encoding**: Support for Unicode, codepoint, and tag-based glyph labeling.
- **Advanced typography**: Kerning, bearing adjustments, and font metrics.
- **Robust parsing**: Handles format variations and provides detailed error messages.

## Cargo Features

This crate has the following cargo features:

- `parsing` (enabled by default): Enables the font parsing functionality. This feature depends on the `regex` crate.
- `encoding` (enabled by default): Enables the font encoding functionality for generating YAFF format output.

## Quick Start

Add `libyaff` to your `Cargo.toml`:

```toml
[dependencies]
libyaff = "0.1.0"
```

Load a YAFF font from a file and inspect its properties:

```rust,no_run
use libyaff::{YaffFont, to_yaff_string};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load a YAFF font from file
    let font = YaffFont::from_path("my_font.yaff")?;
    println!("Loaded font: {}", font.name.unwrap_or_default());

    // Access font metrics
    if let Some(ascent) = font.ascent {
        println!("Font ascent: {}", ascent);
    }

    // Convert back to YAFF format
    let yaff_content = to_yaff_string(&font);
    std::fs::write("output.yaff", yaff_content)?;

    Ok(())
}
```

## Tools

### vector2yaff

The `vector2yaff` tool converts vector fonts (TTF, OTF, etc.) to YAFF format using FreeType.

```bash
# Build the tool
cargo build -p vector2yaff

# Convert a font with specific character range
cargo run -p vector2yaff -- font.ttf 12 0x20-0x7E output.yaff

# With custom DPI
cargo run -p vector2yaff -- --dpi 96 font.ttf 12 0x20-0x7E,0x20AC output.yaff
```

**Arguments:**
- `TTF_PATH`: Path to the font file (TTF, OTF, etc.)
- `POINT_SIZE`: Desired point size for rendering
- `RANGE`: Character range(s) to include (e.g., `0x20-0x7E` for ASCII printable, `32-126` for decimal)
- `OUTPUT_YAFF`: Output YAFF file path

**Options:**
- `--dpi <DPI>`: DPI for rendering (default: 72)

**Supported formats:**
- TrueType (.ttf, .ttc)
- OpenType (.otf, .otc)
- Type 1 (.pfa, .pfb)
- And many more via FreeType

## Building and Testing

This project includes a `test.sh` script that can be used to build and test the library. The script uses the example code in `examples/test.rs` to parse a YAFF file and then write it back out.

To run the tests, execute the following command:

```sh
./test.sh <YAFF_FILES_DIR>
```

**Arguments:**
- `YAFF_FILES_DIR`: Directory containing `.yaff` files to test

**What the test does:**
The test script performs a round-trip validation by:
1. Parsing each `.yaff` file in the specified directory
2. Writing it back out to a temporary file
3. Parsing that temporary file and writing it out again
4. Comparing the two temporary outputs to ensure they're identical

This validates that the library can parse and generate YAFF files consistently without data loss.

**Example usage:**
```sh
# Test with hoard-of-bitfonts repository
git clone https://github.com/robhagemans/hoard-of-bitfonts
./test.sh hoard-of-bitfonts/hellschreiber
./test.sh hoard-of-bitfonts/atari/8-bit
```

* If a file cannot be parsed, the script prints `ERROR` with details and continues.
* If differences are detected, it will show a diff and exit with an error.

## License

This project is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Author

Michael Steil <mist64@mac.com>
