use crate::plugins::health::*;
use crate::plugins::physics::*;
use crate::plugins::targeting::*;
use avian2d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
#[require(CollisionLayers::new(GameLayer::Units, GameLayer::Default | GameLayer::Units | GameLayer::Bullets | GameLayer::TargetDetection))]
#[require(RigidBody::Dynamic)]
#[require(LinearDamping(0.8))]
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
            collider: Collider::circle(radius),
            transform: Transform::from_translation(pos.extend(0.0)),
            health: Health::new(health),
        }
    }
}

fn move_towards_target(
    // helpers
    get_target_pos: GetTargetPos,
    // queries
    mut units: Query<(&mut LinearVelocity, &Transform, &Target)>,
) {
    const MOVE_SPEED: f32 = 100.0;

    units
        .iter_mut()
        .filter_map(|(vel, tf, target)| get_target_pos.run(target).ok().map(|pos| (vel, tf, pos)))
        .for_each(|(mut vel, tf, target_pos)| {
            let dir = (target_pos - tf.translation.truncate()).normalize();
            vel.0 = dir * MOVE_SPEED;
        });
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, move_towards_target);
}
