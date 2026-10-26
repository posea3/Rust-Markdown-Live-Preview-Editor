use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use image::ImageDecoder;
use wgpu::util::DeviceExt;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    ColorTargetState, ColorWrites, Device, Extent3d, FilterMode, FragmentState, MipmapFilterMode,
    MultisampleState, Origin3d, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderStages, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};
use winit::window::Window;

use crate::geometry::ScreenRect;

const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageHostState {
    Missing,
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Debug)]
pub struct ImageDrawRequest {
    destination: String,
    rect: ScreenRect,
}

impl ImageDrawRequest {
    pub fn new(destination: String, rect: ScreenRect) -> Self {
        Self { destination, rect }
    }
}

pub struct ImageDrawBatch {
    destination: String,
    buffer: wgpu::Buffer,
    vertex_count: u32,
}

pub struct ImageWidgetHost {
    base_dir: PathBuf,
    pending: HashSet<String>,
    failed: HashMap<String, String>,
    sender: Sender<DecodedImage>,
    receiver: Receiver<DecodedImage>,
    renderer: ImageRenderer,
    wake_window: Arc<Window>,
    ready_delay: Duration,
    max_texture_dimension: u32,
}

impl ImageWidgetHost {
    pub fn new(
        device: &Device,
        surface_format: TextureFormat,
        wake_window: Arc<Window>,
        base_dir: PathBuf,
        ready_delay: Duration,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            base_dir,
            pending: HashSet::new(),
            failed: HashMap::new(),
            sender,
            receiver,
            renderer: ImageRenderer::new(device, surface_format),
            wake_window,
            ready_delay,
            max_texture_dimension: device.limits().max_texture_dimension_2d,
        }
    }

    pub fn request(&mut self, destination: &str) {
        if destination.is_empty()
            || self.pending.contains(destination)
            || self.failed.contains_key(destination)
            || self.renderer.contains(destination)
        {
            return;
        }

        self.pending.insert(destination.to_owned());

        let sender = self.sender.clone();
        let base_dir = self.base_dir.clone();
        let destination = destination.to_owned();
        let wake_window = self.wake_window.clone();
        let ready_delay = self.ready_delay;
        let max_texture_dimension = self.max_texture_dimension;
        thread::spawn(move || {
            let result = decode_local_image(&base_dir, &destination, max_texture_dimension);
            let message = match result {
                Ok((width, height, rgba)) => {
                    if !ready_delay.is_zero() {
                        thread::sleep(ready_delay);
                    }
                    DecodedImage::Ready {
                        destination,
                        width,
                        height,
                        rgba,
                    }
                }
                Err(error) => DecodedImage::Failed { destination, error },
            };
            let _ = sender.send(message);
            wake_window.request_redraw();
        });
    }

    pub fn drain_ready(&mut self, device: &Device, queue: &Queue) {
        while let Ok(decoded) = self.receiver.try_recv() {
            match decoded {
                DecodedImage::Ready {
                    destination,
                    width,
                    height,
                    rgba,
                } => {
                    self.pending.remove(&destination);
                    match self.renderer.upload(
                        device,
                        queue,
                        destination.clone(),
                        width,
                        height,
                        &rgba,
                    ) {
                        Ok(()) => {
                            self.failed.remove(&destination);
                        }
                        Err(error) => {
                            eprintln!(
                                "[mdedit-ime-lab] image widget upload failed {destination:?}: {error}"
                            );
                            self.failed.insert(destination, error);
                        }
                    }
                }
                DecodedImage::Failed { destination, error } => {
                    self.pending.remove(&destination);
                    eprintln!(
                        "[mdedit-ime-lab] image widget could not resolve {destination:?}: {error}"
                    );
                    self.failed.insert(destination, error);
                }
            }
        }
    }

    pub fn state(&self, destination: &str) -> ImageHostState {
        if self.renderer.contains(destination) {
            ImageHostState::Ready
        } else if self.pending.contains(destination) {
            ImageHostState::Loading
        } else if self.failed.contains_key(destination) {
            ImageHostState::Failed
        } else {
            ImageHostState::Missing
        }
    }

    pub fn contains_texture(&self, destination: &str) -> bool {
        self.renderer.contains(destination)
    }

    pub fn intrinsic_size(&self, destination: &str) -> Option<(u32, u32)> {
        self.renderer.intrinsic_size(destination)
    }

    pub fn upload_rgba(
        &mut self,
        device: &Device,
        queue: &Queue,
        destination: String,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        self.renderer
            .upload(device, queue, destination, width, height, rgba)
    }

    pub fn prepare(
        &self,
        device: &Device,
        draws: &[ImageDrawRequest],
        viewport_width: u32,
        viewport_height: u32,
    ) -> Vec<ImageDrawBatch> {
        self.renderer
            .prepare(device, draws, viewport_width, viewport_height)
    }

    pub fn render<'a>(&'a self, pass: &mut RenderPass<'a>, batches: &'a [ImageDrawBatch]) {
        self.renderer.render(pass, batches);
    }
}

