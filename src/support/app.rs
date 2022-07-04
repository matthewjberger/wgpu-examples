use anyhow::Result;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{Renderer, Viewport};

pub trait Application {
    fn initialize(&mut self, _window: &mut Window) -> Result<()> {
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        Ok(())
    }

    fn render(&mut self, _renderer: &mut Renderer) -> Result<()> {
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
        .build(&event_loop)?;

    let mut renderer = Renderer::new(
        &window,
        &Viewport {
            width: config.width,
            height: config.height,
            ..Default::default()
        },
    )?;

    application.initialize(&mut window)?;

    event_loop.run(move |event, _, control_flow| {
        if let Err(error) = run_loop(
            &mut application,
            &mut renderer,
            &mut window,
            &event,
            control_flow,
        ) {
            log::error!("Application error: {}", error);
        }
    });
}

fn run_loop(
    application: &mut impl Application,
    renderer: &mut Renderer,
    window: &mut Window,
    event: &Event<()>,
    control_flow: &mut ControlFlow,
) -> Result<()> {
    match event {
        Event::MainEventsCleared => {
            application.update()?;
            application.render(renderer)?;
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if *window_id == window.id() => match event {
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
            WindowEvent::MouseInput { button, state, .. } => application.on_mouse(button, state)?,
            WindowEvent::Resized(physical_size) => {
                renderer.resize([physical_size.width, physical_size.height]);
            }
            _ => {}
        },
        Event::LoopDestroyed => {
            application.cleanup()?;
        }
        _ => {}
    }

    application.handle_event(event, window)?;

    Ok(())
}
