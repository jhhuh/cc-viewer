use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

use crate::data::types::{AppState, GraphNode, NodeKind};

/// Persistent glyphon state stored in egui_wgpu's CallbackResources.
pub struct GlyphonState {
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub cache: Cache,
    pub atlas: TextAtlas,
    pub viewport: Viewport,
    pub text_renderer: TextRenderer,
    /// One text buffer per node, keyed by node id.
    pub buffers: Vec<(String, Buffer)>,
}

impl GlyphonState {
    pub fn init(render_state: &eframe::egui_wgpu::RenderState) {
        let device = &render_state.device;
        let queue = &render_state.queue;
        let format = render_state.target_format;

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        let state = GlyphonState {
            font_system,
            swash_cache,
            cache,
            atlas,
            viewport,
            text_renderer,
            buffers: Vec::new(),
        };

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(state);
    }
}

/// Info needed to build a TextArea, collected before borrowing buffers.
struct TextAreaInfo {
    node_id: String,
    text: String,
    screen_x: f32,
    screen_y: f32,
    screen_w: f32,
    screen_h: f32,
    node_w: f32,
    node_h: f32,
    text_color: Color,
}

/// Prepare text areas for all visible nodes.
pub fn prepare_text(
    glyphon: &mut GlyphonState,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    app_state: &AppState,
    viewport_w: f32,
    viewport_h: f32,
) {
    glyphon.viewport.update(
        queue,
        Resolution {
            width: viewport_w as u32,
            height: viewport_h as u32,
        },
    );

    let graph = match app_state
        .active_session
        .as_ref()
        .and_then(|s| app_state.sessions.get(s))
    {
        Some(g) => g,
        None => {
            glyphon.buffers.clear();
            return;
        }
    };

    let zoom = app_state.camera.zoom;
    let offset_x = app_state.camera.offset_x;
    let offset_y = app_state.camera.offset_y;

    // Skip text at very small zoom
    if zoom < 0.15 {
        glyphon.buffers.clear();
        let _ = glyphon.text_renderer.prepare(
            device,
            queue,
            &mut glyphon.font_system,
            &mut glyphon.atlas,
            &glyphon.viewport,
            Vec::<TextArea>::new(),
            &mut glyphon.swash_cache,
        );
        return;
    }

    let base_font_size = 14.0;
    let font_size = (base_font_size * zoom).clamp(2.0, 80.0);
    let line_height = font_size * 1.3;
    let metrics = Metrics::new(font_size, line_height);

    // Phase 1: Collect info for visible nodes
    let mut infos: Vec<TextAreaInfo> = Vec::new();

    for node in &graph.nodes {
        let screen_x = node.x * zoom + offset_x;
        let screen_y = node.y * zoom + offset_y;
        let screen_w = node.w * zoom;
        let screen_h = node.h * zoom;

        // Frustum cull
        if screen_x + screen_w < 0.0
            || screen_x > viewport_w
            || screen_y + screen_h < 0.0
            || screen_y > viewport_h
        {
            continue;
        }

        infos.push(TextAreaInfo {
            node_id: node.id.clone(),
            text: format_node_label(node),
            screen_x,
            screen_y,
            screen_w,
            screen_h,
            node_w: node.w,
            node_h: node.h,
            text_color: text_color_for_kind(node.kind),
        });
    }

    // Phase 2: Rebuild buffer list to match visible nodes
    // Keep buffers in sync with infos
    let mut new_buffers: Vec<(String, Buffer)> = Vec::with_capacity(infos.len());

    for info in &infos {
        // Try to reuse existing buffer
        let pos = glyphon.buffers.iter().position(|(id, _)| id == &info.node_id);
        let mut buffer = if let Some(pos) = pos {
            glyphon.buffers.swap_remove(pos).1
        } else {
            Buffer::new(&mut glyphon.font_system, metrics)
        };

        buffer.set_metrics(&mut glyphon.font_system, metrics);
        buffer.set_size(
            &mut glyphon.font_system,
            Some((info.node_w - 8.0).max(10.0)),
            Some((info.node_h - 4.0).max(10.0)),
        );

        let attrs = Attrs::new().family(Family::Monospace);
        buffer.set_text(&mut glyphon.font_system, &info.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut glyphon.font_system, false);

        new_buffers.push((info.node_id.clone(), buffer));
    }

    glyphon.buffers = new_buffers;

    // Phase 3: Build TextAreas referencing the buffers
    let text_areas: Vec<TextArea> = infos
        .iter()
        .zip(glyphon.buffers.iter())
        .map(|(info, (_id, buffer))| TextArea {
            buffer,
            left: info.screen_x + 4.0,
            top: info.screen_y + 2.0,
            scale: 1.0,
            bounds: TextBounds {
                left: info.screen_x.max(0.0) as i32,
                top: info.screen_y.max(0.0) as i32,
                right: (info.screen_x + info.screen_w).min(viewport_w) as i32,
                bottom: (info.screen_y + info.screen_h).min(viewport_h) as i32,
            },
            default_color: info.text_color,
            custom_glyphs: &[],
        })
        .collect();

    let _ = glyphon.text_renderer.prepare(
        device,
        queue,
        &mut glyphon.font_system,
        &mut glyphon.atlas,
        &glyphon.viewport,
        text_areas,
        &mut glyphon.swash_cache,
    );

    glyphon.atlas.trim();
}

/// Render text into the current render pass.
pub fn render_text(
    glyphon: &GlyphonState,
    render_pass: &mut wgpu::RenderPass<'static>,
) {
    let _ = glyphon
        .text_renderer
        .render(&glyphon.atlas, &glyphon.viewport, render_pass);
}

fn format_node_label(node: &GraphNode) -> String {
    let label = &node.label;
    if node.content_summary.is_empty() {
        label.clone()
    } else {
        let summary = if node.content_summary.len() > 80 {
            let end = node.content_summary.floor_char_boundary(80);
            format!("{}...", &node.content_summary[..end])
        } else {
            node.content_summary.clone()
        };
        format!("{}\n{}", label, summary)
    }
}

fn text_color_for_kind(kind: NodeKind) -> Color {
    match kind {
        NodeKind::User => Color::rgb(220, 230, 255),
        NodeKind::Assistant => Color::rgb(220, 255, 220),
        NodeKind::ToolUse => Color::rgb(255, 240, 200),
        NodeKind::ToolResult => Color::rgb(255, 230, 200),
        NodeKind::Progress => Color::rgb(200, 200, 200),
        NodeKind::Subagent => Color::rgb(240, 210, 240),
        NodeKind::Other => Color::rgb(200, 200, 200),
    }
}
