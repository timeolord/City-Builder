use array2d::Array2D;
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_mod_raycast::prelude::*;

use crate::{
    constants::{CHUNK_SIZE, GRID_THICKNESS, TILE_SIZE},
    cursor::RaycastSet,
    world::heightmap_generator::Heightmap,
};

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

#[derive(Resource)]
pub struct ChunkResource {
    plane_mesh: Handle<Mesh>,
    plane_material: Handle<StandardMaterial>,
    grid_mesh: Handle<Mesh>,
    grid_material: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let plane_mesh = meshes.add(shape::Plane::from_size(TILE_SIZE * CHUNK_SIZE as f32).into());
    let mut material: StandardMaterial = Color::rgb(0.3, 0.5, 0.3).into();
    material.perceptual_roughness = 1.0;
    material.reflectance = 0.0;
    let plane_material = materials.add(material);
    let grid_mesh = meshes.add(create_grid_mesh(None));
    let grid_material = materials.add(Color::rgb(1.0, 1.0, 1.0).into());

    commands.insert_resource(ChunkResource {
        plane_mesh,
        plane_material,
        grid_mesh,
        grid_material,
    })
}

pub fn create_grid_mesh(heightmap: Option<&Heightmap>) -> Mesh {
    fn create_attributes(
        starting_position: (f32, f32),
        heightmap: Option<&Heightmap>,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
        let heights = match heightmap {
            Some(heightmap) => {
                heightmap[(starting_position.0 as usize, starting_position.1 as usize)]
            }
            None => [0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let tile_size = 0.5 * TILE_SIZE;
        let vertices = vec![
            //Outside Square
            [
                starting_position.0 - tile_size * TILE_SIZE,
                heights[0] as f32,
                starting_position.1 - tile_size * TILE_SIZE,
            ],
            [
                starting_position.0 + tile_size * TILE_SIZE,
                heights[1] as f32,
                starting_position.1 - tile_size * TILE_SIZE,
            ],
            [
                starting_position.0 + tile_size * TILE_SIZE,
                heights[2] as f32,
                starting_position.1 + tile_size * TILE_SIZE,
            ],
            [
                starting_position.0 - tile_size * TILE_SIZE,
                heights[3] as f32,
                starting_position.1 + tile_size * TILE_SIZE,
            ],
            //Inside Square
            [
                starting_position.0 - tile_size + GRID_THICKNESS,
                heights[0] as f32,
                starting_position.1 - tile_size * TILE_SIZE + GRID_THICKNESS,
            ],
            [
                starting_position.0 + tile_size - GRID_THICKNESS,
                heights[1] as f32,
                starting_position.1 - tile_size * TILE_SIZE + GRID_THICKNESS,
            ],
            [
                starting_position.0 + tile_size - GRID_THICKNESS,
                heights[2] as f32,
                starting_position.1 + tile_size * TILE_SIZE - GRID_THICKNESS,
            ],
            [
                starting_position.0 - tile_size + GRID_THICKNESS,
                heights[3] as f32,
                starting_position.1 + tile_size * TILE_SIZE - GRID_THICKNESS,
            ],
        ];
        let uv = vec![
            [-1.0, -1.0],
            [1.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            //Inside Square
            [-1.0 + GRID_THICKNESS, -1.0 + GRID_THICKNESS],
            [1.0 - GRID_THICKNESS, -1.0 + GRID_THICKNESS],
            [1.0 - GRID_THICKNESS, 1.0 - GRID_THICKNESS],
            [-1.0 + GRID_THICKNESS, 1.0 - GRID_THICKNESS],
        ];
        let indices_count =
            ((starting_position.0 + starting_position.1 * CHUNK_SIZE as f32) * 8.0) as u32;
        let indices = vec![
            indices_count + 0,
            indices_count + 4,
            indices_count + 1,
            indices_count + 1,
            indices_count + 4,
            indices_count + 5, //Top
            indices_count + 1,
            indices_count + 5,
            indices_count + 2,
            indices_count + 2,
            indices_count + 5,
            indices_count + 6, //Right
            indices_count + 2,
            indices_count + 6,
            indices_count + 3,
            indices_count + 3,
            indices_count + 6,
            indices_count + 7, //Bottom
            indices_count + 3,
            indices_count + 7,
            indices_count + 0,
            indices_count + 0,
            indices_count + 7,
            indices_count + 4, //Left
        ];
        (vertices, uv, indices)
    }

    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            let (new_vertices, uv, index) =
                create_attributes((x as f32 * TILE_SIZE, y as f32 * TILE_SIZE), heightmap);
            vertices.extend(new_vertices);
            uvs.extend(uv);
            indices.extend(index);
        }
    }

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    grid_mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 1.0, 0.0]; vertices.len()],
    );
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    grid_mesh.set_indices(Some(Indices::U32(indices)));

    grid_mesh
}
fn create_plane_mesh(heightmap: &Heightmap) -> Mesh {
    fn create_attributes(
        starting_position: (usize, usize),
        heightmap: &Heightmap,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>) {
        let chunk_offset = ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        let tile_size = 0.5 * TILE_SIZE;
        let heights = heightmap[(starting_position.0, starting_position.1)];
        /* let middle_height =
        if heights[0] == heights[1] && heights[1] == heights[2] && heights[2] == heights[3] {
            heights[0]
        } else {
            heights[4].clamp(*heights[0..3].iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(), *heights[0..3].iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap())
        }; */
        let vert_0 = [
            starting_position.0 as f32 - chunk_offset - tile_size * TILE_SIZE,
            heights[0],
            starting_position.1 as f32 - chunk_offset - tile_size * TILE_SIZE,
        ];
        let vert_1 = [
            starting_position.0 as f32 - chunk_offset + tile_size * TILE_SIZE,
            heights[1],
            starting_position.1 as f32 - chunk_offset - tile_size * TILE_SIZE,
        ];
        let vert_2 = [
            starting_position.0 as f32 - chunk_offset + tile_size * TILE_SIZE,
            heights[2],
            starting_position.1 as f32 - chunk_offset + tile_size * TILE_SIZE,
        ];
        let vert_3 = [
            starting_position.0 as f32 - chunk_offset - tile_size * TILE_SIZE,
            heights[3],
            starting_position.1 as f32 - chunk_offset + tile_size * TILE_SIZE,
        ];
        let vert_4 = [
            starting_position.0 as f32 - chunk_offset * TILE_SIZE,
            heights[4],
            starting_position.1 as f32 - chunk_offset * TILE_SIZE,
        ];
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
        let indices_count = ((starting_position.0 + starting_position.1 * CHUNK_SIZE as usize)
            * vertices.len()) as u32;
        let indices = vec![
            indices_count + 2,
            indices_count + 1,
            indices_count + 0,
            indices_count + 3,
            indices_count + 5,
            indices_count + 4,
            indices_count + 6,
            indices_count + 8,
            indices_count + 7,
            indices_count + 10,
            indices_count + 9,
            indices_count + 11,
        ];
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

        let zero = [0.0, 1.0, 0.0];
        let normals = vec![
            normal_a, normal_a, normal_a, normal_b, normal_b, normal_b, normal_c, normal_c,
            normal_c, normal_d, normal_d, normal_d,
        ];
        //let normals = vec![[0.0, 1.0, 0.0]; vertices.len()];
        (vertices, uv, indices, normals)
    }
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    for y in 0..CHUNK_SIZE as usize {
        for x in 0..CHUNK_SIZE as usize {
            let (new_vertices, uv, index, normal) =
                create_attributes((x * TILE_SIZE as usize, y * TILE_SIZE as usize), heightmap);
            vertices.extend(new_vertices);
            uvs.extend(uv);
            indices.extend(index);
            normals.extend(normal);
        }
    }

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    grid_mesh.set_indices(Some(Indices::U32(indices)));

    grid_mesh
}
#[derive(Component)]
pub struct Grid;

