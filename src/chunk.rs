pub mod chunk_tile_position;

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

use crate::{
    constants::{CHUNK_SIZE, TILE_SIZE},
    cursor::RaycastSet,
    mesh_generator::{create_chunk_mesh, create_grid_mesh},
    world::{heightmap::HeightmapsResource, WorldSettings},
    GameState,
};

use self::chunk_tile_position::ChunkPosition;

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnChunkEvent>();
        app.add_event::<DespawnEntityEvent>();
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            spawn_chunk_event_handler
                .run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
        app.add_systems(
            PostUpdate,
            despawn_entity_event_handler
                .run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
    }
}

#[derive(Resource)]
pub struct ChunkResource {
    plane_material: Handle<StandardMaterial>,
    grid_material: Handle<StandardMaterial>,
}
#[derive(Event)]
pub struct SpawnChunkEvent {
    pub position: ChunkPosition,
}
#[derive(Event)]
pub struct DespawnEntityEvent {
    pub entity: Entity,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut material: StandardMaterial = Color::rgb(0.3, 0.5, 0.3).into();
    material.perceptual_roughness = 1.0;
    material.reflectance = 0.0;
    let plane_material = materials.add(material);
    let grid_material = materials.add(Color::rgb(1.0, 1.0, 1.0).into());

    commands.insert_resource(ChunkResource {
        plane_material,
        grid_material,
    })
}

#[derive(Component)]
pub struct Grid;

#[derive(Component)]
pub struct Chunk;

#[derive(Bundle)]
pub struct ChunkBundle {
    pub pbr: PbrBundle,
    pub chunk_tag: Chunk,
    pub chunk_position: ChunkPosition,
    pub raycast_mesh: RaycastMesh<RaycastSet>,
}

#[derive(Bundle)]
pub struct GridBundle {
    pub grid_pbr: PbrBundle,
    pub grid: Grid,
}

fn despawn_entity_event_handler(
    mut despawn_entity_events: EventReader<DespawnEntityEvent>,
    mut commands: Commands,
) {
    for despawn_entity_event in despawn_entity_events.read() {
        commands
            .entity(despawn_entity_event.entity)
            .despawn_recursive();
    }
}

fn spawn_chunk_event_handler(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut spawn_chunk_events: EventReader<SpawnChunkEvent>,
    mut despawn_entity_events: EventWriter<DespawnEntityEvent>,
    chunk_resources: Res<ChunkResource>,
    world_settings: Res<WorldSettings>,
    chunks: Query<(Entity, &ChunkPosition)>,
    heightmaps: Res<HeightmapsResource>,
) {
    for spawn_chunk_event in spawn_chunk_events.read() {
        let current_chunk_id: Option<(Entity, &ChunkPosition)> = chunks
            .iter()
            .find(|(_, chunk)| **chunk == spawn_chunk_event.position);
        match current_chunk_id {
            Some((current_chunk_id, _)) => {
                despawn_entity_events.send(DespawnEntityEvent {
                    entity: current_chunk_id,
                });
            }
            None => {}
        }
        let heightmap = &heightmaps[spawn_chunk_event.position];
        let starting_position = spawn_chunk_event.position;

        let mesh = meshes.add(create_chunk_mesh(&heightmap));
        let material = chunk_resources.plane_material.clone();
        let grid_material = chunk_resources.grid_material.clone();
        let grid_mesh = meshes.add(create_grid_mesh(&heightmap));

        let chunk_pbr = PbrBundle {
            mesh: mesh,
            material: material,
            transform: Transform::from_xyz(
                (starting_position.position.x * CHUNK_SIZE) as f32,
                0.0,
                (starting_position.position.y * CHUNK_SIZE) as f32,
            ),
            ..default()
        };

        let mut grid_transform = Transform::from_xyz(0.0, 0.0, 0.0);
        grid_transform.translation.y += 0.01;
        grid_transform.translation.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        grid_transform.translation.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;

        let grid_pbr = PbrBundle {
            mesh: grid_mesh,
            material: grid_material,
            transform: grid_transform,
            visibility: world_settings.grid_visibility,
            ..default()
        };

        let chunk_bundle = ChunkBundle {
            pbr: chunk_pbr,
            chunk_tag: Chunk,
            chunk_position: starting_position,
            raycast_mesh: RaycastMesh::<RaycastSet>::default(),
        };

        commands.spawn(chunk_bundle).with_children(|parent| {
            parent.spawn(GridBundle {
                grid_pbr,
                grid: Grid,
            });
        });
    }
}
