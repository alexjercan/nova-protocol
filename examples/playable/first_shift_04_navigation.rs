//! first_shift_04_navigation: the production radar-lock and GOTO lesson.

#[path = "shared/first_shift_scene.rs"]
mod preview;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;
use preview::ShipPose;

const POSES: &[ShipPose] = &[ShipPose {
    id: "cutter",
    position: Meters3::new(200.0, 100.0, -3_000.0),
    rotation: Quat::IDENTITY,
}];

fn main() -> bevy::app::AppExit {
    preview::run(FirstShiftScene::Navigation, POSES)
}
