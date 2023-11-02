use anyhow::Result;
use egui::{ClippedPrimitive, Context as GuiContext, FullOutput, TexturesDelta};
use egui_wgpu::renderer::ScreenDescriptor;
use egui_winit::{EventResponse, State};
use wgpu::{Device, Queue, RenderPass};
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
    // pub gui_renderpass: RenderPass<'static>,
}

impl GuiRender {
    pub fn new(device: &Device, output_format: wgpu::TextureFormat, msaa_samples: u32) -> Self {
        Self::default()
        // let gui_renderpass = RenderPass::new(&device, output_format, msaa_samples);
        // Self { gui_renderpass }
    }

    pub fn update_textures(
        &mut self,
        device: &Device,
        queue: &Queue,
        textures_delta: &TexturesDelta,
    ) {
        // for (id, image_delta) in &textures_delta.set {
        //     self.gui_renderpass
        //         .update_texture(&device, &queue, *id, image_delta);
        // }
        // for id in &textures_delta.free {
        //     self.gui_renderpass.free_texture(id);
        // }
    }

    pub fn update_buffers(
        &mut self,
        device: &Device,
        queue: &Queue,
        screen_descriptor: &ScreenDescriptor,
        paint_jobs: &[ClippedPrimitive],
    ) {
        // self.gui_renderpass
        //     .update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);
    }

    pub fn execute<'a>(
        &'a self,
        encoder: &mut wgpu::CommandEncoder,
        color_attachment: &wgpu::TextureView,
        paint_jobs: &'a [egui::epaint::ClippedPrimitive],
        screen_descriptor: &'a ScreenDescriptor,
        clear_color: Option<wgpu::Color>,
    ) {
        // self.gui_renderpass.execute(
        //     encoder,
        //     color_attachment,
        //     paint_jobs,
        //     screen_descriptor,
        //     clear_color,
        // );
    }
}

pub fn create_screen_descriptor(window: &Window) -> ScreenDescriptor {
    let window_size = window.inner_size();
    ScreenDescriptor {
        size_in_pixels: [window_size.width, window_size.height],
        pixels_per_point: window.scale_factor() as f32,
    }
}
