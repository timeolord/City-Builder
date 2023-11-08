pub mod walls;
use bevy::prelude::*;

use crate::{
    camera::CameraPlugin,
    chunk::{spawn_chunk, ChunkResource},
    cursor::CursorPlugin,
    GameState,
};

use self::walls::WallsPlugin;

pub struct AssetBuilderPlugin;

impl Plugin for AssetBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CameraPlugin, CursorPlugin, WallsPlugin));
        app.add_systems(OnEnter(GameState::AssetBuilder), setup);
    }
}

#[derive(Component)]
struct AssetBuilderEntity;

fn setup(
    mut commands: Commands,
    chunk_resources: Res<ChunkResource>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // plane
    spawn_chunk(
        &mut commands,
        &mut meshes,
        (0.0, 0.0),
        chunk_resources.as_ref(),
        AssetBuilderEntity,
        None,
    );

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}
