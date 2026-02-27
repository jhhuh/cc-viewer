# Source Map

Complete listing of every source file with its purpose and approximate size.

## Entry

| File | Lines | Purpose |
|------|-------|---------|
| `src/main.rs` | 23 | eframe entry point, window configuration |
| `src/app.rs` | 71 | `eframe::App` implementation, main update loop |

## Data layer (`src/data/`)

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 8 | Module declarations, re-exports |
| `types.rs` | 189 | Core types: `Record`, `GraphNode`, `SessionGraph`, `AppState`, `DataEvent` |
| `source.rs` | 9 | `DataSource` trait definition (WASM portability boundary) |
| `native.rs` | 210 | `NativeSource`: inotify file watcher, incremental JSONL tailing |
| `parse.rs` | 207 | JSONL parser: `parse_line()`, content extraction, tool info extraction |

## Graph layer (`src/graph/`)

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 4 | Module declarations |
| `build.rs` | 144 | `add_records_to_graph()`: record integration, progress collapsing |
| `layout.rs` | 93 | `do_layout()`: DFS tree layout, position assignment |
| `state.rs` | 105 | `handle_input()`: pan, zoom, click, hit testing, animation |

## Render layer (`src/render/`)

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 7 | Module declarations, re-exports |
| `canvas.rs` | 215 | `CanvasState`, WGSL shader, `build_vertices()`, `CameraUniform` |
| `callback.rs` | 122 | `CanvasCallback`: `egui_wgpu::CallbackTrait` implementation |
| `text.rs` | 247 | `GlyphonState`, `prepare_text()`, `render_text()` |

## UI layer (`src/ui/`)

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 2 | Module declaration |
| `overlay.rs` | 155 | `draw_sidebar()`: session list, stats, node detail panel |

## Configuration

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust dependencies and project metadata |
| `flake.nix` | Nix flake: dev shell, package build, docs build |
| `Procfile` | overmind process definition (`cargo run`) |
| `CLAUDE.md` | Project-level instructions for Claude Code |
| `.gitignore` | Ignores `target/` and `.config/` |
