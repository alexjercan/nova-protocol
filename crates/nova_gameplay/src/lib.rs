//! `nova_gameplay` is the shared gameplay layer: what every playable thing in
//! the world is made of, with no knowledge of the ship built on top of it.
//! `NovaGameplayPlugin` composes it and owns the top-level [`GameStates`] state
//! machine and the physics, entropy and particle registrations its peers build
//! on. The modules are `integrity` and `damage` (health, disable, destroy),
//! `gravity` (gravity wells), `audio` (the SFX engine), `juice` (combat
//! feedback) over the reusable trauma rig in `shake` and the burst in
//! `impact_spark`, `objectives` (the mission
//! objective list and its conveyance tags), `mesh` and `transform` (the mesh
//! toolkit and the rotation/orbit rigs), `markers` and `projectile_hooks` (the
//! entity vocabulary the layers above tag with), `lifetime` and `cooldown`
//! (transient entities and the countdowns that gate actions), `math`,
//! `relations`, `beacon`, `asset_ref` and `settings` (volume + graphics
//! presets), `transient_light` (the capped brief lights combat throws) and
//! `soft_dot` (the round mask every glowing billboard is drawn through). Nova
//! owns all of it, engine layers included: health, damage and destruction
//! (`integrity`), the transform rigs, the mesh toolkit and SFX playback were
//! vendored in from the shared-helpers crate and are nova's to shape now.
//!
//! The ship itself - `sections`, `input`, `flight`, `camera`, `physics` and the
//! ship's soundtrack - is the peer crate `nova_ship`, which depends on this one.
#![warn(missing_docs)]

use bevy::prelude::*;

pub mod asset_ref;
pub mod audio;
pub mod beacon;
pub mod cooldown;
pub mod damage;
pub mod gravity;
pub mod impact_spark;
pub mod integrity;
pub mod juice;
pub mod lifetime;
pub mod markers;
pub mod math;
pub mod mesh;
pub mod objectives;
pub mod plugin;
pub mod projectile_hooks;
pub mod relations;
pub mod rounds;
pub mod settings;
pub mod shake;
pub mod soft_dot;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod transform;
pub mod transient_light;

