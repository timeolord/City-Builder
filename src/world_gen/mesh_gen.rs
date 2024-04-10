use bevy::{
    prelude::*,
    reflect::List,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    tasks::{block_on, ComputeTaskPool},
};

use itertools::Itertools;
use rand::{prelude::Rng, rngs::StdRng, SeedableRng};
use smooth_bevy_cameras::{controllers::orbit::OrbitCameraController, LookTransform};
use strum::IntoEnumIterator;

use crate::{
    assets::{get_terrain_texture_uv, TerrainTextureAtlas, TerrainType},
    utils::math::unnormalized_normal_array,
    world::WorldEntity,
    world_gen::{
        consts::{CHUNK_SIZE, CHUNK_WORLD_SIZE, LOD_LEVELS, TILE_WORLD_SIZE},
        heightmap::Heightmap,
    },
    GameState,
};

#[derive(Component)]
pub struct WorldMesh;
#[derive(Component)]
pub struct TreeMesh;
#[derive(Component)]
pub struct WaterMesh;
#[derive(Component)]
pub struct LODLevel(pub u32);
#[derive(Component)]
pub struct ChunkPosition(pub [u32; 2]);

use super::{
    consts::{SNOW_HEIGHT, TILE_SIZE, WORLD_HEIGHT_SCALE},
    WorldSettings,
};

pub fn level_of_detail(
    mut meshes: Query<(&LODLevel, &ChunkPosition, &mut Visibility)>,
    cameras: Query<(&OrbitCameraController, &mut LookTransform, &Transform)>,
) {
    let (_, transform, _) = cameras.iter().find(|c| c.0.enabled).expect("No camera");
    for (lod, chunk_position, mut visibility) in meshes.iter_mut() {
        //Convert chunk position to world position
        let chunk_position = [
            (chunk_position.0[0] as f32 * CHUNK_SIZE as f32) + CHUNK_SIZE as f32 / 2.0,
            (chunk_position.0[1] as f32 * CHUNK_SIZE as f32) + CHUNK_SIZE as f32 / 2.0,
        ];
        //let camera_position = transform.eye.xz();
        let camera_position = Vec2::new(
            (CHUNK_WORLD_SIZE[0] * CHUNK_SIZE) as f32 * 0.5,
            (CHUNK_WORLD_SIZE[1] * CHUNK_SIZE) as f32 * 0.5,
        );
        let distance = ((camera_position.distance(Vec2::new(chunk_position[0], chunk_position[1]))
            / CHUNK_SIZE as f32)
            .round() as u32)
            .clamp(1, LOD_LEVELS);
        //Show the correct LOD mesh based on distance
        if lod.0 != distance {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Visible;
        }
    }
}

#[derive(Resource, Default, Copy, Clone, Debug, Eq, PartialEq)]
pub struct ExtractedGameState(pub GameState);

