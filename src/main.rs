#![allow(clippy::too_many_arguments)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::module_name_repetitions)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! A simple 3D scene with light shining over a cube sitting on a plane.
mod camera;
mod menu;
mod save;
mod utils;
mod world;
mod world_gen;

use std::env;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    MainMenu,
    WorldGeneration,
    World,
}

pub const DEBUG: bool = cfg!(debug_assertions);

fn main() {
    let plugins = (
        camera::CameraPlugin,
        menu::MenuPlugin,
        save::SavePlugin,
        world::WorldPlugin,
        world_gen::WorldGenPlugin,
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
    println!("Hello, world!");
}
