# Headless wgpu Screenshot via Xvfb + Software GL

## Problem
wgpu apps (eframe, bevy, etc.) can't render in Xvfb by default because Xvfb doesn't provide a Vulkan ICD. The result is a black screen.

## Solution
Two environment variables force wgpu to use Mesa's software OpenGL renderer (llvmpipe):

```bash
WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1
```

## Full Pipeline
```bash
# Nix devShell needs: xvfb-run, xorg.xorgserver, imagemagick
Xvfb :99 -screen 0 1280x800x24 &
sleep 2
DISPLAY=:99 WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 cargo run &
sleep 6
DISPLAY=:99 import -window root screenshot.png
```

## Why It Works
- `WGPU_BACKEND=gl` — tells wgpu to skip Vulkan and use the OpenGL/GLES backend
- `LIBGL_ALWAYS_SOFTWARE=1` — tells Mesa to use llvmpipe (CPU-based GL rasterizer)
- Xvfb provides a virtual X11 framebuffer with GLX support
- Mesa llvmpipe handles the actual GL draw calls in software
- ImageMagick `import -window root` captures the X11 root window

## Nix Dependencies
```nix
buildInputs = [ xvfb-run xorg.xorgserver imagemagick ];
```

## Gotchas
- The app needs enough sleep time to load data and render at least one frame
- `import -window root` captures the entire virtual screen; use `-window <id>` with xdotool for a specific window
- eframe logs "Using wgpu" even with GL backend — this is correct, wgpu wraps GL
- Don't run on the host display (:0) from a sandbox — use Xvfb on a separate display number
