use anyhow::Result;
use nalgebra_glm as glm;
use std::{borrow::Cow, mem};
use support::{
    camera::MouseOrbit, run, AppConfig, Application, Geometry, Input, Renderer, System, Texture,
};
use wgpu::{
    util::DeviceExt, vertex_attr_array, BindGroup, BindGroupLayout, Buffer, BufferAddress, Device,
    PolygonMode, Queue, RenderPass, RenderPipeline, TextureFormat, VertexAttribute,
};

pub struct ShapeGeometry {
    geometry: Geometry,
}

impl ShapeGeometry {
    pub const VERTICES: [Vertex; 4] = [
        Vertex {
            position: [1.0, -1.0, 0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [-1.0, -1.0, 0.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
        Vertex {
            position: [-1.0, 1.0, 0.0, 1.0],
            color: [1.0, 0.5, 1.0, 1.0],
        },
    ];

    // Clockwise winding order
    pub const INDICES: [u32; 6] = [0, 1, 2, 1, 2, 3];

    fn new(device: &Device) -> Self {
        Self {
            geometry: Geometry::new(device, &Self::VERTICES, &Self::INDICES),
        }
    }

    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}

fn create_instances() -> Vec<Instance> {
    let num_instances_per_row: u32 = 1000;
    let instance_displacement: glm::Vec3 = glm::vec3(
        num_instances_per_row as f32,
        0.0,
        num_instances_per_row as f32,
    );
    (0..num_instances_per_row)
        .flat_map(|z| {
            (0..num_instances_per_row).map(move |x| {
                let position = glm::vec3(x as f32, 0.0, z as f32) - instance_displacement;

                let rotation = if position.is_empty() {
                    // this is needed so an object at (0, 0, 0) won't get scaled to zero
                    // as Quaternions can effect scale if they're not created correctly
                    glm::quat_angle_axis(0.0, &glm::Vec3::z())
                } else {
                    glm::quat_angle_axis(45_f32.to_degrees(), &position.normalize())
                };

                Instance { position, rotation }
            })
        })
        .collect::<Vec<_>>()
}

struct Instance {
    position: glm::Vec3,
    rotation: glm::Quat,
}

impl Instance {
    fn model_matrix(&self) -> glm::Mat4 {
        glm::translation(&self.position) * glm::quat_to_mat4(&self.rotation)
    }
}

impl Instance {
    pub fn vertex_attributes() -> Vec<VertexAttribute> {
        vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4].to_vec()
    }

    pub fn description(attributes: &[VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<glm::Mat4>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl Vertex {
    pub fn vertex_attributes() -> Vec<VertexAttribute> {
        vertex_attr_array![0 => Float32x4, 1 => Float32x4].to_vec()
    }

    pub fn description(attributes: &[VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
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

const SHADER_SOURCE: &str = "
struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
};

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
fn vertex_main(vert: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var position = vert.position;
    position.y *= -1.0;

    var out: VertexOutput;
    out.color = vert.color;
    out.position = ubo.mvp * model_matrix * position;

    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}
";

struct Scene {
    pub shape_geometry: ShapeGeometry,
    pub instance: Buffer,
    pub pipeline_filled: RenderPipeline,
    pub pipeline_lines: RenderPipeline,
    pub uniform: UniformBinding,
    pub instances: Vec<Instance>,
}

impl Scene {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shape_geometry = ShapeGeometry::new(device);
        let uniform = UniformBinding::new(device);

        let primitive_state = Self::primitive_state(wgpu::PolygonMode::Fill);
        let pipeline_filled =
            Self::create_pipeline(device, surface_format, &uniform, &primitive_state);

        let primitive_state = Self::primitive_state(wgpu::PolygonMode::Line);
        let pipeline_lines =
            Self::create_pipeline(device, surface_format, &uniform, &primitive_state);

        let instances = create_instances();

        let instance_matrices = instances
            .iter()
            .map(Instance::model_matrix)
            .collect::<Vec<_>>();

        let instance_descriptor = wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_matrices),
            usage: wgpu::BufferUsages::VERTEX,
        };

        let instance = device.create_buffer_init(&instance_descriptor);

        Self {
            shape_geometry,
            instance,
            uniform,
            pipeline_filled,
            pipeline_lines,
            instances,
        }
    }

    pub fn render<'rpass>(&'rpass self, renderpass: &mut RenderPass<'rpass>) {
        renderpass.set_bind_group(0, &self.uniform.bind_group, &[]);

        let (vertex_buffer_slice, index_buffer_slice) = self.shape_geometry.geometry().slices();
        renderpass.set_vertex_buffer(0, vertex_buffer_slice);
        renderpass.set_vertex_buffer(1, self.instance.slice(..));
        renderpass.set_index_buffer(index_buffer_slice, wgpu::IndexFormat::Uint32);

        renderpass.set_pipeline(&self.pipeline_filled);
        renderpass.draw_indexed(
            0..(ShapeGeometry::INDICES.len() as _),
            0,
            0..self.instances.len() as _,
        );
    }

    pub fn update(&mut self, view_projection_matrix: glm::Mat4, queue: &Queue) {
        self.uniform.update_buffer(
            queue,
            0,
            UniformBuffer {
                mvp: view_projection_matrix,
            },
        )
    }

    fn create_pipeline(
        device: &Device,
        surface_format: TextureFormat,
        uniform: &UniformBinding,
        primitive_state: &wgpu::PrimitiveState,
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

        let vertex_attributes = Vertex::vertex_attributes();
        let instance_attributes = Instance::vertex_attributes();
        let buffers = [
            Vertex::description(&vertex_attributes),
            Instance::description(&instance_attributes),
        ];

        let vertex = wgpu::VertexState {
            module: &shader_module,
            entry_point: "vertex_main",
            buffers: &buffers,
        };

        let fragment_state = wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fragment_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex,
            primitive: *primitive_state,
            depth_stencil: Some(Self::depth_stencil_state()),
            multisample: Self::multisample(),
            fragment: Some(fragment_state),
            multiview: None,
        })
    }

    fn primitive_state(polygon_mode: PolygonMode) -> wgpu::PrimitiveState {
        wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: Some(wgpu::IndexFormat::Uint32),
            front_face: wgpu::FrontFace::Cw,
            cull_mode: None,
            polygon_mode,
            conservative: false,
            unclipped_depth: false,
        }
    }

