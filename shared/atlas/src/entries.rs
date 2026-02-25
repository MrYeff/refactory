use serde::Deserialize;
use serde_with::serde_as;

use super::*;
use std::time::Duration;

#[serde_as]
#[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
pub struct AnimationDefinition {
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    frame_duration: Duration,
    seq: FrameSequence,
}

#[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
pub struct VariantDefinition {
    seq: FrameSequence,
}

#[derive(Asset, Debug, Clone, Component, TypePath, Deserialize)]
pub struct RotationDefinition {
    rotation_count: GridRotations,
    seq: FrameSequence,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GridRotations {
    /// Expects 2 frames: one for 0°+180° and one for 90°+270°
    #[serde(rename = "per_axis")]
    PerAxis,
    /// Expects 4 frames: one for each 90° rotation
    #[serde(rename = "all")]
    All,
}
