use std::fs;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    world::WorldSettings,
    world_gen::{
        heightmap::{self, Heightmap},
        noise_generator::NoiseSettings,
        WorldGenSettings,
    },
};

pub fn initalize_file_structure() {
    std::fs::create_dir_all(SAVE_LOCATION).unwrap();
}

const SAVE_LOCATION: &str = "./saves/";

pub fn save_location(file_name: &str) -> String {
    format!("{}{}", SAVE_LOCATION, file_name)
}

#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    heightmap: Heightmap,
    world_settings: WorldSettings,
    world_gen_settings: WorldGenSettings,
}

//TODO add save name
#[derive(Event, Default)]
pub struct SaveEvent;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveEvent>();
        app.add_systems(Startup, initalize_file_structure);
        app.add_systems(PostUpdate, save_file.run_if(on_event::<SaveEvent>()));
    }
}

pub fn save_file(
    heightmap: Res<Heightmap>,
    world_settings: Res<WorldSettings>,
    world_gen_settings: Res<WorldGenSettings>,
) {
    let save = SaveFile {
        heightmap: heightmap.clone(),
        world_settings: world_settings.clone(),
        world_gen_settings: world_gen_settings.clone(),
    };
    fs::write(save_location("save.ron"), &ron::to_string(&save).unwrap()).unwrap();
}
