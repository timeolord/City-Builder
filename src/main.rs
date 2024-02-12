#![allow(clippy::too_many_arguments)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::module_name_repetitions)]

//! A simple 3D scene with light shining over a cube sitting on a plane.
mod menu;
mod utils;
mod world;
mod world_gen;
mod save;

use std::env;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use save::initalize_file_structure;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    MainMenu,
    WorldGeneration,
    World,
}
fn main() {
    initalize_file_structure();
    let plugins = (
        menu::MenuPlugin,
        world::WorldPlugin,
        world_gen::WorldGenPlugin,
        save::SavePlugin,
    );
    if cfg!(debug_assertions) {
        env::set_var("RUST_BACKTRACE", "1");
    }
    App::new()
        .add_state::<GameState>()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(EguiPlugin)
        .add_plugins(plugins)
        .run();
}

