use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    chunk::chunk_tile_position::TilePosition, constants::DEBUG, cursor::CurrentTile, GameState,
};

use super::{
    game_time::GameUpdate,
    heightmap::HeightmapsResource,
    resources::{Inventory, InventoryStorage, InventoryType},
    road::RoadTilesResource,
    tools::{CurrentTool, ToolType},
    vehicles::{
        VehicleBundle, VehicleGoal, VehicleGoals, VehiclePosition, VehicleSettings, VehicleSpeed,
    },
};

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(
            Update,
            (building_tool).chain().run_if(in_state(GameState::World)),
        );
        app.add_systems(GameUpdate, residential_shopping);
        //app.add_event::<SpawnBuildingEvent>();
        app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(Component)]
pub struct ResidentialBuilding;
#[derive(Component)]
pub struct CommercialBuilding;
//trait BuildingTypeTrait {}
//impl BuildingTypeTrait for ResidentialBuilding {}
//impl BuildingTypeTrait for CommercialBuilding {}
//pub struct BuildingType<T: BuildingTypeTrait> {
//    pub building_type: T,
//}
#[derive(Resource, Default)]
pub struct OccupiedBuildingTiles {
    pub tiles: HashSet<TilePosition>,
}

#[derive(Component)]
pub struct BuildingPosition {
    pub position: TilePosition,
}
#[derive(Component)]
pub struct BuildingEntrance {
    pub position: TilePosition,
}
//impl ResidentialBuildingBundle {
//    pub fn new(position: BuildingPosition, ) {
//
//    }
//}
#[derive(Bundle)]
pub struct BuildingBundle {
    pub position: BuildingPosition,
    //pub building_type: BuildingType,
    pub entrance: BuildingEntrance,
    pub pbr: PbrBundle,
    pub inventory: Inventory,
}
#[derive(Component)]
pub struct NeedsPathFinding {
    pub start: TilePosition,
    pub end: TilePosition,
}

//#[derive(Event)]
//pub struct SpawnBuildingEvent {
//    pub position: TilePosition,
//    //pub building_type: BuildingType,
//}

fn setup(mut commands: Commands) {
    commands.init_resource::<OccupiedBuildingTiles>();
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<OccupiedBuildingTiles>();
}

fn residential_shopping(
    mut commands: Commands,
    commercial_buildings_query: Query<(Entity, &CommercialBuilding, &BuildingEntrance)>,
    mut residential_buildings_query: Query<(
        Entity,
        &ResidentialBuilding,
        &BuildingEntrance,
        &mut Inventory,
    )>,
    vehicle_settings: Res<VehicleSettings>,
    heightmaps: Res<HeightmapsResource>,
) {
    if residential_buildings_query.is_empty() || commercial_buildings_query.is_empty() {
        return;
    }
    for (building_entity, _, residential_building, mut inventory) in
        &mut residential_buildings_query
    {
        let population = &mut inventory.inventory[InventoryType::People];
        if population.current == 0 {
            continue;
        }
        population.current -= 1;
        let mut inventory = Inventory::default();
        inventory.inventory[InventoryType::People] = InventoryStorage { current: 1, max: 5 };

        let (commercial_entity, _, random_commerical_building) = commercial_buildings_query
            .iter()
            .nth(rand::random::<usize>() % commercial_buildings_query.iter().len())
            .expect("Should be at least one commercial building");

        let heightmap = &heightmaps[residential_building.position.chunk_position()];

        let mut goals = vec![
            VehicleGoal::Shopping {
                entity: commercial_entity,
            },
            VehicleGoal::ReturnHome {
                entity: building_entity,
            },
        ];
        goals.reverse();

        commands.spawn((
            VehicleBundle {
                position: VehiclePosition {
                    position: residential_building.position,
                },
                speed: VehicleSpeed { speed: 0.01 },
                pbr: PbrBundle {
                    mesh: vehicle_settings.meshes[0].clone(),
                    material: vehicle_settings.materials[0].clone(),
                    transform: Transform::from_translation(
                        residential_building
                            .position
                            .to_world_position_with_height(heightmap),
                    ),
                    ..Default::default()
                },
                goals: VehicleGoals { goals },
                inventory,
            },
            NeedsPathFinding {
                start: residential_building.position,
                end: random_commerical_building.position,
            },
        ));
    }
}

