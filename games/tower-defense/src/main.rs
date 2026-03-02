#![feature(if_let_guard)]

mod objects;
mod plugins;
mod spawner;
mod suspend;

use avian2d::prelude::*;
use bevy::color::palettes::css;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::objects::bullet::*;
use crate::objects::player::PlayerBundle;
use crate::objects::turret::*;
use crate::objects::unit::*;
use crate::objects::*;
use crate::plugins::cary::Caryable;
use crate::plugins::cary::CaryableFilter;
use crate::plugins::physics::GameLayer;
use crate::plugins::targeting::AppExt;
use crate::plugins::targeting::DetectionFilter;
use crate::plugins::targeting::Target;
use crate::plugins::targeting::TargetDetectorBundle;
use crate::plugins::targeting::TargettingStrategy;
use crate::plugins::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Gravity(Vec2::ZERO))
        .add_plugins((physics::plugin, targeting::plugin, cary::plugin))
        .add_detectiion_filter::<EnemiesFilter>()
        .add_plugins((bullet::plugin, turret::plugin, unit::plugin, player::plugin))
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

#[derive(Component)]
struct EnemyMarker;

#[derive(SystemParam)]
struct EnemiesFilter<'w, 's> {
    enemies: Query<'w, 's, (), With<EnemyMarker>>,
}

impl DetectionFilter for EnemiesFilter<'_, '_> {
    fn is_hit(&self, _detector: Entity, candidate: Entity) -> bool {
        self.enemies.contains(candidate)
    }
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    const TURRET_DETECT_RADIUS: f32 = 200.0;
    const PLAYER_PICKUP_RADIUS: f32 = 50.0;
    const PLAYER_MOVE_SPEED: f32 = 300.0;

    const UNIT_RADIUS: f32 = 15.0;
    const BULLET_RADIUS: f32 = 5.0;
    const PLAYER_RADIUS: f32 = 20.0;

    let enemy_mat = materials.add(ColorMaterial::from(Color::from(css::RED)));
    let bullet_mat = materials.add(ColorMaterial::from(Color::from(css::GOLD)));
    let player_mat = materials.add(ColorMaterial::from(Color::from(css::ALICE_BLUE)));

    let enemy_mesh = meshes.add(Circle::new(UNIT_RADIUS));
    let bullet_mesh = meshes.add(Circle::new(BULLET_RADIUS));
    let player_mesh = meshes.add(Circle::new(PLAYER_RADIUS));

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
                TurretBundle::new(
                    pos,
                    1.0,
                    500.0,
                    spawn_bullet.clone(),
                    TargettingStrategy::Nearest,
                ),
                Collider::circle(20.0), // <--- this
                Caryable,
                CollisionLayers::new(GameLayer::SensorTarget, GameLayer::TargetDetection),
            ))
            .id();

        commands
            .entity(turret)
            .with_child(TargetDetectorBundle::<EnemiesFilter>::new(
                turret,
                TURRET_DETECT_RADIUS,
            ));
    };

    let target = commands
        .spawn((
            Transform::from_translation(Vec3::new(0.0, -100.0, 0.0)),
            UnitTargetMarker,
        ))
        .id();

    let spawn_enemy = |commands: &mut Commands, pos: Vec2| {
        commands.spawn((
            EnemyMarker,
            UnitBundle::new(pos, UNIT_RADIUS, 100),
            MeshMaterial2d(enemy_mat.clone()),
            Mesh2d(enemy_mesh.clone()),
            Target(target),
        ));
    };

    let spawn_player = |commands: &mut Commands, pos: Vec2| {
        let player = commands
            .spawn((
                PlayerBundle::new(pos, PLAYER_RADIUS, PLAYER_MOVE_SPEED), // <--- colider in here
                TargettingStrategy::Nearest,
                MeshMaterial2d(player_mat),
                Mesh2d(player_mesh),
            ))
            .id();

        commands
            .entity(player)
            .with_child(TargetDetectorBundle::<CaryableFilter>::new(
                player,
                PLAYER_PICKUP_RADIUS,
            ));
    };

    spawn_player(&mut commands, Vec2::new(0.0, 0.0));

    spawn_turret(&mut commands, Vec2::new(-100.0, 100.0));
    spawn_turret(&mut commands, Vec2::new(200.0, -50.0));

    [
        (-100.0, -100.0),
        (-50.0, -150.0),
        (0.0, -200.0),
        (50.0, -250.0),
        (100.0, -300.0),
    ]
    .into_iter()
    .for_each(|(x, y)| spawn_enemy(&mut commands, Vec2::new(x, y)));
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
