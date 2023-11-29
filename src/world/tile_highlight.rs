use bevy::prelude::*;

use crate::chunk::chunk_tile_position::TilePosition;

use super::heightmap::HeightmapsResource;

pub struct TileHighlightPlugin;

impl Plugin for TileHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HighlightTileEvent>();
        app.add_systems(PostUpdate, tile_highlight_handler);
    }
}

#[derive(Event)]
pub struct HighlightTileEvent {
    pub position: TilePosition,
    pub color: Color,
}

fn tile_highlight_handler(
    mut tile_highlight_events: EventReader<HighlightTileEvent>,
    mut gizmos: Gizmos,
    heightmap_query: Res<HeightmapsResource>,
) {
    for event in tile_highlight_events.read() {
        let height = heightmap_query[event.position];

        let mut position = event.position.to_world_position();
        position.y = height.into_iter().reduce(f32::max).unwrap_or(0.0);
        gizmos.sphere(position, Quat::IDENTITY, 0.5, event.color);
    }
}
