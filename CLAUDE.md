# cc-viewer

Claude Code session graph visualizer. Native Rust GUI (eframe + wgpu) that watches Claude Code's runtime directory and visualizes live sessions as a directed graph.

## Quick Start

```
nix develop -c cargo run
# or via overmind:
nix develop -c overmind start
```

## Architecture

- **eframe 0.31** (egui + wgpu): Window management and UI overlay
- **Custom wgpu pipeline**: Node rects, edge lines, pan/zoom camera
- **glyphon 0.8**: GPU text rendering at any zoom level
- **notify 8**: inotify-based file watching for live updates

## Data Sources

- Session JSONL: `~/.claude/projects/{project}/{session_id}.jsonl`
- Subagent JSONL: `~/.claude/projects/{project}/{session_id}/subagents/agent-{id}.jsonl`
- Runtime dir: `/tmp/claude-{UID}/` — symlinks to active session files

## Project Structure

```
src/
  main.rs           -- eframe entry point
  app.rs            -- eframe::App impl
  data/
    types.rs        -- Record, GraphNode, SessionGraph, AppState
    source.rs       -- DataSource trait (WASM portability boundary)
    native.rs       -- notify file watcher + incremental JSONL tailing
    parse.rs        -- JSONL parser
  graph/
    build.rs        -- Records → graph with progress collapsing
    layout.rs       -- Tree layout algorithm
    state.rs        -- Pan/zoom/selection input handling
  render/
    canvas.rs       -- wgpu pipeline: node rects, edges
    callback.rs     -- egui_wgpu::CallbackTrait bridge
    text.rs         -- glyphon text rendering
  ui/
    overlay.rs      -- egui sidebar: session list, node detail
```

## Key Design Decisions

- `DataSource` trait abstracts filesystem access for future WASM portability
- Progress records (bash_progress, agent_progress) with same toolUseID collapse into single nodes
- Text renders via glyphon directly in the wgpu pass — scales naturally with camera zoom
- Camera uses world-space coordinates; wgpu shader transforms to clip space
