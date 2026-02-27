# Rendering

cc-viewer uses a two-layer rendering approach: a custom wgpu pipeline for geometry (node rectangles and edge lines) and glyphon for text. Both render into the same wgpu render pass provided by eframe's `PaintCallback` system.

## Render loop

Each frame:

1. **egui layout**: eframe runs the egui immediate-mode UI (sidebar panel)
2. **Canvas allocation**: `CentralPanel` allocates a rect for the wgpu canvas
3. **Input handling**: `graph::state::handle_input()` processes pan/zoom/click on the rect
4. **PaintCallback creation**: a `CanvasCallback` is created with a clone of the current `AppState` and pushed into the egui paint list
5. **Callback execution**: eframe calls our `prepare()` then `paint()` methods during its render pass

The `AppState` clone in step 4 is intentional — it decouples the UI thread's mutable state from the immutable snapshot used by the GPU pipeline.

## Geometry rendering

The geometry pipeline (`canvas.rs`) uses a simple vertex format:

```rust
struct Vertex {
    position: [f32; 2],  // world-space position
    color: [f32; 4],     // RGBA color
}
```

`build_vertices()` generates:

- **Edges**: thin quads (2 triangles per edge) connecting parent and child nodes. Edge direction is from the bottom-center of the parent to the top-center of the child.
- **Nodes**: axis-aligned quads (2 triangles per node) colored by `NodeKind`.

Selected nodes render as white. Recently-active nodes have their color brightened proportionally to recency (linear decay over 2 seconds).

## Text rendering

glyphon handles text separately — see [Text Rendering](../internals/text-rendering.md) for details. Text is rendered **after** geometry in the same render pass, so it appears on top of node rectangles.

## Frame resource lifecycle

Each frame creates fresh GPU buffers for geometry:

- **Camera uniform buffer**: the camera state uploaded to the GPU
- **Vertex buffer**: all node/edge vertices
- **Index buffer**: triangle indices

These are stored in `FrameResources` inside egui's `CallbackResources` (a type-map). The previous frame's resources are replaced each frame.

The pipeline and bind group layout are **persistent** — stored as `Arc<T>` in `CanvasState` and cloned into `FrameResources` for use in the `'static` render pass.

glyphon state (`GlyphonState`) is also persistent across frames, stored in the same `CallbackResources` type-map.
