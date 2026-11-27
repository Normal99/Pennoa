//! GPU wet-ink renderer: new stroke segments are tessellated on the CPU and
//! drawn incrementally (LoadOp::Load) into a persistent ink texture. In
//! subsurface mode the texture is then blitted to our own swapchain.

use bytemuck::{Pod, Zeroable};
use slint::wgpu_30::wgpu;

const PAPER: wgpu::Color = wgpu::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
const INK: [f32; 4] = [0.114, 0.169, 0.361, 1.0]; // #1D2B5C
const CAP_SEGMENTS: usize = 12;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

pub struct InkRenderer {
    pub texture: wgpu::Texture,
    pub size: (u32, u32),
    view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pending: Vec<Vertex>,
    needs_clear: bool,
    last: Option<(f32, f32, f32)>,
}

impl InkRenderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("ink.wgsl"));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ink"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ink view"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ink"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_ink"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_ink"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
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
        Self {
            texture,
            size: (width.max(1), height.max(1)),
            view,
            pipeline,
            uniform,
            bind_group,
            pending: Vec::new(),
            needs_clear: true,
            last: None,
        }
    }

    pub fn clear(&mut self) {
        self.needs_clear = true;
        self.pending.clear();
        self.last = None;
    }

    pub fn end_stroke(&mut self) {
        self.last = None;
    }

    /// Add a point (physical px) with radius `r`. Consecutive points form a stroke.
    pub fn add_point(&mut self, x: f32, y: f32, r: f32, eraser: bool) {
        let color = if eraser { [1.0, 1.0, 1.0, 1.0] } else { INK };
        if let Some((px, py, pr)) = self.last {
            let (dx, dy) = (x - px, y - py);
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.01 {
                let (nx, ny) = (-dy / len, dx / len);
                let a = [px + nx * pr, py + ny * pr];
                let b = [px - nx * pr, py - ny * pr];
                let c = [x + nx * r, y + ny * r];
                let d = [x - nx * r, y - ny * r];
                for pos in [a, b, c, b, d, c] {
                    self.pending.push(Vertex { pos, color });
                }
            }
        }
        // Round cap/join at this point.
        for i in 0..CAP_SEGMENTS {
            let a0 = i as f32 / CAP_SEGMENTS as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / CAP_SEGMENTS as f32 * std::f32::consts::TAU;
            for pos in [
                [x, y],
                [x + r * a0.cos(), y + r * a0.sin()],
                [x + r * a1.cos(), y + r * a1.sin()],
            ] {
                self.pending.push(Vertex { pos, color });
            }
        }
        self.last = Some((x, y, r));
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty() || self.needs_clear
    }

    /// Draw pending geometry into the ink texture.
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.has_pending() {
            return;
        }
        let size = [self.size.0 as f32, self.size.1 as f32, 0.0, 0.0];
        queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&size));
        let vbuf = (!self.pending.is_empty()).then(|| {
            use wgpu::util::DeviceExt;
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ink vertices"),
                contents: bytemuck::cast_slice(&self.pending),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ink"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if self.needs_clear {
                            wgpu::LoadOp::Clear(PAPER)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(vbuf) = &vbuf {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.draw(0..self.pending.len() as u32, 0..1);
            }
        }
        queue.submit([enc.finish()]);
        self.pending.clear();
        self.needs_clear = false;
    }
}

/// Our own swapchain on the canvas subsurface, showing the ink texture.
pub struct CanvasSwapchain {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pub present_mode_used: wgpu::PresentMode,
}

impl CanvasSwapchain {
    /// # Safety
    /// `display` and `surface` must be valid Wayland pointers that outlive the swapchain.
    pub unsafe fn new(
        instance: &wgpu::Instance,
        adapter_device: &wgpu::Device,
        display: *mut std::ffi::c_void,
        surface: *mut std::ffi::c_void,
        width: u32,
        height: u32,
        wanted: &[wgpu::PresentMode],
    ) -> anyhow::Result<Self> {
        use raw_window_handle::{
            RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
        };
        use std::ptr::NonNull;
        let target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                NonNull::new(display).ok_or_else(|| anyhow::anyhow!("null display"))?,
            ))),
            raw_window_handle: RawWindowHandle::Wayland(WaylandWindowHandle::new(
                NonNull::new(surface).ok_or_else(|| anyhow::anyhow!("null surface"))?,
            )),
        };
        // SAFETY: forwarded from the caller.
        let wsurface = unsafe { instance.create_surface_unsafe(target)? };

        // Query capabilities through an adapter for the same instance; we render with
        // Slint's device, which must sit on the same GPU (checked by `configure`).
        let adapter = pollster_block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&wsurface),
            ..Default::default()
        }))?;
        let mut config = wsurface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or_else(|| anyhow::anyhow!("canvas surface not supported by the GPU"))?;
        config.format = if config.format.is_srgb() {
            config.format.remove_srgb_suffix()
        } else {
            config.format
        };
        config.desired_maximum_frame_latency = 1;
        config.present_mode = wgpu::PresentMode::Fifo;
        let caps = wsurface.get_capabilities(&adapter);
        for m in wanted {
            if caps.present_modes.contains(m) {
                config.present_mode = *m;
                break;
            }
        }
        wsurface.configure(adapter_device, &config);

        let shader = adapter_device.create_shader_module(wgpu::include_wgsl!("ink.wgsl"));
        let bgl = adapter_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = adapter_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = adapter_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_blit"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_blit"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = adapter_device.create_sampler(&wgpu::SamplerDescriptor::default());
        let present_mode_used = config.present_mode;
        Ok(Self {
            surface: wsurface,
            config,
            pipeline,
            bgl,
            sampler,
            present_mode_used,
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(device, &self.config);
    }

    /// Blit the ink texture and present. Returns false if no frame was available.
    pub fn present(&self, device: &wgpu::Device, queue: &wgpu::Queue, ink: &InkRenderer) -> bool {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            other => {
                eprintln!("canvas surface: {other:?}");
                self.surface.configure(device, &self.config);
                return false;
            }
        };
        let target = frame.texture.create_view(&Default::default());
        let src = ink.texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(PAPER),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
        queue.present(frame);
        true
    }
}

/// Minimal executor for wgpu's adapter request (resolves immediately on native).
fn pollster_block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop_raw() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            noop_raw()
        }
        fn noop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    // SAFETY: the vtable functions do nothing and never dereference the pointer.
    let waker = unsafe { Waker::from_raw(noop_raw()) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}
