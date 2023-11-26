use anyhow::Result;
use support::{run, AppConfig, Application, Renderer};
use wgpu::RenderPass;

#[derive(Default)]
struct App;

impl Application for App {
	fn update_gui(&mut self, _renderer: &mut Renderer, context: &mut egui::Context) -> Result<()> {
		egui::Window::new("wgpu")
			.resizable(false)
			.fixed_pos((10.0, 10.0))
			.show(context, |ui| {
				ui.heading("Solid Color");
			});
		Ok(())
	}

	fn render<'a: 'b, 'b>(
		&'a mut self,
		view: &'a wgpu::TextureView,
		encoder: &'b mut wgpu::CommandEncoder,
	) -> Result<Option<RenderPass<'b>>> {
		encoder.insert_debug_marker("Render scene");

		let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view,
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
			depth_stencil_attachment: None,
		});

		Ok(Some(render_pass))
	}
}

fn main() -> Result<()> {
	run(
		App,
		AppConfig {
			title: "Solid Color".to_string(),
			width: 800,
			height: 600,
		},
	)
}
