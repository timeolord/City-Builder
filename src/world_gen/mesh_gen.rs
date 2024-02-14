use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};

use crate::{
    utils::math::unnormalized_normal_array, world::WorldEntity, world_gen::heightmap::Heightmap,
};
#[derive(Component)]
pub struct WorldMesh;

use super::{HeightmapLoadBar, WorldSettings, CHUNK_SIZE};

pub const TILE_SIZE: f32 = 1.0;
pub const WORLD_HEIGHT_SCALE: f32 = 200.0;

pub fn generate_world_mesh(
    mut commands: Commands,
    world_mesh_query: Query<Entity, With<WorldMesh>>,
    heightmap: Res<Heightmap>,
    heightmap_load_bar: Res<HeightmapLoadBar>,
    world_settings: Res<WorldSettings>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    if heightmap_load_bar.progress() >= 1.0 && (world_mesh_query.is_empty() || heightmap.is_changed())  {
        let world_size = world_settings.world_size;
        println!("{:?}", world_size);

        for chunk_y in 0..world_size[0] {
            for chunk_x in 0..world_size[1] {
                let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

                let mut vertices = Vec::new();
                let mut uvs = Vec::new();
                let mut indices = Vec::new();
                let mut normals = Vec::new();

                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let (new_vertices, uv, index, normal) = create_attributes(
                            [(chunk_x * CHUNK_SIZE) + x, (chunk_y * CHUNK_SIZE) + y],
                            &heightmap,
                        );
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
                let mesh = mesh_assets.add(grid_mesh);

                commands
                    .spawn(PbrBundle {
                        mesh,
                        ..Default::default()
                    })
                    .insert(WorldMesh)
                    .insert(WorldEntity);
            }
        }
    }
}

type MeshVecs = (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>);

fn create_attributes(starting_position: [u32; 2], heightmap: &Heightmap) -> MeshVecs {
    let tile_size = 0.5 * TILE_SIZE;
    let heights = heightmap[starting_position] as f32 * WORLD_HEIGHT_SCALE;
    let vert_0 = [
        starting_position[0] as f32 - tile_size * TILE_SIZE,
        heights,
        starting_position[1] as f32 - tile_size * TILE_SIZE,
    ];
    let vert_1 = [
        starting_position[0] as f32 + tile_size * TILE_SIZE,
        heights,
        starting_position[1] as f32 - tile_size * TILE_SIZE,
    ];
    let vert_2 = [
        starting_position[0] as f32 + tile_size * TILE_SIZE,
        heights,
        starting_position[1] as f32 + tile_size * TILE_SIZE,
    ];
    let vert_3 = [
        starting_position[0] as f32 - tile_size * TILE_SIZE,
        heights,
        starting_position[1] as f32 + tile_size * TILE_SIZE,
    ];
    let vertices = vec![vert_0, vert_1, vert_2, vert_3];
    let uv_0 = [-1.0, -1.0];
    let uv_1 = [1.0, -1.0];
    let uv_2 = [1.0, 1.0];
    let uv_3 = [-1.0, 1.0];
    let uv = vec![uv_0, uv_1, uv_2, uv_3];
    let indices_count = ((starting_position[0] + starting_position[1] * CHUNK_SIZE)
        * vertices.len() as u32)
        % (CHUNK_SIZE * CHUNK_SIZE * vertices.len() as u32);
    let indices = vec![
        indices_count + 2,
        indices_count + 1,
        indices_count,
        indices_count,
        indices_count + 3,
        indices_count + 2,
    ];
    let normal_a = unnormalized_normal_array(vert_0, vert_3, vert_1)
        .normalize()
        .to_array();
    let normals = vec![normal_a, normal_a, normal_a, normal_a];
    (vertices, uv, indices, normals)
}

pub fn create_chunk_mesh(heightmap: &Heightmap) -> Mesh {
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    for y in 0..CHUNK_SIZE as u32 {
        for x in 0..CHUNK_SIZE as u32 {
            let (new_vertices, uv, index, normal) = create_attributes([x, y], heightmap);
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
