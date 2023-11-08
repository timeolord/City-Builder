use bevy::prelude::*;
use bevy_mod_raycast::{prelude::*, DefaultRaycastingPlugin};

use crate::{chunk::Chunk, world::{WorldSettings, world_position_to_chunk_tile_position}, GameState};

#[derive(Reflect)]
pub struct RaycastSet;

#[derive(Event)]
pub struct CursorMovedTile(Vec2);

pub struct CursorPlugin;
impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultRaycastingPlugin::<RaycastSet>::default());
        app.add_systems(
            Update,
            (
                update_raycast_with_cursor.before(RaycastSystem::BuildRays::<RaycastSet>),
                vertex_cursor,
            )
                .chain()
                .run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
    }
}

// Update our `RaycastSource` with the current cursor position every frame.
fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<RaycastSet>>,
) {
    // Grab the most recent cursor event if it exists:
    let Some(cursor_moved) = cursor.iter().last() else {
        return;
    };
    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(cursor_moved.position);
    }
}

fn vertex_cursor(
    meshes: Query<&RaycastMesh<RaycastSet>, With<Chunk>>,
    mut gizmos: Gizmos
) {
    for (_, intersection) in meshes.iter().flat_map(|mesh| mesh.intersections.iter()) {
        //Snap the cursor to tiles
        let mut rounded_position = intersection.position();
        rounded_position.x = rounded_position.x.round();
        rounded_position.z = rounded_position.z.round();

        /* //Snap the cursor to the heightmap if it exists
        match world_settings {
            Some(ref world_setting) => {
                let (chunk_pos, tile_pos) = world_position_to_chunk_tile_position(intersection.position(), world_setting);
                rounded_position.y = world_setting.heightmaps[(chunk_pos.x as usize, chunk_pos.y as usize)][(tile_pos.x as usize, tile_pos.z as usize)][0];
            },
            None => {},
        } */
        gizmos.sphere(rounded_position, Quat::IDENTITY, 0.1, Color::RED);
    }
}
