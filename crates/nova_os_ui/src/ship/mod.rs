//! NOVA OS `ship` app: a terminal-launched schematic 3D viewer of the player
//! ship, plus the arg-bearing `ship` CLI verbs.
//!
//! Bare `ship` launches this app; `ship view` prints the status summary (built in
//! `nova_os.rs`); `ship section <id>` prints one section's detail; `ship reload
//! <id>` / `ship repair <id>` act on a section. Every section is addressed by a
//! short [`SectionCode`] (`HULL-3`, `PDC-1`, `TRB-1`), assigned stably per session
//! from the section kind + a stable index - the real ships use auto grid-coord
//! `EntityId`s (`engine_port`, `fuselage`) that can be long, so the code is the
//! human/CLI/label handle.
//!
//! The viewer follows the `map` app pattern: a dedicated [`Camera3d`] on its own
//! `RenderLayers` renders proxy BLOCKS (built from each section's authored
//! [`SectionCollider`](nova_ship::prelude::SectionCollider) + local transform) into an offscreen image shown in the
//! app body. Each block is a dim, uniform-green fill wrapped in a bright box
//! outline (a `cuboid_edges` wireframe) with a gap, so adjacent sections read
//! apart instead of merging into one green blob; the block colour does NOT encode
//! status. The interactive SECTIONS
//! ride on top as projected clickable UI blips (a nested 3D mesh is not pickable
//! through the CRT composite, but a UI button is - the same reason `map` uses
//! blips); each blip carries a per-kind glyph + its code, an integrity bar
//! (width = HP, colour = status) and, for weapons, ammo pips - that is where a
//! section's status now shows. Orbit the camera with the `novaos_orbit_*`
//! actions + drag + wheel;
//! `[`/`]` cycle the selection; `G` toggles structural mates; `L` reloads, `P`
//! repairs, and `B` replaces the selected bindable section's input.
//!
//! Actions are instant and free for now, but they route through a single
//! `ShipSectionCommand` seam (CLI verb -> [`NovaOsCommandInvocation`], in-app key
//! -> message) so a future queued/over-time, resource-costed model can replace the
//! handler without touching the callers.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `sections` | Section codes, the live section view and the action seam. |
//! | `app` | The `ship` CLI verbs, the app runtime and its side panel. |
//! | `scene` | The schematic 3D scene, its camera and the projected blips. |

mod app;
mod rebind;
mod scene;
mod sections;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::InputBindings;
use nova_os::prelude::*;

pub use self::sections::SectionCode;
pub(crate) use self::{app::*, rebind::*, scene::*, sections::*};
use crate::bindings::hint;

/// Glob-import surface: `use nova_os_ui::ship::prelude::*`.
pub mod prelude {
    // `sections::SectionCode` explicitly: `super::SectionCode` is reachable
    // both `pub` (the re-export above) and `pub(crate)` (the module glob),
    // which rustc rejects as an ambiguous import visibility.
    pub use super::sections::SectionCode;
}

/// The launch word / stable id of the ship app.
const SHIP_APP_ID: &str = "ship";

/// Render layer the ship schematic scene lives on (isolated from the world on 0
/// and the map on 21).
const SHIP_LAYER: usize = 22;
/// The ship camera renders before the NOVA OS RTT (-20); distinct from the map
/// camera (-30) so the two never share an order even mid-teardown.
const SHIP_CAMERA_ORDER: isize = -31;

const SHIP_RADIUS_MIN: f32 = 3.0;
const SHIP_RADIUS_MAX: f32 = 400.0;
const SHIP_THETA_DEFAULT: f32 = 0.7;
const SHIP_PHI_DEFAULT: f32 = 0.5;
/// Exponential ease rate (1/s) for the orbit center chasing the selected
/// section. Frame-rate independent via `1 - exp(-k*dt)`; higher = snappier.
const SHIP_CENTER_EASE: f32 = 9.0;

