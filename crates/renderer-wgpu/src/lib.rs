//! # `berthacharts-renderer-wgpu`
//!
//! wgpu-based renderer for the Bertha Charts kernel. Targets:
//!
//! - **Offscreen texture** — headless rendering for tests, PNG export, and
//!   worker-based browser rendering. Available on native and wasm32.
//! - **HTML canvas surface** — renders directly into a `<canvas>` element
//!   via WebGL2. Available on wasm32 only.
//!
//! Both targets share the same render pipeline; only construction and the
//! per-frame texture acquisition differ.

#![forbid(unsafe_code)]

use berthacharts_core as core;
use bytemuck::{Pod, Zeroable};
use core::{
    Chart, Coord, Geometry, Layer, LinePrim, Mark, PointPrim, RectPrim, Scene, TessellateCtx,
    TrianglePrim,
};

pub use berthacharts_core as chart_core;

/// Errors produced by the renderer.
#[derive(Debug)]
#[non_exhaustive]
pub enum RenderError {
    /// No compatible GPU adapter was available.
    NoAdapter,
    /// Device creation failed.
    DeviceRequest(String),
    /// Surface creation failed.
    SurfaceCreate(String),
    /// Surface presentation failed (outdated / lost / timeout).
    SurfaceFrame(String),
    /// Texture-to-buffer pixel readback failed.
    Readback(String),
    /// Scene referenced a coord id that isn't registered on the workspace.
    MissingCoord(u32),
    /// Operation is unsupported on the current target (e.g. `read_pixels` on a surface).
    Unsupported(&'static str),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "no compatible GPU adapter available"),
            Self::DeviceRequest(m) => write!(f, "device request failed: {m}"),
            Self::SurfaceCreate(m) => write!(f, "surface creation failed: {m}"),
            Self::SurfaceFrame(m) => write!(f, "surface frame error: {m}"),
            Self::Readback(m) => write!(f, "pixel readback failed: {m}"),
            Self::MissingCoord(id) => write!(f, "coord id {id} not registered on workspace"),
            Self::Unsupported(op) => write!(f, "operation {op} not supported for this target"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Background color applied at clear time.
#[derive(Debug, Clone, Copy)]
pub struct ClearColor(pub [f32; 4]);

impl Default for ClearColor {
    fn default() -> Self {
        Self([1.0, 1.0, 1.0, 1.0])
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ViewportUniform {
    width: f32,
    height: f32,
    _pad0: f32,
    _pad1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RectInstance {
    rect: [f32; 4],
    fill: [f32; 4],
    stroke: [f32; 4],
    stroke_width: f32,
    radius: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct TriangleVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy)]
enum DrawCommand {
    Rects { start: u32, count: u32 },
    Triangles { start: u32, count: u32 },
}

const UNIT_QUAD: [QuadVertex; 4] = [
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [0.0, 1.0] },
    QuadVertex { pos: [1.0, 1.0] },
];
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

/// Default offscreen target format — sRGB so test assertions match web display.
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn align_up(n: u32, align: u32) -> u32 {
    (n + align - 1) & !(align - 1)
}

#[cfg(target_arch = "wasm32")]
fn default_backends() -> wgpu::Backends {
    wgpu::Backends::GL
}

#[cfg(not(target_arch = "wasm32"))]
fn default_backends() -> wgpu::Backends {
    wgpu::Backends::all()
}

/// Internal: which kind of surface the renderer is drawing into.
#[allow(dead_code)] // `Surface` variant is only constructed on wasm32
enum Target {
    Offscreen {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    },
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
}

/// Renderer — holds a wgpu device, render pipeline, and a render target.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,

    pipeline: wgpu::RenderPipeline,
    triangle_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,

    viewport_buf: wgpu::Buffer,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: Option<wgpu::Buffer>,
    triangle_buf: Option<wgpu::Buffer>,

    width: u32,
    height: u32,
    logical_width: f32,
    logical_height: f32,
    format: wgpu::TextureFormat,

    /// Clear color applied at the start of each render.
    pub clear_color: ClearColor,
}

impl Renderer {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// Build a headless renderer rendering into an offscreen `width × height` texture.
    ///
    /// # Errors
    /// Fails when no adapter / device is available.
    pub fn new_offscreen(width: u32, height: u32) -> Result<Self, RenderError> {
        pollster::block_on(Self::new_offscreen_async(width, height))
    }

    /// Async variant of [`Self::new_offscreen`] for wasm32 callers.
    ///
    /// # Errors
    /// See [`Self::new_offscreen`].
    pub async fn new_offscreen_async(width: u32, height: u32) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: default_backends(),
            ..Default::default()
        });
        let (device, queue, adapter) = request_device(&instance, None).await?;
        let _ = adapter;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("berthacharts.offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let (pipeline, bind_group_layout) = build_rect_pipeline(&device, OFFSCREEN_FORMAT);
        let triangle_pipeline =
            build_triangle_pipeline(&device, OFFSCREEN_FORMAT, &bind_group_layout);
        let (viewport_buf, vertex_buf, index_buf) =
            build_fixed_buffers(&device, &queue, width as f32, height as f32);

        Ok(Self {
            device,
            queue,
            target: Target::Offscreen { texture, view },
            pipeline,
            triangle_pipeline,
            bind_group_layout,
            viewport_buf,
            vertex_buf,
            index_buf,
            instance_buf: None,
            triangle_buf: None,
            width,
            height,
            logical_width: width as f32,
            logical_height: height as f32,
            format: OFFSCREEN_FORMAT,
            clear_color: ClearColor::default(),
        })
    }

    /// Build a canvas-backed renderer. Wasm32 only.
    ///
    /// The canvas's CSS size determines presentation; call [`Self::resize`] when
    /// the canvas element is resized.
    ///
    /// # Errors
    /// Fails if a surface cannot be acquired from the canvas or no adapter is
    /// compatible with it.
    #[cfg(target_arch = "wasm32")]
    pub async fn new_for_canvas(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        Self::new_for_canvas_with_logical(canvas, width, height, width as f32, height as f32).await
    }

    /// Build a canvas-backed renderer with a physical render target and a
    /// separate logical viewport used by chart geometry.
    ///
    /// This is the high-DPI path: configure the canvas/surface at physical
    /// device pixels, but keep marks, axes, picking, and overlays in CSS pixels.
    ///
    /// # Errors
    /// See [`Self::new_for_canvas`].
    #[cfg(target_arch = "wasm32")]
    pub async fn new_for_canvas_with_logical(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
        logical_width: f32,
        logical_height: f32,
    ) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: default_backends(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| RenderError::SurfaceCreate(e.to_string()))?;
        let (device, queue, adapter) = request_device(&instance, Some(&surface)).await?;

        let caps = surface.get_capabilities(&adapter);
        // Prefer an sRGB format so colors match CSS and offscreen tests.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or_else(|| caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (pipeline, bind_group_layout) = build_rect_pipeline(&device, format);
        let triangle_pipeline = build_triangle_pipeline(&device, format, &bind_group_layout);
        let (viewport_buf, vertex_buf, index_buf) =
            build_fixed_buffers(&device, &queue, logical_width, logical_height);

        Ok(Self {
            device,
            queue,
            target: Target::Surface { surface, config },
            pipeline,
            triangle_pipeline,
            bind_group_layout,
            viewport_buf,
            vertex_buf,
            index_buf,
            instance_buf: None,
            triangle_buf: None,
            width,
            height,
            logical_width,
            logical_height,
            format,
            clear_color: ClearColor::default(),
        })
    }