/// Test-only helper for asserting on log output: a shared in-memory sink
/// installed as the thread's tracing subscriber. `EntityCommands::remove` and
/// `despawn` bake in the WARN handler at queue time (bevy_ecs
/// commands/mod.rs `queue_handled(_, warn)`), so a `FallbackErrorHandler`
/// swap can never see them - a "no stale command" regression test must
/// assert on the log itself.
///
/// Behind the same `test-support` feature as [`test_support`], and for the same
/// reason: the crates split out of this one assert on the same log.
#[cfg(any(test, feature = "test-support"))]
pub mod test_log {
    /// Cloneable in-memory log sink; every clone shares the same buffer.
    #[derive(Clone, Default)]
    pub struct CapturedLog(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl CapturedLog {
        /// Everything written to the sink so far, as a lossy UTF-8 string.
        pub fn contents(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
        /// Drops everything written so far.
        pub fn clear(&self) {
            self.0.lock().unwrap().clear();
        }
    }

    impl std::io::Write for CapturedLog {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

/// Glob-import surface: `use nova_gameplay::prelude::*` re-exports the public API
/// of this crate's submodules plus the top-level game-state enums.
pub mod prelude {
    // Re-export BY NAME, never by glob. A glob over a vendored engine prelude
    // used to stand here, and it dragged in the retired harness twins -
    // `AutopilotPlugin`, `AutopilotLoop`, `HarnessCompletion` - which shadow
    // nova's harness
    // at every example's `use nova_protocol::prelude::*` and boot the example
    // INERT (task 20260802-183403). The glob is gone with the dependency, but
    // the lesson outlives it: adding a name below is a decision.
    pub use super::{
        asset_ref::prelude::*, audio::prelude::*, beacon::prelude::*, cooldown::prelude::*,
        damage::prelude::*, gravity::prelude::*, impact_spark::prelude::*, integrity::prelude::*,
        juice::prelude::*, lifetime::prelude::*, markers::prelude::*, math::prelude::*,
        mesh::prelude::*, objectives::prelude::*, plugin::prelude::*, projectile_hooks::prelude::*,
        relations::prelude::*, rounds::prelude::*, settings::prelude::*, shake::prelude::*,
        soft_dot::prelude::*, transform::prelude::*, transient_light::prelude::*, EscapeOwner,
        GameMode, GameStates, PauseStates,
    };
}

/// Top-level game lifecycle state.
///
/// `Loading` while assets load, `MainMenu` while the main menu (owned by `nova_menu`)
/// is up, `Playing` once the game is running. Apps without the menu (examples that
/// supply their own game plugins) go straight `Loading -> Playing`. Lives here (the
/// foundational gameplay crate) so the wiring layer (`nova_core`), the editor
/// (`nova_editor`) and the menu (`nova_menu`) can gate on it without depending on
/// each other.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameStates {
    #[default]
    /// Assets are still loading; no menu or gameplay yet.
    Loading,
    /// The `nova_menu` main menu is up.
    MainMenu,
    /// The game is running.
    Playing,
}

/// Whether gameplay is frozen behind a modal overlay. Owned UI-wise by
/// `nova_menu` (ESC toggle + overlay) and `nova_gameplay`'s Tab ship-computer
/// NOVA OS; `nova_gameplay` gates the spaceship input/section system sets on
/// `Unpaused`, and the clocks (`Time<Virtual>` + `Time<Physics>`) pause on
/// entering any frozen variant. Init'd by `AppBuilder` next to [`GameStates`].
/// Only meaningful inside `GameStates::Playing`; leaving Playing must reset it.
///
/// Both frozen variants ([`PauseStates::Paused`] and [`PauseStates::NovaOs`])
/// are entered ONLY from [`PauseStates::Unpaused`] and exit back to it - never
/// one directly into the other - so the freeze/cursor hooks never
/// double-fire.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum PauseStates {
    #[default]
    /// Gameplay is running; input and section systems tick.
    Unpaused,
    /// Gameplay is frozen behind the pause overlay; the clocks are stopped.
    Paused,
    /// Gameplay is frozen behind the Tab ship-computer NOVA OS; the clocks are
    /// stopped and the cursor is freed, exactly like [`PauseStates::Paused`]
    /// but without the pause menu.
    NovaOs,
}

/// Whether a scene-local surface owns Escape right now, so the pause menu must
/// leave the key alone.
///
/// Escape is a BACK gesture before it is a pause gesture: in the editor it
/// leaves the parts gallery, or puts down the armed part, and only when there
/// is nothing left to back out of does it mean "pause". The scene that owns the
/// surface is the only code that knows which of those is true, but it lives
/// downstream of the pause menu and cannot reach into it - so it declares
/// ownership here instead, in `PreUpdate`, and the pause toggle reads it in
/// `Update`. A state flag rather than a per-frame claim: a claim would race the
/// toggle it is meant to suppress.
///
/// Whoever sets it is responsible for clearing it, which is why the editor
/// writes the answer unconditionally every frame rather than only when it
/// changes.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct EscapeOwner(pub bool);

impl PauseStates {
    /// True when gameplay is frozen (any
    /// non-[`Unpaused`](PauseStates::Unpaused) variant): the clocks are stopped
    /// and input observers must suppress. This is the predicate the ~18
    /// observer self-guards use instead of comparing against a single variant,
    /// so a new frozen overlay is covered without re-auditing every guard.
    pub fn is_frozen(&self) -> bool {
        *self != PauseStates::Unpaused
    }
}

/// Which game the main menu handed off to when it set [`GameStates::Playing`].
///
/// `Sandbox` is the default so apps that skip the menu keep the pre-menu behavior
/// (the editor comes up on entering `Playing`). Initialized by `NovaGameplayPlugin`;
/// written by the menu buttons, read on `OnEnter(GameStates::Playing)` by the editor
/// (enter only in `Sandbox`) and the menu's New Game loader (only in `NewGame`).
#[derive(Resource, Clone, Copy, Eq, PartialEq, Debug, Hash, Default, Reflect)]
#[reflect(Resource)]
pub enum GameMode {
    /// Ship editor plus its sandbox scenario (the default game).
    #[default]
    Sandbox,
    /// Jump straight into a ready-to-play scenario.
    NewGame,
}
