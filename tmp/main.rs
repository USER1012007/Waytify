mod audio;
mod ui;

use anyhow::Result;
use egui_wgpu::Renderer;
use egui_winit::State as EguiWinitState;
use std::sync::Arc;
use ui::app::MusicApp;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    egui_ctx: egui::Context,
    egui_state: Option<EguiWinitState>,
    egui_renderer: Option<Renderer>,
    music_app: MusicApp,
    _audio: audio::player::AudioPlayer,
}

impl App {
    fn new(audio: audio::player::AudioPlayer) -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            egui_renderer: None,
            music_app: MusicApp::default(),
            _audio: audio,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Create window
        let window_attrs = WindowAttributes::default()
            .with_title("Waytify - Music Player")
            .with_inner_size(winit::dpi::LogicalSize::new(400.0, 300.0));

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        // Setup GPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(&*window).unwrap();
            instance.create_surface_unsafe(target).unwrap()
        };

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Setup egui
        let viewport_id = self.egui_ctx.viewport_id();
        let egui_state = EguiWinitState::new(
            self.egui_ctx.clone(),
            viewport_id,
            &window,
            None,
            None,
            None,
        );

        let egui_renderer = Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.egui_state = Some(egui_state);
        self.egui_renderer = Some(egui_renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else { return };
        let Some(egui_state) = &mut self.egui_state else {
            return;
        };

        let response = egui_state.on_window_event(window, &event);

        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let (Some(config), Some(surface), Some(device)) =
                    (&mut self.config, &self.surface, &self.device)
                {
                    config.width = physical_size.width.max(1);
                    config.height = physical_size.height.max(1);
                    surface.configure(device, config);
                    window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                let egui_state = self.egui_state.as_mut().unwrap();
                let egui_renderer = self.egui_renderer.as_mut().unwrap();
                let device = self.device.as_ref().unwrap();
                let queue = self.queue.as_ref().unwrap();
                let surface = self.surface.as_ref().unwrap();
                let config = self.config.as_ref().unwrap();

                let raw_input = egui_state.take_egui_input(window);

                let output = self.egui_ctx.run(raw_input, |ctx| {
                    self.music_app.ui(ctx);
                });

                egui_state.handle_platform_output(window, output.platform_output.clone());

                let clipped_primitives = self
                    .egui_ctx
                    .tessellate(output.shapes, output.pixels_per_point);

                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [config.width, config.height],
                    pixels_per_point: output.pixels_per_point,
                };

                // Update textures
                for (id, delta) in &output.textures_delta.set {
                    egui_renderer.update_texture(device, queue, *id, delta);
                }

                // Get next surface texture
                let Ok(frame) = surface.get_current_texture() else {
                    eprintln!("Surface texture lost / timeout");
                    return;
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui encoder"),
                });

                egui_renderer.update_buffers(
                    device,
                    queue,
                    &mut encoder,
                    &clipped_primitives,
                    &screen_descriptor,
                );

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui render pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.06,
                                    g: 0.06,
                                    b: 0.08,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    egui_renderer.render(&mut rpass, &clipped_primitives, &screen_descriptor);
                }

                queue.submit(Some(encoder.finish()));
                frame.present();

                // Cleanup
                for id in output.textures_delta.free {
                    egui_renderer.free_texture(&id);
                }

                if output
                    .viewport_output
                    .get(&self.egui_ctx.viewport_id())
                    .is_some_and(|v| !v.repaint_delay.is_zero())
                {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    // Initialize audio
    let audio = audio::player::AudioPlayer::new()?;

    // Create event loop
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    // Create app
    let mut app = App::new(audio);

    // Run event loop
    event_loop.run_app(&mut app)?;

    Ok(())
}
