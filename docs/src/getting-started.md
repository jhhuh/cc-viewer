# Getting Started

## Prerequisites

cc-viewer requires a Linux system with:

- **Nix** (with flakes enabled) — handles all other dependencies
- **Vulkan-capable GPU** — for wgpu rendering (most modern GPUs work)
- A running display server (X11 or Wayland)

## Building

### With Nix (recommended)

```bash
# Build the binary
nix build

# Run directly
nix run

# Or enter the dev shell and use cargo
nix develop -c cargo run
```

### With Cargo (manual deps)

If you prefer not to use Nix, you need these system libraries:

- `vulkan-loader`
- `libxkbcommon`
- `wayland` (if using Wayland)
- `libGL`
- `libX11`, `libXcursor`, `libXi`, `libXrandr` (if using X11)
- `pkg-config`

Then:

```bash
cargo run
```

## First launch

When cc-viewer starts, it scans `~/.claude/projects/` for projects with active runtime entries. Sessions are listed in the sidebar grouped by project:

![First launch](images/01_overview.png)

If the sidebar says "No sessions found", either:
- Claude Code hasn't been used yet, or
- No sessions have an active runtime entry (check "Show inactive" to see older sessions)

## Viewing live sessions

Start a Claude Code session in another terminal. cc-viewer watches `~/.claude/projects/` recursively via inotify — it will discover the new JSONL file and start rendering the session graph in real time.

The graph grows as Claude Code processes messages, calls tools, and spawns subagents.

## Building the documentation

```bash
# Build static HTML docs
nix build .#docs

# Serve docs locally (opens browser)
nix run .#docs
```
