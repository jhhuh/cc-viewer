use std::sync::Arc;
use egui::Rect;
use eframe::egui_wgpu;
use wgpu::util::DeviceExt;

use crate::data::types::AppState;
use super::canvas::{self, CameraUniform, CanvasState};
use super::text::GlyphonState;

/// The egui_wgpu paint callback that renders the graph canvas.
#[derive(Clone)]
pub struct CanvasCallback {
    pub state: AppState,
    pub canvas: CanvasState,
    pub rect: Rect,
}

impl egui_wgpu::CallbackTrait for CanvasCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        // Draw geometry (nodes + edges)
        if let Some(resources) = callback_resources.get::<FrameResources>() {
            if resources.index_count > 0 {
                render_pass.set_pipeline(&resources.pipeline);
                render_pass.set_bind_group(0, &resources.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    resources.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..resources.index_count, 0, 0..1);
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
        // Prepare geometry
        let (vertices, indices) = canvas::build_vertices(&self.state);

        let camera_uniform = CameraUniform {
            offset: [self.state.camera.offset_x, self.state.camera.offset_y],
            zoom: self.state.camera.zoom,
            aspect: self.rect.width() / self.rect.height(),
            viewport_size: [self.rect.width(), self.rect.height()],
            _pad: [0.0; 2],
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &self.canvas.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

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
            pipeline: Arc::clone(&self.canvas.pipeline),
            vertex_buffer,
            index_buffer,
            camera_bind_group,
            index_count: indices.len() as u32,
        });

        // Prepare text
        if let Some(glyphon) = callback_resources.get_mut::<GlyphonState>() {
            super::text::prepare_text(
                glyphon,
                device,
                queue,
                &self.state,
                self.rect.width(),
                self.rect.height(),
            );
        }

        Vec::new()
    }
}

struct FrameResources {
    pipeline: Arc<wgpu::RenderPipeline>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    index_count: u32,
}
