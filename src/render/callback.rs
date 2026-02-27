use egui::Rect;
use eframe::egui_wgpu;
use wgpu::util::DeviceExt;

use crate::data::types::RenderSnapshot;
use super::canvas::{self, CameraUniform, CanvasState};
use super::text::GlyphonState;

/// Persistent GPU resources (pipeline, camera buffer) — created once.
pub struct PersistentGpuResources {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl PersistentGpuResources {
    pub fn init(render_state: &eframe::egui_wgpu::RenderState) {
        let device = &render_state.device;
        let canvas = CanvasState::new(render_state);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &canvas.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let resources = PersistentGpuResources {
            pipeline: std::sync::Arc::try_unwrap(canvas.pipeline)
                .unwrap_or_else(|arc| (*arc).clone()),
            camera_buffer,
            camera_bind_group,
        };

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);
    }
}

/// Per-frame vertex/index data.
struct FrameResources {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// The egui_wgpu paint callback that renders the graph canvas.
#[derive(Clone)]
pub struct CanvasCallback {
    pub snapshot: RenderSnapshot,
    pub rect: Rect,
}

impl egui_wgpu::CallbackTrait for CanvasCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let persistent = callback_resources.get::<PersistentGpuResources>();
        let frame = callback_resources.get::<FrameResources>();

        if let (Some(p), Some(f)) = (persistent, frame) {
            if f.index_count > 0 {
                render_pass.set_pipeline(&p.pipeline);
                render_pass.set_bind_group(0, &p.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, f.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    f.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..f.index_count, 0, 0..1);
            }
        }

        // Draw text on top
        if let Some(glyphon) = callback_resources.get::<GlyphonState>() {
            super::text::render_text(glyphon, render_pass);
        }
    }

    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        // Update camera uniform via write_buffer (no allocation)
        let camera_uniform = CameraUniform {
            offset: [self.snapshot.camera.offset_x, self.snapshot.camera.offset_y],
            zoom: self.snapshot.camera.zoom,
            aspect: self.rect.width() / self.rect.height(),
            viewport_size: [self.rect.width(), self.rect.height()],
            _pad: [0.0; 2],
        };

        if let Some(persistent) = callback_resources.get::<PersistentGpuResources>() {
            queue.write_buffer(
                &persistent.camera_buffer,
                0,
                bytemuck::cast_slice(&[camera_uniform]),
            );
        }

        // Build geometry
        let (vertices, indices) = canvas::build_vertices(&self.snapshot);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        callback_resources.insert(FrameResources {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        });

        // Prepare text
        if let Some(glyphon) = callback_resources.get_mut::<GlyphonState>() {
            super::text::prepare_text(
                glyphon,
                device,
                queue,
                &self.snapshot,
                self.rect.width(),
                self.rect.height(),
            );
        }

        Vec::new()
    }
}
