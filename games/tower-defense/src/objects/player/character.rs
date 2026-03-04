use avian3d::prelude::*;
use bevy::prelude::*;

use crate::plugins::physics::GameLayer;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, move_player);
}

#[derive(Component)]
#[require(Name::new("Player"))]
#[require(CollisionLayers::new(GameLayer::Units, GameLayer::Default | GameLayer::Units | GameLayer::Bullets | GameLayer::TargetDetection))]
#[require(RigidBody::Dynamic)]
#[require(LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z())]
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
            collider: Collider::capsule(radius, 2.0),
            transform: Transform::from_translation(Vec3::new(pos.x, 0.0, pos.y)),
            velocity: LinearVelocity(Vec3::ZERO),
        }
    }
}

fn move_player(
    mut player: Single<(&mut LinearVelocity, &mut Transform, &Player)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let dir = Dir2::new(
        IVec2::new(
            keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32,
            keys.pressed(KeyCode::KeyS) as i32 - keys.pressed(KeyCode::KeyW) as i32,
        )
        .as_vec2(),
    );

    if let Ok(dir) = dir {
        player.0.x = dir.x * player.2.speed;
        player.0.z = dir.y * player.2.speed;
    } else {
        player.0.x = 0.0;
        player.0.z = 0.0;
    }
}
