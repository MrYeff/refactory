#![feature(if_let_guard)]

mod objects;
mod plugins;
mod spawner;

use avian2d::PhysicsPlugins;
use avian2d::prelude::Gravity;
use avian2d::prelude::PhysicsDebugPlugin;
use bevy::color::palettes::css;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::objects::bullet::*;
use crate::objects::turret::*;
use crate::objects::unit::*;
use crate::objects::*;
use crate::plugins::targeting::Target;
use crate::plugins::targeting::TargetDetectorBundle;
use crate::plugins::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin))
        .insert_resource(Gravity(Vec2::ZERO))
        .add_plugins(targeting::plugin)
        .add_plugins((bullet::plugin, turret::plugin, unit::plugin))
        .add_systems(Startup, (spawn_camera, spawn_scene))
        .add_systems(Update, update_unit_target)
        .add_systems(PostUpdate, draw_target_gizmos)
        .run()
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Component)]
struct UnitTargetMarker;

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    const TURRET_RADIUS: f32 = 25.0;
    const UNIT_RADIUS: f32 = 15.0;
    const BULLET_RADIUS: f32 = 5.0;

    let turret_mat = materials.add(ColorMaterial::from(Color::from(css::SKY_BLUE)));
    let enemy_mat = materials.add(ColorMaterial::from(Color::from(css::RED)));
    let bullet_mat = materials.add(ColorMaterial::from(Color::from(css::GOLD)));

    let turret_mesh = meshes.add(Circle::new(TURRET_RADIUS));
    let enemy_mesh = meshes.add(Circle::new(UNIT_RADIUS));
    let bullet_mesh = meshes.add(Circle::new(BULLET_RADIUS));

    let spawn_bullet = {
        let bullet_mat = bullet_mat.clone();
        let bullet_mesh = bullet_mesh.clone();
        move |params: BulletParams| {
            (
                BulletBundle::new(10, BULLET_RADIUS, params.pos, params.vel),
                MeshMaterial2d(bullet_mat.clone()),
                Mesh2d(bullet_mesh.clone()),
            )
        }
    };

    let spawn_turret = |commands: &mut Commands, pos: Vec2| {
        let turret = commands
            .spawn((
                TurretBundle::new(pos, 1.0, spawn_bullet),
                MeshMaterial2d(turret_mat.clone()),
                Mesh2d(turret_mesh.clone()),
            ))
            .id();

        commands
            .entity(turret)
            .with_child(TargetDetectorBundle::new(turret, 200.0));
    };

    let target = commands
        .spawn((
            Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)),
            UnitTargetMarker,
        ))
        .id();

    let spawn_unit = |commands: &mut Commands, pos: Vec2| {
        commands.spawn((
            UnitBundle::new(pos, UNIT_RADIUS, 100),
            MeshMaterial2d(enemy_mat.clone()),
            Mesh2d(enemy_mesh.clone()),
            Target(target),
        ));
    };

    spawn_turret(&mut commands, Vec2::new(0.0, 100.0));
    [
        (-100.0, -100.0),
        (-50.0, -150.0),
        (0.0, -200.0),
        (50.0, -250.0),
        (100.0, -300.0),
    ]
    .into_iter()
    .for_each(|(x, y)| spawn_unit(&mut commands, Vec2::new(x, y)));
}

fn update_unit_target(
    get_mouse_pos: Option<GetMouseWorldPos>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut target: Single<&mut Transform, With<UnitTargetMarker>>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    let Some(mouse_pos) = get_mouse_pos.and_then(|x| x.run()) else {
        return;
    };

    target.translation = mouse_pos.extend(0.0);
}

#[derive(SystemParam)]
pub struct GetMouseWorldPos<'w, 's> {
    window: Single<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera: Single<'w, 's, (&'static Camera, &'static GlobalTransform)>,
}

impl<'w, 's> GetMouseWorldPos<'w, 's> {
    pub fn run(&self) -> Option<Vec2> {
        let screen_pos = self.window.cursor_position()?;
        let (camera, camera_transform) = *self.camera;
        Some(
            camera
                .viewport_to_world(camera_transform, screen_pos)
                .ok()?
                .origin
                .truncate(),
        )
    }
}

fn draw_target_gizmos(targets: Single<&Transform, With<UnitTargetMarker>>, mut gizmos: Gizmos) {
    gizmos.circle_2d(targets.translation.truncate(), 10.0, Color::WHITE);
}

type GameTime = Time;
