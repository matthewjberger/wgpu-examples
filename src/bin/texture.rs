use anyhow::Result;
use std::{borrow::Cow, mem};
use support::{run, AppConfig, Application, Geometry, Renderer, Texture};
use wgpu::{
    vertex_attr_array, BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline,
    ShaderModule, TextureFormat, VertexAttribute,
};

const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [0.6, -0.6, 0.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        position: [-0.6, -0.6, 0.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [0.6, 0.6, 0.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [-0.6, 0.6, 0.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
];

const INDICES: [u32; 6] = [0, 1, 2, 1, 2, 3]; // Clockwise winding order

const SHADER_SOURCE: &str = "
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = vert.tex_coords;
    out.position = vert.position;
    return out;
};

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;


@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
";

struct Scene {
    pub geometry: Geometry,
    pub pipeline: RenderPipeline,
    pub texture: TextureBinding,
}

impl Scene {
    pub fn new(device: &Device, queue: &Queue, surface_format: TextureFormat) -> Result<Self> {
        let geometry = Geometry::new(device, &VERTICES, &INDICES);
        let texture = TextureBinding::new(device, queue)?;
        let pipeline = Self::create_pipeline(device, surface_format, &texture);
        Ok(Self {
            geometry,
            pipeline,
            texture,
        })
    }

    pub fn render<'rpass>(&'rpass self, renderpass: &mut RenderPass<'rpass>) {
        renderpass.set_pipeline(&self.pipeline);
        renderpass.set_bind_group(0, &self.texture.bind_group, &[]);

        let (vertex_slice, index_slice) = self.geometry.slices();
        renderpass.set_vertex_buffer(0, vertex_slice);
        renderpass.set_index_buffer(index_slice, wgpu::IndexFormat::Uint32);

        renderpass.draw_indexed(0..(INDICES.len() as _), 0, 0..1);
    }

    fn create_pipeline(
        device: &Device,
        surface_format: TextureFormat,
        texture: &TextureBinding,
    ) -> RenderPipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SOURCE)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&texture.bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vertex_main",
                buffers: &[Vertex::description(&Vertex::vertex_attributes())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
    }
}

#[derive(Default)]
struct App {
    scene: Option<Scene>,
}

impl Application for App {
    fn initialize(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.scene = Some(Scene::new(
            &renderer.device,
            &renderer.queue,
            renderer.config.format,
        )?);
        Ok(())
    }

    fn update_gui(&mut self, context: &mut egui::Context) -> Result<()> {
        egui::Window::new("wgpu")
            .resizable(false)
            .fixed_pos((10.0, 10.0))
            .show(&context, |ui| {
                ui.heading("Texture");
            });
        Ok(())
    }

    fn render(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        encoder.insert_debug_marker("Render scene");

        {
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
                depth_stencil_attachment: None,
            });

            if let Some(scene) = self.scene.as_ref() {
                scene.render(&mut renderpass);
            }
        }

        Ok(())
    }
}

struct TextureBinding {
    _texture: Texture,
    pub bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl TextureBinding {
    pub fn new(device: &Device, queue: &Queue) -> Result<Self> {
        let texture_bytes = include_bytes!("../../assets/textures/planks.jpg");
        let texture = Texture::from_bytes(&device, &queue, texture_bytes, "planks.jpg")?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        Ok(Self {
            _texture: texture,
            bind_group,
            bind_group_layout,
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    tex_coords: [f32; 2],
}

impl Vertex {
    pub fn vertex_attributes() -> Vec<VertexAttribute> {
        vertex_attr_array![0 => Float32x4, 1 => Float32x2].to_vec()
    }

    pub fn description<'a>(attributes: &'a [VertexAttribute]) -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}

fn main() -> Result<()> {
    run(
        App::default(),
        AppConfig {
            title: "Texture".to_string(),
            width: 800,
            height: 600,
        },
    )
}
