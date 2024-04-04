use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};
use smooth_bevy_cameras::{
    controllers::orbit::{
        ControlEvent, OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin,
    },
    LookTransform, LookTransformPlugin,
};

use crate::{
    world::WorldEntity,
    world_gen::{
        consts::{CHUNK_SIZE, TILE_WORLD_SIZE, WORLD_HEIGHT_SCALE}, heightmap::Heightmap, WorldSettings,
    },
    GameState, DEBUG,
};

#[derive(Reflect)]
pub struct CameraRaycastSet;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            OrbitCameraPlugin {
                override_input_system: true,
            },
            LookTransformPlugin,
        ));
        app.add_systems(OnEnter(GameState::World), setup);
        app.add_systems(Update, input.run_if(in_state(GameState::World)));
    }
}

#[derive(Component)]
pub struct TerrainRaycaster;

pub fn input(
    mut events: EventWriter<ControlEvent>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    controllers: Query<&OrbitCameraController>,
    mut cameras: Query<(&OrbitCameraController, &mut LookTransform, &Transform)>,
    _world_settings: Res<WorldSettings>,
    mut gizmos: Gizmos,
    heightmap: Res<Heightmap>,
) {
    //Modified from smooth_bevy_cameras
    // Can only control one camera at a time.
    let Some(controller) = controllers.iter().find(|c| c.enabled) else {
        return;
    };

    let Some((_, mut transform, _)) = cameras.iter_mut().find(|c| c.0.enabled) else {
        return;
    };

    let OrbitCameraController {
        mouse_rotate_sensitivity,

        mouse_wheel_zoom_sensitivity,
        pixels_per_line,
        ..
    } = *controller;

    let mut cursor_delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        cursor_delta += event.delta;
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

    //Distance from target
    let distance = (transform.eye - transform.target).length();

    let keyboard_translate_sensitivity = 0.01;

    //Keyboard camera translation
    if keyboard.pressed(KeyCode::KeyW) {
        let mut look_direction = transform.target - transform.eye;
        look_direction.y = 0.0;
        transform.target += look_direction.normalize() * keyboard_translate_sensitivity * distance;
        transform.eye += look_direction.normalize() * keyboard_translate_sensitivity * distance;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        let mut look_direction = transform.target - transform.eye;
        look_direction.y = 0.0;
        transform.target -= look_direction.normalize() * keyboard_translate_sensitivity * distance;
        transform.eye -= look_direction.normalize() * keyboard_translate_sensitivity * distance;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        let look_direction = transform.target - transform.eye;
        let left = Vec3 {
            x: look_direction.z,
            y: 0.0,
            z: -look_direction.x,
        };
        transform.target += left.normalize() * keyboard_translate_sensitivity * distance;
        transform.eye += left.normalize() * keyboard_translate_sensitivity * distance;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        let look_direction = transform.target - transform.eye;
        let left = Vec3 {
            x: look_direction.z,
            y: 0.0,
            z: -look_direction.x,
        };
        transform.target -= left.normalize() * keyboard_translate_sensitivity * distance;
        transform.eye -= left.normalize() * keyboard_translate_sensitivity * distance;
    }

    if transform.eye.y < transform.target.y {
        transform.eye.y = transform.target.y;
    }

    //Keep camera above terrain height

    if (transform.eye.x > 0.0 && transform.eye.x < TILE_WORLD_SIZE[0] as f32)
        && (transform.eye.z > 0.0 && transform.eye.z < TILE_WORLD_SIZE[1] as f32)
    {
        let terrain_height = heightmap.interpolate_height(transform.eye.xz()) + 1.5;
        if transform.eye.y < terrain_height {
            transform.eye.y = terrain_height;
        }
    }

    //Restrict Camera to world bounds
    let eye_delta = transform.eye - transform.target;
    transform.target.x = transform.target.x.clamp(
        CHUNK_SIZE as f32 * 0.5,
        TILE_WORLD_SIZE[0] as f32 - (CHUNK_SIZE as f32 * 0.5),
    );
    transform.target.z = transform.target.z.clamp(
        CHUNK_SIZE as f32 * 0.5,
        TILE_WORLD_SIZE[1] as f32 - (CHUNK_SIZE as f32 * 0.5),
    );
    transform.eye = transform.target + eye_delta;

    //Set target y to terrain height
    transform.target.y = heightmap.interpolate_height(transform.target.xz());

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

fn setup(mut commands: Commands, heightmap: Res<Heightmap>) {
    let orbit_camera_controller = OrbitCameraController {
        mouse_rotate_sensitivity: Vec2::splat(0.2),
        mouse_translate_sensitivity: Vec2::splat(0.1),
        mouse_wheel_zoom_sensitivity: 0.2,
        ..Default::default()
    };
    let middle = [heightmap.size()[0] / 2, heightmap.size()[1] / 2];
    let middle = [
        middle[0] as f32,
        heightmap[middle] as f32 * WORLD_HEIGHT_SCALE,
        middle[1] as f32,
    ];
    let middle = [middle[0] as f32, 0.0, middle[1] as f32];
    let eye_offset: [f32; 3] = [10.0, 10.0, 0.0];
    let orbit_camera_bundle = OrbitCameraBundle::new(
        orbit_camera_controller,
        Into::<Vec3>::into(middle) + Into::<Vec3>::into(eye_offset),
        middle.into(),
        *Direction3d::Y,
    );
    //Spawn Camera
    commands
        .spawn((orbit_camera_bundle, WorldEntity))
        .insert(Camera3dBundle::default());
}
