use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

use crate::{
    chunk::{Chunk, ChunkTilePosition},
    constants::TILE_SIZE,
    world::{heightmap_generator::Heightmap, tile_highlight::HighlightTileEvent, WorldSettings},
    GameState,
};

#[derive(Reflect)]
pub struct RaycastSet;

#[derive(Event)]
pub struct CursorMovedTile(Vec2);

#[derive(Resource, Clone, Copy)]
pub struct CurrentTile {
    pub position: ChunkTilePosition,
}

pub struct CursorPlugin;
impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource({
            CurrentTile {
                position: ChunkTilePosition::default(),
            }
        });
        app.add_plugins(DeferredRaycastingPlugin::<RaycastSet>::default());
        app.insert_resource(RaycastPluginState::<RaycastSet>::default());
        app.add_systems(
            PreUpdate,
            (tile_cursor)
                .run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
    }
}

/* fn vertex_cursor(meshes: Query<&RaycastMesh<RaycastSet>, With<Chunk>>, mut gizmos: Gizmos) {
    for (_, intersection) in meshes.iter().flat_map(|mesh| mesh.intersections.iter()) {
        //Snap the cursor to tiles
        let mut rounded_position = intersection.position();
        rounded_position.x = rounded_position.x.round();
        rounded_position.z = rounded_position.z.round();

        gizmos.sphere(rounded_position, Quat::IDENTITY, 0.1, Color::RED);
    }
} */
fn tile_cursor(
    meshes: Query<(&RaycastMesh<RaycastSet>, &Heightmap), With<Chunk>>,
    world_settings: Option<Res<WorldSettings>>,
    mut current_tile: ResMut<CurrentTile>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
) {
    for (intersection_mesh, _) in meshes.iter() {
        for (_, intersection) in intersection_mesh.intersections.iter() {
            //Sets the current tile resource
            match world_settings {
                Some(_) => {
                    let mut intersection_pos = intersection.position();
                    intersection_pos[0] += 0.5 * TILE_SIZE;
                    intersection_pos[2] += 0.5 * TILE_SIZE;

                    current_tile.position =
                        ChunkTilePosition::from_world_position(intersection_pos);

                    highlight_tile_events.send(HighlightTileEvent {
                        position: current_tile.position,
                        color: Color::BLUE,
                    });
                }
                None => {}
            }
        }
    }
}
