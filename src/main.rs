use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

mod terrain;
use crate::terrain::{CHUNK_SIZE, spawn_chunk};

// camera

#[derive(Debug, Component, Deref, DerefMut)]
struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(Vec2::new(0.003, 0.002))
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(
            CHUNK_SIZE as f32 + 10.0,
            CHUNK_SIZE as f32 + 10.0,
            CHUNK_SIZE as f32 + 10.0,
        )
        .looking_at(Vec3::new(8.0, 8.0, 8.0), Vec3::Y),
        CameraSensitivity::default(),
        DistanceFog {
            color: Color::srgb(0.7, 0.75, 0.8),
            falloff: FogFalloff::Exponential { density: 0.01 },
            ..default()
        },
    ));
}

use std::f32::consts::FRAC_PI_2;
// move camera based on inputs
// https://bevy.org/examples/camera/first-person-view-model/
fn move_camera(
    mut camera_query: Single<(&mut Transform, &CameraSensitivity), With<Camera3d>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // rotation (looking at)
    let (mut transform, camera_sensitivity) = camera_query.into_inner();
    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }

    // translation

    // determine which way the camera is facing
    let forward = *transform.forward();
    let right = *transform.right();

    let mut direction = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction += right;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction -= right;
    }

    let speed = 8.0;
    transform.translation += direction.normalize_or_zero() * speed * time.delta_secs();
}

fn grab_mouse(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }

    if key.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_camera(commands.reborrow());
    spawn_chunk(commands.reborrow(), meshes, materials);
    // spawn light as standard material requires it
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(-1.0, -1.0, -0.5), Vec3::Y),
    ));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, move_camera)
        .add_systems(Update, grab_mouse)
        .run();
}
