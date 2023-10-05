//! A simple 3D scene with light shining over a cube sitting on a plane.
mod asset_builder;
mod constants;
use bevy::prelude::*;

fn main() {
    let plugins = (DefaultPlugins/* .set(ImagePlugin::default_nearest()) */, asset_builder::AssetBuilderPlugin);
    App::new()
        .add_plugins(plugins)
        .run();
}