use avian3d::prelude::*;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_carry_started);
}

#[derive(Bundle)]
pub struct CarryBundle {
    transform: Transform,
    carry: Carry,
    carrying: Carrying,
}

impl CarryBundle {
    pub fn new(pos: Vec2, entity: Entity, handle_at: Vec3) -> Self {
        CarryBundle {
            transform: Transform::from_translation(Vec3::new(pos.x, 0.0, pos.y)),
            carry: Carry {
                handlet_at: handle_at,
            },
            carrying: Carrying(entity),
        }
    }
}

#[derive(Component, Default)]
#[require(PhysicsPickable)]
#[require(RigidBody::Dynamic)]
pub struct Carryable;

/// The point where this object should be carried to
#[derive(Component, Default)]
#[require(Transform)]
#[require(RigidBody::Kinematic)]
pub struct Carry {
    handlet_at: Vec3,
}

#[derive(Component)]
#[relationship(relationship_target=CarriedBy)]
#[require(Carry)]
pub struct Carrying(pub Entity);

#[derive(Component)]
#[relationship_target(relationship=Carrying)]
#[require(Carryable)]
#[component(storage = "SparseSet")]
pub struct CarriedBy(Entity);

fn handle_carry_started(
    tr: On<Add, Carrying>,
    mut commands: Commands,
    carry: Query<(&Carry, &Carrying)>,
) {
    const COMPLIANCE: f32 = 0.002; // 0 = rigid, larger = softer spring
    const DAMPING_LINEAR: f32 = 0.25; // larger = less oscillation
    const RES_LEN: f32 = 0.0;

    let (carry, carrying) = carry.get(tr.entity).expect("relationship");

    commands.entity(tr.entity).with_child((
        DistanceJoint::new(tr.entity, carrying.0)
            .with_limits(RES_LEN, RES_LEN)
            .with_compliance(COMPLIANCE)
            .with_local_anchor2(carry.handlet_at),
        JointDamping {
            linear: DAMPING_LINEAR,
            angular: 0.0,
        },
    ));
}
