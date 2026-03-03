use avian3d::prelude::*;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, apply_carry_force);
}

#[derive(Component)]
#[require(PhysicsPickable)]
#[require(RigidBody::Dynamic)]
pub struct Caryable;

#[derive(Component)]
pub struct CarryStrength(pub f32);

impl Default for CarryStrength {
    fn default() -> Self {
        CarryStrength(100.0)
    }
}

#[derive(Component)]
pub struct CarryInfo {
    pub target: Vec3,
    pub grab_point: Vec3,
}

#[derive(Component)]
#[relationship_target(relationship=Carrying)]
pub struct CarriedBy(Entity);

#[derive(Component)]
#[relationship(relationship_target=CarriedBy)]
#[require(CarryStrength)]
#[require(CarryInfo = panic!("req: CarryInfo") as CarryInfo)]
pub struct Carrying(pub Entity);

fn apply_carry_force(
    carier: Query<(&Carrying, &CarryInfo, &CarryStrength), Without<CarriedBy>>,
    mut caried: Query<(Forces, &GlobalTransform), (With<CarriedBy>, Without<Carrying>)>,
) {
    carier.iter().for_each(|(c, info, strength)| {
        let (mut forces, tf) = caried.get_mut(c.0).expect("relationship");
        let carry_point = tf.transform_point(info.grab_point);
        let delta = info.target - carry_point;
        let force = if delta.length_squared() <= 1.0 {
            delta
        } else {
            delta.normalize()
        } * strength.0;

        forces.apply_force_at_point(force, carry_point);
    });
}
