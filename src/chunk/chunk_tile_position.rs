use std::ops::{Add, Neg, Sub};

use bevy::{
    ecs::component::Component,
    math::{IVec2, IVec3, UVec2, Vec3Swizzles, Vec2},
    prelude::Vec3,
};
use enum_map::{Enum, EnumMap};
use num_traits::AsPrimitive;

use crate::{
    constants::{CHUNK_SIZE, TILE_SIZE},
    math_utils::Mean,
    world::heightmap::Heightmap,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub type TilePosition2D = IVec2;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ChunkPosition {
    pub position: UVec2,
}
impl ChunkPosition {
    pub fn from_world_position(world_position: Vec3) -> ChunkPosition {
        TilePosition::from_world_position(world_position).chunk_position()
    }
    pub fn as_tuple<T>(&self) -> (T, T) where u32: AsPrimitive<T>, T: Copy + 'static{
        (self.position.x.as_(), self.position.y.as_())
    }
}
pub type Neighbours<T> = EnumMap<CardinalDirection, T>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]

pub struct TilePosition {
    pub position: IVec3,
}

impl TilePosition {
    pub fn position_2d(&self) -> TilePosition2D {
        self.position.xz()
    }
    pub fn chunk_position(&self) -> ChunkPosition {
        ChunkPosition {
            position: UVec2::new(
                ((self.position.x as f32 / CHUNK_SIZE as f32).floor()) as u32,
                ((self.position.z as f32 / CHUNK_SIZE as f32).floor()) as u32,
            ),
        }
    }
    pub fn tile_neighbours(&self) -> Neighbours<TilePosition> {
        let mut neighbours = Neighbours::default();
        for direction in CardinalDirection::iter() {
            neighbours[direction] = *self + direction;
        }
        neighbours
    }
    pub fn non_diagonal_tile_neighbours(&self) -> Neighbours<TilePosition> {
        let mut neighbours = Neighbours::default();
        for direction in CardinalDirection::non_compound_directions().into_iter() {
            neighbours[direction] = *self + direction;
        }
        neighbours
    }
    pub fn to_world_position(&self) -> Vec3 {
        let mut world_position = Vec3::new(
            self.position.x as f32 * TILE_SIZE,
            self.position.y as f32 * TILE_SIZE,
            self.position.z as f32 * TILE_SIZE,
        );
        world_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position
    }
    pub fn to_world_position_2d(&self) -> Vec2 {
        let mut world_position = Vec2::new(
            self.position.x as f32 * TILE_SIZE,
            self.position.z as f32 * TILE_SIZE,
        );
        world_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position.y -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position
    }
    pub fn to_world_position_with_height(&self, heightmap: &Heightmap) -> Vec3 {
        let mut world_position = Vec3::new(
            self.chunk_position().position.x as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
            heightmap[*self].into_iter().mean_f32(),
            self.chunk_position().position.y as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
        );
        world_position.x += self.position.x as f32 * TILE_SIZE;
        world_position.y += self.position[1] as f32 * TILE_SIZE;
        world_position.z += self.position.z as f32 * TILE_SIZE;
        world_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position
    }
    pub fn chunk_neighbours(&self) -> Neighbours<ChunkPosition> {
        let mut neighbours = Neighbours::default();
        for direction in CardinalDirection::iter() {
            neighbours[direction] = self.chunk_position() + direction;
        }
        neighbours
    }
    pub fn from_world_position(world_position: Vec3) -> TilePosition {
        let mut position = world_position;
        position.x += ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        position.z += ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        position.x += TILE_SIZE / 2.0;
        position.z += TILE_SIZE / 2.0;
        position.x = position.x.floor();
        position.y = position.y.floor();
        position.z = position.z.floor();
        TilePosition {
            position: position.as_ivec3(),
        }
    }
    pub fn to_relative_tile_position(&self) -> TilePosition {
        let mut position = self.position;
        position.x -= self.chunk_position().position.x as i32 * CHUNK_SIZE as i32;
        position.z -= self.chunk_position().position.y as i32 * CHUNK_SIZE as i32;
        TilePosition { position }
    }
    //pub fn from_world_position(world_position: Vec3) -> TilePosition {
    //    let mut position = Self::world_position_to_tile_position(world_position);
    //    let chunk_position = [
    //        ((position.x as f32 / CHUNK_SIZE as f32).floor()) as i32,
    //        ((position.z as f32 / CHUNK_SIZE as f32).floor()) as i32,
    //    ];
    //    position.x -= (chunk_position.x) * CHUNK_SIZE as i32;
    //    position.z -= (chunk_position[1]) * CHUNK_SIZE as i32;
    //    TilePosition { position }
    //}
    pub fn from_position_2d(position: TilePosition2D) -> TilePosition {
        TilePosition {
            position: IVec3::new(position.x, 0, position.y),
        }
    }
    //pub fn clamp_to_world(&self, world_size: WorldSize) -> TilePosition {
    //    let min_values = IVec3::new(0, i32::MIN, 0);
    //    let max_values = IVec3::new(
    //        world_size[0] as i32 * CHUNK_SIZE as i32 - 1,
    //        i32::MAX,
    //        world_size[1] as i32 * CHUNK_SIZE as i32 - 1,
    //    );
    //    TilePosition {
    //        position: self.position.clamp(min_values, max_values),
    //    }
    //}
}
impl Add<CardinalDirection> for TilePosition {
    type Output = TilePosition;

