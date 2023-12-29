use anyhow::Result;
use gltf::Gltf;
use nalgebra_glm as glm;
use std::{borrow::Cow, mem, path::Path};
use support::{run, AppConfig, Application, Geometry, Input, Renderer, System};
use wgpu::{
    util::DeviceExt, vertex_attr_array, BindGroup, BindGroupLayout, Buffer, BufferAddress, Device,
    Queue, RenderPass, RenderPipeline, TextureFormat, VertexAttribute,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl Vertex {
    pub fn vertex_attributes() -> Vec<VertexAttribute> {
        vertex_attr_array![0 => Float32x4, 1 => Float32x4].to_vec()
    }

    pub fn description(attributes: &[VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformBuffer {
    mvp: glm::Mat4,
}

struct UniformBinding {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl UniformBinding {
    pub fn new(device: &Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[UniformBuffer::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        Self {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update_buffer(
        &mut self,
        queue: &Queue,
        offset: BufferAddress,
        uniform_buffer: UniformBuffer,
    ) {
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::cast_slice(&[uniform_buffer]),
        )
    }
}

const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [1.0, -1.0, 0.0, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
];

// Clockwise winding order
const INDICES: [u32; 3] = [0, 1, 2];

const SHADER_SOURCE: &str = "
struct Uniform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = vert.color;
    out.position = ubo.mvp * vert.position;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}
";

struct Scene {
    pub gltf: Option<gltf::Gltf>,
    pub model: glm::Mat4,
    pub geometry: Geometry,
    pub uniform: UniformBinding,
    pub pipeline: RenderPipeline,
}

impl Scene {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let uniform = UniformBinding::new(device);
        let pipeline = Self::create_pipeline(device, surface_format, &uniform);

        let vertices = VERTICES.to_vec();
        let indices = INDICES.to_vec();
        let geometry = Geometry::new(device, &vertices, &indices);

        Self {
            gltf: None,
            model: glm::Mat4::identity(),
            geometry,
            uniform,
            pipeline,
        }
    }

    pub fn render<'rpass>(&'rpass self, renderpass: &mut RenderPass<'rpass>) {
        renderpass.set_pipeline(&self.pipeline);
        renderpass.set_bind_group(0, &self.uniform.bind_group, &[]);

        let (vertex_buffer_slice, index_buffer_slice) = self.geometry.slices();
        renderpass.set_vertex_buffer(0, vertex_buffer_slice);
        renderpass.set_index_buffer(index_buffer_slice, wgpu::IndexFormat::Uint32);

        renderpass.draw_indexed(0..(INDICES.len() as _), 0, 0..1);
    }

    pub fn update(&mut self, queue: &Queue, aspect_ratio: f32) {
        let projection = glm::perspective_lh_zo(aspect_ratio, 80_f32.to_radians(), 0.1, 1000.0);
        let view = glm::look_at_lh(
            &glm::vec3(0.0, 0.0, 3.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::Vec3::y(),
        );
        self.model = glm::rotate(&self.model, 1_f32.to_radians(), &glm::Vec3::y());

        self.uniform.update_buffer(
            queue,
            0,
            UniformBuffer {
                mvp: projection * view * self.model,
            },
        )
    }

    pub fn load_asset(&mut self, gltf: &Gltf) {
        self.gltf = Some(gltf.clone());
    }

    fn create_pipeline(
        device: &Device,
        surface_format: TextureFormat,
        uniform: &UniformBinding,
    ) -> RenderPipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SOURCE)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform.bind_group_layout],
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
        self.scene = Some(Scene::new(&renderer.device, renderer.config.format));
        Ok(())
    }

    fn update(&mut self, renderer: &mut Renderer, _input: &Input, _system: &System) -> Result<()> {
        if let Some(scene) = self.scene.as_mut() {
            scene.update(&renderer.queue, renderer.aspect_ratio());
        }
        Ok(())
    }

    fn update_gui(&mut self, _renderer: &mut Renderer, context: &mut egui::Context) -> Result<()> {
        egui::Window::new("GLTF Asset")
            .resizable(false)
            .fixed_pos((10.0, 10.0))
            .show(context, |ui| {
                if ui.button("Import GLTF/GLB...").clicked() {
                    self.pick_gltf_file();
                }

                ui.separator();

                ui.heading("Asset Info");

                self.scene
                    .as_ref()
                    .and_then(|scene| scene.gltf.as_ref())
                    .map(|gltf| {
                        gltf.scenes().for_each(|gltf_scene| {
                            draw_scene_tree_ui(ui, gltf_scene);
                        });
                    });
            });
        Ok(())
    }

    fn render<'a: 'b, 'b>(
        &'a mut self,
        view: &'a wgpu::TextureView,
        encoder: &'b mut wgpu::CommandEncoder,
    ) -> Result<Option<RenderPass<'b>>> {
        encoder.insert_debug_marker("Render scene");

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        if let Some(scene) = self.scene.as_ref() {
            scene.render(&mut render_pass);
        }

        Ok(Some(render_pass))
    }
}

fn draw_scene_tree_ui<'a>(ui: &mut egui::Ui, gltf_scene: gltf::Scene<'a>) {
    egui::collapsing_header::CollapsingHeader::new("Scene")
        .id_source(ui.next_auto_id())
        .show(ui, |ui| {
            draw_scene_ui(ui, gltf_scene);
        });
}

fn draw_scene_ui(ui: &mut egui::Ui, gltf_scene: gltf::Scene<'_>) {
    gltf_scene.nodes().for_each(|node| {
        draw_gltf_node_ui(ui, node);
    });
}

fn draw_gltf_node_ui(ui: &mut egui::Ui, node: gltf::Node<'_>) {
    if node.children().len() == 0 {
        ui.label(node.name().unwrap_or("Unnamed"));
    }

    node.children().for_each(|child| {
        egui::collapsing_header::CollapsingHeader::new(node.name().unwrap_or("Unnamed"))
            .id_source(ui.next_auto_id())
            .show(ui, |ui| {
                draw_gltf_node_ui(ui, child);
            });
    });
}

impl App {
    fn pick_gltf_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GLTF / GLB", &["gltf", "glb"])
            .pick_file()
        {
            self.load_gltf_file(path);
        }
    }

    fn load_gltf_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        println!("File picked: {path:#?}");
        match std::fs::read(&path) {
            Ok(bytes) => {
                println!("Loaded {} bytes", bytes.len());
                let gltf = Gltf::from_slice(&bytes).expect("Failed to load GLTF!");
                if let Some(scene) = self.scene.as_mut() {
                    scene.load_asset(&gltf);
                }
            }
            Err(error) => {
                eprintln!("{error}");
            }
        };
    }
}

fn main() -> Result<()> {
    run(
        App::default(),
        AppConfig {
            title: "Gltf".to_string(),
            width: 1920,
            height: 1080,
        },
    )
}
