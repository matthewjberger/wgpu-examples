use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, Device,
};

pub struct Geometry {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

impl Geometry {
    pub fn new<T: bytemuck::Pod>(device: &wgpu::Device, vertices: &[T], indices: &[u32]) -> Self {
        Self {
            vertex_buffer: Self::create_vertex_buffer(device, vertices),
            index_buffer: Self::create_index_buffer(device, indices),
        }
    }

    pub fn slices(&self) -> (wgpu::BufferSlice, wgpu::BufferSlice) {
        (self.vertex_buffer.slice(..), self.index_buffer.slice(..))
    }

    fn create_vertex_buffer(device: &Device, vertices: &[impl bytemuck::Pod]) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    fn create_index_buffer(device: &Device, indices: &[impl bytemuck::Pod]) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}
