use avian2d::prelude::{mass_properties::components::RecomputeMassProperties, *};
use bevy::prelude::*;

use crate::plugins::{
    cary::{Carrying, CaryableFilter},
    physics::GameLayer,
    targeting::*,
};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (move_player, pickup_or_drop))
        .add_detectiion_filter::<CaryableFilter>();
}

#[derive(Component)]
#[require(CollisionLayers::new(GameLayer::Units, GameLayer::Default | GameLayer::Units | GameLayer::Bullets | GameLayer::TargetDetection))]
#[require(RigidBody::Dynamic)]
#[require(LockedAxes = LockedAxes::new().lock_rotation())]
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
    mut player: Single<(&mut LinearVelocity, &mut Transform, &Player)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let dir = Dir2::new(
        IVec2::new(
            keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32,
            keys.pressed(KeyCode::KeyW) as i32 - keys.pressed(KeyCode::KeyS) as i32,
        )
        .as_vec2(),
    );

    if let Ok(dir) = dir {
        player.0.0 = dir * player.2.speed;
        // player.1.look_to(dir.extend(0.0), Vec3::Y);
    } else {
        player.0.0 = Vec2::ZERO;
    }
}

fn pickup_or_drop(
    mut commands: Commands,
    player: Single<(Entity, Has<Carrying>, Option<&Target>), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let (player, is_carying, target) = *player;

    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }

    match is_carying {
        true => {
            commands
                .entity(player)
                .remove::<Carrying>()
                .insert(RecomputeMassProperties);
        }
        false => {
            if let Some(target) = target {
                commands.entity(player).insert(Carrying(target.0));
            }
        }
    }
}
