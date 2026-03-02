use avian2d::prelude::*;
use bevy::prelude::*;

use crate::plugins::health::*;
use crate::plugins::physics::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, detect_hit);
}

#[derive(Component)]
#[require(RigidBody::Dynamic)]
#[require(CollisionEventsEnabled)]
#[require(Sensor)]
#[require(CollisionLayers::new(GameLayer::Bullets, GameLayer::Default | GameLayer::Units))]
pub struct Bullet {
    damage: u32,
}

#[derive(Bundle)]
pub struct BulletBundle {
    bullet: Bullet,
    transform: Transform,
    collider: Collider,
    velocity: LinearVelocity,
    mass_properties: MassPropertiesBundle,
}

impl BulletBundle {
    pub fn new(damage: u32, radius: f32, pos: Vec2, vel: Vec2) -> Self {
        BulletBundle {
            bullet: Bullet { damage },
            transform: Transform::from_translation(pos.extend(0.0)),
            collider: Collider::circle(radius),
            velocity: LinearVelocity(vel),
            mass_properties: MassPropertiesBundle {
                mass: 1.0.into(),
                angular_inertia: 1.0.into(),
                center_of_mass: default(),
            },
        }
    }
}

pub struct BulletParams {
    pub pos: Vec2,
    pub vel: Vec2,
}

fn detect_hit(
    mut commands: Commands,
    // queries
    bullets: Query<&Bullet>,
    // events
    mut col_started: MessageReader<CollisionStart>,
) {
    let swap = |a: Entity, b: Entity| -> Option<((Entity, &Bullet), Entity)> {
        match () {
            _ if let Ok(bullet) = bullets.get(a) => Some(((a, bullet), b)),
            _ if let Ok(bullet) = bullets.get(b) => Some(((b, bullet), a)),
            _ => None,
        }
    };

    col_started
        .read()
        .filter_map(|col| swap(col.collider1, col.collider2))
        .for_each(|(bullet, target)| {
            commands.entity(bullet.0).try_despawn();
            commands.trigger(DealDamageEvent {
                target,
                damage: bullet.1.damage,
            });
        });
}
