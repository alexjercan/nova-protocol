//! Debug overlay for the gravity layer (nova_gameplay::gravity): wire
//! spheres for each well's sphere of influence and a line from every
//! gravity-affected body to the well that currently owns it. Gated behind
//! the F11 debug toggle like the section gizmos; the diegetic (non-debug)
//! readout is the HUD GRAV line and orbit cue of task 20260709-193339, and
//! SOI rings for normal play belong to the diegetic-instruments task
//! (20260709-103454).

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, prelude::*};
use nova_gameplay::prelude::*;

/// Debug overlay plugin for the gravity layer.
///
/// Adds `draw_gravity_wells` and `draw_dominant_well_links` to `PostUpdate`
/// (after transform propagation) under the [`DebugSystems`](super::DebugSystems)
/// set, and inits `GravitySettings` so a debug-only app cannot panic.
pub struct GravityDebugPlugin;

impl Plugin for GravityDebugPlugin {
    fn build(&self, app: &mut App) {
        // Normally initialized by NovaGravityPlugin; init here too so a
        // debug-only app cannot panic on the missing resource.
        app.init_resource::<GravitySettings>();

        app.add_systems(
            PostUpdate,
            (draw_gravity_wells, draw_dominant_well_links)
                .after(TransformSystems::Propagate)
                .in_set(super::DebugSystems),
        );
    }
}

/// Two wire spheres per well: the SOI boundary (outside it, flat space) and
/// the start of the fade band (inside it, the unfaded core where orbits are
/// trustworthy and the future ORBIT verb will park).
fn draw_gravity_wells(
    mut gizmos: Gizmos,
    settings: Res<GravitySettings>,
    q_wells: Query<(&Position, &GravityWell)>,
) {
    for (position, well) in &q_wells {
        let iso = Isometry3d::from_translation(**position);
        gizmos.sphere(iso, well.soi_radius, tailwind::CYAN_500);

        let fade_start = well.soi_radius * (1.0 - settings.fade_fraction.clamp(0.0, 1.0));
        gizmos.sphere(iso, fade_start, tailwind::CYAN_900);
    }
}

/// A line from each affected body to its dominant well, so well ownership
/// (and the hysteresis handoff between overlapping SOIs) is visible.
fn draw_dominant_well_links(
    mut gizmos: Gizmos,
    q_affected: Query<(&Position, &DominantWell)>,
    q_wells: Query<&Position, With<GravityWell>>,
) {
    for (position, dominant) in &q_affected {
        // The owned well can be gone for the current flush (it was just
        // destroyed); skip rather than assume.
        let Ok(well_position) = q_wells.get(**dominant) else {
            continue;
        };
        gizmos.line(**position, **well_position, tailwind::AMBER_500);
    }
}
