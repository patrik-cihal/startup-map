#!/bin/bash
set -e

cd "$(dirname "$0")"

# Clean stale dx output
rm -rf target/dx/visualization/release/web

cd visualization
dx build --platform web --release

BUILD_DIR="../target/dx/visualization/release/web/public"

cp "$BUILD_DIR/index.html" ../
rm -rf ../assets
cp -r "$BUILD_DIR/assets" ../

echo "Done. Review changes with 'git diff' then commit and push."
