use avian3d::prelude::LockedAxes;
use bevy::prelude::*;

use crate::GameTime;
use crate::objects::bullet::*;
use crate::plugins::cary::CarriedBy;
use crate::plugins::targeting::*;
use crate::spawner::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(render::plugin);
    app.add_systems(Update, (tick_turrets, shoot_at_target));
}

#[derive(Component, Deref, DerefMut)]
pub struct ShootCooldown(Timer);

#[derive(Component)]
#[require(Name::new("Turret"))]
#[require(LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z())]
pub struct Turret {
    bullet_spawner: BoxedSpawner<BulletParams>,
    bullet_speed: f32,
}

#[derive(Bundle)]
pub struct TurretBundle {
    turret: Turret,
    transform: Transform,
    shoot_cd: ShootCooldown,
    strategy: TargettingStrategy,
}

impl TurretBundle {
    pub fn new(
        pos: Vec2,
        shots_per_second: f32,
        bullet_speed: f32,
        bullet_fac: impl IntoSpawner<BulletParams>,
        strategy: TargettingStrategy,
    ) -> Self {
        TurretBundle {
            turret: Turret {
                bullet_spawner: bullet_fac.into_spawner(),
                bullet_speed,
            },
            transform: Transform::from_translation(Vec3::new(pos.x, 0.0, pos.y)),
            shoot_cd: ShootCooldown(Timer::from_seconds(1.0 / shots_per_second, TimerMode::Once)),
            strategy,
        }
    }
}

fn tick_turrets(mut turrets: Query<&mut ShootCooldown, With<Turret>>, time: Res<GameTime>) {
    turrets.iter_mut().for_each(|mut cd| {
        cd.tick(time.delta());
    });
}

fn shoot_at_target(
    mut commands: Commands,
    // helpers
    get_target_pos: GetTargetPos,
    // queries
    mut turrets: Query<
        (&mut ShootCooldown, &GlobalTransform, &Target, &Turret),
        Without<CarriedBy>,
    >,
) {
    turrets
        .iter_mut()
        .filter(|entry| entry.0.is_finished())
        .filter_map(|(cd, tf, target, turret)| {
            Some((cd, tf, get_target_pos.run(target).ok()?, turret))
        })
        .for_each(|(mut cd, tf, target, turret)| {
            cd.reset();
            let pos = tf.translation();
            let dir = (target - pos).normalize();

            turret.bullet_spawner.spawn(
                &mut commands,
                BulletParams {
                    pos,
                    vel: dir * turret.bullet_speed,
                },
            );
        });
}

mod render {
    use bevy::color::palettes::css;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    pub fn plugin(app: &mut App) {
        const TURRET_RADIUS: f32 = 1.0;

        let (turret_mat, turret_mesh) = app
            .world_mut()
            .run_system_once(
                |mut meshes: ResMut<Assets<Mesh>>,
                 mut materials: ResMut<Assets<StandardMaterial>>| {
                    (
                        materials.add(StandardMaterial::from(Color::from(css::SKY_BLUE))),
                        meshes.add(Capsule3d::new(TURRET_RADIUS, 2.0)),
                    )
                },
            )
            .unwrap();

        app.add_observer(move |tr: On<Add, Turret>, mut commands: Commands| {
            commands.entity(tr.entity).insert((
                MeshMaterial3d(turret_mat.clone()),
                Mesh3d(turret_mesh.clone()),
            ));
        });
    }
}
