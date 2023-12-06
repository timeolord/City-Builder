pub mod intersection;
pub mod pathfinding;
pub mod road_struct;
pub mod road_tile;

use std::{collections::HashSet, ops::Deref, ops::DerefMut};

use bevy::prelude::*;

use itertools::Itertools;

use crate::{
    chunk::chunk_tile_position::TilePosition, cursor::CurrentTile, math_utils::Mean,
    mesh_generator::create_road_mesh, GameState,
};

use self::{
    intersection::{
        spawn_intersection_event_handler, ConnectedRoads, RoadIntersection,
        RoadIntersectionsResource, SpawnIntersectionEvent,
    },
    pathfinding::PathfindingPlugin,
    road_struct::Road,
};

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
                spawn_intersection_event_handler,
                //create_road_entity
            )
                .chain()
                .run_if(in_state(GameState::World)),
        );
        app.add_systems(
            Update,
            (debug_road_highlight).run_if(in_state(GameState::World)),
        );
        app.add_systems(
            PostUpdate,
            (update_road_mesh_event_handler).run_if(in_state(GameState::World)),
        );
        app.add_event::<SpawnRoadEvent>();
        app.add_event::<SpawnIntersectionEvent>();
        app.add_event::<UpdateRoadMeshEvent>();
        app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(Event)]
pub struct SpawnRoadEvent {
    pub road: Road,
}
#[derive(Event)]
pub struct UpdateRoadMeshEvent {
    pub road: Entity,
}

impl SpawnRoadEvent {
    pub fn new(road: Road) -> Self {
        Self { road }
    }
}

#[derive(Resource, Default)]
pub struct RoadTilesResource {
    pub tiles: HashSet<TilePosition>,
}
impl RoadTilesResource {
    pub fn get_neighbours(&self, tile: TilePosition) -> impl Iterator<Item = TilePosition> {
        let mut new_neighbours = Vec::new();
        let neighbours = tile.tile_neighbours();
        for (_, neighbour) in neighbours {
            if self.tiles.contains(&neighbour) {
                new_neighbours.push(neighbour);
            }
        }
        new_neighbours.into_iter()
    }
}
impl Deref for RoadTilesResource {
    type Target = HashSet<TilePosition>;
    fn deref(&self) -> &Self::Target {
        &self.tiles
    }
}
impl DerefMut for RoadTilesResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tiles
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoadEdge {
    pub distance: usize,
}

fn setup(mut commands: Commands) {
    commands.init_resource::<RoadTilesResource>();
    commands.init_resource::<RoadIntersectionsResource>();
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<RoadTilesResource>();
    commands.remove_resource::<RoadIntersectionsResource>();
}

fn debug_road_highlight(
    roads: Query<&Road>,
    intersections: Res<RoadIntersectionsResource>,
    mut tile_highlight_events: EventWriter<HighlightTileEvent>,
    mut gizmos: Gizmos,
    heightmaps: Res<HeightmapsResource>,
) {
    for road in roads.iter() {
        gizmos.linestrip(
            road.as_world_positions(&heightmaps, 0.1, 0.0),
            Color::VIOLET,
        );
        let _center_line = road.center_line_tiles();
        /* for tile in center_line {
            tile_highlight_events.send(HighlightTileEvent {
                position: tile,
                color: Color::YELLOW,
                duration: Duration::Once,
                size: 1,
            });
        } */
    }
    intersections.values().for_each(|intersection| {
        tile_highlight_events.send(HighlightTileEvent {
            position: intersection.position(),
            color: Color::BLUE,
            duration: Duration::Once,
            size: intersection.size,
        });
        let vectors = intersection.connected_road_vectors(&heightmaps);
        for (start, end) in vectors {
            gizmos.line(start, end, Color::RED);
        }
    });
}

