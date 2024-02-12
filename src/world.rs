

use std::f32::consts::PI;

use crate::GameState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(OnExit(GameState::World), exit);
    }
}

fn exit(mut commands: Commands, query: Query<Entity, With<WorldEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Component)]
struct WorldEntity;

pub type WorldSize = [u32; 2];

#[derive(Resource, Clone, Copy, Serialize, Deserialize)]
pub struct WorldSettings {
    pub world_size: WorldSize,
    pub seed: u32,
}

fn init(mut commands: Commands) {
    let world_size = [4, 4];
    let seed: u32 = 0;
    let world_settings = WorldSettings {
        world_size,
        seed,
    };
    commands.insert_resource(world_settings);
}

fn setup(mut commands: Commands, _world_settings: Res<WorldSettings>) {
    // Sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            // shadow_depth_bias: 0.2,
            illuminance: 50000.0,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 100.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        //cascade_shadow_config: CascadeShadowConfigBuilder {
        //    first_cascade_far_bound: 4.0,
        //    maximum_distance: 1000.0,
        //    ..default()
        //}
        //.into(),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });
}
