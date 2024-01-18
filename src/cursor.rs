use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

use crate::{
    chunk::{chunk_tile_position::TilePosition, Chunk},
    world::{
        heightmap::HeightmapsResource,
        tile_highlight::{Duration, HighlightTileEvent},
        WorldSettings,
    },
    GameState,
};

#[derive(Reflect)]
pub struct RaycastSet;

#[derive(Event)]
pub struct CursorMovedTile(Vec2);

#[derive(Resource, Clone, Copy)]
pub struct CurrentTile {
    pub position: TilePosition,
}

pub struct CursorPlugin;
impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource({
            CurrentTile {
                position: TilePosition::default(),
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
    meshes: Query<&RaycastMesh<RaycastSet>, With<Chunk>>,
    world_settings: Option<Res<WorldSettings>>,
    mut current_tile: ResMut<CurrentTile>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    mut gizmos: Gizmos,
    heightmaps: Res<HeightmapsResource>,
) {
    for intersection_mesh in meshes.iter() {
        for (_, intersection) in &intersection_mesh.intersections {
            //Sets the current tile resource
            if world_settings.is_some() {
                let intersection_pos = intersection.position();

                current_tile.position = TilePosition::from_world_position(intersection_pos);

                highlight_tile_events.send(HighlightTileEvent {
                    position: current_tile.position,
                    color: Color::BLUE,
                    duration: Duration::Once,
                    size: 1.0,
                });

                let pos = heightmaps.get_from_world_position(intersection_pos);
                gizmos.sphere(pos, Quat::IDENTITY, 0.5, Color::SALMON);
            }
        }
    }
}