fn road_tool(
    current_tile: Res<CurrentTile>,
    mut spawn_road_events: EventWriter<SpawnRoadEvent>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    mut current_tool: ResMut<CurrentTool>,
    mouse_button: Res<Input<MouseButton>>,
    occupied_road_tiles: Res<RoadTilesResource>,
    world_settings: Res<WorldSettings>,
    _intersections: Res<RoadIntersectionsResource>,
    _roads: Query<&Road>,
) {
    if current_tool.tool_type == ToolType::BuildRoad {
        let width = current_tool.tool_strength.round() as u32;
        if width == 0 {
            return;
        }
        //Flag to check if the road is conflicting with another road
        let mut conflicting = false;

        //Highlight currently selected tile taking into account road width
        highlight_tile_events.send(HighlightTileEvent {
            position: current_tile.position,
            color: Color::GREEN,
            duration: Duration::Once,
            size: width,
        });
        if let Some(starting_point) = current_tool.starting_point {
            //Highlight Road Starting Point
            highlight_tile_events.send(HighlightTileEvent {
                position: starting_point,
                color: Color::PINK,
                duration: Duration::Once,
                size: width,
            });
        } else {
            //Add starting point on mouse input
            if mouse_button.just_pressed(MouseButton::Left) {
                current_tool.starting_point = Some(current_tile.position);
            }
            return;
        }
        //Unset starting point on right click
        if mouse_button.just_pressed(MouseButton::Right) {
            current_tool.starting_point = None;
            current_tool.ending_point = None;
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
        let starting_wide_tile = current_tool.starting_point.unwrap().to_wide_tile(width);
        let ending_wide_tile = snapped_position.to_wide_tile(width);
        //The road can conflict if the starting or ending point is an intersection or if the tile is from a road that is part of the intersection (this allows for diagonal roads to join properly)
        let mut occupied_road_tiles = occupied_road_tiles.clone();
        starting_wide_tile.tiles().for_each(|tile| {
            occupied_road_tiles.remove(&tile);
        });
        ending_wide_tile.tiles().for_each(|tile| {
            occupied_road_tiles.remove(&tile);
        });
        //if intersections.contains_key()
        for (road_position, _) in road_tiles {
            //Occupied tiles are red, unoccupied are green
            if occupied_road_tiles.contains(road_position)
            //&& !intersections.contains_key(road_position)
            //&& !starting_wide_tile.collides_with_tile(*road_position)
            //&& !ending_wide_tile.collides_with_tile(*road_position)
            //&& !intersections[road_position]
            //    .roads
            //    .tiles(&roads)
            //    .contains(road_position)
            {
                conflicting = true;
                highlight_tile_events.send(HighlightTileEvent {
                    position: *road_position,
                    color: Color::RED,
                    duration: Duration::Once,
                    size: 1,
                });
            } else {
                highlight_tile_events.send(HighlightTileEvent {
                    position: *road_position,
                    color: Color::GREEN,
                    duration: Duration::Once,
                    size: 1,
                });
            }
        }

        //TODO re-add conflict checker
        //Add ending point on mouse input
        if mouse_button.just_pressed(MouseButton::Left) {
            //If starting point and ending point are the same, do nothing
            if current_tool.starting_point.unwrap() == snapped_position {
                return;
            }
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
    mut heightmaps: ResMut<HeightmapsResource>,
    mut spawn_road_events: EventReader<SpawnRoadEvent>,
    mut occupied_road_tiles: ResMut<RoadTilesResource>,
    mut intersection_events: EventWriter<SpawnIntersectionEvent>,
    mut update_road_mesh_events: EventWriter<UpdateRoadMeshEvent>,
    other_roads: Query<&Road>,
) {
    for spawn_road_event in spawn_road_events.read() {
        let road = &spawn_road_event.road;

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
        let (positions, heights): (Vec<_>, Vec<_>) = tiles_to_change.into_iter().unzip();
        heightmaps.edit_tiles(&positions, &heights);

        //Spawns road component
        let road_entity = commands.spawn(road.clone()).id();

        //Spawn intersections for the starting and ending positions of the road
        for position in &[road.starting_position(), road.ending_position()] {
            let mut enum_map = ConnectedRoads::default();
            if position == &road.starting_position() {
                enum_map[road.direction()] = Some(road_entity);
            } else {
                enum_map[-road.direction()] = Some(road_entity);
            }
            let intersection = RoadIntersection::new(*position, road.width(), enum_map);
            intersection_events.send(SpawnIntersectionEvent { intersection });
        }
        //Spawn intersections when a road intersects with another road
        for other_road in other_roads.iter() {
            if let Some(intersection_position) = road.intersection(other_road) {
                let mut enum_map = ConnectedRoads::default();
                enum_map[road.direction()] = Some(road_entity);
                //enum_map[-road.direction()] = Some(other_road);
                let intersection =
                    RoadIntersection::new(intersection_position, road.width(), enum_map);
                intersection_events.send(SpawnIntersectionEvent { intersection });
            }
        }

        //Update Road Mesh
        update_road_mesh_events.send(UpdateRoadMeshEvent { road: road_entity });

        //Adds the road to the occupied tiles
        let road_tiles = road.tiles();
        for (road_position, _road_tile) in road_tiles {
            occupied_road_tiles.tiles.insert(*road_position);
        }
    }
}

fn update_road_mesh_event_handler(
    mut events: EventReader<UpdateRoadMeshEvent>,
    roads: Query<&Road>,
    heightmaps: Res<HeightmapsResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for event in events.read() {
        let road = roads.get(event.road).unwrap();
        let entity = event.road;
        let mesh = create_road_mesh(road, &heightmaps);

        //TODO make unique road material
        let mut material: StandardMaterial = Color::rgb(0.1, 0.1, 0.1).into();
        material.perceptual_roughness = 1.0;
        material.reflectance = 0.0;

        commands.entity(entity).remove::<PbrBundle>();
        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(mesh),
            material: material_assets.add(material),
            ..default()
        });
    }
}
