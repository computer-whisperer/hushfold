//! wgpu renderer: instanced boxes (opaque, translucent, moonbeams), bloom, tonemap, egui.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3, Vec4};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub mod kind {
    pub const PLAIN: u32 = 0;
    pub const WOOD: u32 = 1;
    pub const BOOK: u32 = 2;
    pub const FLOOR: u32 = 3;
    pub const STONE: u32 = 4;
    pub const EMISSIVE: u32 = 5;
    pub const GHOST: u32 = 6;
    pub const RUG: u32 = 7;
    pub const PLAQUE: u32 = 8;
    pub const GLASS: u32 = 9;
    pub const VEIL: u32 = 10;
    pub const SHAFT: u32 = 11;
    pub const PAPER: u32 = 12;
    /// Brass, iron, silver: the colour is the metal's. Grey metals come out rough,
    /// bright or coloured ones polished. param 1: aged (verdigris on warm metals, rust
    /// on grey ones).
    #[allow(dead_code)] // TEMP(visual pass): until something is made of it.
    pub const METAL: u32 = 13;
    /// Banners, upholstery, curtains. Thin upright pieces hang in folds, others are
    /// stuffed like cushions. param: 0 plain, else a trim colour in the low 24 bits
    /// (r | g << 8 | b << 16, like RUG) as a border band, and bits 24..25 a pattern in
    /// it: 1 stripes, 2 diamonds.
    #[allow(dead_code)] // TEMP(visual pass): until something is made of it.
    pub const CLOTH: u32 = 14;
    /// Leaves and stems: a thin flat box is a blade, a long thin one a stem, anything
    /// bigger a mass of foliage. Light behind shines through.
    #[allow(dead_code)] // TEMP(visual pass): until something is made of it.
    pub const LEAF: u32 = 15;
    /// Gems and glass: color.w is how brightly it glows from inside (0 for plain glass).
    #[allow(dead_code)] // TEMP(visual pass): until something is made of it.
    pub const CRYSTAL: u32 = 16;
    /// Maps, charts, scrolls; the colour is the paper's. param picks the ink on its
    /// broad faces: 0 writing, 1 a map, 2 a star chart, 3 none. A thick box shows a
    /// rolled scroll's turns at its ends.
    #[allow(dead_code)] // TEMP(visual pass): until something is made of it.
    pub const PARCHMENT: u32 = 17;
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
pub struct Inst {
    pub pos: [f32; 3],
    pub kind: u32,
    pub rot: [f32; 4],
    pub scale: [f32; 3],
    pub param: u32,
    pub color: [f32; 4],
}

impl Inst {
    pub fn new(pos: Vec3, rot: Quat, scale: Vec3, kind: u32, color: [f32; 4], param: u32) -> Self {
        Self {
            pos: pos.into(),
            kind,
            rot: rot.into(),
            scale: scale.into(),
            param,
            color,
        }
    }

    /// Axis-aligned box from its centre and full size.
    pub fn aabox(center: Vec3, size: Vec3, kind: u32, rgb: [f32; 3]) -> Self {
        Self::new(center, Quat::IDENTITY, size, kind, [rgb[0], rgb[1], rgb[2], 1.0], 0)
    }

    /// Axis-aligned box from min/max corners.
    pub fn minmax(min: Vec3, max: Vec3, kind: u32, rgb: [f32; 3]) -> Self {
        Self::aabox((min + max) * 0.5, max - min, kind, rgb)
    }

