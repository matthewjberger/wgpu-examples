use anyhow::Result;
use nalgebra_glm as glm;
use support::{
    camera::MouseOrbit, load_gltf, run, world_render::render::WorldRender, AppConfig, Application,
    Input, Renderer, System, Texture, World,
};
use wgpu::{Device, Queue, RenderPass, TextureFormat};

struct Scene {
    world: World,
    world_render: WorldRender,
}

impl Scene {
    pub fn new(device: &Device, queue: &Queue, texture_format: TextureFormat) -> Result<Self> {
        let mut world = World::new()?;
        let mut world_render = WorldRender::new(device, texture_format)?;

        load_gltf("assets/DamagedHelmet.glb", &mut world)?;
        world_render.load(device, queue, &mut world)?;

        Ok(Self {
            world,
            world_render,
        })
    }

    pub fn update(&mut self, queue: &Queue, aspect_ratio: f32) -> Result<()> {
        Ok(self
            .world_render
            .update(queue, &mut self.world, aspect_ratio)?)
    }

    pub fn render<'rpass>(&'rpass self, render_pass: &mut RenderPass<'rpass>) -> Result<()> {
        Ok(self.world_render.render(render_pass, &self.world)?)
    }
}

#[derive(Default)]
struct App {
    scene: Option<Scene>,
    camera: MouseOrbit,
    depth_texture: Option<Texture>,
}

impl Application for App {
    fn initialize(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.camera.transform.translation = glm::vec3(4.0, 0.0, 4.0);
        self.camera.orientation.sensitivity = glm::vec2(0.1, 0.1);
        self.scene = Some(Scene::new(
            &renderer.device,
            &renderer.queue,
            renderer.config.format,
        )?);
        self.depth_texture = Some(Texture::create_depth_texture(
            &renderer.device,
            renderer.config.width,
            renderer.config.height,
        ));
        Ok(())
    }

    fn update(&mut self, renderer: &mut Renderer, input: &Input, system: &System) -> Result<()> {
        if let Some(scene) = self.scene.as_mut() {
            scene.update(&renderer.queue, renderer.aspect_ratio())?;
        }
        Ok(())
    }

    fn update_gui(&mut self, _renderer: &mut Renderer, context: &mut egui::Context) -> Result<()> {
        egui::Window::new("wgpu")
            .resizable(false)
            .fixed_pos((10.0, 10.0))
            .show(&context, |ui| {
                ui.heading("GLTF Models");
            });
        Ok(())
    }

    fn resize(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.depth_texture = Some(Texture::create_depth_texture(
            &renderer.device,
            renderer.config.width,
            renderer.config.height,
        ));
        Ok(())
    }

    fn render(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        encoder.insert_debug_marker("Render scene");

        {
            let depth_stencil_attachment = if let Some(depth_texture) = self.depth_texture.as_ref()
            {
                Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                })
            } else {
                None
            };

            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment,
            });

            if let Some(scene) = self.scene.as_ref() {
                scene.render(&mut renderpass)?;
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    run(
        App::default(),
        AppConfig {
            title: "GLTF Models".to_string(),
            width: 800,
            height: 600,
        },
    )
}
