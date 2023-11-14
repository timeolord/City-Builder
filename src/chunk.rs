use std::ops::{Index, IndexMut};

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_mod_raycast::prelude::*;

use crate::{
    constants::{CHUNK_SIZE, GRID_THICKNESS, TILE_SIZE},
    cursor::RaycastSet,
    world::{heightmap_generator::Heightmap, WorldSettings},
    GameState,
};

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnChunkEvent>();
        app.add_event::<DespawnEntityEvent>();
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            spawn_chunk_event_handler
                .run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
        app.add_systems(
            PostUpdate,
            despawn_entity_event_handler
                .run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
    }
}

#[derive(Resource)]
pub struct ChunkResource {
    plane_mesh: Handle<Mesh>,
    plane_material: Handle<StandardMaterial>,
    grid_mesh: Handle<Mesh>,
    grid_material: Handle<StandardMaterial>,
}
#[derive(Event)]
pub struct SpawnChunkEvent {
    pub position: ChunkPosition,
    pub heightmap: Option<Heightmap>,
}
#[derive(Event)]
pub struct DespawnEntityEvent {
    pub entity: Entity,
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

fn create_grid_mesh(heightmap: Option<&Heightmap>) -> Mesh {
    fn create_attributes(
        starting_position: (f32, f32),
        heightmap: Option<&Heightmap>,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
        let heights = match heightmap {
            Some(heightmap) => {
                heightmap[[starting_position.0 as usize, starting_position.1 as usize]]
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
        let heights = heightmap[[starting_position.0, starting_position.1]];
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
pub struct Chunk;

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPosition {
    pub position: [usize; 2],
}
impl Index<usize> for ChunkPosition {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.position[index]
    }
}
impl IndexMut<usize> for ChunkPosition {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.position[index]
    }
}

#[derive(Bundle)]
pub struct ChunkBundle {
    pub pbr: PbrBundle,
    pub chunk_tag: Chunk,
    pub chunk_position: ChunkPosition,
    pub raycast_mesh: RaycastMesh<RaycastSet>,
    pub heightmap: Heightmap,
}

#[derive(Bundle)]
pub struct GridBundle {
    pub grid_pbr: PbrBundle,
    pub grid: Grid,
}

fn despawn_entity_event_handler(
    mut despawn_entity_events: EventReader<DespawnEntityEvent>,
    mut commands: Commands,
) {
    for despawn_entity_event in despawn_entity_events.read() {
        commands
            .entity(despawn_entity_event.entity)
            .despawn_recursive();
    }
}

fn spawn_chunk_event_handler(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut spawn_chunk_events: EventReader<SpawnChunkEvent>,
    mut despawn_entity_events: EventWriter<DespawnEntityEvent>,
    chunk_resources: Res<ChunkResource>,
    world_settings: Res<WorldSettings>,
    chunks: Query<(Entity, &ChunkPosition)>,
) {
    for spawn_chunk_event in spawn_chunk_events.read() {
        let current_chunk_id: Option<(Entity, &ChunkPosition)> = chunks
            .iter()
            .find(|(_, chunk)| **chunk == spawn_chunk_event.position);
        match current_chunk_id {
            Some((current_chunk_id, _)) => {
                despawn_entity_events.send(DespawnEntityEvent {
                    entity: current_chunk_id,
                });
            }
            None => {}
        }
        let heightmap = &spawn_chunk_event.heightmap;
        let starting_position = spawn_chunk_event.position;

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
            transform: Transform::from_xyz(
                (starting_position[0] * CHUNK_SIZE) as f32,
                0.0,
                (starting_position[1] * CHUNK_SIZE) as f32,
            ),
            ..default()
        };

        let mut grid_transform = Transform::from_xyz(0.0, 0.0, 0.0);
        grid_transform.translation.y += 0.01;
        grid_transform.translation.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        grid_transform.translation.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;

        let grid_pbr = PbrBundle {
            mesh: grid_mesh,
            material: grid_material,
            transform: grid_transform,
            visibility: world_settings.grid_visibility,
            ..default()
        };
        let heightmap = match heightmap {
            Some(heightmap) => heightmap.clone(),
            None => Heightmap::new(),
        };

        let chunk_bundle = ChunkBundle {
            pbr: chunk_pbr,
            chunk_tag: Chunk,
            chunk_position: starting_position,
            raycast_mesh: RaycastMesh::<RaycastSet>::default(),
            heightmap,
        };

        commands.spawn(chunk_bundle).with_children(|parent| {
            parent.spawn(GridBundle {
                grid_pbr,
                grid: Grid,
            });
        });
    }
}

pub fn unnormalized_normal_vector(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
    (Vec3::from_array(b) - Vec3::from_array(a)).cross(Vec3::from_array(c) - Vec3::from_array(a))
}

/* fn average_vectors<const N: usize>(list: [Vec3; N]) -> Vec3 {
    let mut sum = Vec3::new(0.0, 0.0, 0.0);
    for vector in list {
        sum += vector;
    }
    sum / N as f32
} */

pub type TilePosition3D = [usize; 3];
pub type TilePosition2D = [usize; 2];
#[derive(Clone, Copy, Debug)]
pub struct ChunkTilePosition {
    pub chunk_position: ChunkPosition,
    pub tile_position: TilePosition3D,
}

#[derive(Clone, Copy, Debug)]
pub struct TileNeighbours {
    pub north: Option<ChunkTilePosition>,
    pub south: Option<ChunkTilePosition>,
    pub east: Option<ChunkTilePosition>,
    pub west: Option<ChunkTilePosition>,
    pub north_east: Option<ChunkTilePosition>,
    pub north_west: Option<ChunkTilePosition>,
    pub south_east: Option<ChunkTilePosition>,
    pub south_west: Option<ChunkTilePosition>,
}
impl TileNeighbours {
    pub fn has_all(&self) -> bool {
        self.north.is_some()
            && self.south.is_some()
            && self.east.is_some()
            && self.west.is_some()
            && self.north_east.is_some()
            && self.north_west.is_some()
            && self.south_east.is_some()
            && self.south_west.is_some()
    }
    pub fn to_array(&self) -> [Option<ChunkTilePosition>; 8] {
        [
            self.north,
            self.south,
            self.east,
            self.west,
            self.north_east,
            self.north_west,
            self.south_east,
            self.south_west,
        ]
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ChunkNeighbours {
    pub north: Option<ChunkPosition>,
    pub south: Option<ChunkPosition>,
    pub east: Option<ChunkPosition>,
    pub west: Option<ChunkPosition>,
    pub north_east: Option<ChunkPosition>,
    pub north_west: Option<ChunkPosition>,
    pub south_east: Option<ChunkPosition>,
    pub south_west: Option<ChunkPosition>,
}
impl ChunkNeighbours {
    pub fn has_all(&self) -> bool {
        self.north.is_some()
            && self.south.is_some()
            && self.east.is_some()
            && self.west.is_some()
            && self.north_east.is_some()
            && self.north_west.is_some()
            && self.south_east.is_some()
            && self.south_west.is_some()
    }
    pub fn to_array(&self) -> [Option<ChunkPosition>; 8] {
        [
            self.north,
            self.south,
            self.east,
            self.west,
            self.north_east,
            self.north_west,
            self.south_east,
            self.south_west,
        ]
    }
}

impl ChunkTilePosition {
    pub fn tile_position_2d(&self) -> TilePosition2D {
        [self.tile_position[0], self.tile_position[2]]
    }
    pub fn tile_neighbours(&self) -> TileNeighbours {
        fn neighbour(
            tile_pos: TilePosition3D,
            x_offset: isize,
            z_offset: isize,
        ) -> Option<ChunkTilePosition> {
            let tile_pos = [
                (tile_pos[0] as isize + x_offset),
                tile_pos[1] as isize,
                (tile_pos[2] as isize + z_offset),
            ];
            if (tile_pos[0] < 0) || (tile_pos[2] < 0) {
                None
            } else {
                let tile_pos = [
                    tile_pos[0] as usize,
                    tile_pos[1] as usize,
                    tile_pos[2] as usize,
                ];
                Some(ChunkTilePosition::from_tile_position(tile_pos))
            }
        }
        let mut tile_position = self.tile_position;
        tile_position[0] += self.chunk_position[0] * CHUNK_SIZE;
        tile_position[2] += self.chunk_position[1] * CHUNK_SIZE;
        TileNeighbours {
            north: neighbour(tile_position, 0, 1),
            south: neighbour(tile_position, 0, -1),
            east: neighbour(tile_position, 1, 0),
            west: neighbour(tile_position, -1, 0),
            north_east: neighbour(tile_position, 1, 1),
            north_west: neighbour(tile_position, -1, 1),
            south_east: neighbour(tile_position, 1, -1),
            south_west: neighbour(tile_position, -1, -1),
        }
    }
    pub fn to_world_position(&self) -> Vec3 {
        let mut world_position = Vec3::new(
            self.chunk_position[0] as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
            0.0,
            self.chunk_position[1] as f32 * CHUNK_SIZE as f32 * TILE_SIZE,
        );
        world_position.x += self.tile_position[0] as f32 * TILE_SIZE;
        world_position.y += self.tile_position[1] as f32 * TILE_SIZE;
        world_position.z += self.tile_position[2] as f32 * TILE_SIZE;
        world_position.x -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position.z -= ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        world_position
    }
    pub fn chunk_neighbours(&self) -> ChunkNeighbours {
        fn neighbour(
            chunk_pos: ChunkPosition,
            x_offset: isize,
            z_offset: isize,
        ) -> Option<ChunkPosition> {
            let chunk_pos = [
                (chunk_pos[0] as isize + x_offset),
                (chunk_pos[1] as isize + z_offset),
            ];
            if (chunk_pos[0] < 0) || (chunk_pos[1] < 0) {
                None
            } else {
                let chunk_pos = [chunk_pos[0] as usize, chunk_pos[1] as usize];
                Some(ChunkPosition {
                    position: chunk_pos,
                })
            }
        }
        ChunkNeighbours {
            north: neighbour(self.chunk_position, 0, 1),
            south: neighbour(self.chunk_position, 0, -1),
            east: neighbour(self.chunk_position, 1, 0),
            west: neighbour(self.chunk_position, -1, 0),
            north_east: neighbour(self.chunk_position, 1, 1),
            north_west: neighbour(self.chunk_position, -1, 1),
            south_east: neighbour(self.chunk_position, 1, -1),
            south_west: neighbour(self.chunk_position, -1, -1),
        }
    }
    pub fn world_position_to_tile_position(world_position: Vec3) -> TilePosition3D {
        let mut tile_position = world_position;
        tile_position.x += ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        tile_position.z += ((TILE_SIZE * CHUNK_SIZE as f32) - TILE_SIZE) / 2.0;
        tile_position.x = tile_position.x.floor();
        tile_position.y = tile_position.y.floor();
        tile_position.z = tile_position.z.floor();
        [
            tile_position.x as usize,
            tile_position.y as usize,
            tile_position.z as usize,
        ]
    }
    pub fn from_world_position(world_position: Vec3) -> ChunkTilePosition {
        let mut tile_position = Self::world_position_to_tile_position(world_position);
        let chunk_position = [
            ((tile_position[0] as f32 / CHUNK_SIZE as f32).floor()) as usize,
            ((tile_position[2] as f32 / CHUNK_SIZE as f32).floor()) as usize,
        ];
        tile_position[0] -= (chunk_position[0]) * CHUNK_SIZE;
        tile_position[2] -= (chunk_position[1]) * CHUNK_SIZE;
        ChunkTilePosition {
            chunk_position: ChunkPosition {
                position: [chunk_position[0] as usize, chunk_position[1] as usize],
            },
            tile_position,
        }
    }
    pub fn from_tile_position(tile_position: TilePosition3D) -> ChunkTilePosition {
        let chunk_position = [
            ((tile_position[0] as f32 / CHUNK_SIZE as f32).floor()) as usize,
            ((tile_position[2] as f32 / CHUNK_SIZE as f32).floor()) as usize,
        ];
        let mut tile_position = tile_position;
        tile_position[0] -= (chunk_position[0]) * CHUNK_SIZE;
        tile_position[2] -= (chunk_position[1]) * CHUNK_SIZE;
        ChunkTilePosition {
            chunk_position: ChunkPosition {
                position: chunk_position,
            },
            tile_position,
        }
    }
    pub fn as_tile_position(&self) -> TilePosition3D {
        let mut tile_position = self.tile_position;
        tile_position[0] += self.chunk_position[0] * CHUNK_SIZE;
        tile_position[2] += self.chunk_position[1] * CHUNK_SIZE;
        tile_position
    }
}
impl Default for ChunkTilePosition {
    fn default() -> Self {
        Self {
            chunk_position: ChunkPosition { position: [0, 0] },
            tile_position: [0, 0, 0],
        }
    }
}
