use crate::GameTime;
use crate::{objects::bullet::*, plugins::targeting::*, spawner::*};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (tick_turrets, shoot_at_target));
}

#[derive(Component, Deref, DerefMut)]
pub struct ShootCooldown(Timer);

#[derive(Component)]
pub struct Turret {
    bullet_spawner: BoxedSpawner<BulletParams>,
    bullet_speed: f32,
}

#[derive(Bundle)]
pub struct TurretBundle {
    turret: Turret,
    transform: Transform,
    shoot_cd: ShootCooldown,
}

impl TurretBundle {
    pub fn new(
        pos: Vec2,
        shoot_cd: f32,
        bullet_speed: f32,
        bullet_fac: impl IntoSpawner<BulletParams>,
    ) -> Self {
        TurretBundle {
            turret: Turret {
                bullet_spawner: bullet_fac.into_spawner(),
                bullet_speed,
            },
            transform: Transform::from_translation(pos.extend(0.0)),
            shoot_cd: ShootCooldown(Timer::from_seconds(shoot_cd, TimerMode::Once)),
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
    mut turrets: Query<(&mut ShootCooldown, &Transform, &Target, &Turret)>,
) {
    turrets
        .iter_mut()
        .filter(|entry| entry.0.is_finished())
        .filter_map(|(cd, tf, target, turret)| {
            Some((cd, tf, get_target_pos.run(target).ok()?, turret))
        })
        .for_each(|(mut cd, tf, target, turret)| {
            cd.reset();
            let pos = tf.translation.truncate();
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
