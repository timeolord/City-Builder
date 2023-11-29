use bevy::prelude::*;
use enum_map::*;

pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, _app: &mut App) {
        //app.add_systems(OnEnter(GameState::World), setup);
        //app.add_systems(
        //    Update,
        //    (building_tool, spawn_building_event_handler)
        //        .chain()
        //        .run_if(in_state(GameState::World)),
        //);
        //app.add_systems(
        //    GameUpdate,
        //    residential_to_commercial,
        //);
        //app.add_event::<SpawnBuildingEvent>();
        //app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(Component, Default, Clone, Debug)]
pub struct Inventory {
    pub inventory: EnumMap<InventoryType, InventoryStorage>,
}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InventoryStorage {
    pub current: usize,
    pub max: usize,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InventoryType {
    People,
}
