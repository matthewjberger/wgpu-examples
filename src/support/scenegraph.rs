use crate::Transform;
use nalgebra_glm as glm;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Mul};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraphNode<T> {
	pub transform: Transform,
	pub node: T,
}

pub struct SceneGraph<T>(DiGraph<SceneGraphNode<T>, ()>);

impl<T> Deref for SceneGraph<T> {
	type Target = DiGraph<SceneGraphNode<T>, ()>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for SceneGraph<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl Mul for Transform {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		let scale = glm::vec3(
			self.scale.x * rhs.scale.x,
			self.scale.y * rhs.scale.y,
			self.scale.z * rhs.scale.z,
		);

		let rotation = self.rotation * rhs.rotation;

		let translation =
			self.translation + glm::quat_rotate_vec3(&self.rotation, &rhs.translation);

		Self {
			translation,
			rotation,
			scale,
		}
	}
}

impl<T> SceneGraph<T> {
	pub fn find_node<F>(&self, mut predicate: F) -> Option<NodeIndex>
	where
		F: FnMut(&SceneGraphNode<T>) -> bool,
	{
		self.node_indices().find(|&index| predicate(&self[index]))
	}

	pub fn global_transform(&self, node: NodeIndex) -> Transform {
		let mut transform = Transform::default();
		let mut current = node;
		while let Some(parent) = self
			.neighbors_directed(current, petgraph::Direction::Incoming)
			.next()
		{
			transform = self[current].transform;
			current = parent;
		}
		transform
	}
}
