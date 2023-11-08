pub mod heightmap_generator;
pub mod terraform;
use std::f32::consts::PI;

use crate::{
    chunk::{spawn_chunk, ChunkResource},
    constants::{CHUNK_SIZE, TILE_SIZE},
    GameState,
};
use array2d::Array2D;
use bevy::{pbr::CascadeShadowConfigBuilder, prelude::*};

use self::heightmap_generator::Heightmap;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(OnEnter(GameState::World), setup);
        // app.add_systems(Update, input.run_if(in_state(GameState::World)));
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

#[derive(Resource)]
pub struct WorldSettings {
    pub world_size: (usize, usize),
    pub seed: u32,
    pub heightmaps: Array2D<Heightmap>,
}

fn init(mut commands: Commands) {
    let world_size = (4, 4);
    let seed = 0;
    let mut heightmaps = Array2D::filled_with(
        heightmap_generator::generate_heightmap(seed, (0, 0)),
        world_size.0,
        world_size.1,
    );
    for x in 0..world_size.0 as usize {
        for y in 0..world_size.1 as usize {
            heightmaps[(x, y)] = heightmap_generator::generate_heightmap(seed, (x, y));
        }
    }
    commands.insert_resource(WorldSettings {
        world_size,
        seed,
        heightmaps,
    });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_resources: Res<ChunkResource>,
    world_settings: Res<WorldSettings>,
) {
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
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 1000.0,
            ..default()
        }
        .into(),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.1,
    });

    let world_size = world_settings.world_size.clone();
    for x in 0..world_size.0 {
        for y in 0..world_size.1 {
            spawn_chunk::<WorldEntity>(
                &mut commands,
                &mut meshes,
                ((x * CHUNK_SIZE) as f32, (y * CHUNK_SIZE) as f32),
                chunk_resources.as_ref(),
                WorldEntity,
                Some(&world_settings.heightmaps[(x as usize, y as usize)]),
            );
        }
    }
}

pub fn world_position_to_tile_position(world_position: Vec3) -> Vec3 {
    let mut tile_position = world_position;
    tile_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
    tile_position.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
    tile_position.x = tile_position.x.floor();
    tile_position.y = tile_position.y.floor();
    tile_position.z = tile_position.z.floor();
    tile_position = tile_position.clamp(Vec3::ZERO, Vec3::from_array([CHUNK_SIZE as f32 - 1.0; 3]));
    tile_position
}
pub fn world_position_to_chunk_tile_position(
    world_position: Vec3,
    world_settings: &WorldSettings,
) -> (Vec2, Vec3) {
    let mut tile_position = world_position_to_tile_position(world_position);
    let mut chunk_position = Vec2::new(
        (tile_position.x / CHUNK_SIZE as f32).floor() + 1.0,
        (tile_position.z / CHUNK_SIZE as f32).floor() + 1.0,
    );
    let world_size = (
        world_settings.world_size.0 as f32 - 1.0,
        world_settings.world_size.1 as f32 - 1.0,
    );
    chunk_position = chunk_position.clamp(Vec2::ZERO, Vec2::from(world_size));
    tile_position.x -= (chunk_position.x - 1.0) * CHUNK_SIZE as f32;
    tile_position.z -= (chunk_position.y - 1.0) * CHUNK_SIZE as f32;
    (chunk_position, tile_position)
}
