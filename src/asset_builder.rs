pub mod walls;
use bevy::prelude::*;

use crate::{
    camera::CameraPlugin,
    chunk::{ChunkPosition, SpawnChunkEvent},
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
    mut spawn_chunk_event: EventWriter<SpawnChunkEvent>,
) {
    // plane
    spawn_chunk_event.send(SpawnChunkEvent {
        position: ChunkPosition { position: [0, 0] },
        heightmap: None,
    });

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
