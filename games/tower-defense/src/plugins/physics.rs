use std::ops::BitOr;

use avian3d::prelude::*;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity(Vec3::new(0.0, -10.0, 0.0)));
}

#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Bullets,
    Units,
    TargetDetection,
    SensorTarget,
}

impl BitOr for GameLayer {
    type Output = u32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.to_bits() | rhs.to_bits()
    }
}

impl BitOr<u32> for GameLayer {
    type Output = u32;

    fn bitor(self, rhs: u32) -> Self::Output {
        self.to_bits() | rhs
    }
}

impl BitOr<GameLayer> for u32 {
    type Output = u32;

    fn bitor(self, rhs: GameLayer) -> Self::Output {
        self | rhs.to_bits()
    }
}
