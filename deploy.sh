#!/bin/bash
set -e

cd visualization
dx build --platform web --release

# Copy build output to repo root for GitHub Pages
BUILD_DIR="target/dx/visualization/release/web/public"
cp "$BUILD_DIR/index.html" ../
rm -rf ../assets
cp -r "$BUILD_DIR/assets" ../

echo "Done. Review changes with 'git diff' then commit and push."
