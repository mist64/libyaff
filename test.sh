#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Usage and argument handling ---
if [ $# -eq 0 ]; then
    echo "Usage: $0 <YAFF_FILES_DIR>"
    echo "  YAFF_FILES_DIR: Directory containing .yaff files to test"
    echo ""
    echo "Example: Test fonts from hoard-of-bitfonts repository"
    echo "  git clone https://github.com/robhagemans/hoard-of-bitfonts"
    echo "  $0 hoard-of-bitfonts/hellschreiber"
    echo "  $0 hoard-of-bitfonts/atari/8-bit"
    exit 1
fi

YAFF_FILES_DIR="$1"

# Build the binary once
echo "Building the test binary..."
cargo build --release --example test
BINARY_PATH="./target/release/examples/test"

TEMP_OUTPUT_1=/tmp/1.yaff
TEMP_OUTPUT_2=/tmp/2.yaff


find "$YAFF_FILES_DIR" -type f -name '*.yaff' -print0 | while IFS= read -r -d $'\0' yaff_file; do
    echo "$yaff_file"

    # Clean up any existing temp files before each test
    rm -f "$TEMP_OUTPUT_1" "$TEMP_OUTPUT_2"

    # 1. First pass: Original file -> temp1
    if ! "$BINARY_PATH" "$yaff_file" "$TEMP_OUTPUT_1" 2>/dev/null; then
        echo "ERROR: Failed in first pass. Skipping."
        continue # Skip to the next file
    fi

    # 2. Second pass: temp1 -> temp2
    if ! "$BINARY_PATH" "$TEMP_OUTPUT_1" "$TEMP_OUTPUT_2" 2>/dev/null; then
        echo "ERROR: Failed in second pass. Skipping."
        continue # Skip to the next file
    fi

    # 3. Compare the two temporary output files
    if ! cmp -s "$TEMP_OUTPUT_1" "$TEMP_OUTPUT_2"; then
        echo "DIFFERENCE DETECTED for file: $yaff_file"
        echo "Output of first pass (original -> temp1) is in: $TEMP_OUTPUT_1"
        echo "Output of second pass (temp1 -> temp2) is in: $TEMP_OUTPUT_2"
        echo "To reproduce this difference:"
        echo "$BINARY_PATH \"$yaff_file\" /tmp/1.yaff && $BINARY_PATH /tmp/1.yaff /tmp/2.yaff && diff -u /tmp/1.yaff /tmp/2.yaff"
        echo "Showing diff between first pass output and second pass output:"
        diff -u "$TEMP_OUTPUT_1" "$TEMP_OUTPUT_2" || true # '|| true' so diff doesn't stop script if files differ
        echo "Stopping script due to difference."
        exit 1 # Exit the script immediately
    fi
done

echo "All processed .yaff files were consistent through two passes."
exit 0