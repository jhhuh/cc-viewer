#!/usr/bin/env bash
set -euo pipefail

# Screenshot capture script for cc-viewer docs.
# Usage: nix run .#screenshots
# Run from the repo root — saves to docs/src/images/.

OUT="${1:-docs/src/images}"
mkdir -p "$OUT"

DISPLAY_NUM=99
export DISPLAY=":${DISPLAY_NUM}"
export WGPU_BACKEND=gl
export LIBGL_ALWAYS_SOFTWARE=1
# Mesa DRI drivers path — needed for swrast_dri.so software rendering
export MESA_LOADER_DRIVER_OVERRIDE=swrast

cleanup() {
  kill "$APP_PID" 2>/dev/null || true
  kill "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "Starting Xvfb on :${DISPLAY_NUM}..."
Xvfb ":${DISPLAY_NUM}" -screen 0 1920x1080x24 &>/dev/null &
XVFB_PID=$!
sleep 2

echo "Starting cc-viewer --all..."
cc-viewer --all &>/dev/null &
APP_PID=$!
sleep 12

shot() {
  local name="$1"
  import -window root "$OUT/$name"
  echo "  captured $name"
}

echo "Taking screenshots..."

# 1: Overview — --all auto-enables "Show inactive", showing all projects+sessions
#    Canvas shows the top of the currently selected session (GulBox/soft-tumbling-crescent)
shot "01_overview.png"

# 2: Uncheck "Show inactive" — reveals only active sessions (shorter list)
#    Checkbox is near (22, 66) based on shot 1 layout at 1920x1080
xdotool mousemove 22 66 click 1
sleep 2
shot "02_all_sessions.png"

# 3: Switch to a different project's session
#    After unchecking inactive, sidebar shows ~4 projects with 1 session each.
#    Click the second project's session (cc-viewer / wise-dancing-stream).
#    At 1920x1080 with the shorter list, this should be around y=239.
xdotool mousemove 175 239 click 1
sleep 3
shot "03_session_switch.png"

# 4: Click on a node in the canvas to expand it
#    Target the "User -> Assistant -> 58 tools" node (around y=464 in the session)
xdotool mousemove 960 464 click 1
sleep 2
shot "04_node_expanded.png"

# 5: Zoom out for bird's-eye view
#    Moderate zoom out — 12 clicks to see the whole session without losing legibility
xdotool mousemove 960 500
for _ in $(seq 1 12); do
  xdotool click 5
  sleep 0.05
done
sleep 2
shot "05_zoomed_out.png"

echo "Done — screenshots saved to $OUT"
