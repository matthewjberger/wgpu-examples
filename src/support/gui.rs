use anyhow::Result;
use egui::{ClippedPrimitive, Context as GuiContext, FullOutput, TexturesDelta};
use egui_wgpu::{renderer::ScreenDescriptor, Renderer};
use egui_winit::{EventResponse, State};
use wgpu::{CommandEncoder, Device, Queue};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

pub struct Gui {
    pub state: State,
    pub context: GuiContext,
}

impl Gui {
    pub fn new<T>(window: &Window, event_loop: &EventLoopWindowTarget<T>) -> Self {
        let state = State::new(&event_loop);
        let context = GuiContext::default();
        context.set_pixels_per_point(window.scale_factor() as f32);
        Self { state, context }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> EventResponse {
        let Gui { state, context } = self;
        state.on_event(context, event)
    }

    pub fn create_frame(
        &mut self,
        window: &Window,
        mut action: impl FnMut(&mut GuiContext) -> Result<()>,
    ) -> Result<FullOutput> {
        self.begin_frame(window);
        action(&mut self.context)?;
        Ok(self.end_frame())
    }

    fn begin_frame(&mut self, window: &Window) {
        let gui_input = self.state.take_egui_input(window);
        self.context.begin_frame(gui_input);
    }

    fn end_frame(&mut self) -> FullOutput {
        self.context.end_frame()
    }
}

#[derive(Default)]
pub struct GuiRender {
    renderer: Option<Renderer>,
}

impl GuiRender {
    pub fn initialize(
        &mut self,
        device: &Device,
        target_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
    ) {
        self.renderer = Some(Renderer::new(
            device,
            target_format,
            depth_format,
            msaa_samples,
        ));
    }

    pub fn initialized(&self) -> bool {
        self.renderer.is_some()
    }

    pub fn update_textures(
        &mut self,
        device: &Device,
        queue: &Queue,
        textures_delta: &TexturesDelta,
    ) {
        let renderer = match self.renderer.as_mut() {
            Some(renderer) => renderer,
            None => return,
        };

        for (id, image_delta) in &textures_delta.set {
            renderer.update_texture(device, queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            renderer.free_texture(id);
        }
    }

    pub fn update_buffers(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        screen_descriptor: &ScreenDescriptor,
        paint_jobs: &[ClippedPrimitive],
    ) {
        let renderer = match self.renderer.as_mut() {
            Some(renderer) => renderer,
            None => return,
        };

        renderer.update_buffers(device, queue, encoder, paint_jobs, screen_descriptor);
    }

    pub fn render<'rp>(
        &'rp mut self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        screen_descriptor: &ScreenDescriptor,
        paint_jobs: &'rp [ClippedPrimitive],
    ) {
        let renderer = match self.renderer.as_mut() {
            Some(renderer) => renderer,
            None => return,
        };

        renderer.render(render_pass, paint_jobs, screen_descriptor);
    }
}

pub fn create_screen_descriptor(window: &Window) -> ScreenDescriptor {
    let window_size = window.inner_size();
    ScreenDescriptor {
        size_in_pixels: [window_size.width, window_size.height],
        pixels_per_point: window.scale_factor() as f32,
    }
}
