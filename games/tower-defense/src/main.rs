#![feature(if_let_guard)]

mod objects;
mod plugins;
mod spawner;

use avian3d::prelude::*;
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
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((physics::plugin, targeting::plugin, cary::plugin))
        .add_detectiion_filter::<EnemiesFilter>()
        .add_plugins((bullet::plugin, turret::plugin, unit::plugin, player::plugin))
        .add_systems(Startup, (spawn_camera, spawn_enviroment, spawn_scene))
        .add_systems(Update, update_unit_target)
        .add_systems(PostUpdate, draw_target_gizmos);

    #[cfg(debug_assertions)]
    app.add_plugins(debug::plugin);

    app.run()
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, 50.0, 20.0).with_rotation(Quat::from_rotation_x(-1.3)),
        Camera3d::default(),
    ));
}

fn spawn_enviroment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_mesh = meshes.add(Rectangle::new(1000.0, 1000.0));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::from(css::LIGHT_GRAY),
        ..default()
    });

    commands.spawn((
        Transform::from_xyz(0.0, -1.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        Collider::cuboid(1000.0, 1000.0, 1.0),
        RigidBody::Static,
        CollisionLayers::new(
            GameLayer::Default,
            GameLayer::Default | GameLayer::Bullets | GameLayer::Units,
        ),
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_mat),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.33, 0.15, 0.0)),
    ));
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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const TURRET_DETECT_RADIUS: f32 = 10.0;
    const PLAYER_PICKUP_RADIUS: f32 = 2.0;
    const PLAYER_MOVE_SPEED: f32 = 10.0;

    const UNIT_RADIUS: f32 = 1.0;
    const BULLET_RADIUS: f32 = 0.2;
    const PLAYER_RADIUS: f32 = 1.0;

    let enemy_mat = materials.add(StandardMaterial::from(Color::from(css::RED)));
    let bullet_mat = materials.add(StandardMaterial::from(Color::from(css::GOLD)));
    let player_mat = materials.add(StandardMaterial::from(Color::from(css::ALICE_BLUE)));

    let enemy_mesh = meshes.add(Capsule3d::new(UNIT_RADIUS, 2.0));
    let bullet_mesh = meshes.add(Sphere::new(BULLET_RADIUS));
    let player_mesh = meshes.add(Capsule3d::new(UNIT_RADIUS, 2.0));

    let spawn_bullet = {
        let bullet_mat = bullet_mat.clone();
        let bullet_mesh = bullet_mesh.clone();
        move |params: BulletParams| {
            (
                BulletBundle::new(10, BULLET_RADIUS, params.pos, params.vel),
                MeshMaterial3d(bullet_mat.clone()),
                Mesh3d(bullet_mesh.clone()),
            )
        }
    };

    let spawn_turret = |commands: &mut Commands, pos: Vec2| {
        let turret = commands
            .spawn((
                TurretBundle::new(
                    pos,
                    4.0,
                    20.0,
                    spawn_bullet.clone(),
                    TargettingStrategy::Nearest,
                ),
                Collider::capsule(1.0, 2.0),
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
        .spawn((Transform::default(), UnitTargetMarker))
        .id();

    let spawn_enemy = |commands: &mut Commands, pos: Vec2| {
        commands.spawn((
            EnemyMarker,
            UnitBundle::new(pos, UNIT_RADIUS, 100),
            MeshMaterial3d(enemy_mat.clone()),
            Mesh3d(enemy_mesh.clone()),
            Target(target),
        ));
    };

    let spawn_player = |commands: &mut Commands, pos: Vec2| {
        let player = commands
            .spawn((
                PlayerBundle::new(pos, PLAYER_RADIUS, PLAYER_MOVE_SPEED),
                TargettingStrategy::Nearest,
                MeshMaterial3d(player_mat),
                Mesh3d(player_mesh),
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

    spawn_turret(&mut commands, Vec2::new(-10.0, 10.0));
    spawn_turret(&mut commands, Vec2::new(10.0, -5.0));

    [
        (-10.0, -10.0),
        (-5.0, -15.0),
        (0.0, -20.0),
        (5.0, -25.0),
        (10.0, -30.0),
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

    target.translation = Vec3::new(mouse_pos.x, 0.0, mouse_pos.y);
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
        let ray = camera
            .viewport_to_world(camera_transform, screen_pos)
            .ok()?;
        let ray_dir = ray.direction.as_vec3();

        if ray_dir.y.abs() <= f32::EPSILON {
            return None;
        }

        let dist = -ray.origin.y / ray_dir.y;
        if dist < 0.0 {
            return None;
        }

        let world_pos = ray.origin + ray_dir * dist;
        Some(Vec2::new(world_pos.x, world_pos.z))
    }
}

fn draw_target_gizmos(target: Single<&Transform, With<UnitTargetMarker>>, mut gizmos: Gizmos) {
    gizmos.sphere(target.translation, 0.5, css::RED);
}

type GameTime = Time;