/// Footer hints while the ship app owns the screen.
///
/// Built per call from the live table, like the map's - see [`crate::map`].
fn ship_hints(bindings: &InputBindings) -> Vec<String> {
    let mut hints = Vec::with_capacity(11);
    hints.extend(hint(
        bindings,
        &["novaos_orbit_left", "novaos_orbit_right"],
        "TURN",
    ));
    hints.extend(hint(
        bindings,
        &["novaos_orbit_up", "novaos_orbit_down"],
        "TILT",
    ));
    hints.push("DRAG: LOOK".to_string());
    hints.push("WHEEL: ZOOM".to_string());
    hints.extend(hint(bindings, &["novaos_next", "novaos_prev"], "SELECT"));
    hints.extend(hint(bindings, &["ship_mates"], "MATES"));
    hints.extend(hint(bindings, &["ship_reload"], "RELOAD"));
    hints.extend(hint(bindings, &["ship_repair"], "REPAIR"));
    hints.extend(hint(bindings, &["ship_rebind"], "REBIND"));
    hints.extend(hint(bindings, &["novaos_reframe"], "RESET"));
    hints.push("ESC: BACK".to_string());
    hints
}

/// Registers the `ship` app + CLI verbs and drives the schematic scene, blips and
/// section actions.
pub(crate) struct NovaOsShipPlugin;

impl Plugin for NovaOsShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();

        // Register the `ship` command tree: bare `ship` launches the app; `ship
        // view` prints the summary (rows filled in nova_os.rs); the arg-bearing
        // verbs dispatch to the gameplay layer (`apply_ship_cli_commands`).
        app.world_mut()
            .resource_mut::<NovaOsCommandRegistry>()
            .register(ship_command_tree());

        // Where `NovaOsShipSystems` sits in the frame is decided by
        // `crate::MonitorFrame`, which is above both apps and the terminal.
        app.add_systems(
            Update,
            (
                assign_section_codes,
                sync_ship_arg_completions,
                apply_ship_cli_commands,
                apply_ship_section_commands,
                manage_ship_scene,
                reconcile_ship_target,
                apply_ship_rebind,
                ship_input,
                drive_ship_camera,
                update_ship_blocks,
                project_ship_blips,
                update_ship_panel,
            )
                .chain()
                .in_set(NovaOsShipSystems),
        );
    }
}

/// The `ship` command tree: the app launch word, the `ship view` snapshot
/// subcommand, and the arg-bearing `section`/`reload`/`repair` gameplay verbs.
/// Shared by the plugin registration and the tests.
fn ship_command_tree() -> TerminalCommand {
    TerminalCommand::app(SHIP_APP_ID, "Open the ship computer", ShipApp)
        .with_subcommand(TerminalCommand::cli(
            "ship view",
            "Print ship status summary",
            CliOutput::Snapshot,
        ))
        .with_subcommand(
            TerminalCommand::gameplay(
                "ship section",
                "Show one section's detail",
                CommandArity::UpTo(1),
            )
            .with_arg_hint("<section>")
            .with_args(&[CommandArg::Live(live::SECTION)]),
        )
        .with_subcommand(
            TerminalCommand::gameplay(
                "ship reload",
                "Reload a weapon section",
                CommandArity::UpTo(1),
            )
            .with_arg_hint("<section>")
            .with_args(&[CommandArg::Live(live::SECTION)]),
        )
        .with_subcommand(
            TerminalCommand::gameplay("ship repair", "Repair a section", CommandArity::UpTo(1))
                .with_arg_hint("<section>")
                .with_args(&[CommandArg::Live(live::SECTION)]),
        )
}

/// System set for the ship app's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaOsShipSystems;

/// Whether the ship app is the active NOVA OS surface right now.
fn ship_is_active(pause: &State<PauseStates>, terminal: &NovaOsTerminal) -> bool {
    *pause.get() == PauseStates::NovaOs
        && terminal.active_mode() == TerminalMode::App { id: SHIP_APP_ID }
}