    // ------------------------------------------------------------------
    // Resizing
    // ------------------------------------------------------------------

    /// Resize the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.resize_with_logical(width, height, width as f32, height as f32);
    }

    /// Resize the physical target while preserving a separate logical viewport.
    pub fn resize_with_logical(
        &mut self,
        width: u32,
        height: u32,
        logical_width: f32,
        logical_height: f32,
    ) {
        if width == 0
            || height == 0
            || logical_width <= 0.0
            || logical_height <= 0.0
            || (width == self.width
                && height == self.height
                && (logical_width - self.logical_width).abs() < f32::EPSILON
                && (logical_height - self.logical_height).abs() < f32::EPSILON)
        {
            return;
        }
        self.width = width;
        self.height = height;
        self.logical_width = logical_width;
        self.logical_height = logical_height;

        match &mut self.target {
            Target::Offscreen { texture, view } => {
                let new_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("berthacharts.offscreen"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: OFFSCREEN_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let new_view = new_tex.create_view(&wgpu::TextureViewDescriptor::default());
                *texture = new_tex;
                *view = new_view;
            }
            Target::Surface { surface, config } => {
                config.width = width;
                config.height = height;
                surface.configure(&self.device, config);
            }
        }

        let viewport = ViewportUniform {
            width: logical_width,
            height: logical_height,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        self.queue
            .write_buffer(&self.viewport_buf, 0, bytemuck::bytes_of(&viewport));
    }

    // ------------------------------------------------------------------
    // Render
    // ------------------------------------------------------------------

    /// Render a chart to the current target.
    ///
    /// # Errors
    /// Fails when a layer references an unregistered coord, or the surface
    /// frame can't be acquired.
    pub fn render(&mut self, chart: &Chart) -> Result<(), RenderError> {
        let scene = chart.scene();
        let workspace = chart.workspace().clone();
        let scales = workspace.scales();
        let datasets = workspace.datasets();

        let mut commands: Vec<DrawCommand> = Vec::new();
        let mut instances: Vec<RectInstance> = Vec::new();
        let mut triangles: Vec<TriangleVertex> = Vec::new();
        for layer in sort_layers(scene) {
            let coord = workspace
                .coord(layer.coord)
                .ok_or(RenderError::MissingCoord(layer.coord.get()))?;
            for mark in &layer.marks {
                let mut buffers = GeometryBuffers {
                    commands: &mut commands,
                    rects: &mut instances,
                    triangles: &mut triangles,
                };
                collect_mark(
                    mark.as_ref(),
                    coord.as_ref(),
                    &scales,
                    &datasets,
                    scene,
                    &mut buffers,
                );
            }
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("berthacharts.rect.bg"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.viewport_buf.as_entire_binding(),
            }],
        });

        self.instance_buf = if instances.is_empty() {
            None
        } else {
            Some(create_buffer_init(
                &self.device,
                "berthacharts.rect.inst",
                bytemuck::cast_slice(&instances),
                wgpu::BufferUsages::VERTEX,
            ))
        };
        self.triangle_buf = if triangles.is_empty() {
            None
        } else {
            Some(create_buffer_init(
                &self.device,
                "berthacharts.triangle.vbo",
                bytemuck::cast_slice(&triangles),
                wgpu::BufferUsages::VERTEX,
            ))
        };

        // Acquire a view for this frame.
        let (view, surface_frame) = match &self.target {
            Target::Offscreen { view, .. } => (view_ref(view), None),
            Target::Surface { surface, .. } => {
                let frame = surface
                    .get_current_texture()
                    .map_err(|e| RenderError::SurfaceFrame(format!("{e:?}")))?;
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                (OwnedOrRef::Owned(view), Some(frame))
            }
        };

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("berthacharts.frame"),
            });
        {
            let [r, g, b, a] = self.clear_color.0;
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("berthacharts.frame.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: view.as_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(r),
                            g: f64::from(g),
                            b: f64::from(b),
                            a: f64::from(a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for command in commands {
                match command {
                    DrawCommand::Rects { start, count } => {
                        let Some(inst) = &self.instance_buf else {
                            continue;
                        };
                        pass.set_pipeline(&self.pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                        pass.set_vertex_buffer(1, inst.slice(..));
                        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..6, 0, start..start + count);
                    }
                    DrawCommand::Triangles { start, count } => {
                        let Some(triangles) = &self.triangle_buf else {
                            continue;
                        };
                        pass.set_pipeline(&self.triangle_pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.set_vertex_buffer(0, triangles.slice(..));
                        pass.draw(start..start + count, 0..1);
                    }
                }
            }
        }
        self.queue.submit(std::iter::once(enc.finish()));

        drop(view); // release any borrow before presenting
        if let Some(frame) = surface_frame {
            frame.present();
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Readback (offscreen only)
    // ------------------------------------------------------------------

    /// Copy the render target back to CPU memory as tightly-packed RGBA bytes.
    ///
    /// # Errors
    /// Fails for surface-backed renderers, or when the readback buffer
    /// can't be mapped.
    pub fn read_pixels(&mut self) -> Result<Vec<u8>, RenderError> {
        let texture = match &self.target {
            Target::Offscreen { texture, .. } => texture,
            Target::Surface { .. } => {
                return Err(RenderError::Unsupported("read_pixels on surface"))
            }
        };

        let unpadded_bpr = self.width * 4;
        let padded_bpr = align_up(unpadded_bpr, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("berthacharts.readback"),
            size: u64::from(padded_bpr) * u64::from(self.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("berthacharts.readback.enc"),
            });
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bpr),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(enc.finish()));

        let (tx, rx) = std::sync::mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| RenderError::Readback(e.to_string()))?
            .map_err(|e| RenderError::Readback(format!("{e:?}")))?;

        let data = staging.slice(..).get_mapped_range();
        let mut out = Vec::with_capacity((unpadded_bpr * self.height) as usize);
        for row in 0..self.height {
            let start = (row * padded_bpr) as usize;
            let end = start + unpadded_bpr as usize;
            out.extend_from_slice(&data[start..end]);
        }
        drop(data);
        staging.unmap();
        Ok(out)
    }

    /// Width of the render target in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height of the render target in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Format of the render target.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

async fn request_device(
    instance: &wgpu::Instance,
    compat_surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter), RenderError> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: compat_surface,
        })
        .await
        .ok_or(RenderError::NoAdapter)?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("berthacharts.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .map_err(|e| RenderError::DeviceRequest(e.to_string()))?;

    Ok((device, queue, adapter))
}

