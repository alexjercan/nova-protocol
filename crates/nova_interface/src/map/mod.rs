//! The TAB interface's Map pane: a schematic 3D view of local space.
//!
//! It reads the `MapContacts` model (player, allies, enemies, asteroids,
//! objective markers with live range/bearing). The pane renders a small
//! schematic 3D scene - concentric distance rings and a central hub - through
//! a dedicated [`Camera3d`] on its own `RenderLayers` into an offscreen image
//! shown in the pane; the interactive CONTACTS ride on top as projected
//! clickable UI blips.
//!
//! The camera is a `MapOrbit` you drive with the `viewer_orbit_*` and
//! `viewer_pan_*` actions plus the wheel (zoom); a wheel over the contact
//! panel scrolls the panel instead. Selecting a contact fills the
//! contact panel beside the view with its code, name, kind, range and bearing;
//! `map_goto` or the panel's GOTO button sets a flight
//! [`Autopilot`](nova_ship::prelude::Autopilot) GOTO and the travel lock on the
//! player ship, which persist after the interface closes. While that GOTO is
//! live, the view tags its target `GOTO`, draws the ship's
//! [`FlightPrediction`](nova_ship::prelude::FlightPrediction) path from the
//! player blip while one exists, and the panel names the destination.
//!
//! `map_chart` or the Chart button swaps the local scene for the chart view
//! while a streamed world runs: the player's [`Chart`] of visited cells,
//! found stations and found wrecks at cell scale, and the player's held
//! [`Jobs`] marked in their target's cell, with the discovery log of those
//! finds as icon rows with relative play times in the panel. Selecting a
//! find shows its cell's SECTOR block: its contents, when the player entered
//! it, the finds there, the stock last seen at a docked station and the jobs
//! held or posted there; a station's head shows when it was found; a job's
//! its title, reward and where it hands in. A STANCES block stands below:
//! with nothing selected, every civilization met and where it stands; a
//! living, unowned Station mark selected, that civilization's entry and a
//! Pay fine row per civilization it stands Enemy with, paid only while
//! docked at that station.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `contacts` | The shared contact model. |
//! | `app` | The pane's components and runtime state. |
//! | `chart` | The chart view of the player's visited cells and finds. |
//! | `stance` | The STANCES block: civilization standing and the Pay fine action. |
//! | `scene` | The schematic 3D scene, its camera and the projected blips. |

mod app;
mod chart;
mod contacts;
mod scene;
mod stance;
mod tiles;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::{Chart, JobPostings, Jobs, WorldClock};
use nova_hud::prelude::PreviewPlugin;

pub use self::contacts::{MapContactCode, MapContactKind, MapContacts};
pub(crate) use self::{app::*, chart::*, contacts::*, scene::*, stance::*};

/// Glob-import surface: `use nova_interface::map::prelude::*`.
pub mod prelude {
    // Named by module path: each item is reachable both `pub` (here) and
    // `pub(crate)` (the module glob above), which rustc rejects as an
    // ambiguous import visibility through `super::`.
    pub use super::{
        chart::stack_chart_labels,
        contacts::{MapContactCode, MapContactKind, MapContacts},
        map_radius_default, map_radius_max, map_ring_radii, map_spread, MAP_RADIUS_MIN,
    };
}

/// Render layer the map scene + camera live on, isolated from the world (0) and
/// the command modal RTT (20).
const MAP_LAYER: usize = 21;
/// The map camera renders before the command modal's offscreen pass (-20).
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
pub const MAP_RADIUS_MIN: f32 = 30.0;

/// Default framing (world units) of a scene with nothing spread out in it.
/// The floor, not the answer: a two-ship skirmish opens at the composition the
/// map has always had, and a scenario spread over 20 km opens framed on all of
/// it instead of on a radius no amount of zooming could reach past.
const MAP_RADIUS_DEFAULT_MIN: f32 = 170.0;

const MAP_THETA_DEFAULT: f32 = 0.8;
const MAP_PHI_DEFAULT: f32 = 0.62;

/// How far the map has to see from `focus`: the distance to the furthest live
/// contact, in world units.
pub fn map_spread(contacts: &MapContacts, focus: Vec3) -> f32 {
    contacts
        .collect()
        .iter()
        .map(|contact| contact.world_pos.distance(focus))
        .fold(0.0f32, f32::max)
}

/// The orbit radius the map opens and re-frames at for a scene of this spread.
pub fn map_radius_default(spread: f32) -> f32 {
    (spread * MAP_FRAMING_MARGIN).max(MAP_RADIUS_DEFAULT_MIN)
}

/// How far the wheel may pull back for a scene of this spread.
pub fn map_radius_max(spread: f32) -> f32 {
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
pub fn map_ring_radii(radius: f32) -> [f32; MAP_RING_COUNT] {
    let step = Meters(nice_step_metres(
        Meters::from_engine(radius / (MAP_RING_COUNT as f32 + 1.0)).0,
    ))
    .to_engine();
    std::array::from_fn(|ring| step * (ring + 1) as f32)
}

/// Drives the Map pane's scene, camera, blips and GOTO.
pub(crate) struct MapPanePlugin;

impl Plugin for MapPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapRuntime>();
        // The open world feeds the chart, the jobs, the job postings and the
        // play-time clock; a scene without one reads them empty and the clock
        // at zero.
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        app.init_resource::<WorldClock>();
        // The chart's tile textures and mark meshes, shared with the menu's
        // previews; the menu adds the same plugin, and one app keeps one.
        if !app.is_plugin_added::<PreviewPlugin>() {
            app.add_plugins(PreviewPlugin);
        }
        // Scene lifecycle runs unconditionally so it can tear down when the
        // interface closes; the interactive systems gate on the Map pane owning
        // the interface.
        app.add_systems(
            Update,
            (
                assign_map_contact_codes,
                manage_map_scene,
                reconcile_map_target,
                toggle_map_chart,
                scroll_map_panel,
                map_input,
                map_focus_follow,
                manage_map_chart,
                update_map_chart_sector,
                update_map_chart_objectives,
                update_map_chart_log,
                update_map_stance_block,
                drive_map_camera,
                project_map_blips,
                project_map_chart_labels,
                project_map_route,
                refresh_map_legend,
                update_map_panel,
            )
                .chain()
                .in_set(MapPaneSystems),
        );
    }
}

/// System set for the Map pane's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapPaneSystems;
