//! first_shift_05_orbit: the production planetoid detour and orbit scene.

#[path = "shared/first_shift_scene.rs"]
mod preview;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;
use preview::ShipPose;

const POSES: &[ShipPose] = &[ShipPose {
    id: "cutter",
    position: Meters3::new(-5_200.0, 200.0, -2_600.0),
    rotation: Quat::IDENTITY,
}];

fn main() -> bevy::app::AppExit {
    preview::run(FirstShiftScene::Orbit, POSES)
}