enum DecodedImage {
    Ready {
        destination: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Failed {
        destination: String,
        error: String,
    },
}

fn decode_local_image(
    base_dir: &Path,
    destination: &str,
    max_dimension: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    if destination.starts_with("http://")
        || destination.starts_with("https://")
        || destination.starts_with("data:")
        || destination.starts_with("file:")
    {
        return Err(
            "remote/data/file URI loading is intentionally delegated to the host".to_owned(),
        );
    }

    let relative = Path::new(destination);
    if relative.is_absolute() {
        return Err("absolute image paths are not allowed by the IME lab resolver".to_owned());
    }

    let base = fs::canonicalize(base_dir)
        .map_err(|error| format!("image base directory is unavailable: {error}"))?;
    let candidate = fs::canonicalize(base.join(relative))
        .map_err(|error| format!("image path is unavailable: {error}"))?;

    if !candidate.starts_with(&base) {
        return Err("image path escapes the configured base directory".to_owned());
    }

    let encoded_len = fs::metadata(&candidate)
        .map_err(|error| format!("image metadata failed: {error}"))?
        .len();
    if encoded_len > MAX_IMAGE_BYTES {
        return Err("encoded image exceeds the 64 MiB resource budget".to_owned());
    }
    let mut reader = image::ImageReader::open(&candidate)
        .map_err(|error| format!("image read failed: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("image format failed: {error}"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(max_dimension);
    limits.max_image_height = Some(max_dimension);
    limits.max_alloc = Some(MAX_IMAGE_BYTES);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|error| format!("image decoder failed: {error}"))?;
    let (width, height) = decoder.dimensions();
    rgba_byte_len(width, height, max_dimension)?;
    if decoder.total_bytes() > MAX_IMAGE_BYTES {
        return Err("decoded image exceeds the 64 MiB resource budget".to_owned());
    }
    let rgba = image::DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("image decode failed: {error}"))?
        .to_rgba8();

    Ok((width, height, rgba.into_raw()))
}

fn rgba_byte_len(width: u32, height: u32, max_dimension: u32) -> Result<usize, String> {
    if width == 0 || height == 0 {
        return Err("image dimensions must be non-zero".to_owned());
    }
    if width > max_dimension || height > max_dimension {
        return Err(format!(
            "image dimensions {width}x{height} exceed GPU limit {max_dimension}"
        ));
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "image byte length overflow".to_owned())?;
    if bytes > MAX_IMAGE_BYTES {
        return Err("RGBA image exceeds the 64 MiB resource budget".to_owned());
    }
    usize::try_from(bytes).map_err(|_| "image byte length overflow".to_owned())
}

fn validate_rgba_upload(
    width: u32,
    height: u32,
    rgba_len: usize,
    max_dimension: u32,
) -> Result<(), String> {
    let expected = rgba_byte_len(width, height, max_dimension)?;
    if rgba_len != expected {
        return Err(format!(
            "RGBA buffer length {rgba_len} does not match expected {expected}"
        ));
    }
    Ok(())
}

struct GpuImage {
    _texture: wgpu::Texture,
    bind_group: BindGroup,
    width: u32,
    height: u32,
}

struct ImageRenderer {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    images: HashMap<String, GpuImage>,
}

impl ImageRenderer {
    fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("mdedit image bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("mdedit image pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("image.wgsl"));
        let vertex_buffers = [Some(VertexBufferLayout {
            array_stride: (4 * size_of::<f32>()) as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: (2 * size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        })];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("mdedit image pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("mdedit image sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            images: HashMap::new(),
        }
    }

    fn contains(&self, destination: &str) -> bool {
        self.images.contains_key(destination)
    }

    fn intrinsic_size(&self, destination: &str) -> Option<(u32, u32)> {
        self.images
            .get(destination)
            .map(|image| (image.width, image.height))
    }

    fn upload(
        &mut self,
        device: &Device,
        queue: &Queue,
        destination: String,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        validate_rgba_upload(
            width,
            height,
            rgba.len(),
            device.limits().max_texture_dimension_2d,
        )?;
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("mdedit image widget texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            rgba,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("mdedit image widget bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.images.insert(
            destination,
            GpuImage {
                _texture: texture,
                bind_group,
                width,
                height,
            },
        );
        Ok(())
    }

    fn prepare(
        &self,
        device: &Device,
        draws: &[ImageDrawRequest],
        viewport_width: u32,
        viewport_height: u32,
    ) -> Vec<ImageDrawBatch> {
        if viewport_width == 0 || viewport_height == 0 {
            return Vec::new();
        }

        draws
            .iter()
            .filter(|draw| self.images.contains_key(&draw.destination))
            .map(|draw| {
                let mut bytes = Vec::with_capacity(6 * 4 * size_of::<f32>());
                append_image_rect(
                    &mut bytes,
                    draw.rect,
                    viewport_width as f32,
                    viewport_height as f32,
                );
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mdedit image vertex buffer"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                ImageDrawBatch {
                    destination: draw.destination.clone(),
                    buffer,
                    vertex_count: 6,
                }
            })
            .collect()
    }

