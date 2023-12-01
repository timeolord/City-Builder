pub mod pathfinding;
pub mod road_tile;

use itertools::Itertools;
use std::{collections::HashMap, f32::consts::PI};
use strict_num::ApproxEq;

use bevy::{math::cubic_splines::CubicCurve, prelude::*};
use line_drawing::Bresenham;

use crate::{
    chunk::chunk_tile_position::{CardinalDirection, TilePosition},
    constants::{F32_TOLERANCE, TILE_SIZE},
    cursor::CurrentTile,
    mesh_generator::{combine_meshes, create_box_mesh, create_road_mesh},
    GameState,
};

use self::{pathfinding::PathfindingPlugin, road_tile::RoadTile};

use super::{
    heightmap::HeightmapsResource,
    terraform::EditTileEvent,
    tile_highlight::HighlightTileEvent,
    tools::{CurrentTool, ToolType},
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
            (highlight_road_segments).run_if(in_state(GameState::World)),
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

#[derive(Resource)]
pub struct RoadTilesResource {
    pub tiles: HashMap<TilePosition, RoadTile>,
}
impl RoadTilesResource {
    pub fn get_neighbours(&self, tile: TilePosition) -> Vec<TilePosition> {
        let mut new_neighbours = Vec::new();
        let neighbours = tile.tile_neighbours();
        for (_, neighbour) in neighbours {
            if self.tiles.contains_key(&neighbour) {
                new_neighbours.push(neighbour);
            }
        }
        new_neighbours
    }
}
impl Default for RoadTilesResource {
    fn default() -> Self {
        Self {
            tiles: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoadEdge {
    pub distance: usize,
}
#[derive(Resource)]
pub struct RoadIntersections {
    pub intersections: HashMap<TilePosition, Vec<Road>>,
}
impl Default for RoadIntersections {
    fn default() -> Self {
        Self {
            intersections: HashMap::new(),
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Road {
    pub starting_position: TilePosition,
    pub ending_position: TilePosition,
    pub width: f32,
    bezier_curve: CubicCurve<Vec2>,
}
impl Road {
    pub fn new(starting_position: TilePosition, ending_position: TilePosition, width: f32) -> Self {
        Self {
            starting_position,
            ending_position,
            width,
            bezier_curve: CubicBezier::new([[
                starting_position.to_world_position_2d(),
                starting_position
                    .to_world_position_2d()
                    .lerp(ending_position.to_world_position_2d(), 1.0 / 3.0),
                starting_position
                    .to_world_position_2d()
                    .lerp(ending_position.to_world_position_2d(), 2.0 / 3.0),
                ending_position.to_world_position_2d(),
            ]])
            .to_curve(),
        }
    }
    pub fn road_direction(&self) -> CardinalDirection {
        let starting_vec = self.starting_position.position_2d();
        let current_vec = self.starting_position.position_2d();
        let relative_vec = current_vec - starting_vec;
        let angle = (relative_vec.y as f32).atan2(relative_vec.x as f32) * 180.0 / PI;
        match angle as i32 {
            0 => CardinalDirection::North,
            45 => CardinalDirection::NorthEast,
            90 => CardinalDirection::East,
            135 => CardinalDirection::SouthEast,
            180 => CardinalDirection::South,
            -45 => CardinalDirection::NorthWest,
            -90 => CardinalDirection::West,
            -135 => CardinalDirection::SouthWest,
            -180 => CardinalDirection::South,
            _ => {
                panic!("Unexpected angle: {}", angle);
            }
        }
    }
    pub fn road_length(&self) -> f32 {
        self.bezier_curve
            .iter_positions(100)
            .tuple_windows()
            .map(|(a, b)| a.distance(b))
            .sum()
    }
    pub fn as_world_positions(
        &self,
        heightmaps: &HeightmapsResource,
        height_offset: f32,
        horizontal_offset: f32,
    ) -> Vec<Vec3> {
        let road_length = self.road_length().round() as usize;
        let subdivisions = road_length * TILE_SIZE as usize;

        let normal_vectors = self.bezier_curve.iter_velocities(subdivisions).map(|v| {
            //Rotate velocity vector 90 degrees
            let rotated = Vec2::new(v.y, -v.x);
            //Normalize vector
            let normalized = rotated.normalize_or_zero();
            //Scale vector
            normalized * horizontal_offset
        });

        self.bezier_curve
            .iter_positions(subdivisions)
            .zip(normal_vectors)
            .map(|(p, normal)| {
                let mut position = heightmaps.get_from_world_position_2d(p + normal);
                position.y += height_offset;
                position
            })
            .collect_vec()
    }
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

fn highlight_road_segments(
    roads: Query<&Road>,
    mut gizmos: Gizmos,
    //heightmaps: Query<&Heightmap, &ChunkPosition>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    heightmaps: Res<HeightmapsResource>,
) {
    for road in roads.iter() {
        //highlight_tile_events.send(HighlightTileEvent {
        //    position: road.clone(),
        //    color: Color::VIOLET,
        //});
        let road_width = road.width / 2.0;
        let cubic_curve = road.as_world_positions(&heightmaps, 0.1, 0.0);
        gizmos.linestrip(cubic_curve, Color::VIOLET);
        let cubic_curve = road.as_world_positions(&heightmaps, 0.1, -road_width);
        gizmos.linestrip(cubic_curve, Color::VIOLET);
        let cubic_curve = road.as_world_positions(&heightmaps, 0.1, road_width);
        gizmos.linestrip(cubic_curve, Color::VIOLET);
        //gizmos.line(
        //    road.starting_position.to_world_position(),
        //    road.ending_position.to_world_position(),
        //    Color::VIOLET,
        //);
    }
}

fn road_tool(
    mut commands: Commands,
    current_tile: Res<CurrentTile>,
    mut spawn_road_events: EventWriter<SpawnRoadEvent>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    mut current_tool: ResMut<CurrentTool>,
    mouse_button: Res<Input<MouseButton>>,
    occupied_road_tiles: Res<RoadTilesResource>,
) {
    match current_tool.tool_type {
        ToolType::BuildRoad => {
            let width = current_tool.tool_strength.round();
            if width < 0.0 {
                return;
            }

            //Highlight currently selected tile taking into account road width
            highlight_tile_events.send(HighlightTileEvent {
                position: current_tile.position,
                color: Color::GREEN,
            });
            match current_tool.starting_point {
                Some(starting_point) => {
                    //Highlight Road Starting Point
                    highlight_tile_events.send(HighlightTileEvent {
                        position: starting_point,
                        color: Color::PINK,
                    });
                }
                None => {
                    //Add starting point on mouse input
                    if mouse_button.just_pressed(MouseButton::Left) {
                        current_tool.starting_point = Some(current_tile.position);
                    }
                    return;
                }
            }
            //Highlight current road path
            let snapped_position =
                snap_to_straight_line(current_tool.starting_point.unwrap(), current_tile.position);
            //.clamp_to_world(world_settings.world_size);
            let road = Road::new(
                current_tool.starting_point.unwrap(),
                snapped_position,
                width,
            );
            let road_tiles = calculate_road_tiles(&road);
            for (road_position, _) in road_tiles {
                //Occupied tiles are red, unoccupied are green
                if occupied_road_tiles.tiles.contains_key(&road_position) {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: road_position,
                        color: Color::RED,
                    });
                } else {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: road_position,
                        color: Color::GREEN,
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
                spawn_road_events.send(SpawnRoadEvent::new(road.clone()));
                commands.spawn(road);
                current_tool.starting_point = None;
                current_tool.ending_point = None;
            }
        }
        _ => {}
    }
}

fn spawn_road_event_handler(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    heightmaps: Res<HeightmapsResource>,
    mut spawn_road_events: EventReader<SpawnRoadEvent>,
    mut occupied_road_tiles: ResMut<RoadTilesResource>,
    mut road_intersections: ResMut<RoadIntersections>,
) {
    for spawn_road_event in spawn_road_events.read() {
        let road = &spawn_road_event.road;
        let road_tiles = calculate_road_tiles(road);
        for (road_position, road_tile) in road_tiles.iter() {
            //Adds the road to the occupied tiles
            match occupied_road_tiles.tiles.insert(*road_position, *road_tile) {
                Some(_) => {
                    road_intersections
                        .intersections
                        .entry(*road_position)
                        .or_default()
                        .push(road.clone());
                }
                None => {}
            }
            //Create Road Mesh
            //let mesh = mesh_assets.add(create_road_mesh(road, &heightmaps));

            //Spawns road bundle
            //TODO add road material
            //commands.spawn(RoadBundle {
            //    road: road.clone(),
            //    pbr: PbrBundle { mesh, ..default() },
            //});
        }
    }
}

fn create_road_entity(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    heightmaps: Res<HeightmapsResource>,
    occupied_road_tiles: Res<RoadTilesResource>,
    mut road_entity: Local<Option<Entity>>,
    //mut road_graph: ResMut<RoadGraph>,
    mut edit_tile_events: EventWriter<EditTileEvent>,
) {
    if occupied_road_tiles.is_changed() || road_entity.is_none() {
        let mut meshes: Vec<Mesh> = Vec::new();
        let mut transforms = Vec::new();
        for (tile_position, road_tile) in occupied_road_tiles.tiles.iter() {
            let mut heights = heightmaps[*tile_position];
            //let sub_tile_heights: Array2D<HeightmapVertex> =
            //    Array2D::filled_with([0.0, 0.0, 0.0, 0.0, 0.0], road_tile.width, road_tile.width);

            match road_tile.direction {
                CardinalDirection::North | CardinalDirection::South => {
                    //Straighten the slope to the direction of the road
                    heights[2] = heights[1];
                    heights[3] = heights[0];
                    //heights[4] = (heights[1] + heights[0]) / 2.0;
                }
                CardinalDirection::East | CardinalDirection::West => {
                    //Straighten the slope to the direction of the road
                    heights[0] = heights[1];
                    heights[3] = heights[2];
                    //heights[4] = (heights[1] + heights[2]) / 2.0;
                }
                CardinalDirection::NorthEast | CardinalDirection::SouthWest => {
                    //Straighten the slope to the direction of the road
                    heights[1] = (heights[0] + heights[2]) / 2.0;
                    heights[3] = (heights[0] + heights[2]) / 2.0;
                    //heights[4] = (heights[0] + heights[2]) / 2.0;
                }
                CardinalDirection::SouthEast | CardinalDirection::NorthWest => {
                    //Straighten the slope to the direction of the road
                    heights[0] = (heights[1] + heights[3]) / 2.0;
                    heights[2] = (heights[1] + heights[3]) / 2.0;
                    //heights[4] = (heights[1] + heights[3]) / 2.0;
                }
            }
            edit_tile_events.send(EditTileEvent {
                tile_position: *tile_position,
                new_vertices: heights,
            });
            let mesh = create_box_mesh(heights, 0.1);
            let transform = Transform::from_translation(tile_position.to_world_position());

            meshes.push(mesh);
            transforms.push(transform);
        }
        let mesh = combine_meshes(
            meshes.as_slice(),
            transforms.as_slice(),
            true,
            false,
            true,
            false,
        );
        let mut material: StandardMaterial = Color::rgb(0.1, 0.1, 0.1).into();
        material.perceptual_roughness = 1.0;
        material.reflectance = 0.0;

        let mesh_handle = mesh_assets.add(mesh.into());
        if let Some(road_entity) = road_entity.as_mut() {
            commands.entity(*road_entity).despawn_recursive();
        }
        *road_entity = Some(
            commands
                .spawn(PbrBundle {
                    mesh: mesh_handle.clone(),
                    material: materials.add(material),
                    transform: Transform::from_translation(Vec3::ZERO),
                    ..default()
                })
                .id(),
        );
    }
}

fn calculate_road_tiles(road: &Road) -> Vec<(TilePosition, RoadTile)> {
    let starting_position = road.starting_position.position_2d();
    let ending_position = road.ending_position.position_2d();
    let road_direction = road.road_direction();
    let tiles = Bresenham::new(starting_position.into(), ending_position.into())
        .into_iter()
        .map(|(x, y)| {
            let position = TilePosition {
                position: IVec3::new(x, 0, y),
            };
            (
                position,
                RoadTile {
                    position,
                    direction: road_direction,
                },
            )
        })
        .into_iter();

    let diagonal_tiles = match road_direction {
        CardinalDirection::NorthEast
        | CardinalDirection::SouthEast
        | CardinalDirection::SouthWest
        | CardinalDirection::NorthWest => {
            let directions = road_direction.split_direction();
            let mut diagonal_tiles = vec![];
            for direction in directions {
                let starting_position = (road.starting_position + direction).position_2d();
                let ending_position =
                    ((road.ending_position + direction) - road_direction).position_2d();
                let tiles = Bresenham::new(starting_position.into(), ending_position.into())
                    .into_iter()
                    .map(|(x, y)| {
                        let position = TilePosition {
                            position: IVec3::new(x, 0, y),
                        };
                        (
                            position,
                            RoadTile {
                                position,
                                direction: road_direction,
                            },
                        )
                    });
                diagonal_tiles.extend(tiles);
            }
            diagonal_tiles
        }
        _ => Vec::new(),
    };
    let tiles = tiles.chain(diagonal_tiles);
    tiles.unique().collect_vec()
}

fn snap_to_straight_line(
    starting_position: TilePosition,
    current_position: TilePosition,
) -> TilePosition {
    let starting_vec = starting_position.position_2d();
    let current_vec = current_position.position_2d();
    let relative_vec = current_vec - starting_vec;
    let length = (relative_vec.distance_squared(IVec2::ZERO) as f32)
        .sqrt()
        .round()
        .abs() as i32;
    let angle = (relative_vec.y as f32).atan2(relative_vec.x as f32) * 180.0 / PI;
    let result_position = if angle.abs() == 0.0
        || angle.abs() == 45.0
        || angle.abs() == 90.0
        || angle.abs() == 135.0
        || angle.abs() == 180.0
    {
        current_position
    } else {
        let quadrant = angle / 45.0;
        match quadrant.round() as i32 {
            0 => {
                let directional_vec = IVec2::X;
                let tile_vec = directional_vec * length + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            1 => {
                let vec_values = (45f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            2 => {
                let directional_vec = IVec2::Y;
                let tile_vec = directional_vec * length + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            3 => {
                let vec_values = (135f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            4 | -4 => {
                let directional_vec = IVec2::X * -1;
                let tile_vec = directional_vec * length + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            -1 => {
                let vec_values = (315f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            -2 => {
                let directional_vec = IVec2::Y * -1;
                let tile_vec = directional_vec * length + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            -3 => {
                let vec_values = (225f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                TilePosition {
                    position: IVec3::new(tile_vec.x as i32, 0, tile_vec.y as i32),
                }
            }
            _ => {
                panic!("Unexpected quadrant: {}", quadrant);
            }
        }
    };
    result_position
}
