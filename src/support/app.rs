use anyhow::Result;
use egui::{Context as GuiContext, FullOutput};
use wgpu::{CommandEncoder, TextureView};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{create_screen_descriptor, Gui, Input, Renderer, System, Viewport};

pub struct Resources<'a> {
    pub application: &'a mut (dyn Application + 'static),
    pub gui: &'a mut Gui,
    pub input: &'a mut Input,
    pub system: &'a mut System,
    pub renderer: &'a mut Renderer,
    pub window: &'a mut Window,
}

pub trait Application {
    fn initialize(&mut self, _renderer: &mut Renderer) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _renderer: &mut Renderer, _input: &Input, _system: &System) -> Result<()> {
        Ok(())
    }

    fn update_gui(&mut self, _context: &mut GuiContext) -> Result<()> {
        Ok(())
    }

    fn resize(&mut self, _renderer: &mut Renderer) -> Result<()> {
        Ok(())
    }

    fn render(&mut self, _view: &TextureView, _encoder: &mut CommandEncoder) -> Result<()> {
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_mouse(&mut self, _button: &MouseButton, _button_state: &ElementState) -> Result<()> {
        Ok(())
    }

    fn on_key(&mut self, _keycode: &VirtualKeyCode, _keystate: &ElementState) -> Result<()> {
        Ok(())
    }

    fn handle_event(&mut self, _event: &Event<()>, _window: &Window) -> Result<()> {
        Ok(())
    }
}

pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub fn run(mut application: impl Application + 'static, config: AppConfig) -> Result<()> {
    env_logger::init();
    log::info!("App started");

    let event_loop = EventLoop::new();
    let mut window = WindowBuilder::new()
        .with_title(config.title)
        .with_inner_size(PhysicalSize::new(config.width, config.height))
        .with_transparent(true)
        .build(&event_loop)?;

    let mut renderer = Renderer::new(
        &window,
        &Viewport {
            width: config.width,
            height: config.height,
            ..Default::default()
        },
    )?;

    let mut gui = Gui::new(&window, &event_loop);

    let window_dimensions = window.inner_size();
    let mut input = Input::default();
    let mut system = System::new(window_dimensions);

    application.initialize(&mut renderer)?;

    event_loop.run(move |event, _, control_flow| {
        let mut resources = Resources {
            application: &mut application,
            gui: &mut gui,
            input: &mut input,
            system: &mut system,
            renderer: &mut renderer,
            window: &mut window,
        };
        if let Err(error) = run_loop(&mut resources, &event, control_flow) {
            log::error!("Application error: {}", error);
        }
    });
}

fn run_loop(
    resources: &mut Resources,
    event: &Event<()>,
    control_flow: &mut ControlFlow,
) -> Result<()> {
    let Resources {
        application,
        gui,
        renderer,
        input,
        system,
        window,
    } = resources;

    system.handle_event(event);
    input.handle_event(event, system.window_center());

    match event {
        Event::MainEventsCleared => {
            let output = gui.create_frame(window, |context| application.update_gui(context))?;
            let FullOutput {
                textures_delta,
                shapes,
                ..
            } = output;
            let paint_jobs = gui.context.tessellate(shapes);
            let screen_descriptor = create_screen_descriptor(&window);
            renderer.update(&textures_delta, &screen_descriptor, &paint_jobs)?;

            application.update(renderer, input, system)?;
            renderer.render_frame(&paint_jobs, &screen_descriptor, |view, encoder| {
                application.render(view, encoder)
            })?;
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if *window_id == window.id() => {
            gui.handle_window_event(event);

            match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    if let (Some(VirtualKeyCode::Escape), ElementState::Pressed) =
                        (input.virtual_keycode, input.state)
                    {
                        *control_flow = ControlFlow::Exit;
                    }

                    if let Some(keycode) = input.virtual_keycode.as_ref() {
                        application.on_key(keycode, &input.state)?;
                    }
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    application.on_mouse(button, state)?
                }
                WindowEvent::Resized(physical_size) => {
                    renderer.resize([physical_size.width, physical_size.height]);
                    application.resize(renderer)?;
                }
                _ => {}
            }
        }
        Event::LoopDestroyed => {
            application.cleanup()?;
        }
        _ => {}
    }

    application.handle_event(event, window)?;

    Ok(())
}
