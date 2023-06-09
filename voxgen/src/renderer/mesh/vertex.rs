use vek::Vec3;

use crate::{
    block::BlockId,
    direction::Direction,
    renderer::atlas::{atlas_uv_mapping, TextureId},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}
impl Vertex {
    pub const INDEX_BUFFER_FORMAT: Option<wgpu::IndexFormat> = Some(wgpu::IndexFormat::Uint16);

    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn new(pos: [f32; 3], uv: [u8; 2], texture_id: &TextureId) -> Self {
        Self {
            pos,
            uv: atlas_uv_mapping(texture_id, uv[0], uv[1]),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }

    pub fn quad(
        v1: f32,
        v2: f32,
        v3: f32,
        at: Vec3<i32>,
        uv: [u8; 2],
        id: &BlockId,
        dir: &Direction,
    ) -> Self {
        let texture_uv = id.map_texture(uv, dir);
        Self {
            pos: [v1 + at.x as f32, v2 + at.y as f32, v3 + at.z as f32],
            uv: texture_uv,
        }
    }
}
