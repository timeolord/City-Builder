use core::panic;
use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    chunk::{
        chunk_tile_position::{ChunkPosition, TilePosition},
        SpawnChunkEvent,
    },
    constants::DEBUG,
    cursor::CurrentTile,
    math_utils::Mean,
    GameState,
};

use super::{
    heightmap::{HeightmapVertex, HeightmapsResource},
    tools::{CurrentTool, ToolType},
    WorldSettings,
};

pub struct TerraformPlugin;

impl Plugin for TerraformPlugin {
    fn build(&self, app: &mut App) {
        /* app.add_event::<EditTileEvent>(); */
        app.add_event::<RegenerateChunkEvent>();
        app.add_systems(
            Update,
            (tile_editor_tool,).run_if(in_state(GameState::World)),
        );
        /* app.add_systems(
            PostUpdate,
            (edit_tile_event_handler, regenerate_chunk_event_handler)
                .chain()
                .run_if(in_state(GameState::World)),
        ); */
        app.add_systems(
            PostUpdate,
            (regenerate_changed_chunks).run_if(in_state(GameState::World)),
        );
    }
}
/*
#[derive(Event)]
pub struct EditTileEvent {
    pub tile_position: TilePosition,
    pub new_vertices: HeightmapVertex,
} */

#[derive(Event, Eq, PartialEq, Debug, Copy, Clone, Hash)]
struct RegenerateChunkEvent {
    chunk_position: ChunkPosition,
}
fn tile_editor_tool(
    tool_resource: Res<CurrentTool>,
    current_tile: Res<CurrentTile>,
    mouse_button: Res<Input<MouseButton>>,
    mut heightmaps: ResMut<HeightmapsResource>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let current_tile = current_tile.position;
        match tool_resource.tool_type {
            ToolType::TileEditor => {
                let tile_heights = heightmaps[current_tile];
                let average_height = tile_heights.into_iter().mean_f32();
                let new_heights = vec![(average_height + tool_resource.tool_strength).floor(); 4]
                    .try_into()
                    .unwrap();
                heightmaps.edit_tile(current_tile, new_heights);
            }
            _ => {}
        }
    }
}

