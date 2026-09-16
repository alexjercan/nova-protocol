//! The docking section: a telescoping port that locks two hulls together.
//!
//! A port is a 1x1x1 cell whose sleeve slides out along the section's own
//! outward axis (local -Z, the axis every section faces the world with). The
//! sleeve is ART: it never grows a collider and never changes what the hull
//! collides with, so two ports whose sleeves overlap are still two cells with
//! a gap between them. What actually holds the ships together is one avian
//! [`FixedJoint`](avian3d::prelude::FixedJoint) between their rigid-body
//! roots, created the instant the pair is accepted - before the sleeves move -
//! so the hulls cannot drift apart while the art plays.
//!
//! The mechanic is deliberately one verb wide. `DOCK` is offered only while a
//! candidate pair exists ([`port`]), the pair is revalidated when the command
//! executes, and ANY fresh translation or rotation intent from either ship
//! takes the joint away again before that intent is applied ([`connection`]).
//! There is no authority transfer, no combined controller and no station
//! service here: two ships are held in the pose they met in, and either one
//! leaves by flying away.
//!
//! While docked a ship's attitude loop is switched off rather than left to
//! fight the joint - see [`DockedShip`]. That is the same shape the flight
//! layer already uses for an engaged autopilot: the loop that is not in charge
//! does not get to apply torque.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::{asset_ref::AssetRef, prelude::*};

use crate::prelude::*;

mod connection;
mod port;
mod render;

use connection::{
    on_docking_connection_request, on_docking_port_removed_release, on_docking_release_request,
    park_docked_helms_and_release_maneuvers, release_broken_docking_connections,
};
pub use connection::{
    DockedPort, DockedShip, DockingConnection, DockingConnectionRequest, DockingReleaseRequest,
    DockingSystems,
};
pub use port::{DockingPair, DockingPorts, PortPose};
use render::insert_docking_section_render;

/// The `docking_section` spawners, its config, marker, port state, connection,
/// candidate search and `DockingSectionPlugin`.
pub mod prelude {
    pub use super::{
        docking_section, preview_docking_section, DockedPort, DockedShip, DockingConnection,
        DockingConnectionRequest, DockingEnvelope, DockingPair, DockingPorts,
        DockingReleaseRequest, DockingSectionConfig, DockingSectionConfigHelper,
        DockingSectionMarker, DockingSectionPlugin, DockingSectionState, DockingSystems, PortPose,
    };
}

/// Authorable config for a docking section.
///
/// The four numbers are the whole capture envelope: how far apart two port
/// faces may be, how far off exactly-opposed their axes may point, and how
/// fast the two hulls may be moving and turning relative to each other. A pair
/// is judged against the STRICTER of its two ports on every one of them, so a
/// delicate port cannot be docked into by a reckless one.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockingSectionConfig {
    /// The render mesh of the port; defaults to the placeholder block.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform applied to the port's render mesh only (never the
    /// collider, never the port face the capture is measured from).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// The widest gap between the two RETRACTED port faces that still
    /// captures. Measured face to face, not origin to origin: a port is
    /// judged by where its mouth is, not by where the cell it sits in is
    /// centred.
    ///
    /// The sleeve's travel is fixed and never fitted to the gap, so this is
    /// also a statement about the art: at the shipped 10 m the two extended
    /// sleeves meet exactly, and closer than that they overlap on purpose.
    pub capture_distance: Meters,
    /// How far off exactly-opposed the two outward axes may point, in
    /// degrees. Roll about the docking axis is NOT part of it: the ports are
    /// cylindrical, so how they are clocked cannot matter.
    pub capture_angle: f32,
    /// The fastest the two hulls may be closing (or separating) at, as the
    /// magnitude of their relative linear velocity.
    pub maximum_relative_speed: MetersPerSecond,
    /// The fastest the two hulls may be turning relative to each other, in
    /// degrees per second.
    pub maximum_relative_angular_speed: f32,
}

impl Default for DockingSectionConfig {
    /// The shipped port: a 10 m reach (one cell), a 15 degree cone, and a
    /// closing rate a hand-flown RCS approach can actually hit.
    fn default() -> Self {
        Self {
            render_mesh: None,
            render_mesh_transform: None,
            capture_distance: Meters(10.0),
            capture_angle: 15.0,
            maximum_relative_speed: MetersPerSecond(5.0),
            maximum_relative_angular_speed: 5.0,
        }
    }
}

impl DockingSectionConfig {
    /// The capture envelope this port flies, in engine units and radians.
    pub fn envelope(&self) -> DockingEnvelope {
        DockingEnvelope {
            capture_distance: self.capture_distance.to_engine(),
            capture_angle: self.capture_angle.to_radians(),
            maximum_relative_speed: self.maximum_relative_speed.to_engine(),
            maximum_relative_angular_speed: self.maximum_relative_angular_speed.to_radians(),
        }
    }
}

/// One port's capture envelope in the units the checks are made in: engine
/// units, engine units per second, and radians.
///
/// The conversion happens once per candidate test rather than per comparison,
/// and [`DockingEnvelope::strictest`] is the ONE place the "a pair is judged by
/// its tighter port" rule lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockingEnvelope {
    /// The widest face gap a dock is offered at, engine units.
    pub capture_distance: f32,
    /// How far the two outward axes may be from opposed, radians.
    pub capture_angle: f32,
    /// The fastest the two hulls may be closing, engine units per second.
    pub maximum_relative_speed: f32,
    /// The same ceiling for their relative spin, radians per second.
    pub maximum_relative_angular_speed: f32,
}

