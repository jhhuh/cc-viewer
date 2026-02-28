# Headless wgpu Screenshot via Xvfb + Software GL

## Problem
wgpu apps (eframe, bevy, etc.) can't render in Xvfb by default because Xvfb doesn't provide a Vulkan ICD. The result is a black screen.

## Solution
Three environment variables + correct nix dependencies:

```bash
WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 MESA_LOADER_DRIVER_OVERRIDE=swrast
```

## Critical: Mesa Must Be on LD_LIBRARY_PATH

`libGL` in nixpkgs is `libglvnd` — a **dispatcher only**. It has no rendering backend. You MUST also include `mesa` in your library path to provide:
- `libGLX_mesa.so` — the actual GLX implementation
- `lib/dri/swrast_dri.so` — the software rasterizer DRI driver

Without `mesa`, `LIBGL_ALWAYS_SOFTWARE=1` has nothing to dispatch to and you get **black screenshots**.

## Nix Flake runtimeLibs

```nix
runtimeLibs = with pkgs; [
  libGL      # libglvnd dispatcher
  mesa       # actual GL implementation + swrast DRI driver
  # ... other libs (vulkan-loader, libxkbcommon, etc.)
];
```

## Full Pipeline
```bash
Xvfb :99 -screen 0 1920x1080x24 &
sleep 2

export DISPLAY=:99
export WGPU_BACKEND=gl
export LIBGL_ALWAYS_SOFTWARE=1
export MESA_LOADER_DRIVER_OVERRIDE=swrast

my-wgpu-app &
sleep 12  # enough time for data loading + first render

import -window root screenshot.png  # ImageMagick X11 capture
```

## Why It Works
- `WGPU_BACKEND=gl` — tells wgpu to skip Vulkan and use the OpenGL/GLES backend
- `LIBGL_ALWAYS_SOFTWARE=1` — tells libglvnd+Mesa to use software rendering
- `MESA_LOADER_DRIVER_OVERRIDE=swrast` — explicitly selects the swrast DRI driver
- `mesa` provides `libGLX_mesa.so` (GLX impl) and `swrast_dri.so` (software rasterizer)
- Xvfb provides a virtual X11 framebuffer with GLX support
- ImageMagick `import -window root` captures the X11 root window

## Nix Dependencies

For the screenshot script wrapper:
```nix
take-screenshots = pkgs.writeShellScriptBin "screenshots" ''
  export PATH="${pkgs.lib.makeBinPath [
    my-app pkgs.xorg.xorgserver pkgs.xdotool pkgs.imagemagick pkgs.coreutils
  ]}:$PATH"
  export LD_LIBRARY_PATH="${runtimeLibPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  exec ${./scripts/screenshots.sh}
'';
```

## Gotchas
- **Black screenshots**: Almost always means Mesa is missing from `LD_LIBRARY_PATH`. The app window will exist (xdotool finds it) but no pixels render.
- The app needs enough sleep time to load data and render — 12s is safe for data-heavy apps.
- `import -window root` captures the entire virtual screen; use xdotool for interactions.
- 1920x1080 gives high-quality screenshots; 1280x800 is fine for smaller demos.
- This approach works outside sandboxed environments (CI, nix flake apps) — the sandbox may have Mesa pre-loaded.