    fn depth_stencil_state() -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }

    fn multisample() -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
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
        self.scene = Some(Scene::new(&renderer.device, renderer.config.format));
        self.depth_texture = Some(Texture::create_depth_texture(
            &renderer.device,
            renderer.config.width,
            renderer.config.height,
        ));
        Ok(())
    }

    fn depth_format(&mut self) -> Option<wgpu::TextureFormat> {
        Some(Texture::DEPTH_FORMAT)
    }

    fn update(&mut self, renderer: &mut Renderer, input: &Input, system: &System) -> Result<()> {
        self.camera.update(input, system)?;
        let projection_view_matrix = self.camera.projection_view_matrix(renderer.aspect_ratio());
        if let Some(scene) = self.scene.as_mut() {
            scene.update(projection_view_matrix, &renderer.queue);
        }
        Ok(())
    }

    fn update_gui(&mut self, _renderer: &mut Renderer, context: &mut egui::Context) -> Result<()> {
        egui::Window::new("wgpu")
            .resizable(false)
            .fixed_pos((10.0, 10.0))
            .show(context, |ui| {
                ui.heading("Instancing");
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

    fn render<'a: 'b, 'b>(
        &'a mut self,
        view: &'a wgpu::TextureView,
        encoder: &'b mut wgpu::CommandEncoder,
    ) -> Result<Option<RenderPass<'b>>> {
        encoder.insert_debug_marker("Render scene");

        let depth_stencil_attachment = self.depth_texture.as_ref().map(|depth_texture| {
            wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }
        });

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
            depth_stencil_attachment,
        });

        if let Some(scene) = self.scene.as_ref() {
            scene.render(&mut render_pass);
        }

        Ok(Some(render_pass))
    }
}

fn main() -> Result<()> {
    run(
        App::default(),
        AppConfig {
            title: "Shapes".to_string(),
            width: 1920,
            height: 1080,
        },
    )
}