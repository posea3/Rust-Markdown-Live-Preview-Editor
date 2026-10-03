use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}

impl ScreenRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
        }
    }
}

pub struct RectBatch {
    buffer: wgpu::Buffer,
    vertex_count: u32,
}

pub struct RectRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl RectRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("rect.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mdedit rect pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let vertex_buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: (6 * size_of::<f32>()) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: (2 * size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        })];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mdedit rect pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn prepare(
        &self,
        device: &wgpu::Device,
        rects: &[ScreenRect],
        viewport_width: u32,
        viewport_height: u32,
        clip: ScreenRect,
    ) -> Option<RectBatch> {
        if rects.is_empty() || viewport_width == 0 || viewport_height == 0 {
            return None;
        }

        let mut bytes = Vec::with_capacity(rects.len() * 6 * 6 * size_of::<f32>());
        let mut vertex_count = 0_u32;

        for rect in rects {
            let Some(rect) = clip_rect(*rect, clip) else {
                continue;
            };
            append_rect(
                &mut bytes,
                rect,
                viewport_width as f32,
                viewport_height as f32,
            );
            vertex_count += 6;
        }

        if vertex_count == 0 {
            return None;
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mdedit rect vertex buffer"),
            contents: &bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });

        Some(RectBatch {
            buffer,
            vertex_count,
        })
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a RectBatch) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, batch.buffer.slice(..));
        pass.draw(0..batch.vertex_count, 0..1);
    }
}

fn clip_rect(rect: ScreenRect, clip: ScreenRect) -> Option<ScreenRect> {
    let left = rect.x.max(clip.x);
    let top = rect.y.max(clip.y);
    let right = (rect.x + rect.width).min(clip.x + clip.width);
    let bottom = (rect.y + rect.height).min(clip.y + clip.height);

    (right > left && bottom > top)
        .then(|| ScreenRect::new(left, top, right - left, bottom - top, rect.color))
}

fn append_rect(bytes: &mut Vec<u8>, rect: ScreenRect, width: f32, height: f32) {
    let left = pixel_to_ndc_x(rect.x, width);
    let right = pixel_to_ndc_x(rect.x + rect.width, width);
    let top = pixel_to_ndc_y(rect.y, height);
    let bottom = pixel_to_ndc_y(rect.y + rect.height, height);

    for [x, y] in [
        [left, top],
        [right, top],
        [right, bottom],
        [left, top],
        [right, bottom],
        [left, bottom],
    ] {
        append_vertex(bytes, [x, y], rect.color);
    }
}

fn append_vertex(bytes: &mut Vec<u8>, position: [f32; 2], color: [f32; 4]) {
    for value in position.into_iter().chain(color) {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

fn pixel_to_ndc_x(x: f32, width: f32) -> f32 {
    (x / width) * 2.0 - 1.0
}

fn pixel_to_ndc_y(y: f32, height: f32) -> f32 {
    1.0 - (y / height) * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipping_reduces_rect_to_text_bounds() {
        let color = [1.0; 4];
        let rect = ScreenRect::new(0.0, 5.0, 20.0, 20.0, color);
        let clip = ScreenRect::new(10.0, 10.0, 50.0, 50.0, color);

        let clipped = clip_rect(rect, clip).unwrap();

        assert_eq!(clipped.x, 10.0);
        assert_eq!(clipped.y, 10.0);
        assert_eq!(clipped.width, 10.0);
        assert_eq!(clipped.height, 15.0);
    }

    #[test]
    fn fully_outside_rect_is_discarded() {
        let color = [1.0; 4];
        let rect = ScreenRect::new(0.0, 0.0, 5.0, 5.0, color);
        let clip = ScreenRect::new(10.0, 10.0, 50.0, 50.0, color);

        assert!(clip_rect(rect, clip).is_none());
    }
}
