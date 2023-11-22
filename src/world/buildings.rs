use std::collections::HashSet;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};

use crate::{
    chunk::{
        chunk_tile_position::{ChunkPosition, ChunkTilePosition, TilePosition2D},
        unnormalized_normal_vector,
    },
    constants::{DEBUG, TILE_SIZE},
    cursor::CurrentTile,
    world::heightmap_generator::Heightmap,
    GameState,
};

use super::{
    game_time::GameUpdate,
    resources::{Inventory, InventoryStorage, InventoryType},
    road::{pathfinding::Pathfind, OccupiedRoadTiles},
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
#[derive(Resource)]
pub struct OccupiedBuildingTiles {
    pub tiles: HashSet<ChunkTilePosition>,
}
impl Default for OccupiedBuildingTiles {
    fn default() -> Self {
        OccupiedBuildingTiles {
            tiles: HashSet::new(),
        }
    }
}
#[derive(Component)]
pub struct BuildingPosition {
    pub position: ChunkTilePosition,
}
#[derive(Component)]
pub struct BuildingEntrance {
    pub position: ChunkTilePosition,
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
    pub start: ChunkTilePosition,
    pub end: ChunkTilePosition,
}

//#[derive(Event)]
//pub struct SpawnBuildingEvent {
//    pub position: ChunkTilePosition,
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
    heightmaps: Query<(&ChunkPosition, &Heightmap)>,
) {
    if residential_buildings_query.is_empty() || commercial_buildings_query.is_empty() {
        return;
    }
    for (building_entity, _, residential_building, mut inventory) in
        residential_buildings_query.iter_mut()
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

        let heightmap = heightmaps
            .iter()
            .find(|(chunk_position, _)| {
                **chunk_position == residential_building.position.chunk_position
            })
            .unwrap()
            .1;

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
    occupied_road_tiles: Res<OccupiedRoadTiles>,
    mut occupied_building_tiles: ResMut<OccupiedBuildingTiles>,
    mut commands: Commands,
    heightmap_query: Query<(&Heightmap, &ChunkPosition)>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    match current_tool.tool_type {
        ToolType::BuildResidentialBuilding | ToolType::BuildCommercialBuilding => {
            if mouse_button.just_pressed(MouseButton::Left) {
                let mut starting_point_y0 = current_tile.position;
                starting_point_y0.tile_position[1] = 0;

                match find_entrance_tile(starting_point_y0, &occupied_road_tiles) {
                    Some(_) => {}
                    None => {
                        if DEBUG {
                            println!("No entrance found at {:?}", current_tile.position);
                        }
                        return;
                    }
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

                let heightmap = heightmap_query
                    .iter()
                    .find(|(_, chunk_position)| {
                        **chunk_position == starting_point_y0.chunk_position
                    })
                    .unwrap()
                    .0;

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

                let mesh_handle = mesh_assets.add(mesh.into());
                let building_bundle = BuildingBundle {
                    position: BuildingPosition {
                        position: starting_point_y0,
                    },
                    entrance: BuildingEntrance {
                        position: find_entrance_tile(starting_point_y0, &occupied_road_tiles)
                            .expect(
                                format!("No entrance found for {:?}", starting_point_y0).as_str(),
                            ),
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

//fn spawn_building_event_handler(
//    mut commands: Commands,
//    heightmap_query: Query<(&Heightmap, &ChunkPosition)>,
//    mut mesh_assets: ResMut<Assets<Mesh>>,
//    mut materials: ResMut<Assets<StandardMaterial>>,
//    occupied_road_tiles: Res<OccupiedRoadTiles>,
//) {
//    for spawn_building_event in spawn_building_events.read() {
//        let heightmap = heightmap_query
//            .iter()
//            .find(|(_, chunk_position)| {
//                **chunk_position == spawn_building_event.position.chunk_position
//            })
//            .unwrap()
//            .0;
//
//        let mesh = create_plane_mesh(spawn_building_event.position.tile_position_2d(), heightmap);
//        let transform =
//            Transform::from_translation(spawn_building_event.position.to_world_position());
//
//        let mut material: StandardMaterial = match spawn_building_event.building_type {
//            BuildingTypeEnum::Residential => Color::DARK_GREEN.into(),
//            BuildingTypeEnum::Commerical => Color::BLUE.into(),
//        };
//
//        material.perceptual_roughness = 1.0;
//        material.reflectance = 0.0;
//
//        let mesh_handle = mesh_assets.add(mesh.into());
//        let building_bundle = BuildingBundle {
//            position: BuildingPosition {
//                position: spawn_building_event.position,
//            },
//            //building_type: BuildingType {
//            //    building_type: spawn_building_event.building_type,
//            //},
//            entrance: BuildingEntrance {
//                position: find_entrance_tile(spawn_building_event.position, &occupied_road_tiles)//
//                    .expect(
//                        format!("No entrance found for {:?}", spawn_building_event.position)
//                            .as_str(),
//                    ),
//            },
//            pbr: PbrBundle {
//                mesh: mesh_handle.clone(),
//                material: materials.add(material),
//                transform,
//                ..default()
//            },
//        };
//        match spawn_building_event.building_type {
//            BuildingTypeEnum::Residential => {
//                commands.spawn((
//                    building_bundle,
//                    BuildingPopulation {
//                        current_population: 1,
//                        max_population: 10,
//                    },
//                ));
//            }
//            BuildingTypeEnum::Commerical => todo!(),
//        };
//    }
//}

fn find_entrance_tile(
    building_position: ChunkTilePosition,
    occupied_road_tiles: &OccupiedRoadTiles,
) -> Option<ChunkTilePosition> {
    let neighbours = building_position.tile_neighbours();
    for neighbour in neighbours.to_array() {
        match neighbour {
            Some(neighbour) => {
                if occupied_road_tiles.tiles.contains_key(&neighbour) {
                    return Some(neighbour);
                }
            }
            None => {}
        }
    }
    None
}

fn create_plane_mesh(starting_position: TilePosition2D, heightmap: &Heightmap) -> Mesh {
    fn create_attributes(
        starting_position: (usize, usize),
        heightmap: &Heightmap,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>) {
        let tile_size = 0.5 * TILE_SIZE;
        let height_offset = 0.03;
        let heights = heightmap[[starting_position.0, starting_position.1]];
        let vert_0 = [-tile_size, heights[0] + height_offset, -tile_size];
        let vert_1 = [tile_size, heights[1] + height_offset, -tile_size];
        let vert_2 = [tile_size, heights[2] + height_offset, tile_size];
        let vert_3 = [-tile_size, heights[3] + height_offset, tile_size];
        let vert_4 = [0.0, heights[4] + height_offset, 0.0];
        let vertices = vec![
            vert_0, vert_1, vert_4, vert_1, vert_2, vert_4, vert_2, vert_3, vert_4, vert_3, vert_0,
            vert_4,
        ];
        let uv_0 = [-1.0, -1.0];
        let uv_1 = [1.0, -1.0];
        let uv_2 = [1.0, 1.0];
        let uv_3 = [-1.0, 1.0];
        let uv_4 = [0.0, 0.0];
        let uv = vec![
            uv_0, uv_1, uv_4, uv_1, uv_2, uv_4, uv_2, uv_3, uv_4, uv_3, uv_0, uv_4,
        ];
        let indices = vec![2, 1, 0, 3, 5, 4, 6, 8, 7, 10, 9, 11];
        let normal_a = unnormalized_normal_vector(vert_0, vert_4, vert_1)
            .normalize()
            .to_array();
        let normal_b = unnormalized_normal_vector(vert_1, vert_4, vert_2)
            .normalize()
            .to_array();
        let normal_c = unnormalized_normal_vector(vert_4, vert_3, vert_2)
            .normalize()
            .to_array();
        let normal_d = unnormalized_normal_vector(vert_0, vert_3, vert_4)
            .normalize()
            .to_array();

        let normals = vec![
            normal_a, normal_a, normal_a, normal_b, normal_b, normal_b, normal_c, normal_c,
            normal_c, normal_d, normal_d, normal_d,
        ];
        //let normals = vec![[0.0, 1.0, 0.0]; vertices.len()];
        (vertices, uv, indices, normals)
    }
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let (vertices, uvs, indices, normals) =
        create_attributes((starting_position[0], starting_position[1]), heightmap);

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    grid_mesh.set_indices(Some(Indices::U32(indices)));

    grid_mesh
}
