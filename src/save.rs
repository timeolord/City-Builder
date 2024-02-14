use std::{env, fs, path::PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    world::WorldSettings,
    world_gen::{heightmap::{self, Heightmap}, WorldGenSettings},
};

pub fn initalize_file_structure() {
    std::fs::create_dir_all(save_path()).unwrap();
}

pub fn save_path() -> PathBuf {
    let mut path = env::current_dir().unwrap();
    path.push("saves");
    path
}

#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    heightmap: Heightmap,
    world_settings: WorldSettings,
    world_gen_settings: WorldGenSettings,
}

#[derive(Event)]
pub struct SaveEvent(pub PathBuf);
#[derive(Event)]
pub struct LoadEvent(pub PathBuf);

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveEvent>();
        app.add_event::<LoadEvent>();
        app.add_systems(Startup, initalize_file_structure);
        app.add_systems(PostUpdate, (save_file, load_file));
    }
}

pub fn save_file(
    heightmap: Option<Res<Heightmap>>,
    world_settings: Option<Res<WorldSettings>>,
    world_gen_settings: Option<Res<WorldGenSettings>>,
    mut save_event: EventReader<SaveEvent>,
) {
    for event in save_event.read() {
        let heightmap = (*heightmap.as_ref().unwrap()).clone();
        let world_settings = (*world_settings.as_ref().unwrap()).clone();
        let world_gen_settings = (*world_gen_settings.as_ref().unwrap()).clone();

        let save = SaveFile {
            heightmap,
            world_settings,
            world_gen_settings,
        };
        let path = save_path().join(&event.0);
        fs::write(path, &ron::to_string(&save).unwrap()).unwrap();
    }
}

pub fn load_file(
    mut commands: Commands,
    mut load_event: EventReader<LoadEvent>,
) {
    for event in load_event.read() {
        let path = save_path().join(&event.0);
        let save: SaveFile = ron::from_str(&fs::read_to_string(path).unwrap()).unwrap();

        commands.insert_resource(save.heightmap.clone());
        commands.insert_resource(save.world_settings.clone());
        commands.insert_resource(save.world_gen_settings.clone());
    }
}
