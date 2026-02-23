use super::*;
use std::time::Duration;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
struct AtlasEntryAnimationDefinition {
    frame_duration: Duration,
    seq: FrameSequence,
}

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
struct AtlasEntryRotationDefinition {
    rotation_count: GridRotations,
    seq: FrameSequence,
}

#[derive(Debug, Clone, Reflect)]
enum GridRotations {
    None,
    VerticalHorizontal,
    All,
}

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
struct AtlasEntryVariantDefinition {
    seq: FrameSequence,
}
