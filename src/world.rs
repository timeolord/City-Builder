pub mod buildings;
pub mod game_time;
pub mod heightmap_generator;
pub mod road;
pub mod terraform;
pub mod tile_highlight;
pub mod tools;
pub mod vehicles;
pub mod resources;
use std::f32::consts::PI;

use crate::{
    chunk::{chunk_tile_position::ChunkPosition, Grid, SpawnChunkEvent},
    GameState,
};
use bevy::{pbr::CascadeShadowConfigBuilder, prelude::*};

use self::{
    buildings::BuildingsPlugin, game_time::GameTimePlugin, road::RoadPlugin,
    terraform::TerraformPlugin, tile_highlight::TileHighlightPlugin, tools::ToolsPlugin,
    vehicles::VehiclesPlugin, resources::ResourcesPlugin,
};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TerraformPlugin,
            RoadPlugin,
            ToolsPlugin,
            TileHighlightPlugin,
            BuildingsPlugin,
            GameTimePlugin,
            VehiclesPlugin,
            ResourcesPlugin,
        ));
        app.add_systems(Startup, init);
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(Update, toggle_grid.run_if(in_state(GameState::World)));
        app.add_systems(OnExit(GameState::World), exit);
    }
}

fn exit(mut commands: Commands, query: Query<Entity, With<WorldEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Component)]
struct WorldEntity;

#[derive(Resource)]
pub struct WorldSettings {
    pub world_size: [usize; 2],
    pub seed: u32,
    pub grid_visibility: Visibility,
}

fn init(mut commands: Commands) {
    let world_size = [4, 4];
    let seed = 0;
    commands.insert_resource(WorldSettings {
        world_size,
        seed,
        grid_visibility: Visibility::Visible,
    });
}

fn setup(
    mut commands: Commands,
    world_settings: Res<WorldSettings>,
    mut spawn_chunk_event: EventWriter<SpawnChunkEvent>,
) {
    // Sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            // shadow_depth_bias: 0.2,
            illuminance: 50000.0,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 100.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 1000.0,
            ..default()
        }
        .into(),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });

    let world_size = world_settings.world_size.clone();
    for x in 0..world_size[0] {
        for y in 0..world_size[1] {
            spawn_chunk_event.send(SpawnChunkEvent {
                position: ChunkPosition { position: [x, y] },
                heightmap: Some(heightmap_generator::generate_heightmap(
                    world_settings.seed,
                    ChunkPosition { position: [x, y] },
                )),
            });
        }
    }
}

trait AsF32<T, const N: usize> {
    fn as_f32(&self) -> [f32; N];
}
impl<T: num_traits::cast::AsPrimitive<f32>, const N: usize> AsF32<T, N> for [T; N] {
    fn as_f32(&self) -> [f32; N] {
        let mut array = [0.0; N];
        for (i, item) in self.iter().enumerate() {
            array[i] = (*item).as_();
        }
        array
    }
}
trait AsU32<T, const N: usize> {
    fn as_u32(&self) -> [u32; N];
}
impl<T: num_traits::cast::AsPrimitive<u32>, const N: usize> AsU32<T, N> for [T; N] {
    fn as_u32(&self) -> [u32; N] {
        let mut array = [0; N];
        for (i, item) in self.iter().enumerate() {
            array[i] = (*item).as_();
        }
        array
    }
}
trait AsI32<T, const N: usize> {
    fn as_i32(&self) -> [i32; N];
}
impl<T: num_traits::cast::AsPrimitive<i32>, const N: usize> AsI32<T, N> for [T; N] {
    fn as_i32(&self) -> [i32; N] {
        let mut array = [0; N];
        for (i, item) in self.iter().enumerate() {
            array[i] = (*item).as_();
        }
        array
    }
}

fn toggle_grid(
    mut query: Query<&mut Visibility, With<Grid>>,
    keyboard: Res<Input<KeyCode>>,
    mut grid_visible: ResMut<WorldSettings>,
) {
    if keyboard.just_pressed(KeyCode::G) {
        grid_visible.grid_visibility = match grid_visible.grid_visibility {
            Visibility::Visible => Visibility::Hidden,
            Visibility::Hidden => Visibility::Visible,
            Visibility::Inherited => Visibility::Inherited,
        };
        for mut visible in query.iter_mut() {
            *visible = grid_visible.grid_visibility;
        }
    }
}
