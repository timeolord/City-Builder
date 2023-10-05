pub mod camera;
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};

use crate::constants::TILE_SIZE;

use self::camera::CameraPlugin;

pub struct AssetBuilderPlugin;

impl Plugin for AssetBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CameraPlugin);
        app.add_systems(Startup, setup);
    }
}

const CHUNK_SIZE: u32 = 32;
const GRID_THICKNESS: f32 = 0.005;

fn create_grid_mesh() -> Mesh {
    fn create_attributes(
        starting_position: (f32, f32),
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
        let tile_size = 0.5 * TILE_SIZE;
        let vertices = vec![
            //Outside Square
            [
                starting_position.0 - tile_size * TILE_SIZE,
                0.0,
                starting_position.1 - tile_size * TILE_SIZE,
            ],
            [
                starting_position.0 + tile_size * TILE_SIZE,
                0.0,
                starting_position.1 - tile_size * TILE_SIZE,
            ],
            [
                starting_position.0 + tile_size * TILE_SIZE,
                0.0,
                starting_position.1 + tile_size * TILE_SIZE,
            ],
            [
                starting_position.0 - tile_size * TILE_SIZE,
                0.0,
                starting_position.1 + tile_size * TILE_SIZE,
            ],
            //Inside Square
            [
                starting_position.0 - tile_size + GRID_THICKNESS,
                0.0,
                starting_position.1 - tile_size * TILE_SIZE + GRID_THICKNESS,
            ],
            [
                starting_position.0 + tile_size - GRID_THICKNESS,
                0.0,
                starting_position.1 - tile_size * TILE_SIZE + GRID_THICKNESS,
            ],
            [
                starting_position.0 + tile_size - GRID_THICKNESS,
                0.0,
                starting_position.1 + tile_size * TILE_SIZE - GRID_THICKNESS,
            ],
            [
                starting_position.0 - tile_size + GRID_THICKNESS,
                0.0,
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
                create_attributes((x as f32 * TILE_SIZE, y as f32 * TILE_SIZE));
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

fn spawn_chunk(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    grid_material: Handle<StandardMaterial>,
    grid_mesh: Handle<Mesh>,
) {
    let plane = PbrBundle {
        mesh: mesh,
        material: material,
        ..default()
    };
    (*commands).spawn(plane.clone());

    let mut grid_transform = plane.transform;
    grid_transform.translation.y += 0.001;
    grid_transform.translation.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
    grid_transform.translation.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
    commands.spawn(PbrBundle {
        mesh: grid_mesh.clone(),
        material: grid_material.clone(),
        transform: grid_transform,
        ..default()
    });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let plane_mesh = meshes.add(shape::Plane::from_size(TILE_SIZE * CHUNK_SIZE as f32).into());
    let plane_material = materials.add(Color::rgb(0.3, 0.5, 0.3).into());
    let grid_mesh = meshes.add(create_grid_mesh());
    let grid_material = materials.add(Color::rgb(1.0, 1.0, 1.0).into());

    // plane
    spawn_chunk(
        &mut commands,
        plane_mesh,
        plane_material,
        grid_material,
        grid_mesh,
    );

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}
