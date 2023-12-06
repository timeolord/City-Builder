#![allow(clippy::too_many_arguments)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::module_name_repetitions)]

//! A simple 3D scene with light shining over a cube sitting on a plane.
mod camera;
mod chunk;
mod constants;
mod cursor;
mod math_utils;
mod menu;
mod mesh_generator;
mod world;

use std::env;

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
    if cfg!(debug_assertions) {
        env::set_var("RUST_BACKTRACE", "1");
    }
    App::new()
        .add_state::<GameState>()
        .add_plugins(plugins)
        .insert_resource(Time::<Fixed>::from_seconds(DEFAULT_TIMESTEP))
        .run();
}
