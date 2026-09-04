//! first_shift_05_orbit: the production planetoid detour and orbit scene.

#[path = "shared/first_shift_scene.rs"]
mod preview;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;
use preview::ShipPose;

// Parked 2.5 km off the inspection body on the +Z side, so the default -Z
// facing frames the planetoid this chapter is built around. The old pose sat
// 13 km out with the body BEHIND the camera: it exercised the scene but showed
// nothing of it, which is no use as a review shot.
const POSES: &[ShipPose] = &[ShipPose {
    id: "cutter",
    position: Meters3::new(-4_500.0, 100.0, -4_000.0),
    rotation: Quat::IDENTITY,
}];

fn main() -> bevy::app::AppExit {
    preview::run(FirstShiftScene::Orbit, POSES)
}