fn building_tool(
    current_tile: Res<CurrentTile>,
    current_tool: ResMut<CurrentTool>,
    mouse_button: Res<Input<MouseButton>>,
    occupied_road_tiles: Res<RoadTilesResource>,
    mut occupied_building_tiles: ResMut<OccupiedBuildingTiles>,
    mut commands: Commands,
    heightmap_query: Res<HeightmapsResource>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    match current_tool.tool_type {
        ToolType::BuildResidentialBuilding | ToolType::BuildCommercialBuilding => {
            if mouse_button.just_pressed(MouseButton::Left) {
                let mut starting_point_y0 = current_tile.position;
                starting_point_y0.position.y = 0;

                if find_entrance_tile(starting_point_y0, &occupied_road_tiles).is_some() {
                } else {
                    if DEBUG {
                        println!("No entrance found at {:?}", current_tile.position);
                    }
                    return;
                }
                if occupied_road_tiles.tiles.contains_key(&starting_point_y0) {
                    if DEBUG {
                        println!("Can't build on road");
                    }
                    return;
                }
                if occupied_building_tiles.tiles.contains(&starting_point_y0) {
                    if DEBUG {
                        println!("Can't build on building");
                    }
                    return;
                }

                occupied_building_tiles.tiles.insert(starting_point_y0);

                let heightmap = &heightmap_query[starting_point_y0.chunk_position()];

                let mesh = Mesh::from(shape::Cube { size: 1.0 });
                let transform = Transform::from_translation(
                    starting_point_y0.to_world_position_with_height(heightmap),
                );

                let mut material: StandardMaterial = match current_tool.tool_type {
                    ToolType::BuildResidentialBuilding => Color::DARK_GREEN.into(),
                    ToolType::BuildCommercialBuilding => Color::BLUE.into(),
                    _ => unreachable!(),
                };
                let inventory = match current_tool.tool_type {
                    ToolType::BuildResidentialBuilding => {
                        let mut inventory = Inventory::default();
                        inventory.inventory[InventoryType::People] = InventoryStorage {
                            current: 1,
                            max: 10,
                        };
                        inventory
                    }
                    ToolType::BuildCommercialBuilding => Inventory::default(),
                    _ => unreachable!(),
                };
                material.perceptual_roughness = 1.0;
                material.reflectance = 0.0;

                let mesh_handle = mesh_assets.add(mesh);
                let building_bundle = BuildingBundle {
                    position: BuildingPosition {
                        position: starting_point_y0,
                    },
                    entrance: BuildingEntrance {
                        position: find_entrance_tile(starting_point_y0, &occupied_road_tiles)
                            .unwrap_or_else(|| {
                                panic!("No entrance found for {starting_point_y0:?}")
                            }),
                    },
                    pbr: PbrBundle {
                        mesh: mesh_handle.clone(),
                        material: materials.add(material),
                        transform,
                        ..default()
                    },
                    inventory,
                };

                match current_tool.tool_type {
                    ToolType::BuildResidentialBuilding => {
                        commands.spawn((building_bundle, ResidentialBuilding));
                    }
                    ToolType::BuildCommercialBuilding => {
                        commands.spawn((building_bundle, CommercialBuilding));
                    }
                    _ => unreachable!(),
                };
            }
        }
        _ => {}
    }
}

fn find_entrance_tile(
    building_position: TilePosition,
    occupied_road_tiles: &RoadTilesResource,
) -> Option<TilePosition> {
    let neighbours = building_position.tile_neighbours();
    neighbours
        .into_iter()
        .map(|(_, neighbour)| neighbour)
        .find(|&neighbour| occupied_road_tiles.tiles.contains_key(&neighbour))
}
