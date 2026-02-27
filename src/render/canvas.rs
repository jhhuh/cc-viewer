use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

use crate::data::types::*;

/// GPU state for rendering the graph canvas.
#[derive(Clone)]
pub struct CanvasState {
    pub pipeline: Arc<wgpu::RenderPipeline>,
    pub camera_bind_group_layout: Arc<wgpu::BindGroupLayout>,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub offset: [f32; 2],
    pub zoom: f32,
    pub aspect: f32,
    pub viewport_size: [f32; 2],
    pub _pad: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub rect_uv: [f32; 2],
    pub half_size: [f32; 2],
}

impl CanvasState {
    pub fn new(render_state: &eframe::egui_wgpu::RenderState) -> Self {
        let device = &render_state.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("canvas_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,  // position
                        1 => Float32x4,  // color
                        2 => Float32x2,  // rect_uv
                        3 => Float32x2,  // half_size
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_state.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline: Arc::new(pipeline),
            camera_bind_group_layout: Arc::new(camera_bind_group_layout),
        }
    }
}

/// Build vertex data for all nodes and edges from the render snapshot.
pub fn build_vertices(snapshot: &RenderSnapshot) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    // Draw edges as curved quads (bezier-approximated)
    for edge in &snapshot.edges {
        let edge_color = [0.5, 0.5, 0.5, 0.5];
        let thickness = 2.0;
        let zero_hs = [0.0f32, 0.0];

        // Bezier: start -> control -> end
        let x1 = edge.x1;
        let y1 = edge.y1;
        let x2 = edge.x2;
        let y2 = edge.y2;
        let cx = (x1 + x2) / 2.0;
        let cy = (y1 + y2) / 2.0;
        // Offset control point for curvature
        let dx = x2 - x1;
        let ctrl_x = cx + (y2 - y1) * 0.1;
        let ctrl_y = cy - dx * 0.1;

        // Tessellate bezier into segments
        let segments = 8u32;
        for s in 0..segments {
            let t0 = s as f32 / segments as f32;
            let t1 = (s + 1) as f32 / segments as f32;

            let px0 = bezier(x1, ctrl_x, x2, t0);
            let py0 = bezier(y1, ctrl_y, y2, t0);
            let px1 = bezier(x1, ctrl_x, x2, t1);
            let py1 = bezier(y1, ctrl_y, y2, t1);

            let sdx = px1 - px0;
            let sdy = py1 - py0;
            let len = (sdx * sdx + sdy * sdy).sqrt().max(0.001);
            let nx = -sdy / len * thickness / 2.0;
            let ny = sdx / len * thickness / 2.0;

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [px0 + nx, py0 + ny], color: edge_color, rect_uv: [0.5, 0.5], half_size: zero_hs });
            vertices.push(Vertex { position: [px0 - nx, py0 - ny], color: edge_color, rect_uv: [0.5, 0.5], half_size: zero_hs });
            vertices.push(Vertex { position: [px1 - nx, py1 - ny], color: edge_color, rect_uv: [0.5, 0.5], half_size: zero_hs });
            vertices.push(Vertex { position: [px1 + nx, py1 + ny], color: edge_color, rect_uv: [0.5, 0.5], half_size: zero_hs });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    // Draw nodes as rounded rects
    for node in &snapshot.nodes {
        let mut color = node.color;

        if node.is_selected {
            color = [1.0, 1.0, 1.0, 1.0];
        }

        // Pulse recently active nodes
        let age = now - node.last_update_time;
        if age < 2.0 {
            let pulse = (1.0 - age as f32 / 2.0) * 0.3;
            color[0] = (color[0] + pulse).min(1.0);
            color[1] = (color[1] + pulse).min(1.0);
            color[2] = (color[2] + pulse).min(1.0);
        }

        let hw = node.w / 2.0;
        let hh = node.h / 2.0;
        let hs = [hw, hh];

        let base = vertices.len() as u32;
        vertices.push(Vertex { position: [node.x, node.y], color, rect_uv: [0.0, 0.0], half_size: hs });
        vertices.push(Vertex { position: [node.x + node.w, node.y], color, rect_uv: [1.0, 0.0], half_size: hs });
        vertices.push(Vertex { position: [node.x + node.w, node.y + node.h], color, rect_uv: [1.0, 1.0], half_size: hs });
        vertices.push(Vertex { position: [node.x, node.y + node.h], color, rect_uv: [0.0, 1.0], half_size: hs });
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
}

fn bezier(a: f32, b: f32, c: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    u * u * a + 2.0 * u * t * b + t * t * c
}

const SHADER_SRC: &str = r#"
struct Camera {
    offset: vec2<f32>,
    zoom: f32,
    aspect: f32,
    viewport_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) rect_uv: vec2<f32>,
    @location(3) half_size: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) rect_uv: vec2<f32>,
    @location(2) half_size: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let screen_pos = in.position * camera.zoom + camera.offset;
    let clip_x = screen_pos.x / camera.viewport_size.x * 2.0 - 1.0;
    let clip_y = -(screen_pos.y / camera.viewport_size.y * 2.0 - 1.0);
    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.color = in.color;
    out.rect_uv = in.rect_uv;
    out.half_size = in.half_size;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Screen-space half-size
    let hs = in.half_size * camera.zoom;

    // For edges (half_size ~ 0): pass through color directly
    if (hs.x < 1.0 || hs.y < 1.0) {
        return in.color;
    }

    // Rounded rectangle SDF
    let p = (in.rect_uv - 0.5) * 2.0 * hs;
    let r = min(10.0, min(hs.x, hs.y) * 0.3);
    let q = abs(p) - (hs - r);
    let d = length(max(q, vec2<f32>(0.0, 0.0))) - r;
    let alpha = 1.0 - smoothstep(-1.0, 1.0, d);

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;
