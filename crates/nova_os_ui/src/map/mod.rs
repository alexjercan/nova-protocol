//! NOVA OS `map` app: a terminal-launched schematic 3D minimap of local space.
//!
//! This is the visual counterpart of the `map view` CLI built-in. Both read the
//! same `MapContacts` model (player, allies, enemies, asteroids, objective
//! markers with live range/bearing). The app renders a small schematic 3D
//! scene - concentric distance rings and a central hub - through a dedicated
//! [`Camera3d`] on its own `RenderLayers` into an offscreen image, shown in the
//! app body; the interactive CONTACTS ride on top as projected clickable UI
//! blips (a nested 3D mesh would not be pickable through the NOVA OS CRT
//! composite, but UI buttons are).
//!
//! The camera is a `MapOrbit` you drive with the `novaos_orbit_*` and
//! `novaos_pan_*` actions plus the wheel (zoom). Selecting a contact fills a
//! readout with kind / name / range / bearing; `G` sets a flight [`Autopilot`](nova_ship::prelude::Autopilot)
//! GOTO on the player ship that persists after the computer closes.
//!
//! The app trait ([`NovaOsAppRuntime`]) only hands apps discrete key presses and
//! no mouse, so all of the interaction runs as this module's OWN systems, gated
//! on the map app being the active NOVA OS surface.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `contacts` | The shared contact model and its terminal rows. |
//! | `app` | The `map` CLI verbs, the app runtime and its readout. |
//! | `scene` | The schematic 3D scene, its camera and the projected blips. |

mod app;
mod contacts;
mod scene;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::InputBindings;
use nova_os::prelude::*;

pub use self::contacts::MapContactCode;
pub(crate) use self::{app::*, contacts::*, scene::*};
use crate::bindings::hint;

/// Glob-import surface: `use nova_os_ui::map::prelude::*`.
pub mod prelude {
    // `contacts::MapContactCode` explicitly: `super::MapContactCode` is
    // reachable both `pub` (the re-export above) and `pub(crate)` (the module
    // glob), which rustc rejects as an ambiguous import visibility.
    pub use super::contacts::MapContactCode;
}

/// The launch word / stable id of the map app.
const MAP_APP_ID: &str = "map";
/// Render layer the map scene + camera live on, isolated from the world (0) and
/// the NOVA OS terminal RTT (20).
const MAP_LAYER: usize = 21;
/// The map camera renders BEFORE the NOVA OS offscreen pass (-20) so its image is
/// ready when the NOVA OS content samples it.
const MAP_CAMERA_ORDER: isize = -30;
/// How many distance rings the map floor draws as a scale reference.
const MAP_RING_COUNT: usize = 3;

/// How much further back than the contact spread the default framing sits.
///
/// The map camera looks at the scene through the default 45 degree vertical
/// fov, which holds a half-height of `r * tan(22.5 deg)` at distance `r`, so
/// the whole spread fits at the reciprocal of that.
const MAP_FRAMING_MARGIN: f32 = 2.42;

/// How far past the default framing the wheel may pull back - the zoom-out
/// room the map has always had (the old 520 over the old 170).
const MAP_ZOOM_OUT_RATIO: f32 = 3.06;

/// Smallest the focus hub is drawn. The hub is the focused body itself, so
/// this only has to keep a nav point - which has no body - visible on the
/// floor as a mark.
const MAP_HUB_MIN: Meters = Meters(16.0);

/// Orbit-radius zoom clamp floor (world units from the focus): close enough to
/// read one hull, whatever else is in the scene.
const MAP_RADIUS_MIN: f32 = 30.0;

/// Default framing (world units) of a scene with nothing spread out in it.
/// The floor, not the answer: a two-ship skirmish opens at the composition the
/// map has always had, and a scenario spread over 20 km opens framed on all of
/// it instead of on a radius no amount of zooming could reach past.
const MAP_RADIUS_DEFAULT_MIN: f32 = 170.0;

const MAP_THETA_DEFAULT: f32 = 0.8;
const MAP_PHI_DEFAULT: f32 = 0.62;

/// How far the map has to see from `focus`: the distance to the furthest live
/// contact, in world units.
fn map_spread(contacts: &MapContacts, focus: Vec3) -> f32 {
    contacts
        .collect()
        .iter()
        .map(|contact| contact.world_pos.distance(focus))
        .fold(0.0f32, f32::max)
}