    /// Its corners, if its rotation only swaps or flips axes.
    pub fn axis_box(&self) -> Option<(Vec3, Vec3)> {
        let m = glam::Mat3::from_quat(Quat::from_array(self.rot));
        let square = (m.to_cols_array().iter()).all(|v| v.abs() < 1e-4 || (v.abs() - 1.0).abs() < 1e-4);
        let half = (m * (Vec3::from(self.scale) * 0.5)).abs();
        let c = Vec3::from(self.pos);
        square.then_some((c - half, c + half))
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct GpuLight {
    pub pos_radius: [f32; 4],
    pub color: [f32; 4],
}

pub const MAX_LIGHTS: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub cam_pos: [f32; 4],
    pub fog: [f32; 4],
    pub ambient_top: [f32; 4],
    pub ambient_bottom: [f32; 4],
    /// The Seeker's Lens: x = the group sought (0 off, 1 department, 2 category,
    /// 3 series), y = which one.
    pub lens: [f32; 4],
    /// Kindred Glow on the runs of carried series: x = reach in metres, y =
    /// strength, z = glint (0 or 1).
    pub glow: [f32; 4],
    pub lights: [GpuLight; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PostParams {
    pub exposure: f32,
    pub bloom: f32,
    pub time: f32,
    pub vignette: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
}

const MSAA: u32 = 4;
const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const BLOOM_LEVELS: usize = 6;

fn unit_box() -> (Vec<Vertex>, Vec<u16>) {
    let mut v = Vec::new();
    let mut idx = Vec::new();
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
    ];
    for (n, a, b) in faces {
        let n = Vec3::from(n);
        let a = Vec3::from(a);
        let b = Vec3::from(b);
        let base = v.len() as u16;
        for (sa, sb) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = (n + a * sa + b * sb) * 0.5;
            v.push(Vertex {
                pos: p.into(),
                normal: n.into(),
            });
        }
        // Wind counter-clockwise when viewed from outside.
        let cross = a.cross(b).dot(n);
        if cross > 0.0 {
            idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            idx.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    (v, idx)
}

struct GrowBuf {
    buf: wgpu::Buffer,
    cap: usize,
    len: u32,
    label: &'static str,
}

impl GrowBuf {
    fn new(device: &wgpu::Device, label: &'static str, cap: usize) -> Self {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (cap * std::mem::size_of::<Inst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buf,
            cap,
            len: 0,
            label,
        }
    }

    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[Inst]) {
        if data.len() > self.cap {
            *self = Self::new(device, self.label, data.len().next_power_of_two());
        }
        if !data.is_empty() {
            queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(data));
        }
        self.len = data.len() as u32;
    }
}

/// GPU timestamps around the scene pass and the whole frame (when the adapter has them).
struct Timer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    period: f32,
}

/// A scene chunk's instances on the GPU, and its bounds for culling.
struct ChunkGpu {
    opaque: GrowBuf,
    trans: GrowBuf,
    min: Vec3,
    max: Vec3,
}

/// The planes of a view frustum (inside where `dot(xyz, p) + w >= 0`).
fn frustum(view_proj: Mat4) -> [Vec4; 6] {
    let r = |i| view_proj.row(i);
    [r(3) + r(0), r(3) - r(0), r(3) + r(1), r(3) - r(1), r(2), r(3) - r(2)]
}

fn box_visible(planes: &[Vec4; 6], min: Vec3, max: Vec3) -> bool {
    planes.iter().all(|p| {
        // The corner furthest along the plane's normal.
        let far = Vec3::select(p.truncate().cmpge(Vec3::ZERO), max, min);
        p.truncate().dot(far) + p.w >= 0.0
    })
}

struct Targets {
    msaa_color: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolved: wgpu::TextureView,
    bloom: Vec<wgpu::TextureView>,
    /// bind groups sampling: [resolved, bloom0, bloom1, ...]
    chain_bgs: Vec<wgpu::BindGroup>,
    tonemap_bg: wgpu::BindGroup,
    /// The depth target, for translucent things to read.
    depth_bg: wgpu::BindGroup,
}

