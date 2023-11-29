use bevy::{
    math::{IVec2, Mat3, Vec3, Vec4Swizzles},
    render::{
        mesh::{Indices, Mesh, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    transform::components::Transform,
};

use crate::{
    chunk::chunk_tile_position::TilePosition2D,
    constants::{CHUNK_SIZE, GRID_THICKNESS, TILE_SIZE},
    math_utils::unnormalized_normal_vector,
    world::heightmap::{Heightmap, HeightmapVertex},
};

pub fn create_plane_mesh(heights: HeightmapVertex, height_offset: f32) -> Mesh {
    let tile_size = 0.5 * TILE_SIZE;
    let vert_0 = [-tile_size, heights[0] + height_offset, -tile_size];
    let vert_1 = [tile_size, heights[1] + height_offset, -tile_size];
    let vert_2 = [tile_size, heights[2] + height_offset, tile_size];
    let vert_3 = [-tile_size, heights[3] + height_offset, tile_size];
    let vert_4 = [0.0, heights[3], 0.0];
    let vertices = vec![
        vert_0, vert_1, vert_4, vert_1, vert_2, vert_4, vert_2, vert_3, vert_4, vert_3, vert_0,
        vert_4,
    ];
    let uv_0 = [-1.0, -1.0];
    let uv_1 = [1.0, -1.0];
    let uv_2 = [1.0, 1.0];
    let uv_3 = [-1.0, 1.0];
    let uv_4 = [0.0, 0.0];
    let uvs = vec![
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
        normal_a, normal_a, normal_a, normal_b, normal_b, normal_b, normal_c, normal_c, normal_c,
        normal_d, normal_d, normal_d,
    ];
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    grid_mesh.set_indices(Some(Indices::U32(indices)));

    grid_mesh
}

pub fn create_box_mesh(heights: HeightmapVertex, height_offset: f32) -> Mesh {
    let tile_size = 0.5 * TILE_SIZE;
    //Top Face
    let vert_0 = [-tile_size, heights[0] + height_offset, -tile_size];
    let vert_1 = [tile_size, heights[1] + height_offset, -tile_size];
    let vert_2 = [tile_size, heights[2] + height_offset, tile_size];
    let vert_3 = [-tile_size, heights[3] + height_offset, tile_size];
    let vert_4 = [0.0, heights[3] + height_offset, 0.0];
    //Bottom Face
    let vert_5 = [-tile_size, heights[0] - 1.0, -tile_size];
    let vert_6 = [tile_size, heights[1] - 1.0, -tile_size];
    let vert_7 = [tile_size, heights[2] - 1.0, tile_size];
    let vert_8 = [-tile_size, heights[3] - 1.0, tile_size];
    let vert_9 = [0.0, heights[3] - 1.0, 0.0];

    let vertices = vec![
        vert_5, vert_6, vert_7, vert_8, vert_9, //Bottom Face
        vert_0, vert_1, vert_4, vert_1, vert_2, vert_4, vert_2, vert_3, vert_4, vert_3, vert_0,
        vert_4, //Top Face
    ];
    let uv_0 = [0.0, 0.0];
    let uv_1 = [1.0, 0.0];
    let uv_2 = [1.0, 1.0];
    let uv_3 = [0.0, 1.0];
    let uv_4 = [0.5, 0.5];
    //TODO make uvs work for each side?
    let uvs = vec![
        uv_0, uv_1, uv_2, uv_3, uv_4, //Bottom Face
        uv_0, uv_1, uv_4, uv_1, uv_2, uv_4, uv_2, uv_3, uv_4, uv_3, uv_0, uv_4, //Top Face
    ];
    let indices = vec![
        //5, 6, 8, 8, 6, 7, //Bottom Face
        0, 1, 3, 3, 1, 2, //Bottom Face
        //0, 1, 5, 5, 1, 6, //Front Face
        5, 6, 0, 0, 6, 1, //Front Face
        //3, 0, 8, 8, 0, 5, //Left Face
        12, 5, 3, 3, 5, 0, //Left Face
        //2, 1, 7, 7, 1, 6, //Right Face
        //9, 6, 2, 2, 6, 1, //Right Face
        2, 6, 9, 1, 6, 2, //Right Face
        //3, 2, 8, 8, 2, 7, //Back Face
        //12, 9, 3, 3, 9, 2, //Back Face
        3, 9, 12, 2, 9, 3, //Back Face
        //2, 1, 0, 3, 5, 4, 6, 8, 7, 10, 9, 11, //Top Face
        7, 6, 5, 8, 10, 9, 11, 13, 12, 15, 14, 16, //Top Face
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

    let normals = vec![
        [0., -1.0, 0.],
        [0., -1.0, 0.],
        [0., -1.0, 0.],
        [0., -1.0, 0.],
        [0., -1.0, 0.], //Bottom Face
        normal_a,
        normal_a,
        normal_a,
        normal_b,
        normal_b,
        normal_b,
        normal_c,
        normal_c,
        normal_c,
        normal_d,
        normal_d,
        normal_d, //Top Face
    ];
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    grid_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

    grid_mesh.set_indices(Some(Indices::U32(indices)));

    grid_mesh
}

pub fn create_chunk_mesh(heightmap: &Heightmap) -> Mesh {
    fn create_attributes(
        starting_position: TilePosition2D,
        heightmap: &Heightmap,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>) {
        let chunk_offset = ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        let tile_size = 0.5 * TILE_SIZE;
        let heights = heightmap[starting_position];
        let vert_0 = [
            starting_position.x as f32 - chunk_offset - tile_size * TILE_SIZE,
            heights[0],
            starting_position.y as f32 - chunk_offset - tile_size * TILE_SIZE,
        ];
        let vert_1 = [
            starting_position.x as f32 - chunk_offset + tile_size * TILE_SIZE,
            heights[1],
            starting_position.y as f32 - chunk_offset - tile_size * TILE_SIZE,
        ];
        let vert_2 = [
            starting_position.x as f32 - chunk_offset + tile_size * TILE_SIZE,
            heights[2],
            starting_position.y as f32 - chunk_offset + tile_size * TILE_SIZE,
        ];
        let vert_3 = [
            starting_position.x as f32 - chunk_offset - tile_size * TILE_SIZE,
            heights[3],
            starting_position.y as f32 - chunk_offset + tile_size * TILE_SIZE,
        ];
        let vertices = vec![vert_0, vert_1, vert_2, vert_3];
        let uv_0 = [-1.0, -1.0];
        let uv_1 = [1.0, -1.0];
        let uv_2 = [1.0, 1.0];
        let uv_3 = [-1.0, 1.0];
        let uv = vec![uv_0, uv_1, uv_2, uv_3];
        let indices_count = ((starting_position.x + starting_position.y * CHUNK_SIZE as i32)
            * vertices.len() as i32) as u32;
        let indices = vec![
            indices_count + 2,
            indices_count + 1,
            indices_count + 0,
            indices_count + 0,
            indices_count + 3,
            indices_count + 2,
        ];
        let normal_a = unnormalized_normal_vector(vert_0, vert_3, vert_1)
            .normalize()
            .to_array();
        let normals = vec![normal_a, normal_a, normal_a, normal_a];
        (vertices, uv, indices, normals)
    }
    let mut grid_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    for y in 0..CHUNK_SIZE as i32 {
        for x in 0..CHUNK_SIZE as i32 {
            let (new_vertices, uv, index, normal) = create_attributes(
                IVec2::new(x * TILE_SIZE as i32, y * TILE_SIZE as i32),
                heightmap,
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

    grid_mesh
}

pub fn create_grid_mesh(heightmap: &Heightmap) -> Mesh {
    fn create_attributes(
        starting_position: TilePosition2D,
        heightmap: &Heightmap,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
        let heights = heightmap[starting_position];
        let tile_size = 0.5 * TILE_SIZE;
        let starting_position = starting_position.as_vec2();
        let vertices = vec![
            //Outside Square
            [
                starting_position.x - tile_size * TILE_SIZE,
                heights[0] as f32,
                starting_position.y - tile_size * TILE_SIZE,
            ],
            [
                starting_position.x + tile_size * TILE_SIZE,
                heights[1] as f32,
                starting_position.y - tile_size * TILE_SIZE,
            ],
            [
                starting_position.x + tile_size * TILE_SIZE,
                heights[2] as f32,
                starting_position.y + tile_size * TILE_SIZE,
            ],
            [
                starting_position.x - tile_size * TILE_SIZE,
                heights[3] as f32,
                starting_position.y + tile_size * TILE_SIZE,
            ],
            //Inside Square
            [
                starting_position.x - tile_size + GRID_THICKNESS,
                heights[0] as f32,
                starting_position.y - tile_size * TILE_SIZE + GRID_THICKNESS,
            ],
            [
                starting_position.x + tile_size - GRID_THICKNESS,
                heights[1] as f32,
                starting_position.y - tile_size * TILE_SIZE + GRID_THICKNESS,
            ],
            [
                starting_position.x + tile_size - GRID_THICKNESS,
                heights[2] as f32,
                starting_position.y + tile_size * TILE_SIZE - GRID_THICKNESS,
            ],
            [
                starting_position.x - tile_size + GRID_THICKNESS,
                heights[3] as f32,
                starting_position.y + tile_size * TILE_SIZE - GRID_THICKNESS,
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
            ((starting_position.x + starting_position.y * CHUNK_SIZE as f32) * 8.0) as u32;
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
    for x in 0..CHUNK_SIZE as i32 {
        for y in 0..CHUNK_SIZE as i32 {
            let (new_vertices, uv, index) =
                create_attributes(IVec2::new(x * TILE_SIZE as i32, y * TILE_SIZE as i32), heightmap);
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

pub fn combine_meshes(
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
