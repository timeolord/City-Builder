use crate::world::WorldSize;

pub const CHUNK_SIZE: u32 = 128;
pub const HEIGHTMAP_CHUNK_SIZE: u32 = CHUNK_SIZE + 1;
pub const CHUNK_WORLD_SIZE: WorldSize = [16, 16];
pub const TILE_WORLD_SIZE: WorldSize = [
    CHUNK_WORLD_SIZE[0] * CHUNK_SIZE,
    CHUNK_WORLD_SIZE[1] * CHUNK_SIZE,
];
pub const TILE_SIZE: f32 = 1.0;
pub const WORLD_HEIGHT_SCALE: f32 = 300.0;

pub const SNOW_HEIGHT: f32 = WORLD_HEIGHT_SCALE * 0.5;

pub const MAX_DROPLET_SIZE: u32 = 12;
pub const MIN_DROPLET_SIZE: u32 = 2;
pub const EROSION_WORKGROUP_SIZE: u64 = 64;
pub const EROSION_DISPATCH_SIZE: u64 = 16;
pub const MAX_EROSION_STEPS: u64 = 500;

pub const LOD_LEVELS: u32 = 5;