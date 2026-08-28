//! `nova_channel`: the process channel - named inputs on stdin, world
//! snapshots on stdout, against the real headless app.
//!
//! One JSON object per line, both directions. Five input lanes (`input`,
//! `aim`, `text`, `key`, `pointer`) plus the bare-`tick` step instruction;
//! back comes `nova_probe`'s world snapshot with the channel's own blocks
//! merged in: `applied` (every consumed line, echoed with its outcome),
//! `input` (what may be pressed THIS tick) and - inside the snapshot itself -
//! `ui` (the screen a GUI player sees, as data). The design record is
//! `tasks/20260820-174148/nova-channel.html`; the executable schema reference
//! is `tasks/20260820-174148/poc/mock_game.py`.
//!
//! ## Where it writes
//!
//! Nothing here interprets the game. The input lane presses what the
//! registry binds ([`nova_input::dispatch`]), the pointer lane drives the
//! autopilot's gestures against the virtual primary window, and the text/key
//! lanes write the keyboard messages the prompt and the fields read - so every
//! condition, modifier and occlusion rule a player faces still runs.
//!
//! ## Wiring
//!
//! The binary installs [`NovaChannelPlugin`] LAST, after `editor_app`, under
//! `--channel <mode>` (debug-only, requires `--norender`): its runner replaces
//! the `ScheduleRunnerPlugin` one the headless builder set, and its writer
//! systems pin to the slots the design record fixed.
#![warn(missing_docs)]

pub mod apply;
pub mod protocol;
pub mod record;
pub mod runner;

use bevy::{
    ecs::message::MessageUpdateSystems, picking::PickingSystems, prelude::*,
    time::TimeUpdateStrategy, window::PrimaryWindow,
};
use bevy_enhanced_input::prelude::EnhancedInputSystems;

use crate::{
    apply::{channel_input_writer, channel_pointer_writer, ChannelAck, ChannelFrame},
    runner::{channel_runner, TICK_DT},
};

/// How the channel gates the clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    /// The clock is the schedule: the world advances only to a line's `tick`,
    /// then answers with a snapshot and waits. EOF is an exit.
    Step,
    /// The app runs at its own pace; lines apply on the next frame, a passed
    /// tick is reported `late`, EOF just closes the lane.
    Free,
}

impl std::str::FromStr for ChannelMode {
    type Err = String;

    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode {
            "step" => Ok(Self::Step),
            "free" => Ok(Self::Free),
            other => Err(format!("no channel mode named `{other}` (step, free)")),
        }
    }
}

/// The whole channel: the virtual window, the two lane writers in their pinned
/// slots, and the stdin/stdout runner.
pub struct NovaChannelPlugin {
    /// How the clock is gated.
    pub mode: ChannelMode,
    /// `Some(dir)` arms the frame recorder: every tick is drawn offscreen and
    /// saved as `dir/frame_%06d.png`. Needs the offscreen assembly
    /// (`AppBuilder::offscreen`) - on the plain headless one there is no GPU
    /// and the screenshots have nothing to read.
    pub record: Option<std::path::PathBuf>,
}

/// The channel's writer systems, for anything that must order against them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct NovaChannelSystems;

impl Plugin for NovaChannelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChannelFrame>();
        app.init_resource::<ChannelAck>();

        // One virtual window buys the whole pointer: picking walks
        // window -> camera -> UI exactly as if a real one existed. A headless
        // app has none; a windowed one keeps its own.
        let has_window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .iter(app.world())
            .next()
            .is_some();
        if !has_window {
            app.world_mut().spawn((
                Window {
                    resolution: (1280, 720).into(),
                    ..default()
                },
                PrimaryWindow,
            ));
        }

        // AFTER the window: the recorder sizes its image to it.
        if let Some(dir) = &self.record {
            record::setup(app, dir.clone());
        }

        // The pointer lane rides the picking backend's own slot - the one the
        // autopilot's cursor pin holds - so a gesture lands the frame it was
        // scheduled for instead of one later.
        app.add_systems(
            First,
            channel_pointer_writer
                .in_set(NovaChannelSystems)
                .after(MessageUpdateSystems)
                .before(PickingSystems::Input),
        );
        // The key/name lanes land after bevy clears the `just_*` edges and
        // before the enhanced-input rigs prepare, the autopilot's slot.
        app.add_systems(
            PreUpdate,
            channel_input_writer
                .in_set(NovaChannelSystems)
                .after(bevy::input::InputSystems)
                .before(EnhancedInputSystems::Prepare),
        );

        if self.mode == ChannelMode::Step {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(TICK_DT));
        }
        app.set_runner(channel_runner(self.mode));
    }
}

/// Glob-import surface: the plugin, its mode, and the wire types a test or a
/// harness names.
pub mod prelude {
    pub use crate::{
        apply::{AckState, AppliedEntry, ChannelAck, ChannelFrame},
        protocol::{parse_line, Envelope, Lane, PointerCmd, PointerTarget},
        record::ChannelRecorder,
        ChannelMode, NovaChannelPlugin, NovaChannelSystems,
    };
}
