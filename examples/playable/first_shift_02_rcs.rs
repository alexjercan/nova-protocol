//! first_shift_02_rcs: the production four-mark RCS briefing and route.
//!
//! Cutter's stopped work-mark pose is preview-only. The production scene owns
//! the map, cast, dialogue, camera, objectives, and route progression.
//!
//! ```text
//! cargo run --example first_shift_02_rcs --features debug
//! ```

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
    preview::run(FirstShiftScene::Rcs, POSES)
}