    fn render<'a>(&'a self, pass: &mut RenderPass<'a>, batches: &'a [ImageDrawBatch]) {
        pass.set_pipeline(&self.pipeline);
        for batch in batches {
            let Some(image) = self.images.get(&batch.destination) else {
                continue;
            };
            pass.set_bind_group(0, &image.bind_group, &[]);
            pass.set_vertex_buffer(0, batch.buffer.slice(..));
            pass.draw(0..batch.vertex_count, 0..1);
        }
    }
}

fn append_image_rect(bytes: &mut Vec<u8>, rect: ScreenRect, width: f32, height: f32) {
    let left = pixel_to_ndc_x(rect.x, width);
    let right = pixel_to_ndc_x(rect.x + rect.width, width);
    let top = pixel_to_ndc_y(rect.y, height);
    let bottom = pixel_to_ndc_y(rect.y + rect.height, height);

    for ([x, y], [u, v]) in [
        ([left, top], [0.0, 0.0]),
        ([right, top], [1.0, 0.0]),
        ([right, bottom], [1.0, 1.0]),
        ([left, top], [0.0, 0.0]),
        ([right, bottom], [1.0, 1.0]),
        ([left, bottom], [0.0, 1.0]),
    ] {
        for value in [x, y, u, v] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
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
    fn rgba_upload_rejects_device_limit_and_invalid_buffers() {
        assert!(validate_rgba_upload(8193, 1, 8193 * 4, 8192).is_err());
        assert!(validate_rgba_upload(1, 8193, 8193 * 4, 8192).is_err());
        assert!(validate_rgba_upload(0, 1, 0, 8192).is_err());
        assert!(validate_rgba_upload(2, 2, 15, 8192).is_err());
        assert!(validate_rgba_upload(2, 2, 17, 8192).is_err());
        assert!(validate_rgba_upload(u32::MAX, u32::MAX, 0, u32::MAX).is_err());
        assert!(validate_rgba_upload(2, 2, 16, 8192).is_ok());
        assert!(validate_rgba_upload(8192, 1, 8192 * 4, 8192).is_ok());
    }

    #[test]
    fn image_size_budget_is_checked_before_rgba_allocation() {
        assert!(rgba_byte_len(8192, 8192, 8192).is_err());
        assert_eq!(rgba_byte_len(96, 48, 8192).unwrap(), 96 * 48 * 4);
    }

    #[test]
    #[ignore = "requires a native graphics adapter; run with --ignored"]
    fn gpu_upload_rejects_oversized_image_and_accepts_next_valid_image() {
        pollster::block_on(async {
            let descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            #[cfg(target_os = "windows")]
            let descriptor = wgpu::InstanceDescriptor {
                backends: wgpu::Backends::DX12,
                ..descriptor
            };
            let instance = wgpu::Instance::new(descriptor);
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .unwrap();
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .unwrap();
            let mut renderer = ImageRenderer::new(&device, TextureFormat::Rgba8UnormSrgb);
            let width = device.limits().max_texture_dimension_2d + 1;
            let oversized = vec![0; width as usize * 4];
            assert!(
                renderer
                    .upload(&device, &queue, "oversized".into(), width, 1, &oversized)
                    .is_err()
            );
            assert!(!renderer.contains("oversized"));
            renderer
                .upload(&device, &queue, "valid".into(), 1, 1, &[255; 4])
                .unwrap();
            assert_eq!(renderer.intrinsic_size("valid"), Some((1, 1)));
        });
    }

    #[test]
    fn local_fixture_decodes_off_the_host_boundary() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let (width, height, rgba) =
            decode_local_image(&base, "phase-06-image.png", 8192).expect("decode fixture");

        assert_eq!((width, height), (96, 48));
        assert_eq!(rgba.len(), width as usize * height as usize * 4);
    }

    #[test]
    fn resolver_rejects_remote_and_absolute_paths_before_io() {
        let base = Path::new(".");

        assert!(decode_local_image(base, "https://example.com/image.png", 8192).is_err());
        assert!(decode_local_image(base, "data:image/png;base64,AAAA", 8192).is_err());

        #[cfg(unix)]
        assert!(decode_local_image(base, "/tmp/image.png", 8192).is_err());
    }

    #[test]
    fn decoder_rejects_image_larger_than_device_before_pixel_decode() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        assert!(decode_local_image(&base, "phase-06-image.png", 32).is_err());
    }
}
