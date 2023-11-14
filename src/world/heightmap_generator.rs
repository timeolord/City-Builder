use std::ops::{Index, IndexMut};

use array2d::Array2D;
use bevy::prelude::Component;
use noise::{NoiseFn, Perlin};

use crate::{
    chunk::{ChunkPosition, ChunkTilePosition, TilePosition2D},
    constants::CHUNK_SIZE,
};

const NOISE_SCALE: f64 = 0.01;
const NOISE_AMPLITUDE: f64 = 10.0;

pub type HeightmapVertex = [f32; 5];
#[derive(Component, Clone)]
pub struct Heightmap {
    heightmap: Array2D<[f32; 5]>,
}
impl Heightmap {
    pub fn new() -> Self {
        Self {
            heightmap: Array2D::filled_with(
                [0.0, 0.0, 0.0, 0.0, 0.0],
                CHUNK_SIZE as usize,
                CHUNK_SIZE as usize,
            ),
        }
    }
}

impl Index<TilePosition2D> for Heightmap {
    type Output = HeightmapVertex;

    fn index(&self, index: TilePosition2D) -> &Self::Output {
        &self.heightmap[index.into()]
    }
}
impl IndexMut<TilePosition2D> for Heightmap {
    fn index_mut(&mut self, index: TilePosition2D) -> &mut Self::Output {
        &mut self.heightmap[index.into()]
    }
}
impl Index<ChunkTilePosition> for Heightmap {
    type Output = HeightmapVertex;

    fn index(&self, index: ChunkTilePosition) -> &Self::Output {
        &self.heightmap[index.tile_position_2d().into()]
    }
}
impl IndexMut<ChunkTilePosition> for Heightmap {
    fn index_mut(&mut self, index: ChunkTilePosition) -> &mut Self::Output {
        &mut self.heightmap[index.tile_position_2d().into()]
    }
}

type Rounding = f32;

pub fn generate_heightmap(seed: u32, position: ChunkPosition) -> Heightmap {
    let perlin = Perlin::new(seed);
    let mut heightmap = Heightmap {
        heightmap: Array2D::filled_with(
            [0.0, 0.0, 0.0, 0.0, 0.0],
            CHUNK_SIZE as usize,
            CHUNK_SIZE as usize,
        ),
    };
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            let chunk_x = (position[0] * CHUNK_SIZE) as f64;
            let chunk_y = (position[1] * CHUNK_SIZE) as f64;
            let x = x as f64;
            let y = y as f64;
            let top_left = (normalize_noise(
                perlin.get([(chunk_x + x) * NOISE_SCALE, (chunk_y + y) * NOISE_SCALE]),
            ) * NOISE_AMPLITUDE) as Rounding;
            let top_right = (normalize_noise(perlin.get([
                (chunk_x + x + 1.0) * NOISE_SCALE,
                (chunk_y + y) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE) as Rounding;
            let bottom_left = (normalize_noise(perlin.get([
                (chunk_x + x) * NOISE_SCALE,
                (chunk_y + y + 1.0) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE) as Rounding;
            let bottom_right = (normalize_noise(perlin.get([
                (chunk_x + x + 1.0) * NOISE_SCALE,
                (chunk_y + y + 1.0) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE) as Rounding;
            let middle = (normalize_noise(perlin.get([
                (chunk_x + x + 0.5) * NOISE_SCALE,
                (chunk_y + y + 0.5) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE) as Rounding;

            heightmap.heightmap[(x as usize, y as usize)] = [
                top_left as f32,
                top_right as f32,
                bottom_right as f32,
                bottom_left as f32,
                middle as f32,
            ];
        }
    }
    heightmap
}

pub fn normalize_noise(noise: f64) -> f64 {
    (noise + 1.0) / 2.0
}
