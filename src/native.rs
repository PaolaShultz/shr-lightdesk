//! Explicit opt-in native window. Default/offscreen commands never construct this backend.
use crate::{
    frontend::{self, Worker},
    native_actions::Semantic,
    render,
    surface::{Page, actions::Action},
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

/// Centered aspect-preserving viewport; zero-sized windows suspend presentation.
pub fn viewport(width: u32, height: u32) -> Option<(f32, f32, f32, f32)> {
    if width == 0 || height == 0 {
        return None;
    }
    let scale = (width as f32 / render::WIDTH as f32).min(height as f32 / render::HEIGHT as f32);
    let scale = if scale >= 1.0 { scale.floor() } else { scale };
    let w = render::WIDTH as f32 * scale;
    let h = render::HEIGHT as f32 * scale;
    Some(((width as f32 - w) / 2.0, (height as f32 - h) / 2.0, w, h))
}
struct Graphics {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    bind: wgpu::BindGroup,
    lost: Arc<AtomicBool>,
}
fn resources(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::BindGroup, wgpu::RenderPipeline) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Scene/font CPU raster"),
        size: wgpu::Extent3d {
            width: render::WIDTH,
            height: render::HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let view = texture.create_view(&Default::default());
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:None,source:wgpu::ShaderSource::Wgsl(r#"
struct Vertex { @builtin(position) position: vec4f, @location(0) uv: vec2f }
@vertex fn vs(@builtin(vertex_index) id:u32)->Vertex {
 var points=array<vec2f,3>(vec2f(-1.,-1.),vec2f(3.,-1.),vec2f(-1.,3.));
 var v:Vertex; v.position=vec4f(points[id],0.,1.); v.uv=vec2f((points[id].x+1.)/2.,(1.-points[id].y)/2.); return v;
}
@group(0) @binding(0) var image:texture_2d<f32>;
@group(0) @binding(1) var filtering:sampler;
@fragment fn fs(v:Vertex)->@location(0) vec4f {return textureSample(image,filtering,v.uv);}
"#.into())});
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs",
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
    });
    (texture, bind, pipeline)
}
fn upload(queue: &wgpu::Queue, texture: &wgpu::Texture, scene: &render::Scene) {
    let ppm = render::ppm(scene);
    let header = format!("P6\n{} {}\n255\n", render::WIDTH, render::HEIGHT).len();
    let mut rgba = Vec::with_capacity((render::WIDTH * render::HEIGHT * 4) as usize);
    for rgb in ppm[header..].chunks_exact(3) {
        rgba.extend_from_slice(rgb);
        rgba.push(255);
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(render::WIDTH * 4),
            rows_per_image: Some(render::HEIGHT),
        },
        wgpu::Extent3d {
            width: render::WIDTH,
            height: render::HEIGHT,
            depth_or_array_layers: 1,
        },
    );
}
impl Graphics {
    async fn new(window: Arc<Window>) -> Result<Self, String> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No compatible adapter")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Lightdesk"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        device.set_device_lost_callback(move |_, _| {
            flag.store(true, Ordering::Release);
        });
        let flag = lost.clone();
        device.on_uncaptured_error(Box::new(move |_| {
            flag.store(true, Ordering::Release);
        }));
        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *caps.formats.first().ok_or("No surface format")?,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let (texture, bind, pipeline) = resources(&device, config.format);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            texture,
            bind,
            lost,
        })
    }
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    fn draw(&mut self, scene: &render::Scene) -> Result<(), wgpu::SurfaceError> {
        if self.lost.load(Ordering::Acquire) {
            return Err(wgpu::SurfaceError::Lost);
        }
        upload(&self.queue, &self.texture, scene);
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (x, y, w, h) = viewport(self.config.width, self.config.height).unwrap();
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
}
struct App {
    worker: Worker,
    window: Option<Arc<Window>>,
    graphics: Option<Graphics>,
    keyboard: frontend::Keyboard,
    notice: String,
    fatal_error: Option<String>,
    help: bool,
    command_mode: bool,
    semantic_editing: bool,
    review_seen: Option<(u64, usize, usize)>,
    review_context: Option<u64>,
    last: Instant,
}
impl App {
    fn send(&mut self, line: String) {
        self.notice.clear();
        self.help = false;
        if let Err(e) = self.worker.enqueue(line) {
            self.notice = e;
            self.keyboard.draft.clear();
        }
    }
    fn action(&mut self, action: Semantic) {
        self.notice.clear();
        self.help = false;
        if let Err(e) = self.worker.action(action) {
            self.notice = e;
            self.lost();
        }
    }
    fn lost(&mut self) {
        self.worker.invalidate();
        self.keyboard.context_lost();
        self.review_seen = None;
        self.review_context = None;
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        match event_loop.create_window(
            Window::default_attributes()
                .with_title("SHR Lightdesk / real Lux / physical unknown")
                .with_inner_size(winit::dpi::LogicalSize::new(1280., 720.)),
        ) {
            Ok(window) => {
                let window = Arc::new(window);
                match pollster::block_on(Graphics::new(window.clone())) {
                    Ok(g) => self.graphics = Some(g),
                    Err(e) => {
                        self.fatal_error = Some(e);
                        event_loop.exit();
                    }
                }
                self.window = Some(window);
            }
            Err(e) => {
                self.fatal_error = Some(e.to_string());
                event_loop.exit();
            }
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.lost();
        self.graphics = None;
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.keyboard.focus(focused);
                self.lost();
            }
            WindowEvent::Resized(size) => {
                self.lost();
                if let Some(g) = &mut self.graphics {
                    g.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.logical_key == Key::Named(NamedKey::Enter)
                    && event.state == ElementState::Released
                {
                    self.keyboard.release_enter();
                    self.action(Semantic::EnterUp);
                    return;
                }
                if !self.keyboard.focused || event.repeat || event.state != ElementState::Pressed {
                    return;
                }
                match event.logical_key {
                    Key::Named(NamedKey::Enter) => {
                        if let Some(line) = self.keyboard.enter() {
                            if self.command_mode {
                                self.send(line);
                                self.command_mode = false;
                            } else {
                                self.action(Semantic::Action(Action::Confirm));
                                self.semantic_editing = false;
                            }
                        }
                    }
                    Key::Named(NamedKey::Escape) => {
                        self.keyboard.draft.clear();
                        self.command_mode = false;
                        self.semantic_editing = false;
                        self.action(Semantic::Action(Action::Cancel));
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if self.command_mode {
                            self.keyboard.draft.pop();
                        } else {
                            self.action(Semantic::Action(Action::Backspace));
                        }
                    }
                    Key::Named(NamedKey::PageDown) => {
                        self.keyboard.scroll = self.keyboard.scroll.saturating_add(20)
                    }
                    Key::Named(NamedKey::PageUp) => {
                        self.keyboard.scroll = self.keyboard.scroll.saturating_sub(20)
                    }
                    Key::Named(NamedKey::F12) => {
                        self.help = !self.help;
                        self.keyboard.scroll = 0;
                    }
                    Key::Named(NamedKey::F1) => {
                        self.action(Semantic::Action(Action::Page(Page::Stage)))
                    }
                    Key::Named(NamedKey::F2) => {
                        self.action(Semantic::Action(Action::Page(Page::Programmer)))
                    }
                    Key::Named(NamedKey::F3) => {
                        self.action(Semantic::Action(Action::Page(Page::Library)))
                    }
                    Key::Named(NamedKey::F4) => {
                        self.action(Semantic::Action(Action::Page(Page::Playbacks)))
                    }
                    Key::Named(NamedKey::F5) => {
                        self.action(Semantic::Action(Action::Page(Page::Health)))
                    }
                    Key::Named(NamedKey::F6) => self.action(Semantic::Grant),
                    Key::Named(NamedKey::Home) => self.action(Semantic::Bank(-1)),
                    Key::Named(NamedKey::End) => self.action(Semantic::Bank(1)),
                    Key::Named(NamedKey::ArrowRight) => {
                        self.action(Semantic::Action(Action::Navigate(1)))
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.action(Semantic::Action(Action::Navigate(-1)))
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.action(Semantic::Action(Action::Attribute(-1)))
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        self.action(Semantic::Action(Action::Attribute(1)))
                    }
                    Key::Named(NamedKey::Space) => {
                        if self.command_mode {
                            self.keyboard.text(" ");
                        } else if self.semantic_editing
                            || self.worker.view.lock().unwrap().editor.is_some()
                        {
                            self.action(Semantic::Action(Action::Text(" ".into())));
                        } else {
                            self.action(Semantic::Action(Action::Select(vec![])));
                        }
                    }
                    Key::Character(text) => {
                        if self.command_mode {
                            self.keyboard.text(&text);
                        } else if self.semantic_editing
                            || self.worker.view.lock().unwrap().editor.is_some()
                        {
                            self.action(Semantic::Action(Action::Text(text.to_string())));
                        } else if text.as_str() == "R"
                            || text.as_str() == "u"
                            || text.as_str() == "U"
                        {
                            self.semantic_editing = true;
                            self.action(Semantic::Action(Action::Record {
                                palette: text.as_str() == "R" || text.as_str() == "U",
                                replace: text.as_str() == "u" || text.as_str() == "U",
                                id: 0,
                            }));
                        } else {
                            match text.to_ascii_lowercase().as_str() {
                                ":" => {
                                    self.command_mode = true;
                                    self.keyboard.draft.clear();
                                }
                                "e" => {
                                    self.semantic_editing = true;
                                    self.action(Semantic::Edit);
                                }
                                "r" => {
                                    self.semantic_editing = true;
                                    self.action(Semantic::Action(Action::Record {
                                        palette: false,
                                        replace: false,
                                        id: 0,
                                    }));
                                }
                                "g" => {
                                    self.semantic_editing = true;
                                    self.action(Semantic::OpenGo);
                                }
                                "c" => self.action(Semantic::Action(Action::ClearHold)),
                                "p" => self.action(Semantic::Preview),
                                "b" => self.action(Semantic::Blackout),
                                "m" => self.action(Semantic::Mode),
                                "k" => self.action(Semantic::Calibrate(false)),
                                "l" => self.action(Semantic::Calibrate(true)),
                                "a" => self.action(Semantic::AutoEnter),
                                "t" => {
                                    self.semantic_editing = true;
                                    self.action(Semantic::AutoGrant);
                                }
                                "x" => self.action(Semantic::AutoRevoke),
                                "d" => self.action(Semantic::Action(Action::Deselect)),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                // A redraw queued before minimization is not a presented review.
                let Some(window) = &self.window else { return };
                let size = window.inner_size();
                if viewport(size.width, size.height).is_none() {
                    return;
                }
                let mut view = self.worker.view.lock().unwrap().current();
                if !self.notice.is_empty() {
                    view.notice = self.notice.clone();
                }
                if self.help {
                    view.page = "help".into();
                }
                if view.review_context != self.review_context {
                    self.keyboard.scroll = 0;
                    self.review_seen = None;
                    self.review_context = view.review_context;
                }
                let scene = frontend::scene(&view, &self.keyboard.draft, self.keyboard.scroll);
                let mut presented = false;
                if let Some(g) = &mut self.graphics {
                    match g.draw(&scene) {
                        Ok(()) => {
                            presented = true;
                        }
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            self.lost();
                            self.graphics = None;
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            self.lost();
                            self.fatal_error = Some("Renderer out of memory".into());
                            event_loop.exit();
                        }
                        Err(wgpu::SurfaceError::Timeout) => {
                            self.notice = "Renderer timeout; state retained".into()
                        }
                    }
                }
                if presented
                    && let Some((context, start, through)) =
                        frontend::visible_review_range(&view, self.keyboard.scroll)
                    && self.review_seen != Some((context, start, through))
                    && self
                        .worker
                        .action(Semantic::Reviewed {
                            context,
                            start,
                            through,
                        })
                        .is_ok()
                {
                    self.review_seen = Some((context, start, through));
                }
                if self.graphics.is_none()
                    && let Some(w) = &self.window
                {
                    match pollster::block_on(Graphics::new(w.clone())) {
                        Ok(g) => self.graphics = Some(g),
                        Err(e) => {
                            self.fatal_error = Some(e);
                            event_loop.exit();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let next = self.last + Duration::from_millis(100);
        if Instant::now() >= next {
            if let Some(w) = &self.window
                && w.inner_size().width > 0
                && w.inner_size().height > 0
            {
                w.request_redraw();
            }
            self.last = Instant::now();
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            self.last + Duration::from_millis(100),
        ));
    }
}
/// Creates a real window only on this explicit command. Never called by tests.
pub fn run(
    path: PathBuf,
    show: String,
    epoch: u64,
    role: Option<crate::role_client::Config>,
) -> Result<(), String> {
    let worker = Worker::spawn_with_role(path, show, epoch, role);
    let mut app = App {
        worker,
        window: None,
        graphics: None,
        keyboard: frontend::Keyboard::default(),
        notice: String::new(),
        fatal_error: None,
        help: false,
        command_mode: false,
        semantic_editing: false,
        review_seen: None,
        review_context: None,
        last: Instant::now(),
    };
    EventLoop::new()
        .map_err(|e| e.to_string())?
        .run_app(&mut app)
        .map_err(|e| e.to_string())?;
    app.fatal_error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires explicit Mesa CPU ICD; opens no window; run with VK_ICD_FILENAMES"]
    fn software_wgpu_upload_shader_readback() {
        assert_eq!(
            std::env::var("VK_ICD_FILENAMES").unwrap(),
            "/usr/share/vulkan/icd.d/lvp_icd.json"
        );
        assert!(std::env::var_os("DISPLAY").is_none());
        assert!(std::env::var_os("WAYLAND_DISPLAY").is_none());
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
                ..Default::default()
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    force_fallback_adapter: true,
                    ..Default::default()
                })
                .await
                .expect("Mesa CPU adapter");
            assert_eq!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("CPU-only offscreen"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                    },
                    None,
                )
                .await
                .unwrap();
            let format = wgpu::TextureFormat::Rgba8UnormSrgb;
            let (texture, bind, pipeline) = resources(&device, format);
            let mut scene = render::Scene::default();
            scene.rect(0, 0, render::WIDTH, render::HEIGHT, "#10151d");
            scene.rect(0, 0, render::WIDTH / 2, render::HEIGHT, "#66dfd3");
            scene.text(24, 24, "REAL LUX / PHYSICAL UNKNOWN", "#e4e8e9");
            upload(&queue, &texture, &scene);
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 64,
                    height: 36,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 64 * 36 * 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.draw(0..3, 0..1);
            }
            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &target,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(36),
                    },
                },
                wgpu::Extent3d {
                    width: 64,
                    height: 36,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
            device.poll(wgpu::Maintain::Wait);
            rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
            let bytes = buffer.slice(..).get_mapped_range();
            assert_eq!(
                &bytes[(18 * 64 + 10) * 4..(18 * 64 + 10) * 4 + 4],
                &[102, 223, 211, 255]
            );
            assert_eq!(
                &bytes[(18 * 64 + 50) * 4..(18 * 64 + 50) * 4 + 4],
                &[16, 21, 29, 255]
            );
            let sum: u64 = bytes.iter().map(|b| u64::from(*b)).sum();
            eprintln!(
                "CPU adapter {:?}; offscreen64x36 checksum_sum={sum}",
                adapter.get_info()
            );
            drop(bytes);
            buffer.unmap();
        });
    }
}
