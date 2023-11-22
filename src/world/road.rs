pub mod pathfinding;

use std::{collections::HashMap, f32::consts::PI};

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    utils::petgraph::prelude::UnGraphMap,
};
use line_drawing::{Bresenham, WalkGrid};

use crate::{
    chunk::{
        chunk_tile_position::{ChunkPosition, ChunkTilePosition, TilePosition2D},
        unnormalized_normal_vector,
    },
    constants::TILE_SIZE,
    cursor::CurrentTile,
    world::heightmap_generator::Heightmap,
    GameState,
};

use self::pathfinding::PathfindingPlugin;

use super::{
    tile_highlight::HighlightTileEvent,
    tools::{CurrentTool, ToolType},
    AsI32,
};

pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PathfindingPlugin);
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(
            Update,
            (road_tool, spawn_road_event_handler, create_road_entity)
                .chain()
                .run_if(in_state(GameState::World)),
        );
        app.add_systems(
            Update,
            (highlight_road_intersections).run_if(in_state(GameState::World)),
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
pub struct OccupiedRoadTiles {
    pub tiles: HashMap<ChunkTilePosition, (Road, RoadType)>,
}
impl OccupiedRoadTiles {
    pub fn get_neighbours(&self, tile: ChunkTilePosition) -> Vec<ChunkTilePosition> {
        let mut new_neighbours = Vec::new();
        let neighbours = tile.non_diagonal_tile_neighbours();
        for neighbour in neighbours.to_array() {
            match neighbour {
                Some(neighbour) if self.tiles.contains_key(&neighbour) => {
                    new_neighbours.push(neighbour);
                }
                _ => {}
            }
        }
        new_neighbours
    }
}
impl Default for OccupiedRoadTiles {
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
pub struct RoadGraph {
    pub graph: UnGraphMap<ChunkTilePosition, RoadEdge>,
}
impl Default for RoadGraph {
    fn default() -> Self {
        Self {
            graph: UnGraphMap::new(),
        }
    }
}
#[derive(Resource)]
pub struct RoadIntersections {
    pub intersections: HashMap<ChunkTilePosition, Vec<Road>>,
}
impl Default for RoadIntersections {
    fn default() -> Self {
        Self {
            intersections: HashMap::new(),
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Road {
    //Starting_position < Ending_position
    pub starting_position: ChunkTilePosition,
    pub ending_position: ChunkTilePosition,
    pub width: usize,
}
impl Road {
    fn road_type(&self) -> RoadType {
        let starting_vec = IVec2::from_array(self.starting_position.as_tile_position_2d().as_i32());
        let current_vec = IVec2::from_array(self.ending_position.as_tile_position_2d().as_i32());
        let relative_vec = current_vec - starting_vec;
        let angle = (relative_vec.y as f32).atan2(relative_vec.x as f32) * 180.0 / PI;
        match angle.abs().round() as i32 {
            0 => RoadType::Straight,
            45 => RoadType::Diagonal,
            90 => RoadType::Straight,
            135 => RoadType::Diagonal,
            180 => RoadType::Straight,
            _ => {
                panic!("Unexpected angle: {}", angle);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoadType {
    Straight,
    Diagonal,
}

#[derive(Bundle)]
pub struct RoadBundle {
    pub road: Road,
    pub pbr: PbrBundle,
}

fn setup(mut commands: Commands) {
    commands.init_resource::<OccupiedRoadTiles>();
    commands.init_resource::<RoadGraph>();
    commands.init_resource::<RoadIntersections>();
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<OccupiedRoadTiles>();
    commands.remove_resource::<RoadGraph>();
    commands.remove_resource::<RoadIntersections>();
}

fn highlight_road_intersections(
    road_graph: Res<RoadIntersections>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
) {
    for road in road_graph.intersections.keys() {
        highlight_tile_events.send(HighlightTileEvent {
            position: road.clone(),
            color: Color::VIOLET,
        });
    }
}

fn road_tool(
    current_tile: Res<CurrentTile>,
    mut spawn_road_events: EventWriter<SpawnRoadEvent>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    mut current_tool: ResMut<CurrentTool>,
    mouse_button: Res<Input<MouseButton>>,
    occupied_road_tiles: Res<OccupiedRoadTiles>,
    world_settings: Res<crate::world::WorldSettings>,
) {
    match current_tool.tool_type {
        ToolType::BuildRoad => {
            match current_tool.starting_point {
                Some(starting_point) => {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: starting_point,
                        color: Color::GREEN,
                    });
                }
                None => {
                    if mouse_button.just_pressed(MouseButton::Left) {
                        current_tool.starting_point = Some(current_tile.position);
                    }
                    return;
                }
            }
            //Highlight current road path
            let snapped_position =
                snap_to_straight_line(current_tool.starting_point.unwrap(), current_tile.position)
                    .clamp_to_world(world_settings.world_size);
            let road = Road {
                starting_position: current_tool.starting_point.unwrap(),
                ending_position: snapped_position,
                width: 1,
            };
            let road_tiles = calculate_road_tiles(&road);
            for (road_tile, _) in road_tiles {
                if occupied_road_tiles.tiles.contains_key(&road_tile) {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: road_tile,
                        color: Color::RED,
                    });
                } else {
                    highlight_tile_events.send(HighlightTileEvent {
                        position: road_tile,
                        color: Color::YELLOW_GREEN,
                    });
                }
            }

            if mouse_button.just_pressed(MouseButton::Left) {
                current_tool.ending_point = Some(current_tile.position);
                let mut starting_point_y0 = current_tool.starting_point.unwrap();
                starting_point_y0.tile_position[1] = 0;
                let mut ending_point_y0 = snapped_position;
                ending_point_y0.tile_position[1] = 0;
                spawn_road_events.send(SpawnRoadEvent::new(Road {
                    starting_position: starting_point_y0,
                    ending_position: ending_point_y0,
                    width: 1,
                }));
                current_tool.starting_point = None;
                current_tool.ending_point = None;
            }
        }
        _ => {}
    }
}

fn spawn_road_event_handler(
    mut spawn_road_events: EventReader<SpawnRoadEvent>,
    mut occupied_road_tiles: ResMut<OccupiedRoadTiles>,
    mut road_intersections: ResMut<RoadIntersections>,
) {
    for spawn_road_event in spawn_road_events.read() {
        let road = &spawn_road_event.road;
        let road_tiles = calculate_road_tiles(road);
        for (road_tile, road_type) in road_tiles.iter() {
            match occupied_road_tiles
                .tiles
                .insert(*road_tile, (road.clone(), *road_type))
            {
                Some(_) => {
                    road_intersections
                        .intersections
                        .entry(*road_tile)
                        .or_default()
                        .push(road.clone());
                }
                None => {}
            };

            //edit_tile_events.send(EditTileEvent {
            //    tile_position: road_tile,
            //    new_vertices: vec![tile_height; 5].try_into().unwrap(),
            //})
        }
    }
}

fn create_road_entity(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    heightmap_query: Query<(&Heightmap, &ChunkPosition)>,
    occupied_road_tiles: Res<OccupiedRoadTiles>,
    mut road_entity: Local<Option<Entity>>,
    //mut road_graph: ResMut<RoadGraph>,
    //mut edit_tile_events: EventWriter<EditTileEvent>,
) {
    if occupied_road_tiles.is_changed() || road_entity.is_none() {
        let mut meshes: Vec<Mesh> = Vec::new();
        let mut transforms = Vec::new();
        //TODO: check if this is an optimization
        //let heightmaps = heightmap_query.iter().collect::<Vec<_>>();
        for (road_tile, (old_road, road_type)) in occupied_road_tiles.tiles.iter() {
            let heightmap = heightmap_query
                .iter()
                .find(|(_, chunk_position)| **chunk_position == road_tile.chunk_position)
                .unwrap()
                .0;

            let new_road = Road {
                starting_position: *road_tile,
                ending_position: *road_tile,
                width: old_road.width,
            };

            let mesh = create_road_mesh(&new_road, heightmap);
            //let transform = Transform::from_translation(road_tile.to_world_position());

            meshes.push(mesh);
            //transforms.push(transform);
            transforms.push(Transform::IDENTITY);
            //edit_tile_events.send(EditTileEvent {
            //    tile_position: road_tile,
            //    new_vertices: vec![tile_height; 5].try_into().unwrap(),
            //})
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

fn create_road_mesh(road: &Road, heightmap: &Heightmap) -> Mesh {
    fn create_attributes(
        starting_position: [usize; 2],
        heightmap: &Heightmap,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>) {
        let tile_size = 0.5 * TILE_SIZE;
        let height_offset = 0.03;
        //let heights = heightmap[[starting_position.0, starting_position.1]];
        //let vert_0 = [-tile_size, heights[0] + height_offset, -tile_size];
        //let vert_1 = [tile_size, heights[1] + height_offset, -tile_size];
        //let vert_2 = [tile_size, heights[2] + height_offset, tile_size];
        //let vert_3 = [-tile_size, heights[3] + height_offset, tile_size];
        //let vert_4 = [0.0, heights[4] + height_offset, 0.0];
        let world_position =
            ChunkTilePosition::from_tile_position_2d(starting_position.into()).to_world_position();
        let vert_0 = [
            world_position.x - tile_size,
            height_offset,
            world_position.y - tile_size,
        ];
        let vert_1 = [
            world_position.x + tile_size,
            height_offset,
            world_position.y - tile_size,
        ];
        let vert_2 = [
            world_position.x + tile_size,
            height_offset,
            world_position.y + tile_size,
        ];
        let vert_3 = [
            world_position.x - tile_size,
            height_offset,
            world_position.y + tile_size,
        ];
        let vert_4 = [
            world_position.x + 0.0,
            height_offset,
            world_position.y + 0.0,
        ];
        //let vert_0 = heightmap.get_from_world_position(vert_0.into()).to_array();
        //let vert_1 = heightmap.get_from_world_position(vert_1.into()).to_array();
        //let vert_2 = heightmap.get_from_world_position(vert_2.into()).to_array();
        //let vert_3 = heightmap.get_from_world_position(vert_3.into()).to_array();
        //let vert_4 = heightmap.get_from_world_position(vert_4.into()).to_array();
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
    let starting_position = road.starting_position.as_tile_position_2d();
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let (vertices, uvs, indices, normals) =
        create_attributes(starting_position, heightmap);

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    grid_mesh.set_indices(Some(Indices::U32(indices)));

    grid_mesh
}

fn calculate_road_tiles(road: &Road) -> (Vec<(ChunkTilePosition, RoadType)>) {
    let mut tiles = Vec::new();
    let starting_position = road.starting_position.as_tile_position();
    let ending_position = road.ending_position.as_tile_position();
    let road_type = road.road_type();

    for (x, y) in Bresenham::new(
        (starting_position[0] as isize, starting_position[2] as isize),
        (ending_position[0] as isize, ending_position[2] as isize),
    ) {
        tiles.push(([x as usize, y as usize], RoadType::Straight));
    }

    if road_type == RoadType::Diagonal {
        let mut new_tiles = Vec::new();
        for (tile, _) in tiles.iter().take(tiles.len() - 1).skip(1) {
            let neighbours =
                ChunkTilePosition::from_tile_position_2d(*tile).non_diagonal_tile_neighbours();
            for neighbour in neighbours.to_array() {
                match neighbour {
                    Some(neighbour) => {
                        new_tiles.push((neighbour.as_tile_position_2d(), RoadType::Diagonal));
                    }
                    _ => {}
                }
            }
        }
        tiles.append(&mut new_tiles);
    }

    let mut tiles = tiles
        .into_iter()
        .map(|(tile, road_type)| {
            (
                ChunkTilePosition::from_tile_position([tile[0], 0, tile[1]]),
                road_type,
            )
        })
        .collect::<Vec<_>>();
    tiles.sort_unstable();
    tiles.dedup();
    tiles
}

fn snap_to_straight_line(
    starting_position: ChunkTilePosition,
    current_position: ChunkTilePosition,
) -> ChunkTilePosition {
    let starting_vec = IVec2::from_array(starting_position.as_tile_position_2d().as_i32());
    let current_vec = IVec2::from_array(current_position.as_tile_position_2d().as_i32());
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
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            1 => {
                let vec_values = (45f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            2 => {
                let directional_vec = IVec2::Y;
                let tile_vec = directional_vec * length + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            3 => {
                let vec_values = (135f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            4 | -4 => {
                let directional_vec = IVec2::X * -1;
                let tile_vec = directional_vec * length + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            -1 => {
                let vec_values = (315f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            -2 => {
                let directional_vec = IVec2::Y * -1;
                let tile_vec = directional_vec * length + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            -3 => {
                let vec_values = (225f32 * PI / 180.0).sin_cos();
                let directional_vec = Vec2::from_array([vec_values.1, vec_values.0]);
                let tile_vec = (directional_vec * length as f32).round().as_ivec2() + starting_vec;
                ChunkTilePosition::from_tile_position([tile_vec.x as usize, 0, tile_vec.y as usize])
            }
            _ => {
                panic!("Unexpected quadrant: {}", quadrant);
            }
        }
    };
    result_position
}

fn combine_meshes(
    meshes: &[Mesh],
    transforms: &[Transform],
    use_normals: bool,
    use_tangents: bool,
    use_uvs: bool,
    use_colors: bool,
) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut tangets: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut indices_offset = 0;

    if meshes.len() != transforms.len() {
        panic!(
            "meshes.len({}) != transforms.len({})",
            meshes.len(),
            transforms.len()
        );
    }

    for (mesh, trans) in meshes.iter().zip(transforms) {
        if let Indices::U32(mesh_indices) = &mesh.indices().unwrap() {
            let mat = trans.compute_matrix();

            let positions_len;

            if let Some(VertexAttributeValues::Float32x3(vert_positions)) =
                &mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                positions_len = vert_positions.len();
                for p in vert_positions {
                    positions.push(mat.transform_point3(Vec3::from(*p)).into());
                }
            } else {
                panic!("no positions")
            }

            if use_uvs {
                if let Some(VertexAttributeValues::Float32x2(vert_uv)) =
                    &mesh.attribute(Mesh::ATTRIBUTE_UV_0)
                {
                    for uv in vert_uv {
                        uvs.push(*uv);
                    }
                } else {
                    panic!("no uvs")
                }
            }

            if use_normals {
                // Comment below taken from mesh_normal_local_to_world() in mesh_functions.wgsl regarding
                // transform normals from local to world coordinates:

                // NOTE: The mikktspace method of normal mapping requires that the world normal is
                // re-normalized in the vertex shader to match the way mikktspace bakes vertex tangents
                // and normal maps so that the exact inverse process is applied when shading. Blender, Unity,
                // Unreal Engine, Godot, and more all use the mikktspace method. Do not change this code
                // unless you really know what you are doing.
                // http://www.mikktspace.com/

                let inverse_transpose_model = mat.inverse().transpose();
                let inverse_transpose_model = Mat3 {
                    x_axis: inverse_transpose_model.x_axis.xyz(),
                    y_axis: inverse_transpose_model.y_axis.xyz(),
                    z_axis: inverse_transpose_model.z_axis.xyz(),
                };

                if let Some(VertexAttributeValues::Float32x3(vert_normals)) =
                    &mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                {
                    for n in vert_normals {
                        normals.push(
                            inverse_transpose_model
                                .mul_vec3(Vec3::from(*n))
                                .normalize_or_zero()
                                .into(),
                        );
                    }
                } else {
                    panic!("no normals")
                }
            }

            if use_tangents {
                if let Some(VertexAttributeValues::Float32x4(vert_tangets)) =
                    &mesh.attribute(Mesh::ATTRIBUTE_TANGENT)
                {
                    for t in vert_tangets {
                        tangets.push(*t);
                    }
                } else {
                    panic!("no tangets")
                }
            }

            if use_colors {
                if let Some(VertexAttributeValues::Float32x4(vert_colors)) =
                    &mesh.attribute(Mesh::ATTRIBUTE_COLOR)
                {
                    for c in vert_colors {
                        colors.push(*c);
                    }
                } else {
                    panic!("no colors")
                }
            }

            for i in mesh_indices {
                indices.push(*i + indices_offset);
            }
            indices_offset += positions_len as u32;
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    if use_normals {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }

    if use_tangents {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangets);
    }

    if use_uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }

    if use_colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    mesh.set_indices(Some(Indices::U32(indices)));

    mesh
}
