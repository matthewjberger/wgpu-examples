use petgraph::Graph;
use serde::{Deserialize, Serialize};

use crate::Transform;

/// A node in the scene graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraphNode {
	transform: Transform,
	node: Node,
}

/// The data associated with a node in the scene graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
	Empty,
	Mesh(String),
}

/// The index of a node in the scene graph.
pub type NodeIndex = petgraph::graph::NodeIndex;

/// The scene graph.
pub type SceneGraph = Graph<SceneGraphNode, ()>;

/// A builder for the scene graph.
#[derive(Default)]
pub struct SceneGraphBuilder {
	graph: SceneGraph,
}

impl SceneGraphBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_node(&mut self, node: SceneGraphNode) -> NodeIndex {
		self.graph.add_node(node)
	}

	pub fn add_edge(&mut self, parent: NodeIndex, child: NodeIndex) {
		self.graph.add_edge(parent, child, ());
	}

	pub fn build(self) -> SceneGraph {
		self.graph
	}
}

mod tests {
	#[test]
	fn test() {
		use super::*;

		let mut builder = SceneGraphBuilder::new();

		let root = builder.add_node(SceneGraphNode {
			transform: Transform::default(),
			node: Node::Empty,
		});

		let child = builder.add_node(SceneGraphNode {
			transform: Transform::default(),
			node: Node::Mesh("Mesh1".to_string()),
		});

		builder.add_edge(root, child);

		let graph = builder.build();

		println!("{graph:#?}");

		assert_eq!(graph.node_count(), 2);
		assert_eq!(graph.edge_count(), 1);
	}
}
