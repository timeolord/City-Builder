use bevy::prelude::*;
use enum_map::{Enum, EnumMap};
use strum_macros::{Display, EnumIter};

pub mod asset_loader;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct TerrainTextures {
    textures: EnumMap<TerrainType, (Handle<Image>, Handle<StandardMaterial>)>
}

#[derive(Enum, EnumIter, Display)]
pub enum TerrainType {
    Grass,
    Dirt,
    Stone,
    Sand,
    Snow,
}