//! The one place that knows how a [`SectionConfig`] becomes preview entities,
//! so the build ship, the gallery tiles and the placement ghost cannot drift
//! apart.
//!
//! Preview entities render and pick but never simulate, and they are inert by
//! CONSTRUCTION: every kind is built from its `preview_*_section` half, which
//! carries the render mesh and the config the render observers read and leaves
//! out the live state - thrust input and magnitude, turret aim and trigger,
//! torpedo fire input, the controller's `PDController`. The simulation systems
//! all demand one of those, so they match a preview against no query at all.
//! Nothing here depends on the preview root being unmarked or on the scenario
//! being dead.

use avian3d::prelude::Collider;
use bevy::ecs::system::EntityCommands;
use nova_gameplay::markers::prelude::SectionMarker;
use nova_ship::prelude::*;

/// What a preview entity IS to the rest of the editor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewRole {
    /// A section of the ship being built: picked, counted, placed against and
    /// handed to the scenario.
    Section,
    /// Scenery that only has to render - a gallery tile, the placement ghost.
    /// It must NOT read as a section: a display copy in a section query would
    /// be one more part on a ship nobody built, and one more collider in the
    /// pointer's way.
    Display,
}

/// Turn `entity` into a preview of `section`: the shared preview bundle plus
/// the kind-specific one that renders it.
///
/// No input bindings. A section's binds are DOCUMENT data
/// ([`SectionNode::binds`](crate::node::SectionNode::binds)), and a preview is a
/// picture of a section: a second copy of the binds out here would have to be
/// kept in step across every despawn of the view that held it.
pub(crate) fn insert_preview_section(
    entity: &mut EntityCommands,
    section: &SectionConfig,
    role: PreviewRole,
) {
    entity.insert(preview_section(section.base.clone()));
    match &section.kind {
        SectionKind::Hull(hull) => {
            entity.insert(preview_hull_section(hull.clone()));
        }
        SectionKind::Controller(controller) => {
            entity.insert(preview_controller_section(controller.clone()));
        }
        SectionKind::Thruster(thruster) => {
            entity.insert(preview_thruster_section(thruster.clone()));
        }
        SectionKind::Turret(turret) => {
            entity.insert(preview_turret_section(turret.clone()));
        }
        SectionKind::Torpedo(torpedo) => {
            entity.insert(preview_torpedo_section(torpedo.clone()));
        }
    }
    if role == PreviewRole::Display {
        // Dropped rather than never inserted: the preview bundle is one shared
        // recipe, and a display copy is that recipe minus its identity.
        entity.remove::<(SectionMarker, Collider)>();
    }
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    use nova_gameplay::prelude::{
        ControllerSectionMarker, SectionClass, ThrusterSectionMarker, TorpedoSectionMarker,
        TurretSectionMarker,
    };

    use super::*;

    fn spawn_preview(world: &mut World, kind: SectionKind) -> Entity {
        let section = SectionConfig {
            base: BaseSectionConfig {
                id: "part".to_string(),
                name: "part".to_string(),
                ..default()
            },
            kind,
        };
        world
            .run_system_once(move |mut commands: Commands| {
                let mut entity = commands.spawn_empty();
                insert_preview_section(&mut entity, &section, PreviewRole::Section);
                entity.id()
            })
            .expect("the preview spawner runs")
    }

    /// A preview section is inert because of WHAT IT IS, not because of where it
    /// is parented or because no scenario is live. Every kind gets the render
    /// half of its bundle and none of the live state the simulation keys on, so
    /// the thrust, aim, fire and steering paths match a preview against no query
    /// at all.
    ///
    /// Before the split the editor inserted the full live bundle and stayed
    /// quiet only because the preview root is not a `SpaceshipRootMarker` and
    /// the ship system sets are gated on scenario-liveness. Either gate moving
    /// would have woken a build-screen ship up.
    #[test]
    fn preview_sections_carry_the_render_half_and_no_live_state() {
        let mut world = World::new();

        let hull = spawn_preview(&mut world, SectionKind::Hull(HullSectionConfig::default()));
        assert!(world.get::<HullSectionMarker>(hull).is_some());
        assert_eq!(world.get::<SectionClass>(hull), Some(&SectionClass::Hull));

        let controller = spawn_preview(
            &mut world,
            SectionKind::Controller(ControllerSectionConfig::default()),
        );
        assert!(world.get::<ControllerSectionMarker>(controller).is_some());
        assert_eq!(
            world.get::<SectionClass>(controller),
            Some(&SectionClass::Controller)
        );
        assert!(
            world.get::<PDController>(controller).is_none(),
            "a preview controller must never try to torque a root"
        );

        let thruster = spawn_preview(
            &mut world,
            SectionKind::Thruster(ThrusterSectionConfig::default()),
        );
        assert!(world.get::<ThrusterSectionMarker>(thruster).is_some());
        assert_eq!(
            world.get::<SectionClass>(thruster),
            Some(&SectionClass::Thruster)
        );
        assert!(
            world.get::<ThrusterSectionInput>(thruster).is_none(),
            "a preview thruster must not be drivable"
        );
        assert!(
            world.get::<ThrusterSectionMagnitude>(thruster).is_none(),
            "a preview thruster must not be able to push a hull"
        );

        let turret = spawn_preview(
            &mut world,
            SectionKind::Turret(TurretSectionConfig::default()),
        );
        assert!(world.get::<TurretSectionMarker>(turret).is_some());
        assert_eq!(
            world.get::<SectionClass>(turret),
            Some(&SectionClass::Turret)
        );
        assert!(
            world.get::<TurretSectionInput>(turret).is_none(),
            "a preview turret must not have a trigger"
        );
        assert!(
            world.get::<TurretSectionAimPoint>(turret).is_none(),
            "a preview turret must not aim"
        );
        assert!(
            world.get::<LoadedBullet>(turret).is_none(),
            "a preview turret must not be loaded"
        );

        let torpedo = spawn_preview(
            &mut world,
            SectionKind::Torpedo(TorpedoSectionConfig::default()),
        );
        assert!(world.get::<TorpedoSectionMarker>(torpedo).is_some());
        assert_eq!(
            world.get::<SectionClass>(torpedo),
            Some(&SectionClass::Torpedo)
        );
        assert!(
            world.get::<TorpedoSectionInput>(torpedo).is_none(),
            "a preview torpedo bay must not be able to fire"
        );
    }

    /// Delivery guard for the test above: the LIVE bundles still carry the state
    /// the split moved out of the preview half, so those assertions are proving
    /// the split rather than a component that no longer exists.
    #[test]
    fn live_sections_still_carry_the_state_the_preview_half_drops() {
        let mut world = World::new();

        let thruster = world
            .spawn(thruster_section(ThrusterSectionConfig::default()))
            .id();
        assert!(world.get::<ThrusterSectionInput>(thruster).is_some());
        assert!(world.get::<ThrusterSectionMagnitude>(thruster).is_some());

        let turret = world
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        assert!(world.get::<TurretSectionInput>(turret).is_some());
        assert!(world.get::<TurretSectionAimPoint>(turret).is_some());
        assert!(world.get::<LoadedBullet>(turret).is_some());

        let torpedo = world
            .spawn(torpedo_section(TorpedoSectionConfig::default()))
            .id();
        assert!(world.get::<TorpedoSectionInput>(torpedo).is_some());

        let controller = world
            .spawn(controller_section(ControllerSectionConfig::default()))
            .id();
        assert!(world.get::<PDController>(controller).is_some());
    }
}
