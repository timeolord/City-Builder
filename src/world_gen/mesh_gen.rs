use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_mod_raycast::deferred::RaycastMesh;
use rand::{prelude::Rng, rngs::StdRng, SeedableRng};

use crate::{
    assets::{get_terrain_texture_uv, TerrainTextureAtlas, TerrainType},
    camera::CameraRaycastSet,
    utils::math::unnormalized_normal_array,
    world::WorldEntity,
    world_gen::heightmap::Heightmap,
};
#[derive(Component)]
pub struct WorldMesh;

use super::{WorldSettings, CHUNK_SIZE};

pub const TILE_SIZE: f32 = 1.0;
pub const WORLD_HEIGHT_SCALE: f32 = 200.0;

pub fn generate_world_mesh(
    mut commands: Commands,
    world_mesh_query: Query<Entity, With<WorldMesh>>,
    heightmap: Res<Heightmap>,
    world_settings: Res<WorldSettings>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    terrain_texture_atlas: Res<TerrainTextureAtlas>,
) {
    if world_mesh_query.is_empty() || heightmap.is_changed() {
        let mut random_number_generator = StdRng::seed_from_u64(world_settings.seed() as u64);
        let world_size = world_settings.world_size;
        for entity in world_mesh_query.iter() {
            commands.entity(entity).despawn();
        }

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
                            &mut random_number_generator,
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

                let material = terrain_texture_atlas.handle.clone();

                commands
                    .spawn(PbrBundle {
                        mesh,
                        material,
                        ..Default::default()
                    })
                    .insert(WorldMesh)
                    .insert(WorldEntity)
                    .insert(RaycastMesh::<CameraRaycastSet>::default());
            }
        }
    }
}

type MeshVecs = (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>, Vec<[f32; 3]>);

fn create_attributes(
    starting_position: [u32; 2],
    heightmap: &Heightmap,
    rng: &mut StdRng,
) -> MeshVecs {
    let tile_size = 0.5 * TILE_SIZE;
    let height = heightmap[starting_position] as f32 * WORLD_HEIGHT_SCALE;
    let mut average_height = height;
    let vert_0 = [
        starting_position[0] as f32 - tile_size * TILE_SIZE,
        height,
        starting_position[1] as f32 - tile_size * TILE_SIZE,
    ];
    let height = heightmap[[
        (starting_position[0] + 1).clamp(0, heightmap.size()[0]),
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
        (starting_position[0] + 1).clamp(0, heightmap.size()[0]),
        (starting_position[1] + 1).clamp(0, heightmap.size()[1]),
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
        (starting_position[1] + 1).clamp(0, heightmap.size()[1]),
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

    let steepness_angle = Into::<Vec3>::into(normal_a)
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
    if height + height_noise > WORLD_HEIGHT_SCALE * 0.5 {
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
