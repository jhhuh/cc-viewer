# Architecture Overview

## Module structure

```
src/
  main.rs              Entry point: eframe window setup
  app.rs               eframe::App — the main update loop

  data/                Data ingestion layer
    types.rs           Core types: Record, GraphNode, AppState, etc.
    source.rs          DataSource trait (portability boundary)
    native.rs          notify-based file watcher + JSONL tailing
    parse.rs           JSONL line parser

  graph/               Graph logic
    build.rs           Records -> SessionGraph with progress collapsing
    layout.rs          Tree layout algorithm
    state.rs           Pan/zoom/selection input handling

  render/              GPU rendering
    canvas.rs          wgpu pipeline, shader, vertex generation
    callback.rs        egui_wgpu::CallbackTrait bridge
    text.rs            glyphon text rendering

  ui/                  UI overlay
    overlay.rs         egui sidebar: session list, node detail
```

## Data flow

```
                 +-----------+
                 | Filesystem|
                 |  (JSONL)  |
                 +-----+-----+
                       |
              inotify events
                       |
                 +-----v-----+
                 |  native.rs |   DataSource::poll()
                 | (NativeSource) |
                 +-----+-----+
                       |
                Vec<DataEvent>
                       |
                 +-----v-----+
                 | types.rs   |   apply_events()
                 | (AppState) |
                 +-----+-----+
                       |
              layout_dirty = true
                       |
                 +-----v-----+
                 | layout.rs  |   do_layout()
                 | (positions)|
                 +-----+-----+
                       |
              AppState with positioned nodes
                       |
          +------------+-------------+
          |                          |
    +-----v-----+            +------v------+
    | canvas.rs  |            |  text.rs    |
    | (vertices) |            | (glyphon)   |
    +-----+------+            +------+------+
          |                          |
    wgpu draw calls            wgpu draw calls
          |                          |
          +------> RenderPass <------+
                       |
                    Screen
```

## Key design boundaries

### DataSource trait

The `DataSource` trait in `source.rs` is the portability boundary:

```rust
pub trait DataSource: Send {
    fn poll(&mut self) -> Vec<DataEvent>;
}
```

The native implementation uses `notify` (inotify) and filesystem reads. A future WASM implementation would use a WebSocket client to receive the same `DataEvent` stream from a server.

### AppState

`AppState` is the single source of truth. It's a `Clone` type passed by value into the render callback each frame. This avoids shared-mutable-state problems between the egui UI thread and the wgpu render pipeline.

### egui_wgpu::CallbackTrait

eframe provides a `PaintCallback` mechanism that lets us inject custom wgpu rendering into the egui render pass. Our `CanvasCallback` implements this trait:

- `prepare()` runs before the render pass — builds GPU buffers, prepares text
- `paint()` runs during the render pass — issues draw calls

Both geometry and text rendering happen in the **same render pass**, which is important for correct layering (text on top of node rectangles).