impl DockingEnvelope {
    /// The tighter of two envelopes, field by field.
    pub fn strictest(self, other: Self) -> Self {
        Self {
            capture_distance: self.capture_distance.min(other.capture_distance),
            capture_angle: self.capture_angle.min(other.capture_angle),
            maximum_relative_speed: self
                .maximum_relative_speed
                .min(other.maximum_relative_speed),
            maximum_relative_angular_speed: self
                .maximum_relative_angular_speed
                .min(other.maximum_relative_angular_speed),
        }
    }
}

/// Tags a live or previewed docking port.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DockingSectionMarker;

/// The port's full config, kept on the section entity. Read-only via `Deref`,
/// so the candidate search grades a pair with the same numbers the content
/// authored, exactly as the turret and the lance do.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct DockingSectionConfigHelper(DockingSectionConfig);

/// Where a port's sleeve is.
///
/// The mechanic owns this and the art follows it: the connection writes
/// [`Extending`](Self::Extending) on capture and [`Retracting`](Self::Retracting)
/// on release, and [`drive_docking_section_state`] turns that into the
/// [`SectionAnimationCue::DockTube`] target every frame. A port whose content
/// authors no tube track still keeps the state honest - it simply arrives at
/// the far end the same frame, because there is nothing to travel.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum DockingSectionState {
    /// Stowed flush in the cell. The rest state, and the only state a free
    /// port is ever in.
    #[default]
    Retracted,
    /// Reaching for the other port.
    Extending,
    /// Fully out.
    Extended,
    /// Coming back in after a release.
    Retracting,
}

impl DockingSectionState {
    /// The [`SectionAnimationCue::DockTube`] target this state holds the
    /// sleeve at: out while extending or extended, in otherwise.
    fn cue_target(self) -> f32 {
        match self {
            Self::Extending | Self::Extended => 1.0,
            Self::Retracted | Self::Retracting => 0.0,
        }
    }
}

/// A live docking port: the art, plus the sleeve state a connection steers.
pub fn docking_section(config: DockingSectionConfig) -> impl Bundle {
    trace!("docking_section: config {:?}", config);

    (
        preview_docking_section(config),
        DockingSectionState::Retracted,
    )
}

/// The render-only half of a port: what an editor view needs to LOOK like a
/// docking section, with no sleeve state, so a preview can never capture.
pub fn preview_docking_section(config: DockingSectionConfig) -> impl Bundle {
    trace!("preview_docking_section: config {:?}", config);

    (
        DockingSectionMarker,
        SectionClass::Docking,
        SectionRenderMeshTransform(config.render_mesh_transform),
        DockingSectionRenderMesh(config.render_mesh.clone()),
        DockingSectionConfigHelper(config),
    )
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct DockingSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// Hold every port's sleeve where its state says, and retire the two
/// transient states once the sleeve has arrived.
///
/// Render clock, ahead of [`SectionAnimationSystems`], so a target written
/// here is travelled the same frame instead of the next one. The arrival test
/// reads the progress that pass left behind, which is why a port with no
/// authored track (a headless fixture, a section whose art has not landed)
/// settles immediately: `cue_progress` is `None` and there is nothing to wait
/// for.
///
/// The animation component is OPTIONAL for the same reason. A spawned section
/// always carries one (`base_section`), but the state machine is the
/// mechanic's and must not be able to stall on a port that is missing its
/// art: a sleeve with nothing to move has already arrived.
fn drive_docking_section_state(
    mut q_ports: Query<
        (&mut DockingSectionState, Option<&mut SectionAnimations>),
        With<DockingSectionMarker>,
    >,
) {
    for (mut state, animations) in &mut q_ports {
        let progress = match animations {
            Some(mut animations) => {
                animations.set_cue(SectionAnimationCue::DockTube, state.cue_target());
                animations.cue_progress(SectionAnimationCue::DockTube)
            }
            None => None,
        };
        let next = match *state {
            DockingSectionState::Extending if progress.is_none_or(|p| p >= 1.0) => {
                DockingSectionState::Extended
            }
            DockingSectionState::Retracting if progress.is_none_or(|p| p <= 0.0) => {
                DockingSectionState::Retracted
            }
            other => other,
        };
        if next != *state {
            *state = next;
        }
    }
}

/// Adds docking: the candidate search's data, the connection lifecycle, the
/// sleeve state driver and (when `render`) the port's body.
#[derive(Default)]
pub struct DockingSectionPlugin {
    /// Whether the render-side half is added (false on headless servers).
    pub render: bool,
}

impl Plugin for DockingSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("DockingSectionPlugin: build");

        app.register_type::<DockingSectionMarker>();
        app.register_type::<DockingSectionState>();
        app.register_type::<DockedPort>();
        app.register_type::<DockedShip>();
        app.register_type::<DockingConnection>();

        app.add_observer(on_docking_connection_request);
        app.add_observer(on_docking_release_request);
        app.add_observer(on_docking_port_removed_release);

        app.add_systems(
            FixedUpdate,
            (
                release_broken_docking_connections,
                park_docked_helms_and_release_maneuvers,
            )
                .chain()
                .in_set(DockingSystems::Release),
        );

        // The release pass has to beat everything that acts on this tick's
        // commands: the attitude command copy (and so the PD), and the flight
        // layer's burns. A hull released this tick flies this tick. Declared
        // here, from the side that cares, exactly as the flight layer declares
        // its own edge against the controller.
        app.configure_sets(
            FixedUpdate,
            DockingSystems::Release
                .before(ControllerSectionSystems::SyncRotationInput)
                .before(super::SpaceshipSectionSystems),
        );

        app.add_systems(
            Update,
            drive_docking_section_state.before(SectionAnimationSystems),
        );

        if self.render {
            app.init_resource::<PlaceholderArt>();
            app.add_observer(insert_docking_section_render);
        }
    }
}

#[cfg(test)]
mod tests;
