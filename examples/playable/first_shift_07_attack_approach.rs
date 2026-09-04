//! first_shift_07_attack_approach: the production reveal, approach, and alignment scene.

#[path = "shared/first_shift_scene.rs"]
mod preview;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;
use preview::ShipPose;

const POSES: &[ShipPose] = &[ShipPose {
    id: "cutter",
    position: Meters3::new(2_000.0, -600.0, 2_400.0),
    rotation: Quat::IDENTITY,
}];

fn main() -> bevy::app::AppExit {
    preview::run(FirstShiftScene::AttackApproach, POSES)
}
