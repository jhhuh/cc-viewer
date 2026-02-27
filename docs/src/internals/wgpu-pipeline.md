# wgpu Pipeline

`render/canvas.rs` defines the custom wgpu render pipeline for drawing node rectangles and edge lines.

## Pipeline setup

The pipeline is created once during `App::new()` in `CanvasState::new()`:

```
Shader (WGSL) -> Pipeline Layout -> Render Pipeline
                      |
              Camera Bind Group Layout
              (1 uniform buffer at binding 0)
```

The pipeline uses:
- **Vertex format**: `[position: float32x2, color: float32x4]` — 24 bytes per vertex
- **Topology**: `TriangleList` — explicit index buffer, no strips
- **Blending**: alpha blending (`ALPHA_BLENDING`)
- **No depth buffer**: 2D rendering, painter's algorithm (edges drawn first, nodes on top)

The pipeline and bind group layout are wrapped in `Arc<T>` because they must outlive the `'static` render pass lifetime. Each frame, an `Arc::clone` is stored in `FrameResources`.

## WGSL shader

The shader transforms world-space vertex positions to clip space using a camera uniform:

```wgsl
struct Camera {
    offset: vec2<f32>,    // screen-space pan offset
    zoom: f32,            // zoom multiplier
    aspect: f32,          // viewport aspect ratio (unused currently)
    viewport_size: vec2<f32>,  // viewport dimensions in pixels
    _pad: vec2<f32>,      // padding to 32 bytes (std140 alignment)
};
```

### Vertex shader

```
screen_pos = world_position * zoom + offset
clip_x = screen_pos.x / viewport_width * 2.0 - 1.0
clip_y = -(screen_pos.y / viewport_height * 2.0 - 1.0)
```

This maps world coordinates through the camera transform to screen pixels, then normalizes to [-1, 1] clip space. The Y axis is flipped (screen Y increases downward, clip Y increases upward).

### Fragment shader

Pass-through: outputs the interpolated vertex color directly.

## Vertex generation

`build_vertices()` constructs the vertex and index arrays from the current `AppState`:

### Edges

Each edge is drawn as a thin quad (4 vertices, 2 triangles):

```
      P1 -------- P2        (from node bottom-center)
       \          /
        \        /
         \      /
      P4 -------- P3        (to node top-center)
```

The quad width is 2 world-space pixels, computed by offsetting each endpoint perpendicular to the edge direction.

Edge color: `[0.5, 0.5, 0.5, 0.6]` (semi-transparent gray).

### Nodes

Each node is a simple axis-aligned quad:

```
(x, y) ---------- (x+w, y)
  |                    |
  |      NODE          |
  |                    |
(x, y+h) -------- (x+w, y+h)
```

Node color comes from `NodeKind::color()`, with two modifications:

1. **Selected node**: overridden to white `[1, 1, 1, 1]`
2. **Active pulse**: if `now - last_update_time < 2.0`, brightness is increased proportionally

The pulse formula:
```rust
let pulse = (1.0 - age / 2.0) * 0.3;
color.rgb += pulse;  // clamped to 1.0
```

## Per-frame buffer creation

Each frame in `prepare()`:

1. `build_vertices()` generates `Vec<Vertex>` and `Vec<u32>` (indices)
2. A `CameraUniform` is built from the current camera state
3. Three buffers are created via `create_buffer_init()`:
   - Camera uniform buffer (32 bytes)
   - Vertex buffer (24 bytes * vertex_count)
   - Index buffer (4 bytes * index_count)
4. A bind group is created binding the camera buffer

These are stored in `FrameResources` and used during `paint()`.

Creating buffers every frame is intentional — the data changes every frame (camera moves, nodes pulse), and the cost is negligible for typical graph sizes (hundreds to low thousands of vertices).
