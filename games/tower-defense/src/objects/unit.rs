use avian3d::prelude::*;
use bevy::prelude::*;

use crate::plugins::health::*;
use crate::plugins::physics::*;
use crate::plugins::targeting::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, move_towards_target);
}

#[derive(Component)]
#[require(Name::new("Unit"))]
#[require(CollisionLayers::new(GameLayer::Units, GameLayer::Default | GameLayer::Units | GameLayer::Bullets | GameLayer::TargetDetection))]
#[require(RigidBody::Dynamic)]
#[require(LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z())]
pub struct Unit;

#[derive(Bundle)]
pub struct UnitBundle {
    unit: Unit,
    collider: Collider,
    transform: Transform,
    health: Health,
}

impl UnitBundle {
    pub fn new(pos: Vec2, radius: f32, health: u32) -> Self {
        UnitBundle {
            unit: Unit,
            collider: Collider::capsule(radius, 2.0),
            transform: Transform::from_translation(Vec3::new(pos.x, 0.0, pos.y)),
            health: Health::new(health),
        }
    }
}

fn move_towards_target(
    // helpers
    get_target_pos: GetTargetPos,
    // queries
    mut units: Query<(&mut LinearVelocity, &Transform, &Target), With<Unit>>,
) {
    const MOVE_SPEED: f32 = 10.0;

    units
        .iter_mut()
        .filter_map(|(vel, tf, target)| get_target_pos.run(target).ok().map(|pos| (vel, tf, pos)))
        .for_each(|(mut vel, tf, target_pos)| {
            let dir = (target_pos - tf.translation).xz().normalize();
            vel.0.x = dir.x * MOVE_SPEED;
            vel.0.z = dir.y * MOVE_SPEED;
        });
}
