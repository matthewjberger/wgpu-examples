use crate::GuiRender;
use anyhow::{anyhow, Context, Result};
use egui::{ClippedPrimitive, TexturesDelta};
use egui_wgpu::renderer::ScreenDescriptor;
use raw_window_handle::HasRawWindowHandle;
use std::cmp::max;
use wgpu::{
    CommandEncoder, Device, Queue, Surface, SurfaceConfiguration, TextureView,
    TextureViewDescriptor,
};

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

pub struct Renderer {
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub gui: GuiRender,
}

impl Renderer {
    pub fn new(window_handle: &impl HasRawWindowHandle, viewport: &Viewport) -> Result<Self> {
        pollster::block_on(Renderer::new_async(window_handle, viewport))
    }

    pub fn update(
        &mut self,
        textures_delta: &TexturesDelta,
        screen_descriptor: &ScreenDescriptor,
        paint_jobs: &[ClippedPrimitive],
    ) -> Result<()> {
        self.gui
            .update_textures(&self.device, &self.queue, &textures_delta);
        self.gui
            .update_buffers(&self.device, &self.queue, screen_descriptor, paint_jobs);
        Ok(())
    }

    pub fn resize(&mut self, dimensions: [u32; 2]) {
        log::info!(
            "Resizing renderer surface to: ({}, {})",
            dimensions[0],
            dimensions[1]
        );
        if dimensions[0] == 0 || dimensions[1] == 0 {
            return;
        }
        self.config.width = dimensions[0];
        self.config.height = dimensions[1];
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render_frame(
        &mut self,
        paint_jobs: &[ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
        mut action: impl FnMut(&TextureView, &mut CommandEncoder) -> Result<()>,
    ) -> Result<()> {
        let surface_texture = self.surface.get_current_texture()?;

        let view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        action(&view, &mut encoder)?;

        self.gui
            .execute(&mut encoder, &view, paint_jobs, screen_descriptor, None);

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / std::cmp::max(1, self.config.height) as f32
    }

    async fn new_async(
        window_handle: &impl HasRawWindowHandle,
        viewport: &Viewport,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(Self::backends());

        let surface = unsafe { instance.create_surface(window_handle) };

        let adapter = Self::create_adapter(&instance, &surface).await?;

        let (device, queue) = Self::request_device(&adapter).await?;

        let swapchain_format = *surface
            .get_supported_formats(&adapter)
            .first()
            .ok_or(anyhow!("Failed to find a support swapchain format!"))?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: viewport.width,
            height: viewport.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let gui = GuiRender::new(&device, config.format, 1);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            gui,
        })
    }

    fn backends() -> wgpu::Backends {
        wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all)
    }

    fn required_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
        wgpu::Limits::default()
            // Use the texture resolution limits from the adapter
            // to support images the size of the surface
            .using_resolution(adapter.limits())
    }

    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
    }

    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }

    async fn create_adapter(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface,
    ) -> Result<wgpu::Adapter> {
        wgpu::util::initialize_adapter_from_env_or_default(
            instance,
            Self::backends(),
            Some(surface),
        )
        .await
        .context("No suitable GPU adapters found on the system!")
    }

    async fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue)> {
        log::info!("WGPU Adapter Features: {:#?}", adapter.features());

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: (Self::optional_features() & adapter.features())
                        | Self::required_features(),
                    limits: Self::required_limits(adapter),
                    label: Some("Render Device"),
                },
                None,
            )
            .await
            .context("Failed to request a device!")
    }
}
