use crate::{
    block::BlockId,
    direction::Direction,
    renderer::{
        buffer::{compute_cube_indices, ChunkBuffer},
        mesh::quad::Quad,
        mesh::vertex::Vertex,
    },
};

use log::info;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use vek::Vec3;

pub const CHUNK_HEIGHT: usize = 256;
pub const CHUNK_DEPTH: usize = 16;
pub const CHUNK_WIDTH: usize = 16;
pub const TOTAL_CHUNK_SIZE: usize = CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_DEPTH;

pub struct Chunk {
    pub blocks: Vec<BlockId>,
    pub pos: ChunkPos,
    pub buffer: ChunkBuffer,
    pub mesh: ChunkMesh,
    pub loaded: bool,
}

impl Chunk {
    pub fn new(device: &wgpu::Device, pos: ChunkPos) -> Self {
        let instant = std::time::Instant::now();
        let (blocks, mesh) = Self::generate(pos);
        let elapsed = instant.elapsed();
        let buffer = ChunkBuffer::new(&device, &mesh.vertices, &mesh.indices, mesh.num_elements);
        info!("Took {}ms to generate chunk", elapsed.as_millis());

        Self {
            blocks,
            buffer,
            mesh,
            pos,
            loaded: true,
        }
    }

    pub fn generate(pos: ChunkPos) -> (Vec<BlockId>, ChunkMesh) {
        let mut blocks = [BlockId::DIRT; TOTAL_CHUNK_SIZE];

        let mut vertices = Vec::with_capacity(TOTAL_CHUNK_SIZE);

        let verts = (0..CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_DEPTH)
            .into_par_iter()
            .map(|index| {
                let x = index % CHUNK_WIDTH;
                let y = (index / CHUNK_WIDTH) % CHUNK_HEIGHT;
                let z = (index / (CHUNK_WIDTH * CHUNK_HEIGHT)) % CHUNK_DEPTH;

                let block_in_chunk = if y == CHUNK_HEIGHT - 1 {
                    BlockId::GRASS
                } else {
                    BlockId::DIRT
                };
                // The position of the block in the chunk
                let local_pos = Vec3::new(x as i32, y as i32, z as i32);
                let world_pos = pos.to_world();
                let translation = Vec3::new(
                    local_pos.x + world_pos.x,
                    local_pos.y,
                    local_pos.z + world_pos.z,
                );

                let mut visible_quads = Vec::new();
                (Direction::ALL).iter().for_each(|dir| {
                    let neighbor_pos = local_pos + dir.normalized();
                    if !Chunk::is_pos_in_bounds(neighbor_pos) {
                        visible_quads.push(Quad::new(&block_in_chunk, *dir, translation));
                        return;
                    }
                });
                (index, visible_quads, block_in_chunk)
            })
            .collect::<Vec<_>>();

        verts.iter().for_each(|quad| {
            let (index, visible_quads, block) = quad;
            visible_quads.iter().for_each(|quad| {
                blocks[*index] = *block;
                vertices.extend(quad.vertices);
            });
        });

        let indices = compute_cube_indices(vertices.len());
        (blocks.to_vec(), ChunkMesh::new(vertices, indices))
    }

    /// Checks if a given position is in bounds of the chunk
    pub fn is_pos_in_bounds(pos: Vec3<i32>) -> bool {
        if pos.x >= 0 && pos.y >= 0 && pos.z >= 0 {
            return pos.x < CHUNK_WIDTH as i32
                && pos.y < CHUNK_HEIGHT as i32
                && pos.z < CHUNK_DEPTH as i32;
        }
        false
    }
}

/// Represents the offset or indices of a chunk
/// relative to the world position.
///
/// Example:
///
/// If a chunk is 16 units wide and 16 units deep:
///
/// World Position: (32, 0, -128) -> ChunkPos: (2, 0, -8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl std::ops::Sub<ChunkPos> for ChunkPos {
    type Output = ChunkPos;

    fn sub(self, rhs: ChunkPos) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Add<ChunkPos> for ChunkPos {
    type Output = ChunkPos;

    fn add(self, rhs: ChunkPos) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
        }
    }
}

impl ChunkPos {
    pub const ORIGIN: ChunkPos = ChunkPos::new(0, 0);

    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Returns the chunk pos at the given world pos
    pub fn from_world(pos: Vec3<f32>) -> Self {
        let x = (pos.x / CHUNK_WIDTH as f32).floor() as i32;
        let z = (pos.z / CHUNK_DEPTH as f32).floor() as i32;
        Self { x, z }
    }

    /// Returns the world pos of the current chunk.
    pub fn to_world(&self) -> Vec3<i32> {
        Vec3::new(self.x * CHUNK_WIDTH as i32, 0, self.z * CHUNK_DEPTH as i32)
    }
}

pub fn compute_1d(x: usize, y: usize, z: usize) -> usize {
    x + y * CHUNK_WIDTH + z * CHUNK_WIDTH * CHUNK_HEIGHT
}

pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub num_elements: u32,
}
impl ChunkMesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            num_elements: indices.len() as u32,
            indices,
        }
    }
}
