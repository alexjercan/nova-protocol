//! first_shift_03_salvage: the production close-work crate scene.

#[path = "shared/first_shift_scene.rs"]
mod preview;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;
use preview::ShipPose;

const POSES: &[ShipPose] = &[ShipPose {
    id: "cutter",
    position: Meters3::new(-500.0, 80.0, 900.0),
    rotation: Quat::IDENTITY,
}];

fn main() -> bevy::app::AppExit {
    preview::run(FirstShiftScene::Salvage, POSES)
}
