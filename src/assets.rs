use bevy::prelude::*;
use enum_map::{Enum, EnumMap};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

pub mod asset_loader;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct TerrainTextures {
    textures: EnumMap<TerrainType, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct TerrainTextureAtlas {
    pub handle: Handle<StandardMaterial>,
}

pub fn get_terrain_texture_uv(terrain_type: TerrainType) -> [[f32; 2]; 4] {
    let uv_height_start = (terrain_type as u32 as f32) / TerrainType::iter().len() as f32;
    let uv_height_end = (terrain_type as u32 as f32 + 1.0) / TerrainType::iter().len() as f32;
    let uv_0 = [0.0, uv_height_start];
    let uv_1 = [1.0, uv_height_start];
    let uv_2 = [1.0, uv_height_end];
    let uv_3 = [0.0, uv_height_end];
    [uv_0, uv_1, uv_2, uv_3]
}

#[derive(Enum, EnumIter, Display, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TerrainType {
    Grass = 0,
    Dirt,
    Stone,
    Sand,
    Snow,
}
