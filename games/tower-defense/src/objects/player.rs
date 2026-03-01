use avian2d::prelude::*;
use bevy::prelude::*;

use crate::plugins::physics::GameLayer;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, move_player);
}

#[derive(Component)]
#[require(CollisionLayers::new(GameLayer::Units, GameLayer::Default | GameLayer::Units | GameLayer::Bullets | GameLayer::TargetDetection))]
#[require(RigidBody::Dynamic)]
pub struct Player {
    speed: f32,
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    collider: Collider,
    transform: Transform,
    velocity: LinearVelocity,
}

impl PlayerBundle {
    pub fn new(pos: Vec2, radius: f32, speed: f32) -> Self {
        PlayerBundle {
            player: Player { speed },
            collider: Collider::circle(radius),
            transform: Transform::from_translation(pos.extend(0.0)),
            velocity: LinearVelocity(Vec2::ZERO),
        }
    }
}

fn move_player(
    mut player: Single<(&mut LinearVelocity, &Player)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let direction = IVec2::new(
        keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32,
        keys.pressed(KeyCode::KeyW) as i32 - keys.pressed(KeyCode::KeyS) as i32,
    )
    .as_vec2()
    .normalize_or_zero();

    player.0.0 = direction * player.1.speed;
}
