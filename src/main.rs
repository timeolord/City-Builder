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
mod assets;
mod camera;
mod debug;
mod menu;
mod save;
mod utils;
mod world;
mod world_gen;

use crate::assets::asset_loader;

use bevy::{
    core::TaskPoolThreadAssignmentPolicy, diagnostic::FrameTimeDiagnosticsPlugin, prelude::*,
};
use bevy_egui::EguiPlugin;
use std::env;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    AssetLoading,
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
        asset_loader::AssetLoaderPlugin,
    );
    if cfg!(debug_assertions) {
        env::set_var("RUST_BACKTRACE", "1");
    }
    let mut app = &mut App::new();
    app = app
        .add_state::<GameState>()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        min_total_threads: 1,
                        max_total_threads: usize::MAX,
                        io: TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: 1,
                            percent: 0.0,
                        },
                        compute: TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: usize::MAX,
                            percent: 0.25,
                        },
                        async_compute: TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: usize::MAX,
                            percent: 0.75,
                        },
                    },
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin)
        .add_plugins(plugins)
        .add_plugins(debug::DebugPlugin);
    app.run();
}
