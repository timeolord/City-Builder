use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
};
use line_drawing::WalkGrid;

use crate::{
    chunk::{unnormalized_normal_vector, ChunkPosition, ChunkTilePosition, TilePosition2D},
    constants::TILE_SIZE,
    cursor::CurrentTile,
    world::heightmap_generator::Heightmap,
    GameState,
};

use super::{
    terraform::EditTileEvent,
    tile_highlight::HighlightTileEvent,
    tools::{CurrentTool, ToolType},
};

pub struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (road_tool, create_road_mesh, spawn_road_event_handler)
                .chain()
                .run_if(in_state(GameState::World)),
        );
        app.add_event::<SpawnRoadEvent>();
    }
}

#[derive(Event)]
pub struct SpawnRoadEvent {
    pub road: Road,
    mesh: Option<Handle<Mesh>>,
}

impl SpawnRoadEvent {
    pub fn new(road: Road) -> Self {
        Self { road, mesh: None }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Road {
    //Starting_position < Ending_position
    pub starting_position: ChunkTilePosition,
    pub ending_position: ChunkTilePosition,
    pub width: usize,
}

#[derive(Bundle)]
pub struct RoadBundle {
    pub road: Road,
    pub pbr: PbrBundle,
}

fn road_tool(
    current_tile: Res<CurrentTile>,
    mut spawn_road_events: EventWriter<SpawnRoadEvent>,
    mut highlight_tile_events: EventWriter<HighlightTileEvent>,
    mut current_tool: ResMut<CurrentTool>,
    mouse_button: Res<Input<MouseButton>>,
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
            let road = Road {
                starting_position: current_tool.starting_point.unwrap(),
                ending_position: current_tile.position,
                width: 1,
            };
            let road_tiles = calculate_road_tiles(&road);
            for road_tile in road_tiles {
                highlight_tile_events.send(HighlightTileEvent {
                    position: road_tile,
                    color: Color::YELLOW_GREEN,
                });
            }

            if mouse_button.just_pressed(MouseButton::Left) {
                current_tool.ending_point = Some(current_tile.position);
                spawn_road_events.send(SpawnRoadEvent::new(Road {
                    starting_position: current_tool.starting_point.unwrap(),
                    ending_position: current_tool.ending_point.unwrap(),
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
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for spawn_road_event in spawn_road_events.read() {
        ////Check if the wall is straight or diagonal
        let road = &spawn_road_event.road;
        //let starting_position = road.starting_position.as_tile_position();
        //let ending_position = road.ending_position.as_tile_position();
        //let delta_x = ending_position[0] - starting_position[0];
        //let delta_z = ending_position[2] - starting_position[2];
        //let mut slope = 0.0;
        //if delta_x != 0 {
        //    slope = delta_z as f32 / delta_x as f32;
        //}
        //if slope != 0.0 && slope != 1.0 && slope != -1.0 {
        //    return;
        //}

        match spawn_road_event.mesh {
            Some(ref mesh) => {
                let mut material: StandardMaterial = Color::rgb(0.0, 0.0, 0.0).into();
                material.perceptual_roughness = 1.0;
                material.reflectance = 0.0;
                commands.spawn(RoadBundle {
                    road: road.clone(),
                    pbr: PbrBundle {
                        mesh: mesh.clone(),
                        material: materials.add(material),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                        ..default()
                    },
                });
            }
            None => {}
        }
    }
}

fn create_road_mesh(
    mut spawn_road_events: ResMut<Events<SpawnRoadEvent>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    heightmap_query: Query<(&Heightmap, &ChunkPosition)>,
    mut edit_tile_events: EventWriter<EditTileEvent>,
) {
    let mut event_vec = Vec::new();
    for spawn_road_event in spawn_road_events.drain() {
        let road = &spawn_road_event.road;
        match spawn_road_event.mesh {
            Some(_) => {}
            None => {
                let road_tiles = calculate_road_tiles(road);
                let mut meshes: Vec<Mesh> = Vec::new();
                let mut transforms = Vec::new();
                for road_tile in road_tiles {
                    let heightmap = heightmap_query
                        .iter()
                        .find(|(_, chunk_position)| **chunk_position == road_tile.chunk_position)
                        .unwrap()
                        .0;

                    let mesh = create_plane_mesh(road_tile.tile_position_2d(), heightmap);
                    let transform = Transform::from_translation(road_tile.to_world_position());

                    meshes.push(mesh);
                    transforms.push(transform);

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
                let mesh_handle = mesh_assets.add(mesh.into());
                event_vec.push(SpawnRoadEvent {
                    road: road.clone(),
                    mesh: Some(mesh_handle),
                });
            }
        }
    }
    spawn_road_events.extend(event_vec.into_iter());
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

//Calculate each tile needed for the road from the starting and ending positions
fn calculate_road_tiles(road: &Road) -> Vec<ChunkTilePosition> {
    let mut tiles = Vec::new();
    let starting_position = road.starting_position.as_tile_position();
    let ending_position = road.ending_position.as_tile_position();
    for (x, y) in WalkGrid::new(
        (starting_position[0] as isize, starting_position[2] as isize),
        (ending_position[0] as isize, ending_position[2] as isize),
    ) {
        tiles.push([x as usize, y as usize]);
    }
    tiles.push([ending_position[0], ending_position[2]]);

    tiles
        .into_iter()
        .map(|tile| ChunkTilePosition::from_tile_position([tile[0], 0, tile[1]]))
        .collect()
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