pub struct Renderer {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    scene_pipe: wgpu::RenderPipeline,
    trans_pipe: wgpu::RenderPipeline,
    beam_pipe: wgpu::RenderPipeline,
    down_first_pipe: wgpu::RenderPipeline,
    down_pipe: wgpu::RenderPipeline,
    up_pipe: wgpu::RenderPipeline,
    tonemap_pipe: wgpu::RenderPipeline,
    chain_layout: wgpu::BindGroupLayout,
    tonemap_layout: wgpu::BindGroupLayout,
    depth_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    post_buf: wgpu::Buffer,
    box_vb: wgpu::Buffer,
    box_ib: wgpu::Buffer,
    static_buf: GrowBuf,
    dyn_buf: GrowBuf,
    trans_buf: GrowBuf,
    /// Moonbeams, drawn apart from the other translucent things (see `beam_pipe`), and
    /// the lists the frame's translucent instances are sorted into.
    beam_buf: GrowBuf,
    split: [Vec<Inst>; 2],
    held_buf: GrowBuf,
    chunks: Vec<ChunkGpu>,
    timer: Option<Timer>,
    targets: Targets,
    pub egui: egui_wgpu::Renderer,
    /// When set, the next frame is also written to this PNG path; `render` returns
    /// how that went.
    pub capture: Option<std::path::PathBuf>,
    can_capture: bool,
    /// Diagnostics: time blocked acquiring the swapchain image, and when `sync_gpu`, GPU
    /// time for the frame and for its scene pass (from timestamps where the adapter
    /// has them, else the wait for the frame to finish).
    pub acquire_ms: f32,
    pub gpu_ms: f32,
    pub gpu_scene_ms: f32,
    pub sync_gpu: bool,
    /// Scene chunks drawn, and uploaded, last frame.
    pub drawn_chunks: usize,
    pub uploaded_chunks: usize,
}