fn build_rect_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("berthacharts.rect.shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader_rect.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("berthacharts.rect.bgl"),
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
        label: Some("berthacharts.rect.layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let vertex_buffers: [wgpu::VertexBufferLayout; 2] = [
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        },
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 48,
                    shader_location: 4,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 52,
                    shader_location: 5,
                },
            ],
        },
    ];

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("berthacharts.rect.pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

    (pipeline, bind_group_layout)
}

fn build_triangle_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("berthacharts.triangle.shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader_triangle.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("berthacharts.triangle.layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    let vertex_buffers: [wgpu::VertexBufferLayout; 1] = [wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<TriangleVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 8,
                shader_location: 1,
            },
        ],
    }];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("berthacharts.triangle.pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn build_fixed_buffers(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    logical_width: f32,
    logical_height: f32,
) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer) {
    let viewport_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("berthacharts.viewport"),
        size: std::mem::size_of::<ViewportUniform>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(
        &viewport_buf,
        0,
        bytemuck::bytes_of(&ViewportUniform {
            width: logical_width,
            height: logical_height,
            _pad0: 0.0,
            _pad1: 0.0,
        }),
    );

    let vertex_buf = create_buffer_init(
        device,
        "berthacharts.rect.vbo",
        bytemuck::cast_slice(&UNIT_QUAD),
        wgpu::BufferUsages::VERTEX,
    );
    let index_buf = create_buffer_init(
        device,
        "berthacharts.rect.ibo",
        bytemuck::cast_slice(&QUAD_INDICES),
        wgpu::BufferUsages::INDEX,
    );
    (viewport_buf, vertex_buf, index_buf)
}

