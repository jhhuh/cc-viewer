# Camera System

The camera maps between **world space** (where nodes are positioned by the layout algorithm) and **screen space** (pixel coordinates on the viewport).

## State

```rust
pub struct CameraState {
    pub offset_x: f32,  // screen-space X translation
    pub offset_y: f32,  // screen-space Y translation
    pub zoom: f32,       // zoom multiplier (0.05 to 10.0)
}
```

## Coordinate transforms

### World to screen

```
screen_x = world_x * zoom + offset_x
screen_y = world_y * zoom + offset_y
```

This is used in:
- The WGSL vertex shader (for geometry)
- `text.rs` (to position text areas in screen space)
- `state.rs` (for frustum culling in hit testing)

### Screen to world

```
world_x = (screen_x - offset_x) / zoom
world_y = (screen_y - offset_y) / zoom
```

This is used for:
- Hit testing (converting click position to world coordinates)
- Zoom-toward-cursor (finding the world point under the mouse)

## Zoom-toward-cursor

When the user scrolls, the zoom level changes. To keep the world point under the cursor stationary:

```rust
// World point under cursor before zoom
let wx = (canvas_x - offset_x) / old_zoom;
let wy = (canvas_y - offset_y) / old_zoom;

// Apply new zoom
zoom = new_zoom;

// Recalculate offset so the same world point maps to the same screen point
offset_x = canvas_x - wx * new_zoom;
offset_y = canvas_y - wy * new_zoom;
```

This creates the natural "zoom into what you're looking at" behavior.

## Panning

Pan is implemented as a direct offset adjustment:

```rust
offset_x += drag_delta.x;
offset_y += drag_delta.y;
```

No world-space conversion needed — pan operates purely in screen space.

## Click-to-zoom animation

When the user clicks a node, a `ZoomTarget` is created:

```rust
ZoomTarget {
    target_x: node_center_x,  // world X
    target_y: node_center_y,  // world Y
    target_zoom: 3.0,         // zoom to 3x
    progress: 0.0,            // animation progress [0, 1]
}
```

Each frame, the animation advances:

```rust
progress += 0.05;  // ~20 frames to complete
let t = ease_out(progress);

// Target offset: center the node on screen at target zoom
let target_offset_x = viewport_w / 2.0 - target_x * target_zoom;
let target_offset_y = viewport_h / 2.0 - target_y * target_zoom;

// Interpolate toward target
offset_x += (target_offset_x - offset_x) * t * 0.1;
offset_y += (target_offset_y - offset_y) * t * 0.1;
zoom     += (target_zoom - zoom)         * t * 0.1;
```

The `ease_out` function provides deceleration:

```rust
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)  // cubic ease-out
}
```

The animation completes when `progress >= 1.0`, at which point `zoom_target` is set to `None`.

## Shader integration

The camera state is uploaded to the GPU as a uniform buffer each frame:

```rust
struct CameraUniform {
    offset: [f32; 2],       // pan offset
    zoom: f32,              // zoom level
    aspect: f32,            // viewport aspect ratio
    viewport_size: [f32; 2], // viewport dimensions in pixels
    _pad: [f32; 2],         // alignment padding
}
```

The vertex shader uses this to transform world positions to clip space:

```wgsl
let screen_pos = position * camera.zoom + camera.offset;
let clip_x = screen_pos.x / camera.viewport_size.x * 2.0 - 1.0;
let clip_y = -(screen_pos.y / camera.viewport_size.y * 2.0 - 1.0);
```
