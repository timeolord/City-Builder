use bevy::prelude::*;

use crate::chunk::{ChunkPosition, ChunkTilePosition};

use super::heightmap_generator::Heightmap;

pub struct TileHighlightPlugin;

impl Plugin for TileHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HighlightTileEvent>();
        app.add_systems(PostUpdate, tile_highlight_handler);
    }
}

#[derive(Event)]
pub struct HighlightTileEvent {
    pub position: ChunkTilePosition,
    pub color: Color,
}

fn tile_highlight_handler(
    mut tile_highlight_events: EventReader<HighlightTileEvent>,
    mut gizmos: Gizmos,
    heightmap_query: Query<(&Heightmap, &ChunkPosition)>,
) {
    for event in tile_highlight_events.read() {
        let heightmap = heightmap_query
            .iter()
            .find(|(_, chunk_position)| **chunk_position == event.position.chunk_position)
            .unwrap()
            .0;
        let height = heightmap[event.position.tile_position_2d()];
        let mut position = event.position.to_world_position();
        position.y = height.into_iter().reduce(f32::max).unwrap_or(0.0);
        gizmos.sphere(position, Quat::IDENTITY, 0.5, event.color);
    }
}