#[derive(Component)]
pub struct Chunk {
    pub position: (usize, usize),
}
#[derive(Bundle)]
pub struct ChunkBundle {
    pub terrain_pbr: PbrBundle,
    pub chunk: Chunk,
    pub raycast_mesh: RaycastMesh<RaycastSet>,
}

#[derive(Bundle)]
pub struct GridBundle {
    pub grid_pbr: PbrBundle,
    pub grid: Grid,
}

impl ChunkBundle {
    fn mesh(self) -> Handle<Mesh> {
        self.terrain_pbr.mesh
    }
}

pub fn spawn_chunk<T: bevy::prelude::Bundle>(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    starting_position: (f32, f32),
    chunk_resources: &ChunkResource,
    tag: T,
    heightmap: Option<&Heightmap>,
) {
    let mesh = match heightmap {
        Some(heightmap) => meshes.add(create_plane_mesh(heightmap)),
        None => chunk_resources.plane_mesh.clone(),
    };
    let material = chunk_resources.plane_material.clone();
    let grid_material = chunk_resources.grid_material.clone();
    let grid_mesh = match heightmap {
        Some(heightmap) => meshes.add(create_grid_mesh(Some(heightmap))),
        None => chunk_resources.grid_mesh.clone(),
    };

    let chunk_pbr = PbrBundle {
        mesh: mesh,
        material: material,
        transform: Transform::from_xyz(starting_position.0, 0.0, starting_position.1),
        ..default()
    };

    let mut grid_transform = Transform::from_xyz(0.0, 0.0, 0.0);
    grid_transform.translation.y += 0.01;
    grid_transform.translation.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
    grid_transform.translation.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;

    let grid_pbr = PbrBundle {
        mesh: grid_mesh,
        material: grid_material,
        //global_transform: GlobalTransform::from(grid_transform),
        transform: grid_transform,
        ..default()
    };

    let chunk_bundle = ChunkBundle {
        terrain_pbr: chunk_pbr,
        chunk: Chunk {
            position: (starting_position.0 as usize, starting_position.1 as usize),
        },
        raycast_mesh: RaycastMesh::<RaycastSet>::default(),
    };

    commands.spawn((chunk_bundle, tag)).with_children(|parent| {
        parent.spawn(GridBundle {
            grid_pbr,
            grid: Grid,
        });
    });
}

fn unnormalized_normal_vector(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
    (Vec3::from_array(b) - Vec3::from_array(a)).cross(Vec3::from_array(c) - Vec3::from_array(a))
}

fn average_vectors<const N: usize>(list: [Vec3; N]) -> Vec3 {
    let mut sum = Vec3::new(0.0, 0.0, 0.0);
    for vector in list {
        sum += vector;
    }
    sum / N as f32
}