fn create_buffer_init(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage,
    })
}

fn sort_layers(scene: &Scene) -> Vec<&Layer> {
    let mut v: Vec<&Layer> = scene.layers.iter().collect();
    v.sort_by_key(|l| l.z);
    v
}

struct GeometryBuffers<'a> {
    commands: &'a mut Vec<DrawCommand>,
    rects: &'a mut Vec<RectInstance>,
    triangles: &'a mut Vec<TriangleVertex>,
}

fn collect_mark(
    mark: &dyn Mark,
    coord: &dyn Coord,
    scales: &core::ScaleRegistry,
    datasets: &core::DatasetRegistry,
    scene: &Scene,
    buffers: &mut GeometryBuffers<'_>,
) {
    let tess_ctx = TessellateCtx::new(
        coord,
        scales,
        datasets,
        scene.viewport.plot_area,
        scene.viewport.device_pixel_ratio,
    );
    walk_geometry(mark.tessellate(&tess_ctx), buffers);
}

fn walk_geometry(g: Geometry, buffers: &mut GeometryBuffers<'_>) {
    match g {
        Geometry::Rects(rs) => {
            let start = buffers.rects.len();
            for r in rs {
                buffers.rects.push(rect_to_instance(&r));
            }
            if let Some((start, count)) = draw_range(start, buffers.rects.len()) {
                buffers.commands.push(DrawCommand::Rects { start, count });
            }
        }
        Geometry::Triangles(ts) => {
            let start = buffers.triangles.len();
            for t in ts {
                push_triangle(&t, buffers.triangles);
            }
            if let Some((start, count)) = draw_range(start, buffers.triangles.len()) {
                buffers
                    .commands
                    .push(DrawCommand::Triangles { start, count });
            }
        }
        Geometry::Mixed(children) => {
            for c in children {
                walk_geometry(c, buffers);
            }
        }
        Geometry::Empty => {}
        Geometry::Points(ps) => {
            let start = buffers.triangles.len();
            for p in ps {
                push_point(&p, buffers.triangles);
            }
            if let Some((start, count)) = draw_range(start, buffers.triangles.len()) {
                buffers
                    .commands
                    .push(DrawCommand::Triangles { start, count });
            }
        }
        Geometry::Lines(ls) => {
            let start = buffers.triangles.len();
            for line in ls {
                push_line(&line, buffers.triangles);
            }
            if let Some((start, count)) = draw_range(start, buffers.triangles.len()) {
                buffers
                    .commands
                    .push(DrawCommand::Triangles { start, count });
            }
        }
        Geometry::Paths(_) => {}
        _ => {}
    }
}

