use bevy::prelude::*;

use crate::{chunk::chunk_tile_position::ChunkTilePosition, GameState};

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(OnExit(GameState::World), exit);
        app.add_systems(PreUpdate, tool_select.run_if(in_state(GameState::World)));
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(CurrentTool {
        tool_type: ToolType::None,
        tool_strength: 0.0,
        tool_increase_amount: 1.1,
        starting_point: None,
        ending_point: None,
    });
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<CurrentTool>();
}

fn tool_select(keyboard: Res<Input<KeyCode>>, mut tool_resource: ResMut<CurrentTool>) {
    if keyboard.just_pressed(KeyCode::T) {
        tool_resource.tool_type = tool_resource.tool_type.next_tool();
        println!("Current Tool: {:?}", tool_resource.tool_type)
    }
    if keyboard.just_pressed(KeyCode::O) {
        tool_resource.tool_strength += tool_resource.tool_increase_amount;
    }
    if keyboard.just_pressed(KeyCode::L) {
        tool_resource.tool_strength -= tool_resource.tool_increase_amount;
    }
}

#[derive(Resource)]
pub struct CurrentTool {
    pub tool_type: ToolType,
    pub tool_strength: f32,
    pub tool_increase_amount: f32,
    pub starting_point: Option<ChunkTilePosition>,
    pub ending_point: Option<ChunkTilePosition>,
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ToolType {
    None,
    VertexEditor,
    TileEditor,
    BuildRoad,
    BuildResidentialBuilding,
    BuildCommercialBuilding,
}
impl ToolType {
    pub fn next_tool(self) -> Self {
        match self {
            ToolType::None => ToolType::VertexEditor,
            ToolType::VertexEditor => ToolType::TileEditor,
            ToolType::TileEditor => ToolType::BuildRoad,
            ToolType::BuildRoad => ToolType::BuildResidentialBuilding,
            ToolType::BuildResidentialBuilding => ToolType::BuildCommercialBuilding,
            ToolType::BuildCommercialBuilding => ToolType::None,
        }
    }
}