    fn add(self, rhs: CardinalDirection) -> Self::Output {
        let mut position = self.position;
        match rhs {
            CardinalDirection::North => {
                position.x += 1;
            }
            CardinalDirection::NorthEast => {
                position.x += 1;
                position.z += 1;
            }
            CardinalDirection::East => {
                position.z += 1;
            }
            CardinalDirection::SouthEast => {
                position.x -= 1;
                position.z += 1;
            }
            CardinalDirection::South => {
                position.x -= 1;
            }
            CardinalDirection::SouthWest => {
                position.x -= 1;
                position.z -= 1;
            }
            CardinalDirection::West => {
                position.z -= 1;
            }
            CardinalDirection::NorthWest => {
                position.x += 1;
                position.z -= 1;
            }
        }
        TilePosition { position }
    }
}
impl Add<CardinalDirection> for ChunkPosition {
    type Output = ChunkPosition;

    fn add(self, rhs: CardinalDirection) -> Self::Output {
        let mut position = self.position;
        match rhs {
            CardinalDirection::North => {
                position.x += 1;
            }
            CardinalDirection::NorthEast => {
                position.x += 1;
                position.y += 1;
            }
            CardinalDirection::East => {
                position.y += 1;
            }
            CardinalDirection::SouthEast => {
                position.x -= 1;
                position.y += 1;
            }
            CardinalDirection::South => {
                position.x -= 1;
            }
            CardinalDirection::SouthWest => {
                position.x -= 1;
                position.y -= 1;
            }
            CardinalDirection::West => {
                position.y -= 1;
            }
            CardinalDirection::NorthWest => {
                position.x += 1;
                position.y -= 1;
            }
        }
        ChunkPosition { position }
    }
}
impl Sub<CardinalDirection> for TilePosition {
    type Output = TilePosition;

    fn sub(self, rhs: CardinalDirection) -> Self::Output {
        self + (-rhs)
    }
}
impl Sub<CardinalDirection> for ChunkPosition {
    type Output = ChunkPosition;

    fn sub(self, rhs: CardinalDirection) -> Self::Output {
        self + (-rhs)
    }
}
#[derive(Enum, EnumIter, Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardinalDirection {
    North = 0,
    NorthEast = 1,
    East = 2,
    SouthEast = 3,
    South = 4,
    SouthWest = 5,
    West = 6,
    NorthWest = 7,
}
impl CardinalDirection {
    pub fn non_compound_directions() -> impl Iterator<Item=CardinalDirection> {
        [
            CardinalDirection::North,
            CardinalDirection::East,
            CardinalDirection::South,
            CardinalDirection::West,
        ].into_iter()
    }
    pub fn split_direction(&self) -> [CardinalDirection; 2] {
        match self {
            CardinalDirection::North => {
                [CardinalDirection::NorthWest, CardinalDirection::NorthEast]
            }
            CardinalDirection::NorthEast => [CardinalDirection::North, CardinalDirection::East],
            CardinalDirection::East => [CardinalDirection::NorthEast, CardinalDirection::SouthEast],
            CardinalDirection::SouthEast => [CardinalDirection::East, CardinalDirection::South],
            CardinalDirection::South => {
                [CardinalDirection::SouthEast, CardinalDirection::SouthWest]
            }
            CardinalDirection::SouthWest => [CardinalDirection::South, CardinalDirection::West],
            CardinalDirection::West => [CardinalDirection::SouthWest, CardinalDirection::NorthWest],
            CardinalDirection::NorthWest => [CardinalDirection::West, CardinalDirection::North],
        }
    }
    pub fn next_clockwise(&self) -> CardinalDirection {
        match self {
            CardinalDirection::North => CardinalDirection::NorthEast,
            CardinalDirection::NorthEast => CardinalDirection::East,
            CardinalDirection::East => CardinalDirection::SouthEast,
            CardinalDirection::SouthEast => CardinalDirection::South,
            CardinalDirection::South => CardinalDirection::SouthWest,
            CardinalDirection::SouthWest => CardinalDirection::West,
            CardinalDirection::West => CardinalDirection::NorthWest,
            CardinalDirection::NorthWest => CardinalDirection::North,
        }
    }
    pub fn next_counter_clockwise(&self) -> CardinalDirection {
        match self {
            CardinalDirection::North => CardinalDirection::NorthWest,
            CardinalDirection::NorthEast => CardinalDirection::North,
            CardinalDirection::East => CardinalDirection::NorthEast,
            CardinalDirection::SouthEast => CardinalDirection::East,
            CardinalDirection::South => CardinalDirection::SouthEast,
            CardinalDirection::SouthWest => CardinalDirection::South,
            CardinalDirection::West => CardinalDirection::SouthWest,
            CardinalDirection::NorthWest => CardinalDirection::West,
        }
    }
    pub fn to_angle(&self) -> f32 {
        match self {
            CardinalDirection::North => 0.0,
            CardinalDirection::NorthEast => 45.0,
            CardinalDirection::East => 90.0,
            CardinalDirection::SouthEast => 135.0,
            CardinalDirection::South => 180.0,
            CardinalDirection::SouthWest => -135.0,
            CardinalDirection::West => -90.0,
            CardinalDirection::NorthWest => -45.0,
        }
    }
}
impl Neg for CardinalDirection {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            CardinalDirection::North => CardinalDirection::South,
            CardinalDirection::NorthEast => CardinalDirection::SouthWest,
            CardinalDirection::East => CardinalDirection::West,
            CardinalDirection::SouthEast => CardinalDirection::NorthWest,
            CardinalDirection::South => CardinalDirection::North,
            CardinalDirection::SouthWest => CardinalDirection::NorthEast,
            CardinalDirection::West => CardinalDirection::East,
            CardinalDirection::NorthWest => CardinalDirection::SouthEast,
        }
    }
}