pub struct FrameInput<'a> {
    pub globals: &'a Globals,
    pub post: PostParams,
    pub dynamic: &'a [Inst],
    pub trans: &'a [Inst],
    pub held: &'a [Inst],
    pub clear: [f32; 3],
    pub egui_prims: &'a [egui::ClippedPrimitive],
    pub egui_textures: &'a egui::TexturesDelta,
    pub pixels_per_point: f32,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no adapter");
        eprintln!("hushfold: using {:?}", adapter.get_info().name);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("hushfold"),
            required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm))
            .unwrap_or(caps.formats[0]);
        let present_mode = match std::env::var("HUSHFOLD_PRESENT").as_deref() {
            Ok("immediate") if caps.present_modes.contains(&wgpu::PresentMode::Immediate) => {
                wgpu::PresentMode::Immediate
            }
            Ok("fifo") => wgpu::PresentMode::Fifo,
            _ if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) => wgpu::PresentMode::Mailbox,
            _ => wgpu::PresentMode::Fifo,
        };
        eprintln!(
            "hushfold: present mode {present_mode:?} (available {:?})",
            caps.present_modes
        );
        let can_capture = caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if can_capture {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let config = wgpu::SurfaceConfiguration {
            usage,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // ---- scene pipelines
        let scene_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
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
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        });
        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        // Translucent things also read the opaque scene's depth: moonbeams are lit only as
        // far as the eye can see into them.
        let depth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene depth"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: true,
                },
                count: None,
            }],
        });
        let trans_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("trans"),
            bind_group_layouts: &[Some(&globals_layout), Some(&depth_layout)],
            immediate_size: 0,
        });
        let vb_layouts = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Inst>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    2 => Float32x3, 3 => Uint32, 4 => Float32x4, 5 => Float32x3, 6 => Uint32, 7 => Float32x4
                ],
            },
        ];
        let make_scene = |label: &str,
                          layout: &wgpu::PipelineLayout,
                          fs: &str,
                          blend: Option<wgpu::BlendState>,
                          (write, compare): (bool, wgpu::CompareFunction),
                          cull: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &scene_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &vb_layouts,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: if cull { Some(wgpu::Face::Back) } else { None },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(write),
                    depth_compare: Some(compare),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: MSAA,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &scene_shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: HDR,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        use wgpu::CompareFunction::{Always, Less};
        let scene_pipe = make_scene("scene", &scene_layout, "fs_main", None, (true, Less), true);
        let trans_pipe = make_scene(
            "trans",
            &trans_layout,
            "fs_trans",
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            (false, Less),
            false,
        );
        // Moonbeams test the depth themselves (`fs_beam`): standing in one, the haze
        // between the eye and a wall must stay where the shaft's far side is behind it.
        let beam_pipe = make_scene(
            "beams",
            &trans_layout,
            "fs_beam",
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            (false, Always),
            false,
        );

        // ---- post pipelines
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let samp_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let chain_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chain"),
            entries: &[tex_entry(0), samp_entry],
        });
        let tonemap_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tonemap"),
            entries: &[
                tex_entry(0),
                samp_entry,
                tex_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let chain_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chain"),
            bind_group_layouts: &[Some(&chain_layout)],
            immediate_size: 0,
        });
        let tonemap_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tonemap"),
            bind_group_layouts: &[Some(&tonemap_layout)],
            immediate_size: 0,
        });
        let make_post = |label: &str, layout: &wgpu::PipelineLayout, fs: &str, format, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let down_first_pipe = make_post("down_first", &chain_pl, "fs_down_first", HDR, None);
        let down_pipe = make_post("down", &chain_pl, "fs_down", HDR, None);
        let up_pipe = make_post("up", &chain_pl, "fs_up", HDR, Some(additive));
        let tonemap_pipe = make_post("tonemap", &tonemap_pl, "fs_tonemap", format, None);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let post_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: std::mem::size_of::<PostParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (verts, idx) = unit_box();
        let box_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("box vb"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let box_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("box ib"),
            contents: bytemuck::cast_slice(&idx),
            usage: wgpu::BufferUsages::INDEX,
        });

        let targets = Self::make_targets(
            &device,
            &config,
            &chain_layout,
            &tonemap_layout,
            &depth_layout,
            &sampler,
            &post_buf,
        );
        let timer = device.features().contains(wgpu::Features::TIMESTAMP_QUERY).then(|| {
            let buf = |label, usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: 3 * 8,
                    usage,
                    mapped_at_creation: false,
                })
            };
            use wgpu::BufferUsages as U;
            Timer {
                set: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("timestamps"),
                    ty: wgpu::QueryType::Timestamp,
                    count: 3,
                }),
                resolve: buf("timestamps", U::QUERY_RESOLVE | U::COPY_SRC),
                read: buf("timestamps read", U::MAP_READ | U::COPY_DST),
                period: queue.get_timestamp_period(),
            }
        });
        let egui = egui_wgpu::Renderer::new(
            &device,
            format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                ..Default::default()
            },
        );

        Self {
            static_buf: GrowBuf::new(&device, "static", 1024),
            dyn_buf: GrowBuf::new(&device, "dynamic", 16384),
            trans_buf: GrowBuf::new(&device, "trans", 16384),
            beam_buf: GrowBuf::new(&device, "beams", 64),
            split: Default::default(),
            held_buf: GrowBuf::new(&device, "held", 256),
            chunks: Vec::new(),
            timer,
            window,
            surface,
            device,
            queue,
            config,
            scene_pipe,
            trans_pipe,
            beam_pipe,
            down_first_pipe,
            down_pipe,
            up_pipe,
            tonemap_pipe,
            chain_layout,
            tonemap_layout,
            depth_layout,
            sampler,
            globals_buf,
            globals_bg,
            post_buf,
            box_vb,
            box_ib,
            targets,
            egui,
            capture: None,
            can_capture,
            acquire_ms: 0.0,
            gpu_ms: 0.0,
            gpu_scene_ms: 0.0,
            sync_gpu: false,
            drawn_chunks: 0,
            uploaded_chunks: 0,
        }
    }

    fn make_targets(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        chain_layout: &wgpu::BindGroupLayout,
        tonemap_layout: &wgpu::BindGroupLayout,
        depth_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        post_buf: &wgpu::Buffer,
    ) -> Targets {
        let tex = |label, w: u32, h: u32, format, samples, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: w.max(1),
                        height: h.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let (w, h) = (config.width, config.height);
        let rt = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let rts = rt | wgpu::TextureUsages::TEXTURE_BINDING;
        let msaa_color = tex("msaa color", w, h, HDR, MSAA, rt);
        let depth = tex("depth", w, h, DEPTH, MSAA, rts);
        let resolved = tex("resolved", w, h, HDR, 1, rts);
        let mut bloom = Vec::new();
        let (mut bw, mut bh) = (w, h);
        for _ in 0..BLOOM_LEVELS {
            bw = (bw / 2).max(1);
            bh = (bh / 2).max(1);
            bloom.push(tex("bloom", bw, bh, HDR, 1, rts));
        }
        let chain_bg = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("chain"),
                layout: chain_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        let mut chain_bgs = vec![chain_bg(&resolved)];
        for b in &bloom {
            chain_bgs.push(chain_bg(b));
        }
        let tonemap_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap"),
            layout: tonemap_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&resolved),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&bloom[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: post_buf.as_entire_binding(),
                },
            ],
        });
        let depth_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene depth"),
            layout: depth_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth),
            }],
        });
        Targets {
            msaa_color,
            depth,
            resolved,
            bloom,
            chain_bgs,
            tonemap_bg,
            depth_bg,
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.targets = Self::make_targets(
            &self.device,
            &self.config,
            &self.chain_layout,
            &self.tonemap_layout,
            &self.depth_layout,
            &self.sampler,
            &self.post_buf,
        );
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    fn save_capture(&self, path: &std::path::Path, buf: &wgpu::Buffer, row: u32) -> Result<(), String> {
        let (w, h) = (self.config.width, self.config.height);
        buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        let data = buf.slice(..).get_mapped_range();
        let bgra = matches!(
            self.config.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let line = &data[(y * row) as usize..(y * row + w * 4) as usize];
            for px in line.as_chunks::<4>().0 {
                if bgra {
                    pixels.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    pixels.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        let result = std::fs::File::create(path).map_err(|e| e.to_string()).and_then(|f| {
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().map_err(|e| e.to_string())?;
            wr.write_image_data(&pixels).map_err(|e| e.to_string())
        });
        match &result {
            Ok(()) => eprintln!("hushfold: saved {}", path.display()),
            Err(e) => eprintln!("hushfold: screenshot failed: {e}"),
        }
        result
    }

    pub fn set_static(&mut self, insts: &[Inst]) {
        self.static_buf.upload(&self.device, &self.queue, insts);
    }

    /// Upload the scene chunks that have changed (and forget any beyond them).
    pub fn sync_chunks(&mut self, chunks: &mut [crate::scene::Chunk]) {
        self.chunks.truncate(chunks.len());
        while self.chunks.len() < chunks.len() {
            self.chunks.push(ChunkGpu {
                opaque: GrowBuf::new(&self.device, "chunk", 64),
                trans: GrowBuf::new(&self.device, "chunk ghosts", 64),
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            });
        }
        self.uploaded_chunks = 0;
        for (g, c) in self.chunks.iter_mut().zip(chunks) {
            if c.upload {
                self.uploaded_chunks += 1;
                g.opaque.upload(&self.device, &self.queue, &c.opaque);
                g.trans.upload(&self.device, &self.queue, &c.trans);
                (g.min, g.max) = (c.min, c.max);
                c.upload = false;
            }
        }
    }

    /// Drop every chunk (a different library is coming).
    pub fn clear_chunks(&mut self) {
        self.chunks.clear();
    }

    /// Draw a frame; if it was captured (see `capture`), where to, or why not.
    pub fn render(&mut self, f: FrameInput) -> Option<Result<std::path::PathBuf, String>> {
        let t_acquire = std::time::Instant::now();
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return None;
            }
            _ => return None,
        };
        self.acquire_ms = t_acquire.elapsed().as_secs_f32() * 1000.0;
        let view = frame.texture.create_view(&Default::default());

        self.queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(f.globals));
        self.queue.write_buffer(&self.post_buf, 0, bytemuck::bytes_of(&f.post));
        self.dyn_buf.upload(&self.device, &self.queue, f.dynamic);
        let [beams, rest] = &mut self.split;
        beams.clear();
        rest.clear();
        for inst in f.trans {
            if inst.kind == kind::SHAFT {
                beams.push(*inst);
            } else {
                rest.push(*inst);
            }
        }
        self.trans_buf.upload(&self.device, &self.queue, rest);
        self.beam_buf.upload(&self.device, &self.queue, beams);
        self.held_buf.upload(&self.device, &self.queue, f.held);

        // Chunks in view, nearest first (so near books hide far ones early).
        let planes = frustum(Mat4::from_cols_array_2d(&f.globals.view_proj));
        let eye = Vec3::from_slice(&f.globals.cam_pos[..3]);
        let mut visible: Vec<(f32, usize)> = (self.chunks.iter().enumerate())
            .filter(|(_, c)| (c.opaque.len > 0 || c.trans.len > 0) && box_visible(&planes, c.min, c.max))
            .map(|(i, c)| (((c.min + c.max) * 0.5).distance_squared(eye), i))
            .collect();
        visible.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.drawn_chunks = visible.len();

        let mut enc = self.device.create_command_encoder(&Default::default());
        let t = &self.targets;
        let timer = self.timer.as_ref().filter(|_| self.sync_gpu);
        let stamp = |begin, end| {
            timer.map(|tm| wgpu::RenderPassTimestampWrites {
                query_set: &tm.set,
                beginning_of_pass_write_index: begin,
                end_of_pass_write_index: end,
            })
        };
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa_color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f.clear[0] as f64,
                            g: f.clear[1] as f64,
                            b: f.clear[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: stamp(Some(0), None),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_bind_group(0, &self.globals_bg, &[]);
            rp.set_vertex_buffer(0, self.box_vb.slice(..));
            rp.set_index_buffer(self.box_ib.slice(..), wgpu::IndexFormat::Uint16);
            rp.set_pipeline(&self.scene_pipe);
            // Walls and floors first: they hide whole rooms of books behind them.
            let opaque = visible.iter().map(|&(_, i)| &self.chunks[i].opaque);
            for b in std::iter::once(&self.static_buf).chain(opaque).chain([&self.dyn_buf]) {
                if b.len > 0 {
                    rp.set_vertex_buffer(1, b.buf.slice(..));
                    rp.draw_indexed(0..36, 0, 0..b.len);
                }
            }
        }
        {
            // Translucent things in a pass of their own, with the depth read-only, so they
            // can also read it.
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("trans"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa_color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: stamp(None, Some(1)),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_bind_group(0, &self.globals_bg, &[]);
            rp.set_bind_group(1, &t.depth_bg, &[]);
            rp.set_vertex_buffer(0, self.box_vb.slice(..));
            rp.set_index_buffer(self.box_ib.slice(..), wgpu::IndexFormat::Uint16);
            rp.set_pipeline(&self.trans_pipe);
            let ghosts = visible.iter().rev().map(|&(_, i)| &self.chunks[i].trans);
            for b in ghosts.chain([&self.trans_buf]) {
                if b.len > 0 {
                    rp.set_vertex_buffer(1, b.buf.slice(..));
                    rp.draw_indexed(0..36, 0, 0..b.len);
                }
            }
            if self.beam_buf.len > 0 {
                rp.set_pipeline(&self.beam_pipe);
                rp.set_vertex_buffer(1, self.beam_buf.buf.slice(..));
                rp.draw_indexed(0..36, 0, 0..self.beam_buf.len);
            }
        }
        {
            // Held books: cleared depth so they never clip into shelves; resolves MSAA.
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("held"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa_color,
                    depth_slice: None,
                    resolve_target: Some(&t.resolved),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.held_buf.len > 0 {
                rp.set_bind_group(0, &self.globals_bg, &[]);
                rp.set_vertex_buffer(0, self.box_vb.slice(..));
                rp.set_index_buffer(self.box_ib.slice(..), wgpu::IndexFormat::Uint16);
                rp.set_pipeline(&self.scene_pipe);
                rp.set_vertex_buffer(1, self.held_buf.buf.slice(..));
                rp.draw_indexed(0..36, 0, 0..self.held_buf.len);
            }
        }
        // Bloom: downsample chain then additive upsample back to level 0.
        let simple_pass = |enc: &mut wgpu::CommandEncoder,
                           target: &wgpu::TextureView,
                           pipe: &wgpu::RenderPipeline,
                           bg: &wgpu::BindGroup,
                           load: wgpu::LoadOp<wgpu::Color>| {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(pipe);
            rp.set_bind_group(0, bg, &[]);
            rp.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        for lvl in 0..BLOOM_LEVELS {
            let pipe = if lvl == 0 {
                &self.down_first_pipe
            } else {
                &self.down_pipe
            };
            simple_pass(&mut enc, &t.bloom[lvl], pipe, &t.chain_bgs[lvl], clear);
        }
        for lvl in (1..BLOOM_LEVELS).rev() {
            simple_pass(
                &mut enc,
                &t.bloom[lvl - 1],
                &self.up_pipe,
                &t.chain_bgs[lvl + 1],
                wgpu::LoadOp::Load,
            );
        }
        simple_pass(&mut enc, &view, &self.tonemap_pipe, &t.tonemap_bg, clear);

        // egui
        let sd = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: f.pixels_per_point,
        };
        for (id, delta) in &f.egui_textures.set {
            self.egui.update_texture(&self.device, &self.queue, *id, delta);
        }
        let extra = self
            .egui
            .update_buffers(&self.device, &self.queue, &mut enc, f.egui_prims, &sd);
        {
            let rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: stamp(None, Some(2)),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut rp = rp.forget_lifetime();
            self.egui.render(&mut rp, f.egui_prims, &sd);
        }
        for id in &f.egui_textures.free {
            self.egui.free_texture(id);
        }
        let mut captured = None;
        let capture = self.capture.take().filter(|_| {
            if !self.can_capture {
                eprintln!("hushfold: screenshot failed: the surface can't be copied from");
                captured = Some(Err("this display doesn't allow it".to_string()));
            }
            self.can_capture
        });
        let readback = capture.as_ref().map(|_| {
            let (w, h) = (self.config.width, self.config.height);
            let row = (w * 4).div_ceil(256) * 256;
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("capture"),
                size: (row * h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                frame.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(row),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            (buf, row)
        });
        if let Some(tm) = timer {
            enc.resolve_query_set(&tm.set, 0..3, &tm.resolve, 0);
            enc.copy_buffer_to_buffer(&tm.resolve, 0, &tm.read, 0, 3 * 8);
        }
        let t_gpu = std::time::Instant::now();
        self.queue
            .submit(extra.into_iter().chain(std::iter::once(enc.finish())));
        if self.sync_gpu {
            if let Some(tm) = &self.timer {
                tm.read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            }
            let _ = self.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            });
            self.gpu_ms = t_gpu.elapsed().as_secs_f32() * 1000.0;
            if let Some(tm) = &self.timer {
                let ticks: Vec<u64> = {
                    let data = tm.read.slice(..).get_mapped_range();
                    bytemuck::cast_slice::<u8, u64>(&data).to_vec()
                };
                tm.read.unmap();
                let ms = |a: u64, b: u64| b.saturating_sub(a) as f32 * tm.period / 1e6;
                self.gpu_ms = ms(ticks[0], ticks[2]);
                self.gpu_scene_ms = ms(ticks[0], ticks[1]);
            }
        }
        if let (Some(path), Some((buf, row))) = (capture, readback) {
            captured = Some(self.save_capture(&path, &buf, row).map(|()| path));
        }
        self.window.pre_present_notify();
        frame.present();
        captured
    }
}