fn draw_range(start: usize, end: usize) -> Option<(u32, u32)> {
    if end == start {
        return None;
    }
    Some((
        u32::try_from(start).expect("draw start exceeds u32"),
        u32::try_from(end - start).expect("draw count exceeds u32"),
    ))
}

fn push_triangle(t: &TrianglePrim, out: &mut Vec<TriangleVertex>) {
    out.push(TriangleVertex {
        pos: t.a,
        color: t.fill,
    });
    out.push(TriangleVertex {
        pos: t.b,
        color: t.fill,
    });
    out.push(TriangleVertex {
        pos: t.c,
        color: t.fill,
    });
}

fn push_point(p: &PointPrim, out: &mut Vec<TriangleVertex>) {
    if p.r <= 0.0 {
        return;
    }
    if p.stroke_width > 0.0 && p.stroke[3] > 0.0 {
        push_point_shape([p.x, p.y], p.r + p.stroke_width, p.shape, p.stroke, out);
    }
    if p.fill[3] > 0.0 {
        push_point_shape([p.x, p.y], p.r, p.shape, p.fill, out);
    }
}

fn push_point_shape(
    center: [f32; 2],
    radius: f32,
    shape: u32,
    color: [f32; 4],
    out: &mut Vec<TriangleVertex>,
) {
    match shape {
        1 => {
            let [x, y] = center;
            push_triangle_raw(
                [x - radius, y - radius],
                [x + radius, y - radius],
                [x - radius, y + radius],
                color,
                out,
            );
            push_triangle_raw(
                [x + radius, y - radius],
                [x + radius, y + radius],
                [x - radius, y + radius],
                color,
                out,
            );
        }
        2 => {
            let [x, y] = center;
            push_triangle_raw(
                [x, y - radius],
                [x + radius * 0.92, y + radius * 0.72],
                [x - radius * 0.92, y + radius * 0.72],
                color,
                out,
            );
        }
        3 => {
            let [x, y] = center;
            push_triangle_raw(
                [x, y - radius],
                [x + radius, y],
                [x, y + radius],
                color,
                out,
            );
            push_triangle_raw(
                [x, y - radius],
                [x, y + radius],
                [x - radius, y],
                color,
                out,
            );
        }
        _ => push_circle(center, radius, color, out),
    }
}

