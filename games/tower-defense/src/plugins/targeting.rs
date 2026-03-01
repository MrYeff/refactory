use std::marker::PhantomData;

use crate::plugins::physics::*;
use avian2d::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (select_first_target, select_nearest_target));
    app.add_systems(Update, remove_target_if_not_detected);
}

#[derive(Component, Deref, DerefMut)]
#[component(storage = "SparseSet")]
pub struct Target(pub Entity);

#[derive(SystemParam)]
pub struct GetTargetPos<'w, 's> {
    targets: Query<'w, 's, &'static Transform>,
}

impl<'w, 's> GetTargetPos<'w, 's> {
    pub fn run(&self, target: &Target) -> Result<Vec2, QueryEntityError> {
        self.targets.get(target.0).map(|t| t.translation.truncate())
    }
}

#[derive(Component, Default, Deref, DerefMut, Debug)]
pub struct DetectedTargets(HashSet<Entity>);

#[derive(Component, Deref, DerefMut)]
#[require(CollisionEventsEnabled)]
#[require(CollisionLayers::new(GameLayer::TargetDetection, GameLayer::Units))]
#[component(on_add=Self::on_add)]
pub struct TargetDetectorOf<F: 'static>(#[deref] Entity, PhantomData<fn() -> F>);

impl<F: 'static> TargetDetectorOf<F> {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        #[cfg(debug_assertions)]
        {
            if !world.contains_resource::<DetectionFilterMarker<F>>() {
                panic!(
                    "A TargetDetectorOf<{}> was added, but no corresponding DetectionFilter was registered. Make sure to call .add_detectiion_filter<{}>() when building the app.",
                    std::any::type_name::<F>(),
                    std::any::type_name::<F>()
                );
            }
        }

        let detector_of = world
            .get_entity(ctx.entity)
            .expect("self")
            .get::<Self>()
            .expect("self")
            .0;

        world
            .commands()
            .entity(detector_of)
            .insert_if_new(DetectedTargets::default());
    }
}

#[derive(Bundle)]
pub struct TargetDetectorBundle<F: 'static> {
    target_detector: TargetDetectorOf<F>,
    collider: Collider,
}

impl<F: 'static> TargetDetectorBundle<F> {
    pub fn new(target: Entity, radius: f32) -> Self {
        TargetDetectorBundle {
            target_detector: TargetDetectorOf(target, default()),
            collider: Collider::circle(radius),
        }
    }
}

#[derive(Resource)]
struct DetectionFilterMarker<F>(PhantomData<fn() -> F>);

pub trait AppExt {
    fn add_detectiion_filter<F>(&mut self) -> &mut Self
    where
        F: SystemParam + 'static,
        for<'w, 's> <F as SystemParam>::Item<'w, 's>: DetectionFilter;
}

impl AppExt for App {
    fn add_detectiion_filter<F>(&mut self) -> &mut Self
    where
        F: SystemParam + 'static,
        for<'w, 's> <F as SystemParam>::Item<'w, 's>: DetectionFilter,
    {
        if self.world().contains_resource::<DetectionFilterMarker<F>>() {
            return self;
        }

        self.insert_resource(DetectionFilterMarker::<F>(default()));
        self.add_systems(Update, detect_targets::<F>);

        self
    }
}

pub trait DetectionFilter: SystemParam {
    fn is_hit(&self, detector: Entity, candidate: Entity) -> bool;
}

fn detect_targets<F>(
    f: StaticSystemParam<F>,
    // queries
    detectors: Query<&TargetDetectorOf<F>, With<Collider>>,
    mut detections: Query<&mut DetectedTargets>,
    // events
    mut col_started: MessageReader<CollisionStart>,
    mut col_ended: MessageReader<CollisionEnd>,
) where
    F: SystemParam + 'static,
    for<'w, 's> <F as SystemParam>::Item<'w, 's>: DetectionFilter,
{
    let f = f.into_inner();
    let swap = |a: Entity, b: Entity| -> Option<(Entity, Entity)> {
        match () {
            _ if let Ok(det) = detectors.get(a)
                && f.is_hit(a, b) =>
            {
                Some((det.0, b))
            }
            _ if let Ok(det) = detectors.get(b)
                && f.is_hit(b, a) =>
            {
                Some((det.0, a))
            }
            _ => None,
        }
    };

    col_started
        .read()
        .filter_map(|col| swap(col.collider1, col.collider2))
        .for_each(|(detector_of, target)| {
            let mut targets = detections.get_mut(detector_of).expect("required");
            targets.0.insert(target);
        });

    col_ended
        .read()
        .filter_map(|col| swap(col.collider1, col.collider2))
        .for_each(|(detector_of, target)| {
            let mut targets = detections.get_mut(detector_of).expect("required");
            targets.0.remove(&target);
        });
}

fn remove_target_if_not_detected(
    mut commands: Commands,
    // queries
    detected: Query<(Entity, &DetectedTargets, &Target)>,
) {
    detected
        .iter()
        .filter(|(_, targets, target)| !targets.contains(&target.0))
        .for_each(|(e, _, _)| {
            commands.entity(e).remove::<Target>();
        });
}

#[derive(Component, Default)]
pub enum TargettingStrategy {
    #[default]
    First,
    Nearest,
    None,
}

fn select_first_target(
    mut commands: Commands,
    // queries
    detected: Query<(Entity, &DetectedTargets, &TargettingStrategy)>,
) {
    detected
        .iter()
        .filter(|(_, _, strategy)| matches!(**strategy, TargettingStrategy::First))
        .filter_map(|(e, targets, _)| targets.iter().next().map(|t| (e, *t)))
        .for_each(|(e, target)| {
            commands.entity(e).insert(Target(target));
        });
}

fn select_nearest_target(
    mut commands: Commands,
    // helpers
    get_target_pos: GetTargetPos,
    // queries
    detected: Query<(Entity, &DetectedTargets, &Transform, &TargettingStrategy)>,
) {
    detected
        .iter()
        .filter(|(_, _, _, strategy)| matches!(**strategy, TargettingStrategy::Nearest))
        .filter_map(|(e, targets, tf, _)| {
            targets
                .iter()
                .filter_map(|t| get_target_pos.run(&Target(*t)).ok().map(|pos| (t, pos)))
                .min_by_key(|(_, pos)| {
                    let dist = (*pos - tf.translation.truncate()).length_squared();
                    (dist * 1000.0) as u32;
                })
                .map(|(t, _)| (e, *t))
        })
        .for_each(|(e, target)| {
            commands.entity(e).insert(Target(target));
        });
}
