use std::collections::HashSet;

use log::info;
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use vek::Vec3;

use crate::{
    scene::camera::Camera,
    world::chunk::{Chunk, ChunkPos},
};

use super::{atlas::Atlas, buffer::ChunkBuffer, pipelines::voxel::VoxelPipeline, Renderable};

pub const RENDER_DISTANCE: i32 = 4;

pub struct WorldRenderer {
    chunks: Vec<Chunk>,
    chunks_pos: HashSet<ChunkPos>,
    pipeline: VoxelPipeline,
    pipeline_wireframe: VoxelPipeline,
    pub wireframe: bool,
    pub atlas: Atlas,
}
impl Renderable for WorldRenderer {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.wireframe {
            render_pass.set_pipeline(&self.pipeline_wireframe.pipeline);
        } else {
            render_pass.set_pipeline(&self.pipeline.pipeline);
        }
        render_pass.set_bind_group(0, &self.atlas.bind_group, &[]);
        for chunk in &self.chunks {
            render_pass.set_vertex_buffer(0, chunk.buffer.vertex_buf.buf.slice(..));
            render_pass.set_index_buffer(
                chunk.buffer.index_buf.buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..chunk.buffer.indices_len, 0, 0..1);
        }
    }
}

impl WorldRenderer {
    pub fn new(
        camera: &Camera,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shader: &wgpu::ShaderModule,
        cfg: &wgpu::SurfaceConfiguration,
        transform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture_atlas = include_bytes!("../../../assets/atlas.png");
        let atlas = Atlas::new(texture_atlas, &device, &queue);
        let pipeline = VoxelPipeline::new(
            device,
            shader,
            cfg,
            &[&atlas.bind_group_layout, &transform_bind_group_layout],
            wgpu::PolygonMode::Fill,
        );

        let pipeline_wireframe = VoxelPipeline::new(
            device,
            shader,
            cfg,
            &[&atlas.bind_group_layout, &transform_bind_group_layout],
            wgpu::PolygonMode::Line,
        );
        let mut world = Self {
            chunks: vec![],
            pipeline,
            pipeline_wireframe,
            atlas,
            wireframe: false,
            chunks_pos: HashSet::new(),
        };
        world.load_initial_chunks(device, camera);
        world
    }

    pub fn on_update(&mut self, player_pos: Vec3<f32>, device: &wgpu::Device) {
        let player_chunk_pos = ChunkPos::from_world(player_pos);
        let mut dirty = false;
        for chunk in self.chunks.iter_mut() {
            let distance = chunk.world_offset - player_chunk_pos;
            let squared_distance = distance.x * distance.x + distance.z * distance.z;
            if squared_distance > RENDER_DISTANCE * RENDER_DISTANCE {
                dirty = true;
                chunk.loaded = false;
                self.chunks_pos.remove(&chunk.world_offset);
            }
        }
        if dirty {
            self.unload_chunks();
            let instant = std::time::Instant::now();
            self.load_chunks(player_chunk_pos, device);
            info!("Took {}ms to generate chunk", instant.elapsed().as_millis());
        }
    }

    pub fn unload_chunks(&mut self) {
        self.chunks.retain(|chunk| {
            if !chunk.loaded {
                self.chunks_pos.remove(&chunk.world_offset);
            }
            chunk.loaded
        });
    }
    pub fn load_chunks(&mut self, player_pos: ChunkPos, device: &wgpu::Device) {
        let distance = RENDER_DISTANCE / 2;
        let start_x = player_pos.x - distance;
        let end_x = player_pos.x + distance;
        let start_z = player_pos.z - distance;
        let end_z = player_pos.z + distance;

        let new_positions: Vec<ChunkPos> = (start_z..=end_z)
            .into_par_iter()
            .flat_map(|z| {
                (start_x..=end_x)
                    .into_par_iter()
                    .map(move |x| ChunkPos::new(x, z))
            })
            .filter(|chunk_pos| self.chunks_pos.get(chunk_pos).is_none())
            .collect();

        let new_chunks: Vec<Chunk> = new_positions
            .par_iter()
            .map(|&chunk_pos| Chunk::new(device, chunk_pos))
            .collect();

        self.chunks_pos.extend(new_positions);
        self.chunks.extend(new_chunks);
    }

    pub fn load_initial_chunks(&mut self, device: &wgpu::Device, camera: &Camera) {
        for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let pos = ChunkPos::new(x, z);
                self.chunks_pos.insert(pos.clone());
                self.chunks.push(Chunk::new(device, pos));
            }
        }
    }
}
