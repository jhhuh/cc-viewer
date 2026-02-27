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
                    visibility: wgpu::ShaderStages::VERTEX,
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
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
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

/// Build vertex data for all nodes and edges in the active session.
pub fn build_vertices(state: &AppState) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let graph = match state.active_session.as_ref().and_then(|s| state.sessions.get(s)) {
        Some(g) => g,
        None => return (vertices, indices),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    // Draw edges first (behind nodes)
    for edge in &graph.edges {
        let from_idx = graph.node_index.get(&edge.from);
        let to_idx = graph.node_index.get(&edge.to);
        if let (Some(&fi), Some(&ti)) = (from_idx, to_idx) {
            let from = &graph.nodes[fi];
            let to = &graph.nodes[ti];

            let x1 = from.x + from.w / 2.0;
            let y1 = from.y + from.h;
            let x2 = to.x + to.w / 2.0;
            let y2 = to.y;

            let edge_color = [0.5, 0.5, 0.5, 0.6];
            let thickness = 2.0;

            // Line as thin quad
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let nx = -dy / len * thickness / 2.0;
            let ny = dx / len * thickness / 2.0;

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [x1 + nx, y1 + ny], color: edge_color });
            vertices.push(Vertex { position: [x1 - nx, y1 - ny], color: edge_color });
            vertices.push(Vertex { position: [x2 - nx, y2 - ny], color: edge_color });
            vertices.push(Vertex { position: [x2 + nx, y2 + ny], color: edge_color });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    // Draw nodes
    for node in &graph.nodes {
        let mut color = node.kind.color();

        // Highlight selected node
        if state.selected_node.as_ref() == Some(&node.id) {
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

        let base = vertices.len() as u32;
        vertices.push(Vertex { position: [node.x, node.y], color });
        vertices.push(Vertex { position: [node.x + node.w, node.y], color });
        vertices.push(Vertex { position: [node.x + node.w, node.y + node.h], color });
        vertices.push(Vertex { position: [node.x, node.y + node.h], color });
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
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
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // World -> screen: apply zoom and offset, then normalize to clip space
    let screen_pos = in.position * camera.zoom + camera.offset;
    let clip_x = screen_pos.x / camera.viewport_size.x * 2.0 - 1.0;
    let clip_y = -(screen_pos.y / camera.viewport_size.y * 2.0 - 1.0);
    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
