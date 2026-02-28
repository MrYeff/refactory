use crate::plugins::physics::*;
use avian2d::prelude::*;
use bevy::ecs::query::QueryEntityError;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

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

#[derive(Component, Deref)]
#[require(DetectedTargets)]
#[relationship_target(relationship=TargetDetectorOf)]
pub struct TargetDetectors(Vec<Entity>);

#[derive(Component, Deref, DerefMut)]
#[require(CollisionEventsEnabled)]
#[require(CollisionLayers::new(GameLayer::TargetDetection, GameLayer::Units))]
#[relationship(relationship_target=TargetDetectors)]
pub struct TargetDetectorOf(Entity);

#[derive(Bundle)]
pub struct TargetDetectorBundle {
    target_detector: TargetDetectorOf,
    collider: Collider,
}

impl TargetDetectorBundle {
    pub fn new(target: Entity, radius: f32) -> Self {
        TargetDetectorBundle {
            target_detector: TargetDetectorOf(target),
            collider: Collider::circle(radius),
        }
    }
}

fn detect_targets(
    // queries
    detectors: Query<&TargetDetectorOf, With<Collider>>,
    mut detected: Query<&mut DetectedTargets>,
    // events
    mut col_started: MessageReader<CollisionStart>,
    mut col_ended: MessageReader<CollisionEnd>,
) {
    let swap = |a: Entity, b: Entity| -> Option<(Entity, Entity)> {
        match () {
            _ if let Ok(det) = detectors.get(a) => Some((det.0, b)),
            _ if let Ok(det) = detectors.get(b) => Some((det.0, a)),
            _ => None,
        }
    };

    col_started
        .read()
        .filter_map(|col| swap(col.collider1, col.collider2))
        .for_each(|(detector_of, target)| {
            let mut targets = detected.get_mut(detector_of).expect("required");
            targets.0.insert(target);
        });

    col_ended
        .read()
        .filter_map(|col| swap(col.collider1, col.collider2))
        .for_each(|(detector_of, target)| {
            let mut targets = detected.get_mut(detector_of).expect("required");
            targets.0.remove(&target);
        });
}

fn select_first_target(
    mut commands: Commands,
    // queries
    detected: Query<(Entity, &DetectedTargets)>,
) {
    detected
        .iter()
        .filter_map(|(e, targets)| targets.iter().next().map(|t| (e, *t)))
        .for_each(|(e, target)| {
            commands.entity(e).insert(Target(target));
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

pub fn plugin(app: &mut App) {
    app.add_systems(Update, detect_targets);
    app.add_systems(Update, select_first_target);
    app.add_systems(Update, remove_target_if_not_detected);
}
