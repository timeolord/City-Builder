use bevy::prelude::*;

use crate::{
    chunk::chunk_tile_position::TilePosition,
    GameState,
};

use super::{
    buildings::ResidentialBuilding,
    heightmap::HeightmapsResource,
    resources::{Inventory, InventoryType},
    road::pathfinding::Pathfind,
};

pub struct VehiclesPlugin;

impl Plugin for VehiclesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(
            Update,
            (move_vehicle, vehicle_complete_goal_handler).run_if(in_state(GameState::World)),
        );
        app.add_systems(OnExit(GameState::World), exit);
    }
}

#[derive(Component, Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct VehicleCompletedGoal {
    pub goal: VehicleGoal,
}
#[derive(Component)]
pub struct VehiclePosition {
    pub position: TilePosition,
}
#[derive(Component)]
pub struct VehicleSpeed {
    pub speed: f32,
}
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum VehicleGoal {
    Shopping { entity: Entity },
    ReturnHome { entity: Entity },
}
#[derive(Component, Clone, Eq, PartialEq, Debug, Hash)]
pub struct VehicleGoals {
    pub goals: Vec<VehicleGoal>,
}
#[derive(Bundle)]
pub struct VehicleBundle {
    pub position: VehiclePosition,
    pub speed: VehicleSpeed,
    pub pbr: PbrBundle,
    pub goals: VehicleGoals,
    pub inventory: Inventory,
}

#[derive(Resource)]
pub struct VehicleSettings {
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

fn setup(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let meshes = vec![mesh_assets.add(Mesh::from(shape::Cube { size: 0.8 }))];
    let materials = vec![material_assets.add(Color::BISQUE.into())];
    commands.insert_resource(VehicleSettings { meshes, materials });
}

fn exit(mut commands: Commands) {
    commands.remove_resource::<VehicleSettings>();
}

fn move_vehicle(
    mut commands: Commands,
    mut vehicle_query: Query<
        (
            Entity,
            &VehicleSpeed,
            &mut VehicleGoals,
            &mut Pathfind,
            &mut VehiclePosition,
            &mut Transform,
        ),
        Without<VehicleCompletedGoal>,
    >,
    heightmaps: Res<HeightmapsResource>,
) {
    for (entity, speed, mut goals, mut pathfind, mut tile_position, mut transform) in
        vehicle_query.iter_mut()
    {
        //Check if the car has reached the end of the path and if so, complete the current goal
        if TilePosition::from_position_2d(
            *pathfind.path.last().expect("Path should not be empty"),
        )
        .to_world_position()
        .xz()
        .abs_diff_eq(transform.translation.xz(), speed.speed * 2.0)
        {
            commands.entity(entity).insert(VehicleCompletedGoal {
                goal: goals.goals.pop().expect("Goals should not be empty"),
            });
            continue;
        }

        //Move the car towards the next tile
        let direction = pathfind.path[pathfind.current_index].as_vec2()
            - tile_position.position.position_2d().as_vec2();
        let velocity = direction.round().normalize_or_zero() * speed.speed;
        let velocity3 = Vec3::new(velocity.x, 0.0, velocity.y);
        transform.translation += velocity3;

        //If the car has reached the next tile, increment the index
        if TilePosition::from_position_2d(pathfind.path[pathfind.current_index])
            .to_world_position()
            .xz()
            .abs_diff_eq(transform.translation.xz(), speed.speed * 2.0)
        {
            tile_position.position =
                TilePosition::from_position_2d(pathfind.path[pathfind.current_index]);
            pathfind.current_index += 1;
            pathfind.current_index = pathfind.current_index.clamp(0, pathfind.path.len() - 1);
        }

        //Sticks the car to the ground
        transform.translation = heightmaps.get_from_world_position(transform.translation);
    }
}

fn vehicle_complete_goal_handler(
    mut commands: Commands,
    mut vehicle_query: Query<(
        Entity,
        &mut VehicleGoals,
        &mut Pathfind,
        &mut Inventory,
        &VehicleCompletedGoal,
    )>,
    mut home_query: Query<
        (Entity, &mut Inventory),
        (
            With<ResidentialBuilding>,
            Without<VehicleCompletedGoal>,
            Without<Pathfind>,
            Without<VehicleGoals>,
        ),
    >,
) {
    for (vehicle_entity, mut goals, mut pathfind, mut inventory, completed_goal) in
        vehicle_query.iter_mut()
    {
        match completed_goal.goal {
            VehicleGoal::Shopping { entity: _ } => {
                pathfind.current_index = 0;
                pathfind.path.reverse();

                commands
                    .entity(vehicle_entity)
                    .remove::<VehicleCompletedGoal>();
            }
            VehicleGoal::ReturnHome { entity } => {
                let mut building_inventory = home_query
                    .get_mut(entity)
                    .expect("Vehicle should have a valid home to return to")
                    .1;

                building_inventory.inventory[InventoryType::People].current +=
                    inventory.inventory[InventoryType::People].current;

                inventory.inventory[InventoryType::People].current = 0;

                commands.entity(vehicle_entity).despawn_recursive();
            }
        }
    }
}