fn push_circle(center: [f32; 2], radius: f32, color: [f32; 4], out: &mut Vec<TriangleVertex>) {
    const STEPS: usize = 20;
    let [cx, cy] = center;
    let mut prev = [cx + radius, cy];
    for i in 1..=STEPS {
        let theta = (i as f32 / STEPS as f32) * std::f32::consts::TAU;
        let next = [cx + theta.cos() * radius, cy + theta.sin() * radius];
        push_triangle_raw(center, prev, next, color, out);
        prev = next;
    }
}

fn push_line(line: &LinePrim, out: &mut Vec<TriangleVertex>) {
    if line.points.len() < 2 || line.width <= 0.0 || line.stroke[3] <= 0.0 {
        return;
    }

    let dash = line
        .dash
        .as_ref()
        .filter(|pattern| !pattern.is_empty() && pattern.iter().all(|v| *v > 0.0));
    let Some(pattern) = dash else {
        for segment in line.points.windows(2) {
            push_line_segment(segment[0], segment[1], line.width, line.stroke, out);
        }
        return;
    };

    let mut dash_index = 0usize;
    let mut dash_remaining = pattern[0];
    let mut drawing = true;

    for segment in line.points.windows(2) {
        let a = segment[0];
        let b = segment[1];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len <= f32::EPSILON {
            continue;
        }

        let mut consumed = 0.0;
        while consumed < len {
            let step = dash_remaining.min(len - consumed);
            if drawing {
                let t0 = consumed / len;
                let t1 = (consumed + step) / len;
                push_line_segment(
                    [a[0] + dx * t0, a[1] + dy * t0],
                    [a[0] + dx * t1, a[1] + dy * t1],
                    line.width,
                    line.stroke,
                    out,
                );
            }

            consumed += step;
            dash_remaining -= step;
            if dash_remaining <= f32::EPSILON {
                dash_index = (dash_index + 1) % pattern.len();
                dash_remaining = pattern[dash_index];
                drawing = dash_index.is_multiple_of(2);
            }
        }
    }
}

fn push_line_segment(
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 4],
    out: &mut Vec<TriangleVertex>,
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = dx.hypot(dy);
    if len <= f32::EPSILON {
        return;
    }

    let nx = -dy / len * width * 0.5;
    let ny = dx / len * width * 0.5;
    let a0 = [a[0] + nx, a[1] + ny];
    let a1 = [a[0] - nx, a[1] - ny];
    let b0 = [b[0] + nx, b[1] + ny];
    let b1 = [b[0] - nx, b[1] - ny];
    push_triangle_raw(a0, b0, a1, color, out);
    push_triangle_raw(b0, b1, a1, color, out);
}

fn push_triangle_raw(
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    color: [f32; 4],
    out: &mut Vec<TriangleVertex>,
) {
    out.push(TriangleVertex { pos: a, color });
    out.push(TriangleVertex { pos: b, color });
    out.push(TriangleVertex { pos: c, color });
}

fn rect_to_instance(r: &RectPrim) -> RectInstance {
    RectInstance {
        rect: [r.x, r.y, r.w, r.h],
        fill: r.fill,
        stroke: r.stroke,
        stroke_width: r.stroke_width,
        radius: r.radius,
        _pad: [0.0, 0.0],
    }
}

/// Wrapper so the frame-view can be either borrowed (offscreen) or owned (surface).
enum OwnedOrRef<'a> {
    Owned(wgpu::TextureView),
    Ref(&'a wgpu::TextureView),
}
impl<'a> OwnedOrRef<'a> {
    fn as_view(&self) -> &wgpu::TextureView {
        match self {
            OwnedOrRef::Owned(v) => v,
            OwnedOrRef::Ref(v) => v,
        }
    }
}
fn view_ref(v: &wgpu::TextureView) -> OwnedOrRef<'_> {
    OwnedOrRef::Ref(v)
}
