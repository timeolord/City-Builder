//! A simple 3D scene with light shining over a cube sitting on a plane.
mod camera;
mod chunk;
mod constants;
mod cursor;
mod math_utils;
mod menu;
mod mesh_generator;
mod world;
use bevy::prelude::*;
use constants::DEFAULT_TIMESTEP;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    MainMenu,
    AssetBuilder,
    World,
}

fn main() {
    let plugins = (
        DefaultPlugins,
        menu::MenuPlugin,
        world::WorldPlugin,
        chunk::ChunkPlugin,
    );
    App::new()
        .add_state::<GameState>()
        .add_plugins(plugins)
        .insert_resource(Time::<Fixed>::from_seconds(DEFAULT_TIMESTEP))
        .run();
}
