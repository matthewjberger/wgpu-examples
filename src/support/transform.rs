use nalgebra::{Isometry3, Translation3, UnitQuaternion};
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
	pub translation: glm::Vec3,
	pub rotation: glm::Quat,
	pub scale: glm::Vec3,
}

impl Default for Transform {
	fn default() -> Self {
		Self {
			translation: glm::vec3(0.0, 0.0, 0.0),
			rotation: glm::quat_conjugate(&glm::quat_look_at(&glm::Vec3::z(), &glm::Vec3::y())),
			scale: glm::vec3(1.0, 1.0, 1.0),
		}
	}
}

impl Transform {
	pub fn new(translation: glm::Vec3, rotation: glm::Quat, scale: glm::Vec3) -> Self {
		Self {
			translation,
			rotation,
			scale,
		}
	}

	pub fn matrix(&self) -> glm::Mat4 {
		glm::translation(&self.translation)
			* glm::quat_to_mat4(&self.rotation)
			* glm::scaling(&self.scale)
	}

	pub fn as_isometry(&self) -> Isometry3<f32> {
		Isometry3::from_parts(
			Translation3::from(self.translation),
			UnitQuaternion::from_quaternion(self.rotation),
		)
	}

	pub fn as_view_matrix(&self) -> glm::Mat4 {
		let eye = self.translation;
		let target = self.translation + self.forward();
		let up = self.up();
		glm::look_at(&eye, &target, &up)
	}

	pub fn right(&self) -> glm::Vec3 {
		glm::quat_rotate_vec3(&self.rotation.normalize(), &glm::Vec3::x())
	}

	pub fn up(&self) -> glm::Vec3 {
		glm::quat_rotate_vec3(&self.rotation.normalize(), &glm::Vec3::y())
	}

	pub fn forward(&self) -> glm::Vec3 {
		glm::quat_rotate_vec3(&self.rotation.normalize(), &(-glm::Vec3::z()))
	}

	pub fn rotate(&mut self, increment: &glm::Vec3) {
		self.translation = glm::rotate_x_vec3(&self.translation, increment.x);
		self.translation = glm::rotate_y_vec3(&self.translation, increment.y);
		self.translation = glm::rotate_z_vec3(&self.translation, increment.z);
	}

	pub fn look_at(&mut self, target: &glm::Vec3, up: &glm::Vec3) {
		self.rotation = glm::quat_conjugate(&glm::quat_look_at(target, up));
	}
}
