//! wgpu plumbing: one headless device, fullscreen-triangle passes, uploads and readback.

use crate::RenderError;
use std::collections::HashMap;
use wgpu::util::DeviceExt;

pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// `ctx` uniform of the effect prelude: size.xy, time, duration, progress, frame.
pub(crate) type CtxVals = [f32; 6];

pub(crate) struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    /// 1x1 transparent: stands in for an unused or missing input.
    pub empty: wgpu::Texture,
    /// Keyed by the full shader source; holds the compile error too so a bad shader fails fast.
    pipelines: HashMap<String, Result<wgpu::RenderPipeline, String>>,
    max_dim: u32,
}

fn gpu_err(e: impl std::fmt::Display) -> RenderError {
    RenderError::Gpu(e.to_string())
}

impl Gpu {
    pub fn new() -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let request = |fallback| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: fallback,
                ..Default::default()
            }))
        };
        let adapter = request(false)
            .or_else(|_| request(true))
            .map_err(|e| gpu_err(format!("no GPU adapter, not even a software one: {e}")))?;
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("adit-render"),
            required_limits: limits.clone(),
            ..Default::default()
        }))
        .map_err(gpu_err)?;
        // Every operation runs inside an error scope; the default handler would panic on a stray one.
        device.on_uncaptured_error(std::sync::Arc::new(|e| {
            eprintln!("adit-render: uncaptured GPU error: {e}")
        }));
        let uniform = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[uniform(0), uniform(1), texture(2), texture(3)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let mut gpu = Gpu {
            empty: new_texture(&device, 1, 1),
            device,
            queue,
            layout,
            pipeline_layout,
            pipelines: HashMap::new(),
            max_dim: limits.max_texture_dimension_2d,
        };
        gpu.empty = gpu.upload(&[0; 4], 1, 1)?;
        Ok(gpu)
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    /// Catches GPU validation and out-of-memory errors until `Scope::end` (thread-local).
    pub fn scope(&self) -> Scope {
        let oom = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let val = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        Scope { val, oom }
    }

    fn check_size(&self, w: u32, h: u32) -> Result<(), RenderError> {
        if w == 0 || h == 0 || w > self.max_dim || h > self.max_dim {
            return Err(gpu_err(format!(
                "texture size {w}x{h} outside 1..={}",
                self.max_dim
            )));
        }
        Ok(())
    }

    /// Straight-alpha RGBA8 rows into a new texture. `rgba.len()` must be `w * h * 4`.
    pub fn upload(&self, rgba: &[u8], w: u32, h: u32) -> Result<wgpu::Texture, RenderError> {
        self.check_size(w, h)?;
        let tex = new_texture(&self.device, w, h);
        self.queue.write_texture(
            tex.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            tex.size(),
        );
        Ok(tex)
    }

    /// New `w`x`h` texture filled with `color` (straight RGBA 0..1).
    pub fn fill(
        &self,
        enc: &mut wgpu::CommandEncoder,
        w: u32,
        h: u32,
        color: [f64; 4],
    ) -> Result<wgpu::Texture, RenderError> {
        self.check_size(w, h)?;
        let tex = new_texture(&self.device, w, h);
        let [r, g, b, a] = color.map(|c| c.clamp(0.0, 1.0));
        let view = tex.create_view(&Default::default());
        enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        Ok(tex)
    }

    fn pipeline(&mut self, source: &str) -> Result<&wgpu::RenderPipeline, RenderError> {
        if !self.pipelines.contains_key(source) {
            let built = self.build_pipeline(source);
            self.pipelines.insert(source.to_string(), built);
        }
        match &self.pipelines[source] {
            Ok(p) => Ok(p),
            Err(e) => Err(RenderError::Shader(e.clone())),
        }
    }

    fn build_pipeline(&self, source: &str) -> Result<wgpu::RenderPipeline, String> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("adit_vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("adit_fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(FORMAT.into())],
                }),
                multiview_mask: None,
                cache: None,
            });
        match pollster::block_on(scope.pop()) {
            Some(e) => Err(e.to_string()),
            None => Ok(pipeline),
        }
    }

    /// One fullscreen pass of `source` (an assembled effect shader) into a new `out` texture.
    pub fn pass(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        source: &str,
        inputs: [&wgpu::Texture; 2],
        ctx: CtxVals,
        params: &[f32],
        out: (u32, u32),
    ) -> Result<wgpu::Texture, RenderError> {
        self.check_size(out.0, out.1)?;
        self.pipeline(source)?;
        let pipeline = self.pipelines[source].as_ref().expect("built above");
        let buffer = |vals: &[f32]| {
            // Uniform bindings are padded to 16 bytes; an empty params struct still has one field.
            let mut v = vals.to_vec();
            v.resize(vals.len().div_ceil(4).max(1) * 4, 0.0);
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&v),
                    usage: wgpu::BufferUsages::UNIFORM,
                })
        };
        let (ctx_buf, p_buf) = (buffer(&ctx), buffer(params));
        let views = inputs.map(|t| t.create_view(&Default::default()));
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ctx_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: p_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
            ],
        });
        let tex = new_texture(&self.device, out.0, out.1);
        let view = tex.create_view(&Default::default());
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        rp.set_pipeline(pipeline);
        rp.set_bind_group(0, &bind, &[]);
        rp.draw(0..3, 0..1);
        drop(rp);
        Ok(tex)
    }

    /// Submits `enc` and reads `tex` back as tightly packed RGBA8 rows.
    pub fn finish(
        &self,
        mut enc: wgpu::CommandEncoder,
        tex: &wgpu::Texture,
    ) -> Result<Vec<u8>, RenderError> {
        let (w, h) = (tex.width(), tex.height());
        let row = 4 * w;
        let padded =
            row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        enc.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            tex.size(),
        );
        self.queue.submit([enc.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buf.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(gpu_err)?;
        rx.recv()
            .map_err(gpu_err)?
            .map_err(|e| gpu_err(format!("readback: {e}")))?;
        let data = buf.get_mapped_range(..).map_err(gpu_err)?;
        let mut out = Vec::with_capacity((row * h) as usize);
        for y in 0..h as usize {
            let start = y * padded as usize;
            out.extend_from_slice(&data[start..start + row as usize]);
        }
        Ok(out)
    }
}

pub(crate) struct Scope {
    val: wgpu::ErrorScopeGuard,
    oom: wgpu::ErrorScopeGuard,
}

impl Scope {
    /// `r`, or the first GPU error raised while the scope was open.
    pub fn end<T>(self, r: Result<T, RenderError>) -> Result<T, RenderError> {
        let val = pollster::block_on(self.val.pop());
        let oom = pollster::block_on(self.oom.pop());
        match (r, val.or(oom)) {
            (Err(e), _) => Err(e),
            (Ok(_), Some(e)) => Err(gpu_err(e)),
            (Ok(v), None) => Ok(v),
        }
    }
}

fn new_texture(device: &wgpu::Device, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
