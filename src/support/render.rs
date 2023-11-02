use crate::GuiRender;
use anyhow::{Context, Result};
use egui::{ClippedPrimitive, TexturesDelta};
use egui_wgpu::renderer::ScreenDescriptor;
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
    pub fn new<W>(window_handle: &W, viewport: &Viewport) -> Result<Self>
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        pollster::block_on(Renderer::new_async(window_handle, viewport))
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
        textures_delta: &TexturesDelta,
        paint_jobs: &[ClippedPrimitive],
        depth_format: Option<wgpu::TextureFormat>,
        screen_descriptor: &ScreenDescriptor,
        mut action: impl FnMut(&TextureView, &mut CommandEncoder, &mut GuiRender) -> Result<()>,
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

        if !self.gui.initialized() {
            self.gui
                .initialize(&self.device, self.config.format, depth_format, 1);
        }

        self.gui
            .update_textures(&self.device, &self.queue, textures_delta);
        self.gui.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            screen_descriptor,
            paint_jobs,
        );

        action(&view, &mut encoder, &mut self.gui)?;

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / std::cmp::max(1, self.config.height) as f32
    }

    async fn new_async<W>(window_handle: &W, viewport: &Viewport) -> Result<Self>
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: Self::backends(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window_handle) }.unwrap();

        let adapter = Self::create_adapter(&instance, &surface).await.unwrap();

        let (device, queue) = Self::request_device(&adapter).await?;

        let surface_capabilities = surface.get_capabilities(&adapter);

        // This assumes an sRGB surface texture
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: viewport.width,
            height: viewport.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            gui: GuiRender::default(),
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
    ) -> Option<wgpu::Adapter> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
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
