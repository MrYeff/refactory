use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::plugins::targeting::DetectionFilter;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        update_caried_by_position.before(TransformSystems::Propagate),
    );
}

#[derive(SystemParam)]
pub struct CaryableFilter<'w, 's> {
    caryable: Query<'w, 's, (), With<Caryable>>,
}

impl DetectionFilter for CaryableFilter<'_, '_> {
    fn is_hit(&self, _detector: Entity, candidate: Entity) -> bool {
        self.caryable.contains(candidate)
    }
}
#[derive(Component)]
pub struct Caryable;

#[derive(Component)]
#[relationship_target(relationship=Carrying)]
pub struct CarriedBy(Entity);

#[derive(Component)]
#[relationship(relationship_target=CarriedBy)]
pub struct Carrying(pub Entity);

fn update_caried_by_position(
    carier: Query<(&Transform, &Carrying), Without<CarriedBy>>,
    mut caried: Query<&mut Transform, (With<CarriedBy>, Without<Carrying>)>,
) {
    carier.iter().for_each(|(carier_tf, c)| {
        let mut caried_tf = caried.get_mut(c.0).expect("relationship");
        caried_tf.translation = carier_tf.translation + Vec3::Z * 1.0;
    });
}
