pub mod intersection;
pub mod pathfinding;
pub mod road_struct;
pub mod road_tile;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    chunk::chunk_tile_position::TilePosition, cursor::CurrentTile, math_utils::Mean,
    mesh_generator::create_road_mesh, GameState,
};

use self::{pathfinding::PathfindingPlugin, road_struct::Road, road_tile::RoadTile};

use super::{
    heightmap::{HeightmapVertex, HeightmapsResource},
    tile_highlight::{Duration, HighlightTileEvent},
    tools::{CurrentTool, ToolType},
    WorldSettings,
};

pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PathfindingPlugin);
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(
            Update,
            (
                road_tool,
                spawn_road_event_handler,
                //create_road_entity
            )
                .chain()
                .run_if(in_state(GameState::World)),
        );
        app.add_systems(
            Update,
            (debug_road_highlight).run_if(in_state(GameState::World)),
        );
        app.add_event::<SpawnRoadEvent>();
        app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(Event)]
pub struct SpawnRoadEvent {
    pub road: Road,
}

impl SpawnRoadEvent {
    pub fn new(road: Road) -> Self {
        Self { road }
    }
}

#[derive(Resource, Default)]
pub struct RoadTilesResource {
    pub tiles: HashMap<TilePosition, RoadTile>,
}
impl RoadTilesResource {
    pub fn get_neighbours(&self, tile: TilePosition) -> impl Iterator<Item = TilePosition> {
        let mut new_neighbours = Vec::new();
        let neighbours = tile.tile_neighbours();
        for (_, neighbour) in neighbours {
            if self.tiles.contains_key(&neighbour) {
                new_neighbours.push(neighbour);
            }
        }
        new_neighbours.into_iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoadEdge {
    pub distance: usize,
}
#[derive(Resource, Default)]
pub struct RoadIntersections {
    pub intersections: HashMap<TilePosition, Vec<Road>>,
}

#[derive(Bundle)]
pub struct RoadBundle {
    pub road: Road,
    pub pbr: PbrBundle,
}

fn setup(mut commands: Commands) {
    commands.init_resource::<RoadTilesResource>();
    //commands.init_resource::<RoadGraph>();
    commands.init_resource::<RoadIntersections>();
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<RoadTilesResource>();
    //commands.remove_resource::<RoadGraph>();
    commands.remove_resource::<RoadIntersections>();
}

fn debug_road_highlight(
    roads: Query<&Road>,
    mut gizmos: Gizmos,
    heightmaps: Res<HeightmapsResource>,
) {
    for road in roads.iter() {
        gizmos.linestrip(
            road.as_world_positions(&heightmaps, 0.1, 0.0),
            Color::VIOLET,
        );
    }
}

fn road_tool(
    current_tile: Res<CurrentTile>,
    mut spawn_road_events: EventWriter<SpawnRoadEvent>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    mut current_tool: ResMut<CurrentTool>,
    mouse_button: Res<Input<MouseButton>>,
    occupied_road_tiles: Res<RoadTilesResource>,
    world_settings: Res<WorldSettings>,
) {
    if current_tool.tool_type == ToolType::BuildRoad {
        let width = current_tool.tool_strength.round();
        if width <= 0.0 {
            return;
        }

        //Highlight currently selected tile taking into account road width
        highlight_tile_events.send(HighlightTileEvent {
            position: current_tile.position,
            color: Color::GREEN,
            duration: Duration::Once,
        });
        if let Some(starting_point) = current_tool.starting_point {
            //Highlight Road Starting Point
            highlight_tile_events.send(HighlightTileEvent {
                position: starting_point,
                color: Color::PINK,
                duration: Duration::Once,
            });
        } else {
            //Add starting point on mouse input
            if mouse_button.just_pressed(MouseButton::Left) {
                current_tool.starting_point = Some(current_tile.position);
            }
            return;
        }
        //Highlight current road path
        let snapped_position = current_tool
            .starting_point
            .unwrap()
            .snap_to_straight_line(current_tile.position)
            .clamp_to_world(world_settings.world_size);
        let road = Road::new(
            current_tool.starting_point.unwrap(),
            snapped_position,
            width,
        );
        let road_tiles = road.tiles();
        for (road_position, _) in road_tiles {
            //Occupied tiles are red, unoccupied are green
            if occupied_road_tiles.tiles.contains_key(road_position) {
                highlight_tile_events.send(HighlightTileEvent {
                    position: *road_position,
                    color: Color::RED,
                    duration: Duration::Once,
                });
            } else {
                highlight_tile_events.send(HighlightTileEvent {
                    position: *road_position,
                    color: Color::GREEN,
                    duration: Duration::Once,
                });
            }
        }

        //Add ending point on mouse input
        if mouse_button.just_pressed(MouseButton::Left) {
            current_tool.ending_point = Some(current_tile.position);
            //y value has to be 0 for surface roads, TODO: add support for layers
            let mut starting_point_y0 = current_tool.starting_point.unwrap();
            starting_point_y0.position.y = 0;
            let mut ending_point_y0 = snapped_position;
            ending_point_y0.position.y = 0;
            let road = Road::new(starting_point_y0, ending_point_y0, width);
            spawn_road_events.send(SpawnRoadEvent::new(road));
            current_tool.starting_point = None;
            current_tool.ending_point = None;
        }
    }
}

fn spawn_road_event_handler(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut heightmaps: ResMut<HeightmapsResource>,
    mut spawn_road_events: EventReader<SpawnRoadEvent>,
    mut occupied_road_tiles: ResMut<RoadTilesResource>,
    mut road_intersections: ResMut<RoadIntersections>,
) {
    for spawn_road_event in spawn_road_events.read() {
        let road = &spawn_road_event.road;
        let road_tiles = road.tiles();
        //Adds the road to the occupied tiles
        for (road_position, road_tile) in road_tiles {
            if occupied_road_tiles
                .tiles
                .insert(*road_position, *road_tile)
                .is_some()
            {
                road_intersections
                    .intersections
                    .entry(*road_position)
                    .or_default()
                    .push(road.clone());
            }
        }
        //Flatten road tiles along each row
        let mut tiles_to_change = Vec::new();
        for row in road.row_tiles() {
            let average_tile = row
                .iter()
                .map(|(p, _)| Vec4::from_array(heightmaps[p].into()))
                .mean_f32();
            for (position, _) in row {
                let mut tile: HeightmapVertex = average_tile.to_array().into();
                let tile = tile.flatten_with_direction(road.direction());
                tiles_to_change.push((position, *tile));
            }
        }
        let (positions, heights) = tiles_to_change.into_iter().unzip();
        heightmaps.edit_tiles(positions, heights);

        //Create Road Mesh
        let mesh = mesh_assets.add(create_road_mesh(road, &heightmaps));

        //TODO make unique road material
        let mut material: StandardMaterial = Color::rgb(0.1, 0.1, 0.1).into();
        material.perceptual_roughness = 1.0;
        material.reflectance = 0.0;
        let material = material_assets.add(material);
        //Spawns road bundle
        commands.spawn(RoadBundle {
            road: road.clone(),
            pbr: PbrBundle {
                mesh,
                material,
                ..default()
            },
        });
    }
}
