use std::ops::{Index, IndexMut};

use bevy::prelude::{Component, Vec3};

use crate::{constants::{CHUNK_SIZE, TILE_SIZE}, world::heightmap_generator::Heightmap};

pub type TilePosition3D = [usize; 3];
pub type TilePosition2D = [usize; 2];

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPosition {
    pub position: [usize; 2],
}
impl Index<usize> for ChunkPosition {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.position[index]
    }
}
impl IndexMut<usize> for ChunkPosition {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.position[index]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]

pub struct ChunkTilePosition {
    pub chunk_position: ChunkPosition,
    pub tile_position: TilePosition3D,
}

#[derive(Clone, Copy, Debug)]
pub struct TileNeighbours {
    pub north: Option<ChunkTilePosition>,
    pub south: Option<ChunkTilePosition>,
    pub east: Option<ChunkTilePosition>,
    pub west: Option<ChunkTilePosition>,
    pub north_east: Option<ChunkTilePosition>,
    pub north_west: Option<ChunkTilePosition>,
    pub south_east: Option<ChunkTilePosition>,
    pub south_west: Option<ChunkTilePosition>,
}
impl TileNeighbours {
    pub fn has_all(&self) -> bool {
        self.north.is_some()
            && self.south.is_some()
            && self.east.is_some()
            && self.west.is_some()
            && self.north_east.is_some()
            && self.north_west.is_some()
            && self.south_east.is_some()
            && self.south_west.is_some()
    }
    pub fn to_array(&self) -> [Option<ChunkTilePosition>; 8] {
        [
            self.north,
            self.south,
            self.east,
            self.west,
            self.north_east,
            self.north_west,
            self.south_east,
            self.south_west,
        ]
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ChunkNeighbours {
    pub north: Option<ChunkPosition>,
    pub south: Option<ChunkPosition>,
    pub east: Option<ChunkPosition>,
    pub west: Option<ChunkPosition>,
    pub north_east: Option<ChunkPosition>,
    pub north_west: Option<ChunkPosition>,
    pub south_east: Option<ChunkPosition>,
    pub south_west: Option<ChunkPosition>,
}
impl ChunkNeighbours {
    pub fn has_all(&self) -> bool {
        self.north.is_some()
            && self.south.is_some()
            && self.east.is_some()
            && self.west.is_some()
            && self.north_east.is_some()
            && self.north_west.is_some()
            && self.south_east.is_some()
            && self.south_west.is_some()
    }
    pub fn to_array(&self) -> [Option<ChunkPosition>; 8] {
        [
            self.north,
            self.south,
            self.east,
            self.west,
            self.north_east,
            self.north_west,
            self.south_east,
            self.south_west,
        ]
    }
}

impl ChunkTilePosition {
    pub fn tile_position_2d(&self) -> TilePosition2D {
        [self.tile_position[0], self.tile_position[2]]
    }
    pub fn tile_neighbours(&self) -> TileNeighbours {
        fn neighbour(
            tile_pos: TilePosition3D,
            x_offset: isize,
            z_offset: isize,
        ) -> Option<ChunkTilePosition> {
            let tile_pos = [
                (tile_pos[0] as isize + x_offset),
                tile_pos[1] as isize,
                (tile_pos[2] as isize + z_offset),
            ];
            if (tile_pos[0] < 0) || (tile_pos[2] < 0) {
                None
            } else {
                let tile_pos = [
                    tile_pos[0] as usize,
                    tile_pos[1] as usize,
                    tile_pos[2] as usize,
                ];
                Some(ChunkTilePosition::from_tile_position(tile_pos))
            }
        }
        let mut tile_position = self.tile_position;
        tile_position[0] += self.chunk_position[0] * CHUNK_SIZE;
        tile_position[2] += self.chunk_position[1] * CHUNK_SIZE;
        TileNeighbours {
            north: neighbour(tile_position, 0, 1),
            south: neighbour(tile_position, 0, -1),
            east: neighbour(tile_position, 1, 0),
            west: neighbour(tile_position, -1, 0),
            north_east: neighbour(tile_position, 1, 1),
            north_west: neighbour(tile_position, -1, 1),
            south_east: neighbour(tile_position, 1, -1),
            south_west: neighbour(tile_position, -1, -1),
        }
    }
    pub fn non_diagonal_tile_neighbours(&self) -> TileNeighbours{
        fn neighbour(
            tile_pos: TilePosition3D,
            x_offset: isize,
            z_offset: isize,
        ) -> Option<ChunkTilePosition> {
            let tile_pos = [
                (tile_pos[0] as isize + x_offset),
                tile_pos[1] as isize,
                (tile_pos[2] as isize + z_offset),
            ];
            if (tile_pos[0] < 0) || (tile_pos[2] < 0) {
                None
            } else {
                let tile_pos = [
                    tile_pos[0] as usize,
                    tile_pos[1] as usize,
                    tile_pos[2] as usize,
                ];
                Some(ChunkTilePosition::from_tile_position(tile_pos))
            }
        }
        let mut tile_position = self.tile_position;
        tile_position[0] += self.chunk_position[0] * CHUNK_SIZE;
        tile_position[2] += self.chunk_position[1] * CHUNK_SIZE;
        TileNeighbours {
            north: neighbour(tile_position, 0, 1),
            south: neighbour(tile_position, 0, -1),
            east: neighbour(tile_position, 1, 0),
            west: neighbour(tile_position, -1, 0),
            north_east: None,
            north_west: None,
            south_east: None,
            south_west: None,
        }
    }
    pub fn to_world_position(&self) -> Vec3 {
        let mut world_position = Vec3::new(
            self.chunk_position[0] as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
            0.0,
            self.chunk_position[1] as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
        );
        world_position.x += self.tile_position[0] as f32 * TILE_SIZE;
        world_position.y += self.tile_position[1] as f32 * TILE_SIZE;
        world_position.z += self.tile_position[2] as f32 * TILE_SIZE;
        world_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position
    }
    pub fn to_world_position_with_height(&self, heighmap: &Heightmap) -> Vec3 {
        let mut world_position = Vec3::new(
            self.chunk_position[0] as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
            heighmap[*self][4],
            self.chunk_position[1] as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
        );
        world_position.x += self.tile_position[0] as f32 * TILE_SIZE;
        world_position.y += self.tile_position[1] as f32 * TILE_SIZE;
        world_position.z += self.tile_position[2] as f32 * TILE_SIZE;
        world_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position
    }
    pub fn chunk_neighbours(&self) -> ChunkNeighbours {
        fn neighbour(
            chunk_pos: ChunkPosition,
            x_offset: isize,
            z_offset: isize,
        ) -> Option<ChunkPosition> {
            let chunk_pos = [
                (chunk_pos[0] as isize + x_offset),
                (chunk_pos[1] as isize + z_offset),
            ];
            if (chunk_pos[0] < 0) || (chunk_pos[1] < 0) {
                None
            } else {
                let chunk_pos = [chunk_pos[0] as usize, chunk_pos[1] as usize];
                Some(ChunkPosition {
                    position: chunk_pos,
                })
            }
        }
        ChunkNeighbours {
            north: neighbour(self.chunk_position, 0, 1),
            south: neighbour(self.chunk_position, 0, -1),
            east: neighbour(self.chunk_position, 1, 0),
            west: neighbour(self.chunk_position, -1, 0),
            north_east: neighbour(self.chunk_position, 1, 1),
            north_west: neighbour(self.chunk_position, -1, 1),
            south_east: neighbour(self.chunk_position, 1, -1),
            south_west: neighbour(self.chunk_position, -1, -1),
        }
    }
    pub fn world_position_to_tile_position(world_position: Vec3) -> TilePosition3D {
        let mut tile_position = world_position;
        tile_position.x += ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        tile_position.z += ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        tile_position.x += TILE_SIZE / 2.0;
        tile_position.z += TILE_SIZE / 2.0;
        tile_position.x = tile_position.x.floor();
        tile_position.y = tile_position.y.floor();
        tile_position.z = tile_position.z.floor();
        [
            tile_position.x as usize,
            tile_position.y as usize,
            tile_position.z as usize,
        ]
    }
    pub fn from_world_position(world_position: Vec3) -> ChunkTilePosition {
        let mut tile_position = Self::world_position_to_tile_position(world_position);
        let chunk_position = [
            ((tile_position[0] as f32 / CHUNK_SIZE as f32).floor()) as usize,
            ((tile_position[2] as f32 / CHUNK_SIZE as f32).floor()) as usize,
        ];
        tile_position[0] -= (chunk_position[0]) * CHUNK_SIZE;
        tile_position[2] -= (chunk_position[1]) * CHUNK_SIZE;
        ChunkTilePosition {
            chunk_position: ChunkPosition {
                position: [chunk_position[0] as usize, chunk_position[1] as usize],
            },
            tile_position,
        }
    }
    pub fn from_tile_position(tile_position: TilePosition3D) -> ChunkTilePosition {
        let chunk_position = [
            ((tile_position[0] as f32 / CHUNK_SIZE as f32).floor()) as usize,
            ((tile_position[2] as f32 / CHUNK_SIZE as f32).floor()) as usize,
        ];
        let mut tile_position = tile_position;
        tile_position[0] -= (chunk_position[0]) * CHUNK_SIZE;
        tile_position[2] -= (chunk_position[1]) * CHUNK_SIZE;
        ChunkTilePosition {
            chunk_position: ChunkPosition {
                position: chunk_position,
            },
            tile_position,
        }
    }
    pub fn from_tile_position_2d(tile_position: TilePosition2D) -> ChunkTilePosition {
        ChunkTilePosition::from_tile_position([tile_position[0], 0, tile_position[1]])
    }
    pub fn as_tile_position(&self) -> TilePosition3D {
        let mut tile_position = self.tile_position;
        tile_position[0] += self.chunk_position[0] * CHUNK_SIZE;
        tile_position[2] += self.chunk_position[1] * CHUNK_SIZE;
        tile_position
    }
    pub fn as_tile_position_2d(&self) -> TilePosition2D {
        let tile_position = self.as_tile_position();
        [tile_position[0], tile_position[2]]
    }
    pub fn clamp_to_world(&self, world_size: [usize; 2]) -> ChunkTilePosition {
        let mut tile_position = self.as_tile_position();
        tile_position[0] = tile_position[0].clamp(0, world_size[0] * CHUNK_SIZE - 1);
        tile_position[2] = tile_position[2].clamp(0, world_size[1] * CHUNK_SIZE - 1);
        ChunkTilePosition::from_tile_position(tile_position)
    }
}
impl Default for ChunkTilePosition {
    fn default() -> Self {
        Self {
            chunk_position: ChunkPosition { position: [0, 0] },
            tile_position: [0, 0, 0],
        }
    }
}
