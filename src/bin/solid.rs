use anyhow::Result;
use support::{run, AppConfig, Application};

#[derive(Default)]
struct DemoApp;

impl Application for DemoApp {
    fn render(&mut self, renderer: &mut support::Renderer) -> Result<()> {
        renderer.render_frame(|view, encoder| {
            encoder.insert_debug_marker("Render Entities");

            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                depth_stencil_attachment: None,
            });

            Ok(())
        })
    }
}

fn main() -> Result<()> {
    run(
        DemoApp::default(),
        AppConfig {
            title: "Hello".to_string(),
            width: 800,
            height: 600,
        },
    )
}