/* fn edit_tile_event_handler(
    mut edit_tile_events: EventReader<EditTileEvent>,
    mut heightmaps: ResMut<HeightmapsResource>,
    mut regenerate_chunk_event: EventWriter<RegenerateChunkEvent>,
) {
    let mut changed_chunks: HashSet<ChunkPosition> = HashSet::new();
    let edit_tile_events = edit_tile_events.read().collect::<Vec<_>>();
    let tiles_to_change = edit_tile_events
        .iter()
        .map(|edit_tile_event| edit_tile_event.tile_position)
        .collect::<Vec<_>>();

    for edit_tile_event in edit_tile_events.iter() {
        let tile_position = edit_tile_event.tile_position;
        let new_vertices = edit_tile_event.new_vertices;
        let mut neighbours = tile_position.tile_neighbours();

        for (direction, neighbour) in neighbours.into_iter() {
            if tiles_to_change.contains(&neighbour) {
                continue;
            }
        }

        /* if neighbours.north.is_some() && tiles_to_change.contains(&neighbours.north.unwrap()) {
            neighbours.north = None;
        }
        if neighbours.south.is_some() && tiles_to_change.contains(&neighbours.south.unwrap()) {
            neighbours.south = None;
        }
        if neighbours.east.is_some() && tiles_to_change.contains(&neighbours.east.unwrap()) {
            neighbours.east = None;
        }
        if neighbours.west.is_some() && tiles_to_change.contains(&neighbours.west.unwrap()) {
            neighbours.west = None;
        }
        if neighbours.north_east.is_some()
            && tiles_to_change.contains(&neighbours.north_east.unwrap())
        {
            neighbours.north_east = None;
        }
        if neighbours.north_west.is_some()
            && tiles_to_change.contains(&neighbours.north_west.unwrap())
        {
            neighbours.north_west = None;
        }
        if neighbours.south_east.is_some()
            && tiles_to_change.contains(&neighbours.south_east.unwrap())
        {
            neighbours.south_east = None;
        }
        if neighbours.south_west.is_some()
            && tiles_to_change.contains(&neighbours.south_west.unwrap())
        {
            neighbours.south_west = None;
        }
        for (chunk_position, mut heightmap) in heightmaps.iter_mut() {
            if *chunk_position == tile_position.chunk_position {
                heightmap[tile_position] = new_vertices;
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.south.is_some()
                && *chunk_position == neighbours.south.unwrap().chunk_position
            {
                heightmap[neighbours.south.unwrap()][2] = new_vertices[1];
                heightmap[neighbours.south.unwrap()][3] = new_vertices[0];
                heightmap[neighbours.south.unwrap()][4] = heightmap[neighbours.south.unwrap()]
                    .into_iter()
                    .take(4)
                    .reduce(|a, b| a + b)
                    .unwrap()
                    / 4.0;
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.north.is_some()
                && *chunk_position == neighbours.north.unwrap().chunk_position
            {
                heightmap[neighbours.north.unwrap()][0] = new_vertices[3];
                heightmap[neighbours.north.unwrap()][1] = new_vertices[2];
                heightmap[neighbours.north.unwrap()][4] = heightmap[neighbours.north.unwrap()]
                    .into_iter()
                    .take(4)
                    .reduce(|a, b| a + b)
                    .unwrap()
                    / 4.0;
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.east.is_some()
                && *chunk_position == neighbours.east.unwrap().chunk_position
            {
                heightmap[neighbours.east.unwrap()][0] = new_vertices[1];
                heightmap[neighbours.east.unwrap()][3] = new_vertices[2];
                heightmap[neighbours.east.unwrap()][4] = heightmap[neighbours.east.unwrap()]
                    .into_iter()
                    .take(4)
                    .reduce(|a, b| a + b)
                    .unwrap()
                    / 4.0;
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.west.is_some()
                && *chunk_position == neighbours.west.unwrap().chunk_position
            {
                heightmap[neighbours.west.unwrap()][1] = new_vertices[0];
                heightmap[neighbours.west.unwrap()][2] = new_vertices[3];
                heightmap[neighbours.west.unwrap()][4] = heightmap[neighbours.west.unwrap()]
                    .into_iter()
                    .take(4)
                    .reduce(|a, b| a + b)
                    .unwrap()
                    / 4.0;
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.south_east.is_some()
                && *chunk_position == neighbours.south_east.unwrap().chunk_position
            {
                heightmap[neighbours.south_east.unwrap()][3] = new_vertices[1];
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.south_west.is_some()
                && *chunk_position == neighbours.south_west.unwrap().chunk_position
            {
                heightmap[neighbours.south_west.unwrap()][2] = new_vertices[0];
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.north_east.is_some()
                && *chunk_position == neighbours.north_east.unwrap().chunk_position
            {
                heightmap[neighbours.north_east.unwrap()][0] = new_vertices[2];
                changed_chunks.insert(*chunk_position);
            }
            if neighbours.north_west.is_some()
                && *chunk_position == neighbours.north_west.unwrap().chunk_position
            {
                heightmap[neighbours.north_west.unwrap()][1] = new_vertices[3];
                changed_chunks.insert(*chunk_position);
            }
        } */
    }
    for chunk in changed_chunks.into_iter() {
        regenerate_chunk_event.send(RegenerateChunkEvent {
            chunk_position: chunk,
        });
    }
}

fn regenerate_chunk_event_handler(
    mut regenerate_chunk_events: EventReader<RegenerateChunkEvent>,
    mut spawn_chunk_events: EventWriter<SpawnChunkEvent>,
    chunks: Query<&ChunkPosition>,
    heightmaps: Res<HeightmapsResource>,
) {
    let mut regenerate_chunk_events = regenerate_chunk_events
        .read()
        .collect::<HashSet<&RegenerateChunkEvent>>();
    for regenerate_chunk_event in regenerate_chunk_events {
        let current_chunk_id = chunks
            .iter()
            .find(|chunk| **chunk == regenerate_chunk_event.chunk_position);

        match current_chunk_id {
            Some(chunk_position) => {
                spawn_chunk_events.send(SpawnChunkEvent {
                    position: *chunk_position,
                });
            }
            None => {
                if DEBUG {
                    println!(
                        "Can't find Chunk {:?} to update",
                        regenerate_chunk_event.chunk_position
                    );
                    println!(
                        "Chunks: {:?}",
                        chunks.iter().map(|chunk| chunk).collect::<Vec<_>>()
                    );
                } else {
                    panic!(
                        "Can't find Chunk {:?} to update",
                        regenerate_chunk_event.chunk_position
                    );
                }
            }
        }
    }
} */

fn regenerate_changed_chunks(
    chunks: Query<&ChunkPosition>,
    mut heightmaps: ResMut<HeightmapsResource>,
    mut spawn_chunk_events: EventWriter<SpawnChunkEvent>,
) {
    if !heightmaps.is_changed() {
        return;
    }
    //Regenerate Dirty Chunks
    heightmaps
        .get_dirty_chunks()
        .iter()
        .for_each(|chunk_position| {
            spawn_chunk_events.send(SpawnChunkEvent {
                position: *chunk_position,
            });
        });
}