pub fn generate_world_mesh(
    mut commands: Commands,
    world_mesh_query: Query<Entity, With<WorldMesh>>,
    heightmap: Res<Heightmap>,
    world_settings: Res<WorldSettings>,
    water_mesh: Query<Entity, With<WaterMesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    terrain_texture_atlas: Res<TerrainTextureAtlas>,
) {
    if world_mesh_query.is_empty() || heightmap.is_changed() {
        //Generate Water Mesh
        if water_mesh.is_empty() {
            commands
                .spawn(PbrBundle {
                    mesh: mesh_assets.add(
                        Plane3d::default()
                            .mesh()
                            .size(TILE_WORLD_SIZE[0] as f32, TILE_WORLD_SIZE[1] as f32),
                    ),
                    material: material_assets.add(Color::BLUE),
                    transform: Transform::from_translation(Vec3::new(
                        (TILE_WORLD_SIZE[0] as f32 / 2.0) - TILE_SIZE / 2.,
                        world_settings.water_level as f32,
                        (TILE_WORLD_SIZE[1] as f32 / 2.0) - TILE_SIZE / 2.,
                    )),
                    ..default()
                })
                .insert(WaterMesh);
        }

        let start_time = std::time::Instant::now();

        let random_number_generator = StdRng::seed_from_u64(world_settings.seed() as u64);

        //Despawn old meshes
        for entity in world_mesh_query.iter() {
            commands.entity(entity).despawn();
        }
        //Generate chunk meshes
        let thread_pool = ComputeTaskPool::get();
        let heightmap_ref = &heightmap;
        for lod in 1..=LOD_LEVELS as usize {
            let results = thread_pool.scope(|s| {
                for chunk_y in 0..CHUNK_WORLD_SIZE[1] {
                    for chunk_x in 0..CHUNK_WORLD_SIZE[0] {
                        let mut rng = random_number_generator.clone();
                        s.spawn(async move {
                            let mut grid_mesh = Mesh::new(
                                PrimitiveTopology::TriangleList,
                                RenderAssetUsages::RENDER_WORLD,
                            );
                            let mut vertices = Vec::new();
                            let mut uvs = Vec::new();
                            let mut indices = Vec::new();
                            let mut normals = Vec::new();
                            let mut indices_count = 0;

                            for y in (0..CHUNK_SIZE).step_by(lod * 2) {
                                for x in (0..CHUNK_SIZE).step_by(lod * 2) {
                                    let (new_vertices, uv, index, normal) = create_terrain_mesh(
                                        [(chunk_x * CHUNK_SIZE) + x, (chunk_y * CHUNK_SIZE) + y],
                                        heightmap_ref,
                                        &mut rng,
                                        indices_count,
                                        lod * 2,
                                    );
                                    indices_count += new_vertices.len() as u32;
                                    vertices.extend(new_vertices);
                                    uvs.extend(uv);
                                    indices.extend(index);
                                    normals.extend(normal);
                                }
                            }

                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

                            grid_mesh.insert_indices(Indices::U32(indices));

                            (grid_mesh, [chunk_x, chunk_y])
                        });
                    }
                }
            });
            for (mesh, position) in results {
                let mesh = mesh_assets.add(mesh);

                let material = terrain_texture_atlas.handle.clone();

                commands
                    .spawn(PbrBundle {
                        mesh,
                        material,
                        ..Default::default()
                    })
                    .insert(WorldMesh)
                    .insert(WorldEntity)
                    .insert(LODLevel(lod as u32))
                    .insert(ChunkPosition(position));
            }
        }
        //Generate Edge meshes
        let results = thread_pool.scope(|s| {
            for chunk_y in 0..CHUNK_WORLD_SIZE[1] {
                for chunk_x in 0..CHUNK_WORLD_SIZE[0] {
                    let mut x_offset = 0;
                    let mut y_offset = 0;
                    let x_direction = match chunk_x {
                        0 => Some(FaceDirection::East),
                        x if x == CHUNK_WORLD_SIZE[0] - 1 => {
                            x_offset = CHUNK_SIZE - 1;
                            Some(FaceDirection::West)
                        }
                        _ => None,
                    };
                    let y_direction = match chunk_y {
                        0 => Some(FaceDirection::South),
                        y if y == CHUNK_WORLD_SIZE[1] - 1 => {
                            y_offset = CHUNK_SIZE - 1;
                            Some(FaceDirection::North)
                        }
                        _ => None,
                    };
                    if let Some(direction) = x_direction {
                        s.spawn(async move {
                            let mut grid_mesh = Mesh::new(
                                PrimitiveTopology::TriangleList,
                                RenderAssetUsages::RENDER_WORLD,
                            );
                            let mut vertices = Vec::new();
                            let mut uvs = Vec::new();
                            let mut indices = Vec::new();
                            let mut normals = Vec::new();
                            let mut indices_count = 0;

                            for y in 0..CHUNK_SIZE {
                                let (new_vertices, uv, index, normal) = create_terrain_edge_mesh(
                                    [
                                        (chunk_x * CHUNK_SIZE) + x_offset,
                                        (chunk_y * CHUNK_SIZE) + y,
                                    ],
                                    heightmap_ref,
                                    direction,
                                    indices_count,
                                );
                                indices_count += new_vertices.len() as u32;
                                vertices.extend(new_vertices);
                                uvs.extend(uv);
                                indices.extend(index);
                                normals.extend(normal);
                            }

                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

                            grid_mesh.insert_indices(Indices::U32(indices));

                            grid_mesh
                        });
                    }
                    if let Some(direction) = y_direction {
                        s.spawn(async move {
                            let mut grid_mesh = Mesh::new(
                                PrimitiveTopology::TriangleList,
                                RenderAssetUsages::RENDER_WORLD,
                            );
                            let mut vertices = Vec::new();
                            let mut uvs = Vec::new();
                            let mut indices = Vec::new();
                            let mut normals = Vec::new();
                            let mut indices_count = 0;

                            for x in 0..CHUNK_SIZE {
                                let (new_vertices, uv, index, normal) = create_terrain_edge_mesh(
                                    [
                                        (chunk_x * CHUNK_SIZE) + x,
                                        (chunk_y * CHUNK_SIZE) + y_offset,
                                    ],
                                    heightmap_ref,
                                    direction,
                                    indices_count,
                                );
                                indices_count += new_vertices.len() as u32;
                                vertices.extend(new_vertices);
                                uvs.extend(uv);
                                indices.extend(index);
                                normals.extend(normal);
                            }

                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                            grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

                            grid_mesh.insert_indices(Indices::U32(indices));

                            grid_mesh
                        });
                    }
                }
            }
        });
        let edge_mesh = results.into_iter().reduce(|mut mesh_a, mesh_b| {
            mesh_a.merge(mesh_b);
            mesh_a
        });
        let mesh = mesh_assets.add(edge_mesh.unwrap());
        let material = terrain_texture_atlas.handle.clone();
        commands
            .spawn(PbrBundle {
                mesh,
                material,
                ..Default::default()
            })
            .insert(WorldMesh)
            .insert(WorldEntity);
        println!("World mesh generation took: {:?}", start_time.elapsed());
    }
}

