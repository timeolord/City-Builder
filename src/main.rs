//! A simple 3D scene with light shining over a cube sitting on a plane.
mod asset_builder;
pub mod camera;
mod chunk;
mod constants;
pub mod cursor;
mod menu;
mod world;
use bevy::prelude::*;

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
        asset_builder::AssetBuilderPlugin,
        menu::MenuPlugin,
        world::WorldPlugin,
        chunk::ChunkPlugin,
    );
    App::new()
        .add_state::<GameState>()
        .add_plugins(plugins)
        .run();
}
