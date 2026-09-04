//! first_shift_03_salvage: the production close-contact crate scene.
//!
//! Cutter starts 70 m abeam of the first crate, outside its pickup envelope.
//! Use a short lateral RCS push to bring the hull visibly alongside the box.

#[path = "shared/first_shift_scene.rs"]
mod preview;

use bevy::prelude::*;
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;
use preview::ShipPose;

const POSES: &[ShipPose] = &[ShipPose {
    id: "cutter",
    position: Meters3::new(-130.0, -60.0, -1_400.0),
    rotation: Quat::IDENTITY,
}];

fn main() -> bevy::app::AppExit {
    preview::run(FirstShiftScene::Salvage, POSES)
}
