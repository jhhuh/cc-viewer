#!/usr/bin/env bash
set -e

# Capture a screenshot of cc-viewer via headless Xvfb + software GL.
# Requires: Xvfb, ImageMagick (import), cargo — all provided by nix devShell.
# Usage: nix develop -c bash scripts/capture_demo.sh

mkdir -p artifacts

cleanup() {
  kill $XVFB_PID 2>/dev/null || true
  kill $APP_PID 2>/dev/null || true
}
trap cleanup EXIT

echo "Starting Xvfb..."
Xvfb :99 -screen 0 1280x800x24 &
XVFB_PID=$!
sleep 2

echo "Starting cc-viewer (software GL)..."
DISPLAY=:99 WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 cargo run &
APP_PID=$!
sleep 6  # Wait for window + data load

echo "Taking screenshot..."
DISPLAY=:99 import -window root artifacts/demo.png

echo "Captured: artifacts/demo.png"