/// The orbit radius the map opens and re-frames at for a scene of this spread.
fn map_radius_default(spread: f32) -> f32 {
    (spread * MAP_FRAMING_MARGIN).max(MAP_RADIUS_DEFAULT_MIN)
}

/// How far the wheel may pull back for a scene of this spread.
fn map_radius_max(spread: f32) -> f32 {
    map_radius_default(spread) * MAP_ZOOM_OUT_RATIO
}

/// Round `metres` to the nearest 1 / 2 / 5 step of its own decade - the ladder
/// a scale reading is taken off, so the floor rings land on 500 m or 10 km
/// rather than on 437 m.
fn nice_step_metres(metres: f32) -> f32 {
    if metres.is_nan() || metres <= 0.0 {
        return 0.0;
    }
    let decade = 10f32.powf(metres.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|step| step * decade)
        .min_by(|a, b| (a - metres).abs().total_cmp(&(b - metres).abs()))
        .unwrap_or(metres)
}

/// The floor rings for a scene framed at `radius` (world units): evenly spaced
/// at a round metric step that reaches most of the way out to the framing.
fn map_ring_radii(radius: f32) -> [f32; MAP_RING_COUNT] {
    let step = Meters(nice_step_metres(
        Meters::from_engine(radius / (MAP_RING_COUNT as f32 + 1.0)).0,
    ))
    .to_engine();
    std::array::from_fn(|ring| step * (ring + 1) as f32)
}

/// Footer hints while the map owns the screen (swapped in by the runtime).
///
/// Built per call from the live table so a rebind moves the footer with it.
/// DRAG and WHEEL stay literal: they are pointer gestures, not bound actions,
/// and ESC is the fixed back-out.
fn map_hints(bindings: &InputBindings) -> Vec<String> {
    let mut hints = Vec::with_capacity(9);
    hints.extend(hint(
        bindings,
        &[
            "novaos_pan_forward",
            "novaos_pan_left",
            "novaos_pan_back",
            "novaos_pan_right",
        ],
        "MOVE",
    ));
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
    hints.extend(hint(bindings, &["novaos_next", "novaos_prev"], "CYCLE"));
    hints.extend(hint(bindings, &["map_goto"], "GOTO"));
    hints.extend(hint(bindings, &["novaos_reframe"], "RESET"));
    hints.push("ESC: BACK".to_string());
    hints
}

/// Registers the `map` app and drives its scene, camera, blips and GOTO.
pub(crate) struct NovaOsMapPlugin;

impl Plugin for NovaOsMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapRuntime>();
        // Register the `map` command tree into the unified NOVA OS command
        // registry (created by NovaOsPlugin, added before this plugin): the launch
        // word `map` (which spawns the app), its `map view` CLI subcommand, and the
        // `MapApp` runtime, all declared together. `sync_nova_os_commands` mirrors
        // these into the terminal's command set.
        app.world_mut()
            .resource_mut::<NovaOsCommandRegistry>()
            .register(
                TerminalCommand::app(MAP_APP_ID, "Open the local-space map", MapApp)
                    .with_subcommand(TerminalCommand::cli(
                        "map view",
                        "Print local-space contacts",
                        CliOutput::Snapshot,
                    ))
                    .with_subcommand(
                        TerminalCommand::gameplay(
                            "map goto",
                            "Fly the ship to a contact label",
                            CommandArity::UpTo(1),
                        )
                        .with_arg_hint("<label>")
                        .with_args(&[CommandArg::Live(live::CONTACT)]),
                    ),
            );

        // Scene lifecycle runs unconditionally so it can tear down when the
        // computer closes; the interactive systems gate on the map being active.
        // Where `NovaOsMapSystems` sits in the frame is decided by
        // `crate::MonitorFrame`, which is above both apps and the terminal.
        app.add_systems(
            Update,
            (
                assign_map_contact_codes,
                sync_map_arg_completions,
                apply_map_cli_commands,
                manage_map_scene,
                reconcile_map_target,
                map_input,
                map_focus_follow,
                drive_map_camera,
                project_map_blips,
                update_map_readout,
            )
                .chain()
                .in_set(NovaOsMapSystems),
        );
    }
}

/// System set for the map app's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaOsMapSystems;

/// Whether the map app is the active NOVA OS surface right now.
fn map_is_active(pause: &State<PauseStates>, terminal: &NovaOsTerminal) -> bool {
    *pause.get() == PauseStates::NovaOs
        && terminal.active_mode() == TerminalMode::App { id: MAP_APP_ID }
}
