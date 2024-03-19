use crate::world::WorldSize;

pub const CHUNK_SIZE: u32 = 128;
pub const HEIGHTMAP_CHUNK_SIZE: u32 = CHUNK_SIZE + 1;
pub const CHUNK_WORLD_SIZE: WorldSize = [16, 16];
pub const TILE_WORLD_SIZE: WorldSize = [
    CHUNK_WORLD_SIZE[0] * CHUNK_SIZE,
    CHUNK_WORLD_SIZE[1] * CHUNK_SIZE,
];
