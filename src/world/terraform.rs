use core::panic;
use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    chunk::{ChunkPosition, ChunkTilePosition, SpawnChunkEvent},
    constants::DEBUG,
    cursor::CurrentTile,
    GameState,
};

use super::{
    heightmap_generator::{Heightmap, HeightmapVertex},
    tools::{CurrentTool, ToolType},
};

pub struct TerraformPlugin;

impl Plugin for TerraformPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EditTileEvent>();
        app.add_event::<RegenerateChunkEvent>();
        app.add_systems(
            Update,
            (
                tile_editor_tool,
                edit_tile_event_handler,
                regenerate_chunk_event_handler,
            )
                .chain()
                .run_if(in_state(GameState::World)),
        );
    }
}

#[derive(Event)]
pub struct EditTileEvent {
    pub tile_position: ChunkTilePosition,
    pub new_vertices: HeightmapVertex,
}

#[derive(Event, Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
struct RegenerateChunkEvent {
    chunk_position: ChunkPosition,
}
fn tile_editor_tool(
    tool_resource: Res<CurrentTool>,
    current_tile: Res<CurrentTile>,
    mut edit_tile_event: EventWriter<EditTileEvent>,
    mouse_button: Res<Input<MouseButton>>,
    heightmaps: Query<(&ChunkPosition, &Heightmap)>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let current_tile = current_tile.to_owned().position;
        match tool_resource.tool_type {
            ToolType::TileEditor => {
                match heightmaps
                    .iter()
                    .find(|(chunk, _)| **chunk == current_tile.chunk_position)
                {
                    Some((_, heightmap)) => {
                        let tile_heights = heightmap[current_tile];
                        let average_height =
                            tile_heights.into_iter().sum::<f32>() / tile_heights.len() as f32;
                        let new_heights =
                            vec![(average_height + tool_resource.tool_strength).floor(); 5]
                                .try_into()
                                .unwrap();
                        edit_tile_event.send(EditTileEvent {
                            tile_position: current_tile,
                            new_vertices: new_heights,
                        })
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }
}

fn edit_tile_event_handler(
    mut edit_tile_events: EventReader<EditTileEvent>,
    mut heightmaps: Query<(&ChunkPosition, &mut Heightmap)>,
    mut regenerate_chunk_event: EventWriter<RegenerateChunkEvent>,
) {
    let mut changed_chunks: HashSet<ChunkPosition> = HashSet::new();

    for edit_tile_event in edit_tile_events.read() {
        let tile_position = edit_tile_event.tile_position;
        let new_vertices = edit_tile_event.new_vertices;
        let neighbours = tile_position.tile_neighbours();
        if !neighbours.has_all() {
            if DEBUG {
                println!("Can't find all neighbours of {:?}", tile_position);
                println!("Neighbours: {:?}", neighbours);
            }
            return;
        }
        for (chunk_position, mut heightmap) in heightmaps.iter_mut() {
            if *chunk_position == tile_position.chunk_position {
                heightmap[tile_position] = new_vertices;
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.south.unwrap().chunk_position {
                heightmap[neighbours.south.unwrap()][2] = new_vertices[0];
                heightmap[neighbours.south.unwrap()][3] = new_vertices[1];
                heightmap[neighbours.south.unwrap()][4] = (heightmap[neighbours.south.unwrap()][2]
                    + heightmap[neighbours.south.unwrap()][1])
                    / 2.0;
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.north.unwrap().chunk_position {
                heightmap[neighbours.north.unwrap()][0] = new_vertices[2];
                heightmap[neighbours.north.unwrap()][1] = new_vertices[3];
                heightmap[neighbours.north.unwrap()][4] = (heightmap[neighbours.north.unwrap()][0]
                    + heightmap[neighbours.north.unwrap()][3])
                    / 2.0;
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.east.unwrap().chunk_position {
                heightmap[neighbours.east.unwrap()][0] = new_vertices[1];
                heightmap[neighbours.east.unwrap()][3] = new_vertices[2];
                heightmap[neighbours.east.unwrap()][4] = (heightmap[neighbours.east.unwrap()][0]
                    + heightmap[neighbours.east.unwrap()][2])
                    / 2.0;
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.west.unwrap().chunk_position {
                heightmap[neighbours.west.unwrap()][1] = new_vertices[0];
                heightmap[neighbours.west.unwrap()][2] = new_vertices[3];
                heightmap[neighbours.west.unwrap()][4] = (heightmap[neighbours.west.unwrap()][1]
                    + heightmap[neighbours.west.unwrap()][3])
                    / 2.0;
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.south_east.unwrap().chunk_position {
                heightmap[neighbours.south_east.unwrap()][3] = new_vertices[1];
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.south_west.unwrap().chunk_position {
                heightmap[neighbours.south_west.unwrap()][2] = new_vertices[0];
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.north_east.unwrap().chunk_position {
                heightmap[neighbours.north_east.unwrap()][0] = new_vertices[2];
                changed_chunks.insert(*chunk_position);
            }
            if *chunk_position == neighbours.north_west.unwrap().chunk_position {
                heightmap[neighbours.north_west.unwrap()][1] = new_vertices[3];
                changed_chunks.insert(*chunk_position);
            }
        }
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
    chunks: Query<(&Heightmap, &ChunkPosition)>,
) {
    let mut regenerate_chunk_events = regenerate_chunk_events
        .read()
        .collect::<Vec<&RegenerateChunkEvent>>();
    regenerate_chunk_events.sort();
    regenerate_chunk_events.dedup();
    for regenerate_chunk_event in regenerate_chunk_events {
        let current_chunk_id: Option<(&Heightmap, &ChunkPosition)> = chunks
            .iter()
            .find(|(_, chunk)| **chunk == regenerate_chunk_event.chunk_position);

        match current_chunk_id {
            Some((heightmap, chunk_position)) => {
                spawn_chunk_events.send(SpawnChunkEvent {
                    position: *chunk_position,
                    heightmap: Some(heightmap.to_owned()),
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
                        chunks.iter().map(|(_, chunk)| chunk).collect::<Vec<_>>()
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
}
