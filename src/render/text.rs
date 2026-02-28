use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
};

use crate::data::types::RenderSnapshot;

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
    is_terminal: bool,
    is_expanded: bool,
}

/// Prepare text areas for all visible nodes.
pub fn prepare_text(
    glyphon: &mut GlyphonState,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    snapshot: &RenderSnapshot,
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

    if snapshot.nodes.is_empty() {
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

    let zoom = snapshot.camera.zoom;
    let offset_x = snapshot.camera.offset_x;
    let offset_y = snapshot.camera.offset_y;

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

    let base_font_size = 18.0;
    let font_size = (base_font_size * zoom).clamp(2.0, 80.0);
    let line_height = font_size * 1.3;
    let metrics = Metrics::new(font_size, line_height);

    let term_font_size = (13.0 * zoom).clamp(2.0, 50.0);
    let term_line_height = term_font_size * 1.2;
    let term_metrics = Metrics::new(term_font_size, term_line_height);

    // Phase 1: Collect info for visible nodes
    let mut infos: Vec<TextAreaInfo> = Vec::new();

    for node in &snapshot.nodes {
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

        let tc = node.text_color;
        infos.push(TextAreaInfo {
            node_id: node.id.clone(),
            text: node.label.clone(),
            screen_x,
            screen_y,
            screen_w,
            screen_h,
            node_w: node.w,
            node_h: node.h,
            text_color: Color::rgba(tc[0], tc[1], tc[2], tc[3]),
            is_terminal: node.is_terminal,
            is_expanded: node.is_expanded,
        });
    }

    // Phase 2: Rebuild buffer list to match visible nodes
    let mut new_buffers: Vec<(String, Buffer)> = Vec::with_capacity(infos.len());

    for info in &infos {
        let is_body = info.is_expanded || info.is_terminal;
        let m = if is_body { term_metrics } else { metrics };

        // Try to reuse existing buffer
        let pos = glyphon.buffers.iter().position(|(id, _)| id == &info.node_id);
        let mut buffer = if let Some(pos) = pos {
            glyphon.buffers.swap_remove(pos).1
        } else {
            Buffer::new(&mut glyphon.font_system, m)
        };

        buffer.set_metrics(&mut glyphon.font_system, m);

        let padding = if info.is_terminal { 6.0 } else { 8.0 };
        buffer.set_size(
            &mut glyphon.font_system,
            Some((info.node_w - padding).max(10.0)),
            Some((info.node_h - padding).max(10.0)),
        );

        if info.is_expanded {
            // Rich text: bold sans-serif title + monospace body
            let (title, body) = match info.text.split_once('\n') {
                Some((t, b)) => (t, b),
                None => (info.text.as_str(), ""),
            };
            let title_attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::BOLD);
            let body_attrs = Attrs::new().family(Family::Monospace);

            let mut spans: Vec<(&str, Attrs)> = vec![(title, title_attrs)];
            if !body.is_empty() {
                spans.push(("\n", body_attrs));
                spans.push((body, body_attrs));
            }
            buffer.set_rich_text(
                &mut glyphon.font_system,
                spans,
                Attrs::new().family(Family::Monospace),
                Shaping::Advanced,
            );
        } else {
            // Collapsed: bold sans-serif label
            let attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::BOLD);
            buffer.set_text(&mut glyphon.font_system, &info.text, attrs, Shaping::Advanced);
        }

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