type MeshVecs = (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceDirection {
    North,
    East,
    South,
    West,
}

fn create_terrain_edge_mesh(
    starting_position: [u32; 2],
    heightmap: &Heightmap,
    side: FaceDirection,
    indices_count: u32,
) -> MeshVecs {
    let mut vertices_ = Vec::new();
    let mut uvs_ = Vec::new();
    let mut indices_ = Vec::new();
    let mut normals_ = Vec::new();
    let mut indices_count = indices_count;
    let edge_depth = WORLD_HEIGHT_SCALE as u32 / 2;
    for current_depth in 0..edge_depth {
        let positions = match side {
            FaceDirection::North => [
                [starting_position[0], starting_position[1] + 1],
                [starting_position[0] + 1, starting_position[1] + 1],
            ],
            FaceDirection::East => [
                [starting_position[0], starting_position[1] + 1],
                starting_position,
            ],
            FaceDirection::South => [
                [starting_position[0], starting_position[1]],
                [starting_position[0] + 1, starting_position[1]],
            ],
            FaceDirection::West => [
                [starting_position[0] + 1, starting_position[1]],
                [starting_position[0] + 1, starting_position[1] + 1],
            ],
        };
        let tile_size = 0.5 * TILE_SIZE;

        let offsets = [-tile_size * TILE_SIZE, -tile_size * TILE_SIZE];
        let heights = [
            (heightmap[positions[0]] * WORLD_HEIGHT_SCALE) - (current_depth as f32 * TILE_SIZE),
            (heightmap[positions[1]] * WORLD_HEIGHT_SCALE) - (current_depth as f32 * TILE_SIZE),
        ];

        let vert_0 = [
            positions[0][0] as f32 + offsets[0],
            heights[0],
            positions[0][1] as f32 + offsets[1],
        ];
        let vert_1 = [
            positions[1][0] as f32 + offsets[0],
            heights[1],
            positions[1][1] as f32 + offsets[1],
        ];
        let vert_2 = [
            positions[1][0] as f32 + offsets[0],
            heights[1] - TILE_SIZE,
            positions[1][1] as f32 + offsets[1],
        ];
        let vert_3 = [
            positions[0][0] as f32 + offsets[0],
            heights[0] - TILE_SIZE,
            positions[0][1] as f32 + offsets[1],
        ];

        let vertices = vec![vert_0, vert_1, vert_2, vert_3];

        let indices = match side {
            FaceDirection::North => vec![
                indices_count + 2,
                indices_count + 1,
                indices_count,
                indices_count,
                indices_count + 3,
                indices_count + 2,
            ],
            FaceDirection::East | FaceDirection::South | FaceDirection::West => vec![
                indices_count,
                indices_count + 1,
                indices_count + 2,
                indices_count + 2,
                indices_count + 3,
                indices_count,
            ],
        };
        let normal = unnormalized_normal_array(vert_0, vert_3, vert_1)
            .normalize_or_zero()
            .to_array();
        let normals = match side {
            FaceDirection::South => {
                let normal = [-normal[0], -normal[1], -normal[2]];
                vec![normal, normal, normal, normal]
            }
            FaceDirection::East | FaceDirection::North | FaceDirection::West => {
                vec![normal, normal, normal, normal]
            }
        };

        let uv = get_terrain_texture_uv(TerrainType::Dirt).to_vec();
        indices_count += vertices.len() as u32;

        vertices_.extend(vertices);
        uvs_.extend(uv);
        indices_.extend(indices);
        normals_.extend(normals);
    }
    (vertices_, uvs_, indices_, normals_)
}

fn create_terrain_mesh(
    starting_position: [u32; 2],
    heightmap: &Heightmap,
    rng: &mut StdRng,
    indices_count: u32,
    lod: usize,
) -> MeshVecs {
    let tile_size = 0.5 * TILE_SIZE * lod as f32;
    let lod_offset = (lod - 1) as u32;
    let height = heightmap[starting_position] * WORLD_HEIGHT_SCALE;
    let mut average_height = height;
    let vert_0 = [
        starting_position[0] as f32 - tile_size * TILE_SIZE,
        height,
        starting_position[1] as f32 - tile_size * TILE_SIZE,
    ];
    let height = heightmap[[
        (starting_position[0] + 1 + lod_offset).clamp(0, heightmap.size()[0]),
        starting_position[1],
    ]] as f32
        * WORLD_HEIGHT_SCALE;
    average_height += height;
    let vert_1 = [
        starting_position[0] as f32 + tile_size * TILE_SIZE,
        height,
        starting_position[1] as f32 - tile_size * TILE_SIZE,
    ];
    let height = heightmap[[
        (starting_position[0] + 1 + lod_offset).clamp(0, heightmap.size()[0]),
        (starting_position[1] + 1 + lod_offset).clamp(0, heightmap.size()[1]),
    ]] as f32
        * WORLD_HEIGHT_SCALE;
    average_height += height;
    let vert_2 = [
        starting_position[0] as f32 + tile_size * TILE_SIZE,
        height,
        starting_position[1] as f32 + tile_size * TILE_SIZE,
    ];
    let height = heightmap[[
        starting_position[0],
        (starting_position[1] + 1 + lod_offset).clamp(0, heightmap.size()[1]),
    ]] as f32
        * WORLD_HEIGHT_SCALE;
    average_height += height;
    let vert_3 = [
        starting_position[0] as f32 - tile_size * TILE_SIZE,
        height,
        starting_position[1] as f32 + tile_size * TILE_SIZE,
    ];
    average_height /= 4.0;
    let vertices = vec![vert_0, vert_1, vert_2, vert_3];

    let indices = vec![
        indices_count + 2,
        indices_count + 1,
        indices_count,
        indices_count,
        indices_count + 3,
        indices_count + 2,
    ];
    let normal = unnormalized_normal_array(vert_0, vert_3, vert_1)
        .normalize()
        .to_array();
    let normals = vec![normal, normal, normal, normal];

    let steepness_angle = Into::<Vec3>::into(normal)
        .normalize()
        .dot(Vec3::new(0.0, 1.0, 0.0))
        .acos()
        .to_degrees();

    let terrain_type = get_terrain_type(average_height, steepness_angle, rng);

    let uv = get_terrain_texture_uv(terrain_type).to_vec();

    (vertices, uv, indices, normals)
}

fn get_terrain_type(height: f32, steepness_angle: f32, rng: &mut StdRng) -> TerrainType {
    let angle_variance = (steepness_angle * 0.1).max(0.1);
    let angle_noise = rng.gen_range(-angle_variance..angle_variance);
    let mut terrain_type = match steepness_angle + angle_noise {
        x if x < 40.0 => TerrainType::Grass,
        x if x < 60.0 => TerrainType::Dirt,
        x if x < 90.0 => TerrainType::Stone,
        _ => TerrainType::Sand,
    };
    //Snow
    let height_variance = (height * 0.1).max(0.1);
    let height_noise = rng.gen_range(-height_variance..height_variance);
    if height + height_noise > SNOW_HEIGHT {
        if terrain_type == TerrainType::Stone {
            let stone_to_snow_chance = 0.2;
            if stone_to_snow_chance > rng.gen_range(0.0..1.0) {
                terrain_type = TerrainType::Snow;
            }
        } else {
            terrain_type = TerrainType::Snow
        }
    }
    //Chance for dirt to become grass
    if terrain_type == TerrainType::Dirt {
        let dirt_to_grass_chance = 0.2;
        if dirt_to_grass_chance > rng.gen_range(0.0..1.0) {
            terrain_type = TerrainType::Grass;
        }
    }

    terrain_type
}

/* pub fn generate_tree_mesh(
    mut commands: Commands,
    tree_mesh_query: Query<Entity, With<WorldMesh>>,
    heightmap: Res<Heightmap>,
    world_settings: Res<WorldSettings>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    terrain_texture_atlas: Res<TerrainTextureAtlas>,
) {
    if tree_mesh_query.is_empty() || heightmap.is_changed() {
        let mut random_number_generator = StdRng::seed_from_u64(world_settings.seed() as u64);
        let world_size = world_settings.world_size;
        for entity in tree_mesh_query.iter() {
            commands.entity(entity).despawn();
        }

        for chunk_y in 0..world_size[0] {
            for chunk_x in 0..world_size[1] {
                let mut grid_mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                );

                let mut vertices = Vec::new();
                let mut uvs = Vec::new();
                let mut indices = Vec::new();
                let mut normals = Vec::new();

                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let starting_position =
                            [x + chunk_x * CHUNK_SIZE, y + chunk_y * CHUNK_SIZE];
                        let chance_for_tree = heightmap.tree_density(starting_position);
                        if chance_for_tree < random_number_generator.gen_range(0.0..1.0) {
                            let (new_vertices, uv, index, normal) = create_tree_mesh(
                                starting_position,
                                &heightmap,
                                indices.len() as u32,
                            );

                            vertices.extend(new_vertices);
                            uvs.extend(uv);
                            indices.extend(index);
                            normals.extend(normal);
                        }
                    }
                }

                grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

                grid_mesh.insert_indices(Indices::U32(indices));
                let mesh = mesh_assets.add(grid_mesh);

                let material = terrain_texture_atlas.handle.clone();

                commands
                    .spawn(PbrBundle {
                        mesh,
                        material,
                        ..Default::default()
                    })
                    .insert(TreeMesh)
                    .insert(WorldEntity);
            }
        }
    }
}
fn create_tree_mesh(
    starting_position: [u32; 2],
    heightmap: &Heightmap,
    current_index: u32,
) -> MeshVecs {
    let cylinder = shape::Cylinder {
        height: 1.0,
        radius: 0.1,
        resolution: 5,
        segments: 1,
        ..Default::default()
    };
    let mesh = Mesh::from(cylinder);
    let mut positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap()
        .to_vec();
    let height = heightmap[starting_position] as f32 * WORLD_HEIGHT_SCALE;

    positions.iter_mut().for_each(|pos| {
        pos[0] += starting_position[0] as f32;
        pos[1] += height;
        pos[2] += starting_position[1] as f32;
    });

    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap()
        .to_vec();

    let uvs = mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .unwrap()
        .get_bytes()
        .chunks_exact(4);
    let uvs = uvs
        .map(|uv| {
            let uv = f32::from_ne_bytes([uv[0], uv[1], uv[2], uv[3]]);
            [uv, uv]
        })
        .collect();

    let indices = mesh
        .indices()
        .unwrap()
        .iter()
        .map(|x| x as u32 + current_index)
        .collect_vec();

    (positions, uvs, indices, normals)
}

 */
