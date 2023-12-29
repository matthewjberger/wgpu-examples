use std::cmp::max;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn required_features() -> wgpu::Features {
    wgpu::Features::empty()
}

fn optional_features() -> wgpu::Features {
    wgpu::Features::empty()
}

#[derive(Default, Copy, Clone)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / max(self.height, 0) as f32
    }
}

fn main() {
    let (title, width, height) = ("Standalone Winit/Wgpu Example", 800, 600);

    let event_loop = EventLoop::new();

    let mut window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(width, height))
        .with_transparent(true)
        .build(&event_loop)
        .expect("Failed to create winit window!");

    let viewport = Viewport {
        width,
        height,
        ..Default::default()
    };

    let (surface, device, queue, surface_config) = pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");

        fn required_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
            // Use the texture resolution limits from the adapter
            // to support images the size of the surface
            wgpu::Limits::default().using_resolution(adapter.limits())
        }

        let (device, queue) = {
            println!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        features: (optional_features() & adapter.features()) | required_features(),
                        limits: required_limits(&adapter),
                        label: Some("Render Device"),
                    },
                    None,
                )
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        // This assumes an sRGB surface texture
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: viewport.width,
            height: viewport.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        (surface, device, queue, surface_config)
    });

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // Draw
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::KeyboardInput { input, .. } => {
                    if let (Some(VirtualKeyCode::Escape), ElementState::Pressed) =
                        (input.virtual_keycode, input.state)
                    {
                        *control_flow = ControlFlow::Exit;
                    }

                    if let Some(keycode) = input.virtual_keycode.as_ref() {
                        // Handle a key press
                    }
                }

                WindowEvent::MouseInput { button, state, .. } => {
                    // Handle a mouse button press
                }

                WindowEvent::Resized(PhysicalSize { width, height }) => {
                    // Handle resizing
                }
                _ => {}
            },
            Event::LoopDestroyed => {
                // Handle cleanup
            }
            _ => {}
        }
    });
}
