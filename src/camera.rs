use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bevy_mod_raycast::prelude::RaycastSource;
use smooth_bevy_cameras::{
    controllers::orbit::{
        ControlEvent, OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin,
    },
    LookTransform, LookTransformPlugin,
};

use crate::{
    chunk::{ChunkPosition, ChunkTilePosition},
    constants::DEBUG,
    world::{heightmap_generator::Heightmap, WorldSettings},
    GameState,
};

use super::cursor::RaycastSet;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            OrbitCameraPlugin {
                override_input_system: true,
            },
            LookTransformPlugin,
        ));
        app.add_systems(OnEnter(GameState::AssetBuilder), setup);
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(
            Update,
            input.run_if(in_state(GameState::AssetBuilder).or_else(in_state(GameState::World))),
        );
    }
}

pub fn input(
    mut events: EventWriter<ControlEvent>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    controllers: Query<&OrbitCameraController>,
    mut cameras: Query<(&OrbitCameraController, &mut LookTransform, &Transform)>,
    mut gizmos: Gizmos,
    world_settings: Option<Res<WorldSettings>>,
    heightmaps: Query<(&ChunkPosition, &Heightmap)>,
) {
    //Modified from smooth_bevy_cameras
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().find(|c| c.enabled) {
        controller
    } else {
        return;
    };

    let (mut transform, _) =
        if let Some((_, transform, scene_transform)) = cameras.iter_mut().find(|c| c.0.enabled) {
            (transform, scene_transform)
        } else {
            return;
        };

    let OrbitCameraController {
        mouse_rotate_sensitivity,
        mouse_translate_sensitivity,
        mouse_wheel_zoom_sensitivity,
        pixels_per_line,
        ..
    } = *controller;

    let mut cursor_delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        cursor_delta += event.delta;
    }

    //World Camera
    match world_settings {
        Some(_) => {
            //todo add lerping or use a raycast
            let chunk_tile_position = ChunkTilePosition::from_world_position(transform.target);
            match heightmaps
                .iter()
                .find(|(chunk, _)| **chunk == chunk_tile_position.chunk_position)
            {
                Some((_, heightmap)) => {
                    let height = heightmap[chunk_tile_position.tile_position_2d()]
                        .into_iter()
                        .reduce(f32::max)
                        .unwrap();
                    transform.target.y = height + 0.1;
                }
                None => {}
            }
            //println!(
            //    "{:?}, {:?}, {:?}",
            //    transform.target,
            //    chunk_tile_position.chunk_position,
            //    chunk_tile_position.tile_position_2d()
            //);
        }
        None => {}
    }

    if mouse_buttons.pressed(MouseButton::Middle) {
        events.send(ControlEvent::Orbit(mouse_rotate_sensitivity * cursor_delta));
    }

    //TODO Fix this
    /* if mouse_buttons.pressed(MouseButton::Right) {
        let delta = mouse_translate_sensitivity * 0.05 * cursor_delta;
        transform.target.x -= delta.x;
        transform.target.z -= delta.y;
        transform.eye.x -= delta.x;
        transform.eye.z -= delta.y;
    } */

    //Keyboard camera translation
    if keyboard.pressed(KeyCode::W) {
        let mut look_direction = transform.target - transform.eye;
        look_direction.y = 0.0;
        transform.target += look_direction.normalize() * mouse_translate_sensitivity.x;
        transform.eye += look_direction.normalize() * mouse_translate_sensitivity.x;
    }
    if keyboard.pressed(KeyCode::S) {
        let mut look_direction = transform.target - transform.eye;
        look_direction.y = 0.0;
        transform.target -= look_direction.normalize() * mouse_translate_sensitivity.x;
        transform.eye -= look_direction.normalize() * mouse_translate_sensitivity.x;
    }
    if keyboard.pressed(KeyCode::A) {
        let look_direction = transform.target - transform.eye;
        let left = Vec3 {
            x: look_direction.z,
            y: 0.0,
            z: -look_direction.x,
        };
        transform.target += left.normalize() * mouse_translate_sensitivity.y;
        transform.eye += left.normalize() * mouse_translate_sensitivity.y;
    }
    if keyboard.pressed(KeyCode::D) {
        let look_direction = transform.target - transform.eye;
        let left = Vec3 {
            x: look_direction.z,
            y: 0.0,
            z: -look_direction.x,
        };
        transform.target -= left.normalize() * mouse_translate_sensitivity.y;
        transform.eye -= left.normalize() * mouse_translate_sensitivity.y;
    }

    if transform.eye.y < transform.target.y {
        transform.eye.y = transform.target.y;
    }

    if DEBUG {
        gizmos.sphere(transform.target, Quat::IDENTITY, 0.1, Color::RED);
    }

    // Zoom
    let mut scalar = 1.0;
    for event in mouse_wheel_reader.read() {
        // scale the event magnitude per pixel or per line
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / pixels_per_line,
        };
        scalar *= 1.0 - scroll_amount * mouse_wheel_zoom_sensitivity;
    }
    events.send(ControlEvent::Zoom(scalar));
}

fn setup(mut commands: Commands) {
    let orbit_camera_controller = OrbitCameraController {
        mouse_rotate_sensitivity: Vec2::splat(0.2),
        mouse_translate_sensitivity: Vec2::splat(0.1),
        mouse_wheel_zoom_sensitivity: 0.2,
        ..Default::default()
    };
    let orbit_camera_bundle = OrbitCameraBundle::new(
        orbit_camera_controller,
        Vec3 {
            x: -2.0,
            y: 2.5,
            z: 5.0,
        },
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Vec3::Y,
    );
    //Spawn Camera
    commands
        .spawn(orbit_camera_bundle)
        .insert(Camera3dBundle::default())
        .insert(RaycastSource::<RaycastSet>::new_cursor());
}
