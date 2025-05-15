# bin/build_icons.sh
#!/bin/bash
# Master script for building the Iconify fonts

set -e

# Step 1: Make sure dependencies are installed
echo "Installing dependencies..."
bun install --no-save @iconify/json @iconify/utils
dart pub get

# Step 2: Extract SVGs from Iconify JSON files
echo "Generating JSON files with SVGs..."
bun gen/icon-extract.ts

# Step 3: Generate the fonts and Dart files
echo "Generating Tabler font..."
dart gen/icon-font.dart \
  --prefix="tabler" \
  --icon-set-name="Tabler"

echo "Generating Phosphor font..."
dart gen/icon-font.dart \
  --prefix="ph" \
  --icon-set-name="Phosphor"
