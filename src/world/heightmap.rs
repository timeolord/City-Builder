use array2d::Array2D;
use bevy::{
    ecs::system::Resource,
    math::{UVec2, Vec2, Vec3, Vec3Swizzles},
};
use noise::{NoiseFn, Perlin};
use std::ops::{Index, IndexMut};
use bevy_easings::Lerp;

use crate::{
    chunk::chunk_tile_position::{CardinalDirection, ChunkPosition, TilePosition, TilePosition2D},
    constants::{CHUNK_SIZE, HEIGHT_STEP, TILE_SIZE},
    math_utils::{unnormalized_normal_array, Mean, RoundBy},
};

use super::WorldSize;

const NOISE_SCALE: f64 = 0.025;
const NOISE_AMPLITUDE: f64 = 20.0;

#[derive(Resource, Clone)]
pub struct HeightmapsResource {
    heightmaps: Array2D<Heightmap>,
    dirty_chunks: Array2D<bool>,
}
impl HeightmapsResource {
    pub fn new(world_size: WorldSize, seed: u32) -> Self {
        let mut heightmaps = Array2D::filled_with(
            Heightmap::default(),
            world_size[0] as usize,
            world_size[1] as usize,
        );
        for x in 0..world_size[0] {
            for y in 0..world_size[1] {
                heightmaps[(x as usize, y as usize)] = generate_heightmap(
                    seed,
                    ChunkPosition {
                        position: UVec2::new(x as u32, y as u32),
                    },
                );
            }
        }
        let dirty_chunks =
            Array2D::filled_with(false, world_size[0] as usize, world_size[1] as usize);

        Self {
            heightmaps,
            dirty_chunks,
        }
    }
    pub fn get_from_world_position(&self, position: Vec3) -> Vec3 {
        self[TilePosition::from_world_position(position).chunk_position()]
            .get_from_world_position(position)
    }
    pub fn get_from_world_position_2d(&self, position: Vec2) -> Vec3 {
        let position = Vec3::new(position.x, 0.0, position.y);
        self[TilePosition::from_world_position(position).chunk_position()]
            .get_from_world_position(position)
    }
    pub fn edit_tile(&mut self, position: TilePosition, heights: [f32; 4]) {
        self.edit_tiles(vec![position], vec![heights])
    }
    pub fn edit_tiles(&mut self, positions: Vec<TilePosition>, heights: Vec<[f32; 4]>) {
        for (position, heights) in positions.iter().zip(heights.into_iter()) {
            self.dirty_chunks[position.chunk_position().as_tuple()] = true;
            self[*position] = heights;
            for (direction, neighbour) in position.tile_neighbours() {
                if positions.contains(&neighbour) {
                    continue;
                }
                self.dirty_chunks[neighbour.chunk_position().as_tuple()] = true;
                //Make neighbours conform to the edited tile
                match direction {
                    crate::chunk::chunk_tile_position::CardinalDirection::North => {
                        self[neighbour][0] = heights[1];
                        self[neighbour][3] = heights[2];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::NorthEast => {
                        self[neighbour][0] = heights[2];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::East => {
                        self[neighbour][0] = heights[3];
                        self[neighbour][1] = heights[2];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::SouthEast => {
                        self[neighbour][1] = heights[3];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::South => {
                        self[neighbour][1] = heights[0];
                        self[neighbour][2] = heights[3];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::SouthWest => {
                        self[neighbour][2] = heights[0];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::West => {
                        self[neighbour][2] = heights[1];
                        self[neighbour][3] = heights[0];
                    }
                    crate::chunk::chunk_tile_position::CardinalDirection::NorthWest => {
                        self[neighbour][3] = heights[1];
                    }
                }
            }
        }
    }
    pub fn get_dirty_chunks(&mut self) -> impl Iterator<Item = ChunkPosition> {
        let mut dirty_chunks = Vec::new();
        for x in 0..self.dirty_chunks.num_rows() {
            for y in 0..self.dirty_chunks.num_columns() {
                if self.dirty_chunks[(x, y)] {
                    dirty_chunks.push(ChunkPosition {
                        position: UVec2::new(x as u32, y as u32),
                    });
                }
            }
        }
        dirty_chunks.iter().for_each(|chunk| {
            self.dirty_chunks[chunk.as_tuple()] = false;
        });
        dirty_chunks.into_iter()
    }
}
impl Default for HeightmapsResource {
    fn default() -> Self {
        Self {
            heightmaps: Array2D::filled_with(Heightmap::default(), 0, 0),
            dirty_chunks: Array2D::filled_with(false, 0, 0),
        }
    }
}
impl Index<ChunkPosition> for HeightmapsResource {
    type Output = Heightmap;

    fn index(&self, index: ChunkPosition) -> &Self::Output {
        let index = (index.position.x as usize, index.position.y as usize);
        &self.heightmaps[index]
    }
}
impl Index<&ChunkPosition> for HeightmapsResource {
    type Output = Heightmap;

    fn index(&self, index: &ChunkPosition) -> &Self::Output {
        &self[*index]
    }
}
impl Index<&mut ChunkPosition> for HeightmapsResource {
    type Output = Heightmap;

    fn index(&self, index: &mut ChunkPosition) -> &Self::Output {
        &self[*index]
    }
}
impl IndexMut<ChunkPosition> for HeightmapsResource {
    fn index_mut(&mut self, index: ChunkPosition) -> &mut Self::Output {
        let index = (index.position.x as usize, index.position.y as usize);
        &mut self.heightmaps[index]
    }
}
impl IndexMut<&ChunkPosition> for HeightmapsResource {
    fn index_mut(&mut self, index: &ChunkPosition) -> &mut Self::Output {
        &mut self[*index]
    }
}
impl IndexMut<&mut ChunkPosition> for HeightmapsResource {
    fn index_mut(&mut self, index: &mut ChunkPosition) -> &mut Self::Output {
        &mut self[*index]
    }
}
impl Index<TilePosition> for HeightmapsResource {
    type Output = HeightmapVertex;

    fn index(&self, index: TilePosition) -> &Self::Output {
        &self[index.chunk_position()][index.to_relative_tile_position().position_2d()]
    }
}
impl Index<&TilePosition> for HeightmapsResource {
    type Output = HeightmapVertex;

    fn index(&self, index: &TilePosition) -> &Self::Output {
        &self[*index]
    }
}
impl Index<&mut TilePosition> for HeightmapsResource {
    type Output = HeightmapVertex;

    fn index(&self, index: &mut TilePosition) -> &Self::Output {
        &self[*index]
    }
}
impl IndexMut<TilePosition> for HeightmapsResource {
    fn index_mut(&mut self, index: TilePosition) -> &mut Self::Output {
        &mut self[index.chunk_position()][index.to_relative_tile_position().position_2d()]
    }
}
impl IndexMut<&TilePosition> for HeightmapsResource {
    fn index_mut(&mut self, index: &TilePosition) -> &mut Self::Output {
        &mut self[*index]
    }
}
impl IndexMut<&mut TilePosition> for HeightmapsResource {
    fn index_mut(&mut self, index: &mut TilePosition) -> &mut Self::Output {
        &mut self[*index]
    }
}

pub type HeightmapVertex = [f32; 4];
pub trait FlattenWithDirection {
    fn flatten_with_direction(&mut self, direction: CardinalDirection) -> &mut Self;
}
impl FlattenWithDirection for HeightmapVertex {
    fn flatten_with_direction(&mut self, direction: CardinalDirection) -> &mut Self {
        match direction {
            CardinalDirection::North | CardinalDirection::South => {
                self[2] = self[1];
                self[3] = self[0];
            }
            CardinalDirection::NorthEast | CardinalDirection::SouthWest => {
                let mean = [self[0], self[2]].into_iter().mean_f32();
                self[1] = mean;
                self[3] = mean;
            }
            CardinalDirection::East | CardinalDirection::West => {
                self[0] = self[1];
                self[3] = self[2];
            }
            CardinalDirection::SouthEast | CardinalDirection::NorthWest => {
                let mean = [self[1], self[3]].into_iter().mean_f32();
                self[0] = mean;
                self[2] = mean;
            }
        }
        self
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Heightmap {
    heightmap: Array2D<HeightmapVertex>,
}
impl Heightmap {
    pub fn new() -> Self {
        Self::default()
    }
    fn get_from_world_position(&self, position: Vec3) -> Vec3 {
        let tile_position = TilePosition::from_world_position(position);
        let normalized_world_position = position.xz().fract().abs() + f32::EPSILON;
        let heights = self[tile_position.to_relative_tile_position()];
        //Bilinear Interpolation to get the height
        //Not sure that this is actually correct, but visually I can't tell
        let x_1 = &[heights[0]].lerp(&[heights[1]], &normalized_world_position.x);
        let x_2 = &[heights[3]].lerp(&[heights[2]], &normalized_world_position.x);
        let y = x_1.lerp(x_2, &normalized_world_position.y);
        Vec3::new(position.x, y[0], position.z)
    }
}
impl Default for Heightmap {
    fn default() -> Self {
        Self {
            heightmap: Array2D::filled_with(
                vec![0.0; 4].try_into().unwrap(),
                CHUNK_SIZE as usize,
                CHUNK_SIZE as usize,
            ),
        }
    }
}

impl Index<TilePosition2D> for Heightmap {
    type Output = HeightmapVertex;

    fn index(&self, index: TilePosition2D) -> &Self::Output {
        let index = (index.x as usize, index.y as usize);
        &self.heightmap[index]
    }
}
impl IndexMut<TilePosition2D> for Heightmap {
    fn index_mut(&mut self, index: TilePosition2D) -> &mut Self::Output {
        let index = (index.x as usize, index.y as usize);
        &mut self.heightmap[index]
    }
}
impl Index<TilePosition> for Heightmap {
    type Output = HeightmapVertex;

    fn index(&self, index: TilePosition) -> &Self::Output {
        let index = index.position_2d();
        let index = (index.x as usize, index.y as usize);
        &self.heightmap[index]
    }
}
impl IndexMut<TilePosition> for Heightmap {
    fn index_mut(&mut self, index: TilePosition) -> &mut Self::Output {
        let index = index.position_2d();
        let index = (index.x as usize, index.y as usize);
        &mut self.heightmap[index]
    }
}

pub fn generate_heightmap(seed: u32, position: ChunkPosition) -> Heightmap {
    let perlin = Perlin::new(seed);
    let mut heightmap = Heightmap::new();
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            let chunk_x = (position.position.x * CHUNK_SIZE) as f64;
            let chunk_y = (position.position.y * CHUNK_SIZE) as f64;
            let x = x as f64;
            let y = y as f64;
            let top_left = normalize_noise(
                perlin.get([(chunk_x + x) * NOISE_SCALE, (chunk_y + y) * NOISE_SCALE]),
            ) * NOISE_AMPLITUDE;
            let top_right = normalize_noise(perlin.get([
                (chunk_x + x + TILE_SIZE as f64) * NOISE_SCALE,
                (chunk_y + y) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE;
            let bottom_left = normalize_noise(perlin.get([
                (chunk_x + x) * NOISE_SCALE,
                (chunk_y + y + TILE_SIZE as f64) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE;
            let bottom_right = normalize_noise(perlin.get([
                (chunk_x + x + TILE_SIZE as f64) * NOISE_SCALE,
                (chunk_y + y + TILE_SIZE as f64) * NOISE_SCALE,
            ])) * NOISE_AMPLITUDE;

            let heights = [
                (top_left as f32).round_by(HEIGHT_STEP),
                (top_right as f32).round_by(HEIGHT_STEP),
                (bottom_right as f32).round_by(HEIGHT_STEP),
                (bottom_left as f32).round_by(HEIGHT_STEP),
            ];

            heightmap.heightmap[(x as usize, y as usize)] = heights;
        }
    }
    heightmap
}

pub fn normalize_noise(noise: f64) -> f64 {
    (noise + 1.0) / 2.0
}
