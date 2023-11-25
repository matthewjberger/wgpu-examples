use anyhow::Result;
use std::{borrow::Cow, fmt, mem};
use support::{run, AppConfig, Application, Geometry, Renderer, Texture};
use wgpu::{
    vertex_attr_array, BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline,
    TextureFormat, VertexAttribute,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Node {
    Rectangle,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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

        let mut graph: NodeGraph<String, String, Node> = NodeGraph::new();
        graph.add_node("node1".to_string(), Node::Rectangle);
        graph.add_node("node2".to_string(), Node::Rectangle);
        graph
            .add_edge(
                "node1".to_string(),
                "node2".to_string(),
                "edge1-2".to_string(),
            )
            .unwrap();

        let edges = graph
            .get_edges_connected_to_node(&"node1".to_string())
            .unwrap();

        assert_eq!(edges, vec![("node2".to_string(), "edge1-2".to_string())]);

        Ok(())
    }

    fn update_gui(&mut self, _renderer: &mut Renderer, context: &mut egui::Context) -> Result<()> {
        egui::Window::new("wgpu")
            .resizable(false)
            .fixed_pos((10.0, 10.0))
            .show(context, |ui| {
                ui.heading("Texture");
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

struct TextureBinding {
    _texture: Texture,
    pub bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl TextureBinding {
    pub fn new(device: &Device, queue: &Queue) -> Result<Self> {
        let texture_bytes = include_bytes!("../../assets/textures/planks.jpg");
        let texture = Texture::from_bytes(device, queue, texture_bytes, "planks.jpg")?;

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

    pub fn description(attributes: &[VertexAttribute]) -> wgpu::VertexBufferLayout {
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
use std::error::Error;

#[derive(Debug)]
pub enum NodeGraphError {
    NodeAlreadyExists,
    NodeNotFound,
    EdgeError,
    SerializationError(String),
    DeserializationError(String),
}

impl std::fmt::Display for NodeGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NodeGraphError::NodeAlreadyExists => {
                write!(f, "Node with this ID already exists")
            }
            NodeGraphError::NodeNotFound => write!(f, "Node with this ID does not exist"),
            NodeGraphError::EdgeError => write!(f, "One of the node IDs does not exist"),
            NodeGraphError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            NodeGraphError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
        }
    }
}

impl Error for NodeGraphError {}

use petgraph::{
    dot::{Config, Dot},
    graph::NodeIndex,
    visit::{Dfs, EdgeRef},
    Direction, Graph,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, hash::Hash};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeGraph<Node, Edge, NodeData>
where
    Node: Eq + Hash + Clone + Ord + Debug,
    Edge: Clone + PartialEq + Debug,
    NodeData: Serialize + Clone + PartialEq + Debug,
{
    graph: Graph<NodeData, Edge>,
    index_map: HashMap<Node, NodeIndex>,
}

impl<Node, Edge, NodeData> Default for NodeGraph<Node, Edge, NodeData>
where
    Node: Eq + Hash + Clone + Ord + Debug,
    Edge: Clone + PartialEq + Debug,
    NodeData: Serialize + Clone + PartialEq + Debug,
{
    fn default() -> Self {
        Self {
            graph: Graph::new(),
            index_map: HashMap::new(),
        }
    }
}

impl<Node, Edge, NodeData> NodeGraph<Node, Edge, NodeData>
where
    Node: Eq + Hash + Clone + Ord + Debug,
    Edge: Clone + PartialEq + Debug,
    NodeData: Serialize + Clone + PartialEq + Debug,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, id: Node, data: NodeData) -> Node {
        let index = self.graph.add_node(data);
        self.index_map.insert(id.clone(), index);
        id
    }

    pub fn remove_node(&mut self, id: Node) -> Option<NodeData> {
        if let Some(index) = self.index_map.remove(&id) {
            return self.graph.remove_node(index);
        }
        None
    }

    pub fn add_edge(&mut self, from: Node, to: Node, value: Edge) -> Result<(), NodeGraphError> {
        let from_index = self
            .index_map
            .get(&from)
            .ok_or(NodeGraphError::NodeNotFound)?;
        let to_index = self
            .index_map
            .get(&to)
            .ok_or(NodeGraphError::NodeNotFound)?;
        self.graph.add_edge(*from_index, *to_index, value);
        Ok(())
    }

    pub fn get_edges_connected_to_node(&self, id: &Node) -> Option<Vec<(Node, Edge)>> {
        self.index_map.get(id).map(|&index| {
            self.graph
                .edges(index)
                .filter_map(|edge| {
                    let (source, target) = (edge.source(), edge.target());
                    let adjacent_node_id = if source == index { target } else { source };
                    let adjacent_node_id = self.index_map.iter().find_map(|(id, &idx)| {
                        if idx == adjacent_node_id {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })?;
                    let edge_weight = edge.weight().clone();
                    Some((adjacent_node_id, edge_weight))
                })
                .collect()
        })
    }

    pub fn remove_edge(&mut self, from: Node, to: Node) -> Option<Edge> {
        if let (Some(&from_index), Some(&to_index)) =
            (self.index_map.get(&from), self.index_map.get(&to))
        {
            if let Some(edge) = self.graph.find_edge(from_index, to_index) {
                return self.graph.remove_edge(edge);
            }
        }
        None
    }

    pub fn contains_node(&self, id: &Node) -> bool {
        self.index_map.contains_key(id)
    }

    pub fn contains_edge(&self, from: &Node, to: &Node) -> bool {
        if let (Some(&from_index), Some(&to_index)) =
            (self.index_map.get(from), self.index_map.get(to))
        {
            return self.graph.contains_edge(from_index, to_index);
        }
        false
    }

    pub fn node_data(&self, id: &Node) -> Option<&NodeData> {
        self.index_map
            .get(id)
            .and_then(|index| self.graph.node_weight(*index))
    }

    pub fn to_dot(&self) -> String {
        let dot = Dot::with_config(&self.graph, &[Config::GraphContentOnly]);
        format!("{dot:?}")
    }

    // Traverse using DFS from the given start node
    pub fn traverse_dfs(&self, start_id: &Node) -> Option<Vec<Node>> {
        let start_index = self.index_map.get(start_id)?;
        let mut dfs = Dfs::new(&self.graph, *start_index);
        let mut result = Vec::new();

        while let Some(nx) = dfs.next(&self.graph) {
            if let Some(node_id) =
                self.index_map
                    .iter()
                    .find_map(|(id, &idx)| if idx == nx { Some(id.clone()) } else { None })
            {
                result.push(node_id);
            }
        }

        Some(result)
    }

    pub fn get_edges_from(&self, id: &Node) -> Option<Vec<(Node, Edge)>> {
        let index = self.index_map.get(id)?;
        let edges: Vec<(Node, Edge)> = self
            .graph
            .edges_directed(*index, Direction::Outgoing)
            .filter_map(|edge| {
                let target_id = self
                    .index_map
                    .iter()
                    .find(|(_id, &idx)| idx == edge.target())
                    .map(|(id, _)| id.clone());
                let weight = edge.weight().clone();
                target_id.map(|tid| (tid, weight))
            })
            .collect();
        Some(edges)
    }

    pub fn add_nodes(&mut self, nodes: &[(Node, NodeData)]) {
        for (id, data) in nodes {
            self.add_node(id.clone(), data.clone());
        }
    }

    pub fn add_edges(&mut self, edges: &[(Node, Node, Edge)]) -> Result<(), NodeGraphError> {
        for (from, to, value) in edges {
            self.add_edge(from.clone(), to.clone(), value.clone())?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub enum NodeData {
        Text(String),
        Number(i32),
        Position(u8, u8),
    }

    impl fmt::Display for NodeData {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[test]
    fn test_new_graph_is_empty() {
        let graph: NodeGraph<i32, String, String> = NodeGraph::new();
        assert_eq!(graph.graph.node_count(), 0);
        assert_eq!(graph.graph.edge_count(), 0);
    }

    #[test]
    fn test_add_and_remove_node() {
        let mut graph: NodeGraph<i32, String, String> = NodeGraph::new();
        let node_id = 1;
        let node_data = "Node 1 data".to_string();
        graph.add_node(node_id, node_data.clone());

        assert!(graph.contains_node(&node_id));
        assert_eq!(graph.node_data(&node_id), Some(&node_data));

        let removed_data = graph.remove_node(node_id).unwrap();
        assert_eq!(removed_data, node_data);
        assert!(!graph.contains_node(&node_id));
    }

    #[test]
    fn test_add_and_remove_edge() {
        let mut graph: NodeGraph<i32, String, String> = NodeGraph::new();
        graph.add_node(1, "Node 1".to_string());
        graph.add_node(2, "Node 2".to_string());
        let edge_value = "connects".to_string();

        graph.add_edge(1, 2, edge_value.clone()).unwrap();
        assert!(graph.contains_edge(&1, &2));

        let removed_edge_value = graph.remove_edge(1, 2).unwrap();
        assert_eq!(removed_edge_value, edge_value);
        assert!(!graph.contains_edge(&1, &2));
    }

    #[test]
    fn test_edge_cases() {
        let mut graph: NodeGraph<i32, String, String> = NodeGraph::new();
        graph.add_node(1, "Node 1".to_string());
        // Attempt to add an edge where one node does not exist
        assert!(graph.add_edge(1, 2, "connects".to_string()).is_err());
        // Attempt to remove a non-existent edge
        assert!(graph.remove_edge(1, 2).is_none());
        // Attempt to remove a non-existent node
        assert!(graph.remove_node(2).is_none());
    }

    #[test]
    fn test_graph_with_enum_node_data() {
        let mut graph: NodeGraph<i32, String, NodeData> = NodeGraph::new();

        graph.add_node(1, NodeData::Text("Node 1 data".to_string()));
        graph.add_node(2, NodeData::Number(42));
        graph.add_node(3, NodeData::Position(3, 4));

        graph.add_edge(1, 2, "Edge 1-2".to_string()).unwrap();
        graph.add_edge(2, 3, "Edge 2-3".to_string()).unwrap();

        assert!(graph.contains_edge(&1, &2));
        assert!(graph.contains_edge(&2, &3));

        graph.remove_node(2);
        assert!(!graph.contains_node(&2));
        assert!(!graph.contains_edge(&1, &2));
        assert!(!graph.contains_edge(&2, &3));
    }

    #[test]
    fn test_serialization_and_deserialization() {
        let mut graph: NodeGraph<u32, String, String> = NodeGraph::new();
        graph.add_node(1, "Node1".to_string());
        graph.add_node(2, "Node2".to_string());
        graph.add_edge(1, 2, "Edge1-2".to_string()).unwrap();

        let serialized = serde_json::to_string(&graph).expect("Failed to serialize graph");
        let deserialized: NodeGraph<u32, String, String> =
            serde_json::from_str(&serialized).expect("Failed to deserialize graph");

        assert!(deserialized.contains_node(&1));
        assert!(deserialized.contains_node(&2));
        assert!(deserialized.contains_edge(&1, &2));
        assert_eq!(deserialized.node_data(&1), Some(&"Node1".to_string()));
        assert_eq!(deserialized.node_data(&2), Some(&"Node2".to_string()));
    }

    #[test]
    fn test_graphviz_output() {
        let mut graph: NodeGraph<u32, &'static str, NodeData> = NodeGraph::new();
        graph.add_node(1, NodeData::Text("Hello!".to_string()));
        graph.add_node(2, NodeData::Number(3));
        graph.add_node(3, NodeData::Position(0, 1));
        graph.add_edge(1, 2, "Edge 1-2").unwrap();
        graph.add_edge(2, 3, "Edge 2-3").unwrap();

        let dot_output = graph.to_dot();
        assert!(!dot_output.is_empty());
        println!("DOT GraphViz Representation:\n{}", dot_output);
    }

    #[test]
    fn test_get_node_data_by_id() {
        let mut graph: NodeGraph<String, String, NodeData> = NodeGraph::new();
        let node_id = "node1".to_string();
        graph.add_node(node_id.clone(), NodeData::Text("Node 1 data".to_string()));

        let node_data = graph.node_data(&node_id);
        assert_eq!(node_data, Some(&NodeData::Text("Node 1 data".to_string())));
    }

    #[test]
    fn test_traverse_dfs() {
        let mut graph: NodeGraph<String, String, NodeData> = NodeGraph::new();
        graph.add_node("root".to_string(), NodeData::Text("Root".to_string()));
        graph.add_node("child1".to_string(), NodeData::Text("Child 1".to_string()));
        graph.add_node("child2".to_string(), NodeData::Text("Child 2".to_string()));
        graph
            .add_edge(
                "root".to_string(),
                "child1".to_string(),
                "edge1".to_string(),
            )
            .unwrap();
        graph
            .add_edge(
                "root".to_string(),
                "child2".to_string(),
                "edge2".to_string(),
            )
            .unwrap();

        let dfs_nodes = graph.traverse_dfs(&"root".to_string());
        assert_eq!(
            dfs_nodes,
            Some(vec![
                "root".to_string(),
                "child1".to_string(),
                "child2".to_string()
            ])
        );
    }

    #[test]
    fn test_get_edges_connected_to_node() {
        let mut graph: NodeGraph<String, String, NodeData> = NodeGraph::new();
        graph.add_node("node1".to_string(), NodeData::Text("Node 1".to_string()));
        graph.add_node("node2".to_string(), NodeData::Number(2));
        graph
            .add_edge(
                "node1".to_string(),
                "node2".to_string(),
                "edge1-2".to_string(),
            )
            .unwrap();

        let edges = graph
            .get_edges_connected_to_node(&"node1".to_string())
            .unwrap();

        assert_eq!(edges, vec![("node2".to_string(), "edge1-2".to_string())]);
    }
}
