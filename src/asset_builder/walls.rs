use bevy::prelude::*;
use bevy_mod_raycast::prelude::RaycastMesh;

use crate::{
    constants::{DEBUG, TILE_SIZE, WALL_HEIGHT, WALL_THICKNESS},
    GameState,
};

use crate::cursor::RaycastSet;

#[derive(Resource)]
struct WallAssetResource {
    straight_wall_mesh: Handle<Mesh>,
    diagonal_wall_mesh: Handle<Mesh>,
}

#[derive(Resource, Default)]
struct Walls {
    walls: Vec<Wall>,
}
pub struct WallsPlugin;

impl Plugin for WallsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Walls>();
        app.add_systems(Startup, setup);
        if DEBUG {
            app.add_systems(OnEnter(GameState::AssetBuilder), test);
            app.add_systems(
                PostUpdate,
                debug_walls.run_if(in_state(GameState::AssetBuilder)),
            );
        }
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let straight_wall_mesh = meshes.add(Mesh::from(shape::Box {
        min_x: 0.0 - WALL_THICKNESS / 2.0,
        max_x: TILE_SIZE + WALL_THICKNESS / 2.0,
        min_y: 0.0,
        max_y: WALL_HEIGHT,
        min_z: -WALL_THICKNESS / 2.0,
        max_z: WALL_THICKNESS / 2.0,
    }));

    let diagonal_wall_mesh = meshes.add(Mesh::from(shape::Box {
        min_x: 0.0,
        max_x: (TILE_SIZE.powi(2) + TILE_SIZE.powi(2)).sqrt(),
        min_y: 0.0,
        max_y: WALL_HEIGHT,
        min_z: -WALL_THICKNESS / 2.0,
        max_z: WALL_THICKNESS / 2.0,
    }));

    commands.insert_resource(WallAssetResource {
        straight_wall_mesh,
        diagonal_wall_mesh,
    });
}

fn test(
    wall_resource: Res<WallAssetResource>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut walls: ResMut<Walls>,
) {
    let material = materials.add(Color::rgb(0.7, 0.7, 0.7).into());
    let wall_resource = wall_resource.into_inner();
    let walls = walls.as_mut();
    spawn_walls(
        &mut commands,
        wall_resource,
        material.clone(),
        (0, 0),
        (0, 10),
        walls,
    );
    spawn_walls(
        &mut commands,
        wall_resource,
        material.clone(),
        (0, 10),
        (10, 10),
        walls,
    );
    spawn_walls(
        &mut commands,
        wall_resource,
        material.clone(),
        (0, 0),
        (10, 10),
        walls,
    );
    spawn_walls(
        &mut commands,
        wall_resource,
        material.clone(),
        (0, 0),
        (-10, -10),
        walls,
    );
    spawn_walls(
        &mut commands,
        wall_resource,
        material.clone(),
        (0, 0),
        (10, -10),
        walls,
    );
    spawn_walls(
        &mut commands,
        wall_resource,
        material.clone(),
        (0, 0),
        (-10, 10),
        walls,
    );
}

fn debug_walls(walls: Option<Res<Walls>>) {
    if let Some(walls) = walls {
        if walls.is_changed() {
            for wall in walls.walls.iter() {
                println!(
                    "Wall: {:?} -> {:?}",
                    wall.starting_position, wall.ending_position
                );
            }
        }
    }
}
fn spawn_walls(
    commands: &mut Commands,
    wall_res: &WallAssetResource,
    material: Handle<StandardMaterial>,
    starting_position: (i32, i32),
    ending_position: (i32, i32),
    walls: &mut Walls,
) {
    let delta_x = ending_position.0 - starting_position.0;
    let delta_y = ending_position.1 - starting_position.1;
    let mut slope = 0.0;
    if delta_x != 0 {
        slope = delta_y as f32 / delta_x as f32;
    }
    //Check if the wall is straight
    if slope != 0.0 && slope != 1.0 && slope != -1.0 {
        return;
    }
    walls.walls.push(Wall {
        starting_position,
        ending_position,
    });
    if DEBUG {
        println!(
            "Starting position: {:?}, Ending position: {:?}, Slope: {}",
            starting_position, ending_position, slope
        )
    }
    //Wall in the z direction
    if starting_position.0 == ending_position.0 {
        let length = (ending_position.1 - starting_position.1).abs();
        let starting_pos_z = if starting_position.1 < ending_position.1 {
            starting_position.1
        } else {
            ending_position.1
        };
        for i in 0..length {
            let wall = WallBundle::new(
                (starting_position.0, starting_pos_z + i),
                (starting_position.0, starting_pos_z + i + 1),
                wall_res.straight_wall_mesh.clone(),
                material.clone(),
                270.0,
            );
            commands
                .spawn(wall)
                .insert(RaycastMesh::<RaycastSet>::default());
        }
    //Wall in the x direction
    } else if starting_position.1 == ending_position.1 {
        let length = (ending_position.0 as i32 - starting_position.0 as i32).abs();
        let starting_pos_x = if starting_position.0 < ending_position.0 {
            starting_position.0
        } else {
            ending_position.0
        };
        for i in 0..length {
            let wall = WallBundle::new(
                (starting_pos_x + i, starting_position.1),
                (starting_pos_x + i + 1, starting_position.1),
                wall_res.straight_wall_mesh.clone(),
                material.clone(),
                0.0,
            );
            commands
                .spawn(wall)
                .insert(RaycastMesh::<RaycastSet>::default());
        }
    }
    //Diagonal wall
    else if slope == 1.0 {
        let mut current_pos = starting_position.min(ending_position);
        let ending_pos = starting_position.max(ending_position);
        while current_pos < ending_pos {
            let wall = WallBundle::new(
                current_pos,
                (current_pos.0 + 1, current_pos.1 + 1),
                wall_res.diagonal_wall_mesh.clone(),
                material.clone(),
                -45.0,
            );
            commands
                .spawn(wall)
                .insert(RaycastMesh::<RaycastSet>::default());
            current_pos.0 += 1;
            current_pos.1 += 1;
        }
    } else if slope == -1.0 {
        let mut current_pos = starting_position.min(ending_position);
        let ending_pos = starting_position.max(ending_position);
        while current_pos < ending_pos {
            let wall = WallBundle::new(
                current_pos,
                (current_pos.0 + 1, current_pos.1 - 1),
                wall_res.diagonal_wall_mesh.clone(),
                material.clone(),
                45.0,
            );
            commands
                .spawn(wall)
                .insert(RaycastMesh::<RaycastSet>::default());
            current_pos.0 += 1;
            current_pos.1 -= 1;
        }
    }
}

#[derive(Component)]
struct Wall {
    starting_position: (i32, i32),
    ending_position: (i32, i32),
}

#[derive(Bundle)]
struct WallBundle {
    material_bundle: MaterialMeshBundle<StandardMaterial>,
    wall: Wall,
}

impl WallBundle {
    fn new(
        starting_position: (i32, i32),
        ending_position: (i32, i32),
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        rotation: f32,
    ) -> Self {
        let mut material_bundle = MaterialMeshBundle {
            mesh,
            material,
            transform: Transform::from_xyz(
                starting_position.0 as f32,
                0.0,
                starting_position.1 as f32,
            ),
            ..Default::default()
        };
        material_bundle
            .transform
            .rotate_local_y(rotation * std::f32::consts::PI / 180.0);
        Self {
            material_bundle,
            wall: Wall {
                starting_position,
                ending_position,
            },
        }
    }
}
