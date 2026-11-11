//! The port's body: its authored scene, or the placeholder block a section
//! with no art wears.
//!
//! Nothing here drives the sleeve. The sleeve is a named node inside that
//! scene, moved by the shared section-animation rig from the
//! [`SectionAnimationCue::DockTube`] target the port's state writes - so a
//! docking port gets its motion from the same machinery as a bay door and a
//! turret lid, and adds no second animator of its own.
//!
//! The clamp closing throws one cold blink at the port ([`on_dock_clamped_light`]),
//! the visual half of the clamp sound: the dock's one physical event, lit
//! where it happened.

use bevy::prelude::*;
use nova_gameplay::prelude::LightFlash;

use super::{DockClamped, DockingSectionMarker, DockingSectionRenderMesh};
use crate::prelude::*;

/// Brightness of the clamp light, in lumens. The torpedo launch puff's
/// level: a latch closing is a small event on the hull, and it has to be
/// lit on a collar that may be sitting in a station's shadow.
const CLAMP_LIGHT_LUMENS: f32 = 20_000.0;

/// How far the clamp light reaches, in world units (100 m): the two collars
/// and the plating around them.
const CLAMP_LIGHT_RANGE: f32 = 10.0;

/// How long the clamp light burns, in seconds. A beat longer than the launch
/// blink, so it reads as a latch seating rather than a flash.
const CLAMP_LIGHT_SECS: f32 = 0.2;

/// Light the port when its clamp closes: one cold blink at the port of the
/// hull that pressed `DOCK`.
///
/// Budgeted by the transient-light cap only: the Low tier's cap of zero is
/// what turns it off, and nothing here reads the tier. The port's
/// `GlobalTransform` is the one it was captured at, so a port spawned this
/// frame cannot dock and never reaches here with a stale pose.
pub(super) fn on_dock_clamped_light(
    clamped: On<DockClamped>,
    q_ports: Query<&GlobalTransform, With<DockingSectionMarker>>,
    mut commands: Commands,
) {
    let Ok(at) = q_ports.get(clamped.entity) else {
        return;
    };
    commands.trigger(LightFlash {
        at: at.translation(),
        color: Color::srgb(0.55, 0.72, 1.0),
        peak_intensity: CLAMP_LIGHT_LUMENS,
        range: CLAMP_LIGHT_RANGE,
        duration: CLAMP_LIGHT_SECS,
    });
}

/// Spawn a docking port's body.
pub(super) fn insert_docking_section_render(
    add: On<Add<DockingSectionMarker>>,
    mut commands: Commands,
    placeholder: Res<PlaceholderArt>,
    asset_server: Res<AssetServer>,
    q_ports: Query<
        (&DockingSectionRenderMesh, &SectionRenderMeshTransform),
        With<DockingSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_docking_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform)) = q_ports.get(entity) else {
        error!("insert_docking_section_render: entity {entity:?} not found in q_ports");
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert(children![(
                Name::new("Docking Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            )]);
        }
        None => {
            commands.entity(entity).insert(children![(
                Name::new("Docking Section Body"),
                SectionRenderOf(entity),
                placeholder.bundle(PlaceholderArtType::Body),
            )]);
        }
    }
}
