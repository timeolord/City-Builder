pub mod pathfinding;
pub mod road_tile;

use itertools::Itertools;
use std::{collections::HashMap, f32::consts::PI};
use strict_num::ApproxEq;

use bevy::{math::cubic_splines::CubicCurve, prelude::*};
use line_drawing::Bresenham;

use crate::{
    chunk::chunk_tile_position::{CardinalDirection, TilePosition},
    constants::TILE_SIZE,
    cursor::CurrentTile,
    math_utils::{straight_bezier_curve, Arclength, Mean, RoundBy},
    mesh_generator::{combine_meshes, create_box_mesh, create_road_mesh},
    GameState,
};

use self::{pathfinding::PathfindingPlugin, road_tile::RoadTile};

use super::{
    heightmap::{FlattenWithDirection, HeightmapsResource},
    tile_highlight::{self, Duration, HighlightTileEvent},
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

#[derive(Resource)]
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
    starting_position: TilePosition,
    ending_position: TilePosition,
    pub width: f32,
    bezier_curve: CubicCurve<Vec2>,
    length: f32,
    tiles: Option<Vec<(TilePosition, RoadTile)>>,
    direction: CardinalDirection,
}
impl Road {
    pub fn new(starting_position: TilePosition, ending_position: TilePosition, width: f32) -> Self {
        let bezier_curve = straight_bezier_curve(
            starting_position.to_world_position_2d(),
            ending_position.to_world_position_2d(),
        );
        let length = bezier_curve.arclength();
        let mut result = Self {
            starting_position,
            ending_position,
            width,
            bezier_curve,
            length,
            tiles: None,
            direction: Self::calculate_direction(starting_position, ending_position),
        };
        result.calculate_road_tiles();
        result
    }
    fn calculate_direction(
        starting_position: TilePosition,
        ending_position: TilePosition,
    ) -> CardinalDirection {
        let starting_vec = starting_position.position_2d();
        let current_vec = ending_position.position_2d();
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
    pub fn direction(&self) -> CardinalDirection {
        self.direction
    }
    pub fn length(&self) -> f32 {
        self.length
    }
    pub fn subdivisions(&self) -> usize {
        let road_length = self.length().round() as usize;
        let subdivisions = road_length * TILE_SIZE as usize;
        subdivisions * 2
    }
    pub fn tiles(&self) -> &Vec<(TilePosition, RoadTile)> {
        &self
            .tiles
            .as_ref()
            .expect("Road tiles should be calculated")
    }
    pub fn normal_vectors(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.normal_vectors_with_subdivisions(self.subdivisions())
    }
    pub fn normal_vectors_with_subdivisions(
        &self,
        subdivision: usize,
    ) -> impl Iterator<Item = Vec2> + '_ {
        self.bezier_curve.iter_velocities(subdivision).map(|v| {
            //Rotate velocity vector 90 degrees
            let rotated = Vec2::new(v.y, -v.x);
            //Normalize vector
            rotated.normalize_or_zero()
        })
    }
    pub fn as_world_positions<'a>(
        &'a self,
        heightmaps: &'a HeightmapsResource,
        height_offset: f32,
        horizontal_offset: f32,
    ) -> impl Iterator<Item = Vec3> + '_ {
        self.as_2d_positions(horizontal_offset).map(move |p| {
            let mut position = heightmaps.get_from_world_position_2d(p);
            position.y += height_offset;
            position
        })
    }
    pub fn as_2d_positions(&self, horizontal_offset: f32) -> impl Iterator<Item = Vec2> + '_ {
        self.as_2d_positions_with_subdivision(horizontal_offset, self.subdivisions())
    }
    pub fn as_2d_positions_with_subdivision(
        &self,
        horizontal_offset: f32,
        subdivision: usize,
    ) -> impl Iterator<Item = Vec2> + '_ {
        self.bezier_curve
            .iter_positions(subdivision)
            .zip_eq(self.normal_vectors_with_subdivisions(subdivision))
            //We round here to prevent floating point errors from screwing us over later. Like 0.9999999999999999 instead of 1.0
            .map(move |(p, normal)| {
                Vec2::new(p.x.round_by(0.1), p.y.round_by(0.1)) + (normal * horizontal_offset)
            })
    }
    fn calculate_road_tiles(&mut self) {
        let mut road_tiles: Vec<(TilePosition, RoadTile)> = Vec::new();
        let road_width = (self.width / 2.0) - (self.width / 1000.0);
        let subdivison_multipler = 10;

        let positions = self
            .as_2d_positions_with_subdivision(
                -road_width,
                self.subdivisions() * subdivison_multipler,
            )
            .zip_eq(self.as_2d_positions_with_subdivision(
                road_width,
                self.subdivisions() * subdivison_multipler,
            ));
        for (starting, ending) in positions {
            let curve = straight_bezier_curve(starting, ending);
            let curve_length = curve.arclength().ceil() as usize;
            let subdivisions = curve_length * TILE_SIZE as usize;
            let curve_positions = curve.iter_positions(subdivisions);
            for position in curve_positions {
                let position = Vec3::new(position.x, 0.0, position.y);
                let position = TilePosition::from_world_position(position);
                road_tiles.push((
                    position,
                    RoadTile {
                        position,
                        direction: self.direction(),
                    },
                ));
            }
        }
        self.tiles = Some(road_tiles.into_iter().unique().collect_vec());
    }

    fn row_tiles(&self) -> Vec<Vec<(TilePosition, RoadTile)>> {
        let mut road_tiles = Vec::new();
        let road_width = (self.width / 2.0) - (self.width / 1000.0);
        let subdivisions = match self.direction {
            CardinalDirection::North
            | CardinalDirection::South
            | CardinalDirection::East
            | CardinalDirection::West => self.length().round() as usize,
            //Subdivide the diagonal roads by the length of the hypotenuse so that each segment is extactly one tile
            CardinalDirection::NorthEast
            | CardinalDirection::SouthWest
            | CardinalDirection::NorthWest
            | CardinalDirection::SouthEast => (self.length() / 2.0f32.sqrt()).round() as usize,
        };

        let positions = self
            .as_2d_positions_with_subdivision(-road_width, subdivisions)
            .zip_eq(self.as_2d_positions_with_subdivision(road_width, subdivisions));
        for (starting, ending) in positions {
            let curve = straight_bezier_curve(starting, ending);
            let curve_length = curve.arclength().ceil() as usize;
            let subdivisions = curve_length * TILE_SIZE as usize;
            let curve_positions = curve.iter_positions(subdivisions);
            let mut row_tiles = Vec::new();
            for position in curve_positions {
                let position = Vec3::new(position.x, 0.0, position.y);
                let position = TilePosition::from_world_position(position);
                row_tiles.push((
                    position,
                    RoadTile {
                        position,
                        direction: self.direction(),
                    },
                ));
            }
            road_tiles.push(row_tiles);
        }
        road_tiles.into_iter().unique().collect_vec()
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
) {
    match current_tool.tool_type {
        ToolType::BuildRoad => {
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
            match current_tool.starting_point {
                Some(starting_point) => {
                    //Highlight Road Starting Point
                    highlight_tile_events.send(HighlightTileEvent {
                        position: starting_point,
                        color: Color::PINK,
                        duration: Duration::Once,
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
            let snapped_position = current_tool
                .starting_point
                .unwrap()
                .snap_to_straight_line(current_tile.position);
            //.clamp_to_world(world_settings.world_size);
            let road = Road::new(
                current_tool.starting_point.unwrap(),
                snapped_position,
                width,
            );
            let road_tiles = road.tiles();
            for (road_position, _) in road_tiles {
                //Occupied tiles are red, unoccupied are green
                if occupied_road_tiles.tiles.contains_key(&road_position) {
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
        _ => {}
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
        }
        //Flatten road tiles along each row
        let mut tiles_to_change = Vec::new();
        for row in road.row_tiles() {
            let average_tile = row
                .iter()
                .map(|(p, _)| Vec4::from_array(heightmaps[p]))
                .mean_f32();
            for (position, _) in row {
                let mut tile = average_tile.to_array();
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
