use bevy::prelude::*;

use crate::GetMouseWorldPos;
use crate::plugins::cary::*;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_carry_started)
        .add_observer(handle_carying_dragged)
        .add_observer(handle_cary_ended);
}

fn handle_carry_started(
    tr: On<Pointer<DragStart>>,
    mut commands: Commands,
    carryable: Query<(), (With<Carryable>, Without<CarriedBy>)>,
    get_mouse_world_pos: GetMouseWorldPos,
) {
    let Some(target) = get_mouse_world_pos.run() else {
        return;
    };

    if !carryable.contains(tr.entity) {
        return;
    }

    commands.spawn(CarryBundle::new(target, tr.entity, Vec3::ZERO));
}

fn handle_carying_dragged(
    tr: On<Pointer<Drag>>,
    carried: Query<&CarriedBy>,
    mut hands: Query<&mut Transform>,
    get_mouse_world_pos: GetMouseWorldPos,
) {
    let Some(target) = get_mouse_world_pos.run() else {
        return;
    };

    let Ok(hand) = carried.get(tr.entity) else {
        return;
    };

    let mut tf = hands.get_mut(*hand.collection()).expect("relationship");

    tf.translation = Vec3::new(target.x, 0.0, target.y);
}

fn handle_cary_ended(tr: On<Pointer<DragEnd>>, mut commands: Commands, carried: Query<&CarriedBy>) {
    if let Ok(hand) = carried.get(tr.entity) {
        commands.entity(*hand.collection()).despawn();
    };
}
