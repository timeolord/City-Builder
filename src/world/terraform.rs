use bevy::prelude::*;

use crate::{chunk::SpawnChunkEvent, cursor::CurrentTile, math_utils::Mean, GameState};

use super::{
    heightmap::HeightmapsResource,
    tools::{CurrentTool, ToolType},
};

pub struct TerraformPlugin;

impl Plugin for TerraformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (tile_editor_tool,).run_if(in_state(GameState::World)),
        );
        app.add_systems(
            PostUpdate,
            (regenerate_changed_chunks).run_if(in_state(GameState::World)),
        );
    }
}

fn tile_editor_tool(
    tool_resource: Res<CurrentTool>,
    current_tile: Res<CurrentTile>,
    mouse_button: Res<Input<MouseButton>>,
    mut heightmaps: ResMut<HeightmapsResource>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let current_tile = current_tile.position;
        if tool_resource.tool_type == ToolType::TileEditor {
            let tile_heights = heightmaps[current_tile];
            let average_height = tile_heights.into_iter().mean_f32();
            let new_heights = vec![(average_height + tool_resource.tool_strength).floor(); 4]
                .try_into()
                .unwrap();
            heightmaps.edit_tile(current_tile, new_heights);
        }
    }
}

fn regenerate_changed_chunks(
    mut heightmaps: ResMut<HeightmapsResource>,
    mut spawn_chunk_events: EventWriter<SpawnChunkEvent>,
) {
    if !heightmaps.is_changed() {
        return;
    }
    //Regenerate Dirty Chunks
    heightmaps.get_dirty_chunks().for_each(|chunk_position| {
        spawn_chunk_events.send(SpawnChunkEvent {
            position: chunk_position,
        });
    });
}
