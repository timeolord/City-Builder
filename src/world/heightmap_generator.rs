use std::ops::{Index, IndexMut};

use array2d::Array2D;
use bevy::{
    gizmos::gizmos::Gizmos,
    math::{Vec3, Vec3Swizzles},
    prelude::Component,
    render::color::Color,
};
use noise::{NoiseFn, Perlin};

use crate::{
    chunk::{
        chunk_tile_position::{ChunkPosition, ChunkTilePosition, TilePosition2D},
        unnormalized_normal_vector,
    },
    constants::{CHUNK_SIZE, TILE_SIZE},
};

const NOISE_SCALE: f64 = 0.1;
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
    pub fn get_from_world_position(&self, position: Vec3) -> Vec3 {
        let tile_position = ChunkTilePosition::from_world_position(position);
        let normalized_world_position = position.xz().fract().abs() + f32::EPSILON;
        let triangle_position = (
            normalized_world_position.y - normalized_world_position.x,
            normalized_world_position.y + normalized_world_position.x - 1.0,
        );
        let heights = self.heightmap[tile_position.tile_position_2d().into()];
        let vert_0 = [0.0, heights[0], 0.0];
        let vert_1 = [1.0, heights[1], 0.0];
        let vert_2 = [1.0, heights[2], 1.0];
        let vert_3 = [0.0, heights[3], 1.0];
        let vert_4 = [0.5, heights[4], 0.5];
        //let tile_size = 0.5 * TILE_SIZE;
        //let vert_0 = [-tile_size, heights[0], -tile_size];
        //let vert_1 = [tile_size, heights[1], -tile_size];
        //let vert_2 = [tile_size, heights[2], tile_size];
        //let vert_3 = [-tile_size, heights[3], tile_size];
        //let vert_4 = [0.0, heights[4], 0.0];
        if triangle_position.0 < 0.0 && triangle_position.1 < 0.0 {
            interpolate_height(vert_0, vert_4, vert_1, normalized_world_position, position)
        } else if triangle_position.0 > 0.0 && triangle_position.1 > 0.0 {
            interpolate_height(vert_4, vert_3, vert_2, normalized_world_position, position)
        } else if triangle_position.0 < 0.0 && triangle_position.1 > 0.0 {
            interpolate_height(vert_1, vert_4, vert_2, normalized_world_position, position)
        } else if triangle_position.0 > 0.0 && triangle_position.1 < 0.0 {
            interpolate_height(vert_3, vert_4, vert_0, normalized_world_position, position)
        //} else if triangle_position == (1.0, 0.0) {
        //    Vec3::new(position.x, vert_0[1], position.z)
        //} else if triangle_position == (0.0, 1.0) {
        //    Vec3::new(position.x, vert_1[1], position.z)
        //} else if triangle_position == (-1.0, 0.0) {
        //    Vec3::new(position.x, vert_2[1], position.z)
        //} else if triangle_position == (0.0, -1.0) {
        //    Vec3::new(position.x, vert_3[1], position.z)
        //} else if triangle_position == (0.0, 0.0) {
        //    Vec3::new(position.x, vert_4[1], position.z)
        } else {
            panic!("Triangle position: {:?}", triangle_position)
        }
    }
}

fn interpolate_height(
    vert_0: [f32; 3],
    vert_1: [f32; 3],
    vert_2: [f32; 3],
    normalized_world_position: bevy::prelude::Vec2,
    position: Vec3,
) -> Vec3 {
    let normal_vector = unnormalized_normal_vector(vert_0, vert_1, vert_2);
    let d = normal_vector.dot(vert_0.into());
    let height = (-normal_vector.x * normalized_world_position.x
        - normal_vector.z * normalized_world_position.y
        + d)
        / normal_vector.y;
    //gizmos.ray(position, normal_vector, Color::RED);
    //println!(
    //    "{:?} {:?} {:?} {:?} {:?}",
    //    tile_position,
    //    triangle_position,
    //    normalized_world_position,
    //    normal_vector,
    //    height.abs()
    //);

    Vec3::new(position.x, height.abs(), position.z)
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
