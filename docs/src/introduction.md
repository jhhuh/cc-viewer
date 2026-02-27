# cc-viewer

**cc-viewer** is a native GPU-accelerated application that visualizes Claude Code sessions as interactive directed graphs on an infinite zoomable canvas.

When Claude Code runs, it writes append-only JSONL logs recording every message, tool call, subagent spawn, and progress update. cc-viewer watches these files in real time, parses them into a directed acyclic graph, and renders the result using wgpu with glyphon text — giving you a live, navigable map of what Claude Code is doing.

## What it looks like

At a high zoom level, you see the full session as a tree of colored rectangles connected by edges:

- **Blue** nodes are user messages
- **Green** nodes are assistant responses
- **Orange** nodes are tool calls (Bash, Read, Write, etc.)
- **Tan** nodes are tool results
- **Gray** nodes are collapsed progress updates
- **Purple** nodes are subagent tasks

Scroll to zoom in and the text on each node becomes readable — labels, content summaries, tool names. Click any node to snap-zoom to it and see its full content in the sidebar.

## Key features

- **Live updates**: watches JSONL files via inotify; graph grows as Claude Code works
- **Infinite canvas**: pan and zoom freely across the entire session graph
- **GPU text**: glyphon renders text directly in the wgpu pass — no pixelation at any zoom level
- **Progress collapsing**: thousands of `bash_progress` / `agent_progress` records collapse into single nodes
- **Multi-session**: discovers and lists all sessions under `~/.claude/projects/`
- **Click-to-zoom**: click a node to animate the camera to it and read its full content
- **Active highlighting**: recently-updated nodes pulse brighter for 2 seconds

## Tech stack

| Component | Choice |
|-----------|--------|
| Window + UI | eframe 0.31 (egui + wgpu) |
| Graph rendering | Custom wgpu pipeline (WGSL shaders) |
| Text rendering | glyphon 0.8 |
| File watching | notify 8 + crossbeam-channel |
| JSON parsing | serde_json |
| Build system | Nix flake + Cargo |
