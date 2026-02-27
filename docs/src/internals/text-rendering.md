# Text Rendering

cc-viewer uses [glyphon](https://github.com/grovesNL/glyphon) for GPU-accelerated text rendering. glyphon rasterizes glyphs into a texture atlas and renders them as instanced quads in the wgpu render pass.

## Why glyphon instead of egui text?

egui rasterizes text at the current DPI and renders it as textured quads. When you zoom the wgpu canvas, egui text doesn't re-rasterize — it scales the existing texture, causing pixelation.

glyphon renders directly into the wgpu render pass at the correct font size for the current zoom level. Text is always crisp, from 0.15x zoom (minimap) to 10x zoom (close-up).

## Persistent state

`GlyphonState` is stored in egui's `CallbackResources` (a type-map) and persists across frames:

```rust
pub struct GlyphonState {
    font_system: FontSystem,     // font database (cosmic-text)
    swash_cache: SwashCache,     // glyph rasterization cache
    cache: Cache,                // GPU resource cache
    atlas: TextAtlas,            // glyph texture atlas
    viewport: Viewport,          // screen resolution
    text_renderer: TextRenderer, // the rendering pipeline
    buffers: Vec<(String, Buffer)>,  // per-node text buffers
}
```

This is initialized once in `App::new()` via `GlyphonState::init()`.

## Per-frame text preparation

`prepare_text()` runs during the callback's `prepare()` phase, before the render pass:

### Phase 1: Collect visible nodes

For each node in the active session graph:
1. Transform world position to screen position using the camera
2. **Frustum cull**: skip nodes entirely outside the viewport
3. **Zoom threshold**: skip all text if zoom < 0.15 (too small to read)
4. Collect text content, position, and color info into `TextAreaInfo` structs

### Phase 2: Update text buffers

For each visible node:
1. Try to reuse an existing buffer (matched by node ID)
2. If no buffer exists, create a new one
3. Update metrics (font size scales with zoom: `14.0 * zoom`, clamped to 2..80 px)
4. Set text content and trigger shaping

Buffers from nodes that are no longer visible are dropped when the list is rebuilt.

### Phase 3: Build TextAreas

Create a `TextArea` for each buffer, specifying:
- **Screen position**: `(screen_x + 4, screen_y + 2)` — 4px left padding, 2px top padding
- **Bounds**: clipped to the node's screen rectangle and the viewport edges
- **Color**: determined by `text_color_for_kind()` — light variants for contrast on dark node backgrounds

### Phase 4: Render preparation

Call `text_renderer.prepare()` with all TextAreas. This uploads glyph instances to the GPU.

Then `atlas.trim()` evicts unused glyphs from the texture atlas.

## Rendering

In the `paint()` phase, `render_text()` calls `text_renderer.render()` which:
1. Sets its own internal pipeline
2. Binds the atlas texture and viewport uniform
3. Draws all glyph instances in a single draw call

This happens **after** our geometry draw calls in the same render pass, so text renders on top of node rectangles.

## Text content

`format_node_label()` builds the text shown on each node:

```
{label}
{content_summary}   (max 80 chars)
```

For example:
```
Tool: Bash
[tool: Bash] | cargo build
```

## Text colors

| NodeKind | Text Color |
|----------|-----------|
| User | light blue `rgb(220, 230, 255)` |
| Assistant | light green `rgb(220, 255, 220)` |
| ToolUse | light orange `rgb(255, 240, 200)` |
| ToolResult | light tan `rgb(255, 230, 200)` |
| Progress | light gray `rgb(200, 200, 200)` |
| Subagent | light purple `rgb(240, 210, 240)` |

## Font

glyphon uses the system font database via `FontSystem::new()`. Text is rendered in the system's default monospace font (`Family::Monospace`).
