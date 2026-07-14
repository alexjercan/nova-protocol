//! Section modifications: small, closed, data-only deltas an authored ship
//! section applies on top of a resolved [`SectionConfig`] (whether the config
//! was inlined or pulled from the section-prototype catalog by id).
//!
//! The model (spike 20260714, user direction): each authored
//! [`SectionModification`] is inserted at spawn as a distinct COMPONENT on the
//! resolved section entity, and a small `On<Add, _>` observer per component
//! applies it WHERE RELEVANT (it queries for the target component) and is INERT
//! elsewhere. A `DisableVerb(Orbit)` on a controller clears the verb on that
//! controller's [`ControllerVerbs`]; the same modification on a hull matches no
//! target and does nothing. Extending the model is a new variant + component +
//! observer, no central match to grow.

use bevy::{ecs::system::EntityCommands, prelude::*};
use bevy_common_systems::prelude::Health;
use nova_gameplay::prelude::{ControllerVerbs, FlightVerb};

pub mod prelude {
    pub use super::{
        SectionDisableVerb, SectionHealthOverride, SectionModification, SectionRename,
    };
}

/// A single, closed, data-only delta applied to a ship section at spawn on top
/// of its resolved [`SectionConfig`]. A small starter set - each variant is
/// well-grounded in an existing section component (a runtime concept the engine
/// already owns), not speculative. Grows by adding a variant plus its component
/// and observer below.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionModification {
    /// Withhold one flight verb on this section's controller (inert on a
    /// non-controller section). Mirrors what the `SetControllerVerb` scenario
    /// action does at runtime, but authored on the section from the instant it
    /// is built.
    DisableVerb(FlightVerb),
    /// Override this section's starting health (current and max), regardless of
    /// the prototype's authored value. Inert on a section with no `Health`
    /// (e.g. an editor preview section).
    SetHealth(f32),
    /// Rename this section entity (its `Name`).
    Rename(String),
}

impl SectionModification {
    /// Insert a whole authored modification list onto the section entity as
    /// components; the per-variant observers then apply them where relevant.
    ///
    /// `DisableVerb` modifications ACCUMULATE into a single
    /// [`SectionDisableVerb`] carrying every listed verb: a component type can
    /// exist only once per entity, so inserting one component per DisableVerb
    /// would let the last verb win and silently drop the others (the shakedown
    /// controller withholds GOTO+LOCK+ORBIT at once). Merging into one insert
    /// also means the component is complete by the time its `On<Add>` observer
    /// fires. The other variants are one component each.
    pub fn insert_all(modifications: &[SectionModification], entity: &mut EntityCommands) {
        let mut disabled_verbs: Vec<FlightVerb> = Vec::new();
        for modification in modifications {
            match modification {
                SectionModification::DisableVerb(verb) => {
                    if !disabled_verbs.contains(verb) {
                        disabled_verbs.push(*verb);
                    }
                }
                SectionModification::SetHealth(health) => {
                    entity.insert(SectionHealthOverride(*health));
                }
                SectionModification::Rename(name) => {
                    entity.insert(SectionRename(name.clone()));
                }
            }
        }
        if !disabled_verbs.is_empty() {
            entity.insert(SectionDisableVerb(disabled_verbs));
        }
    }
}

/// Marker/data component: withhold every listed verb on this section's
/// controller. Accumulates across several `DisableVerb` modifications on the
/// same section (see [`SectionModification::insert_all`]).
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionDisableVerb(pub Vec<FlightVerb>);

/// Marker/data component: override this section's starting health.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionHealthOverride(pub f32);

/// Marker/data component: rename this section entity.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionRename(pub String);

/// Register the modification components and their apply-on-add observers. Called
/// from `SpaceshipPlugin::build`.
pub(crate) fn register_section_modifications(app: &mut App) {
    app.register_type::<SectionDisableVerb>()
        .register_type::<SectionHealthOverride>()
        .register_type::<SectionRename>();

    app.add_observer(apply_section_disable_verb);
    app.add_observer(apply_section_health_override);
    app.add_observer(apply_section_rename);
}

/// Apply [`SectionDisableVerb`]: clear the verb on this entity's
/// [`ControllerVerbs`]. Inert when the entity has no `ControllerVerbs` (e.g. a
/// hull), which is the "match no target, do nothing" contract.
fn apply_section_disable_verb(
    add: On<Add, SectionDisableVerb>,
    q_disable: Query<&SectionDisableVerb>,
    mut q_verbs: Query<&mut ControllerVerbs>,
) {
    let entity = add.entity;
    let Ok(disable) = q_disable.get(entity) else {
        return;
    };
    // Inert on a section with no controller verbs (a hull, thruster, ...).
    let Ok(mut verbs) = q_verbs.get_mut(entity) else {
        return;
    };
    for verb in &disable.0 {
        verbs.set(*verb, false);
    }
}

/// Apply [`SectionHealthOverride`]: set the section's `Health` (current and max)
/// to the override. Inert when the entity has no `Health` (an editor preview
/// section carries none).
fn apply_section_health_override(
    add: On<Add, SectionHealthOverride>,
    q_override: Query<&SectionHealthOverride>,
    mut q_health: Query<&mut Health>,
) {
    let entity = add.entity;
    let Ok(over) = q_override.get(entity) else {
        return;
    };
    let Ok(mut health) = q_health.get_mut(entity) else {
        return;
    };
    *health = Health::new(over.0);
}

/// Apply [`SectionRename`]: set the section entity's `Name`.
fn apply_section_rename(
    add: On<Add, SectionRename>,
    mut commands: Commands,
    q_rename: Query<&SectionRename>,
) {
    let entity = add.entity;
    let Ok(rename) = q_rename.get(entity) else {
        return;
    };
    commands.entity(entity).insert(Name::new(rename.0.clone()));
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{
        base_section, controller_section, hull_section, BaseSectionConfig, ControllerSectionConfig,
        HullSectionConfig,
    };

    use super::*;

    fn app_with_observers() -> App {
        let mut app = App::new();
        register_section_modifications(&mut app);
        app
    }

    /// DisableVerb(Orbit) on a controller section clears the orbit verb on its
    /// ControllerVerbs while leaving the other verbs granted - the observer
    /// applies the delta where the target component exists.
    #[test]
    fn disable_verb_clears_the_verb_on_a_controller() {
        let mut app = app_with_observers();
        let controller = app
            .world_mut()
            .spawn((
                base_section(BaseSectionConfig {
                    id: "controller".to_string(),
                    health: 100.0,
                    ..default()
                }),
                controller_section(ControllerSectionConfig::default()),
            ))
            .id();
        SectionModification::insert_all(
            &[SectionModification::DisableVerb(FlightVerb::Orbit)],
            &mut app.world_mut().commands().entity(controller),
        );
        app.world_mut().flush();

        let verbs = app.world().get::<ControllerVerbs>(controller).unwrap();
        assert!(!verbs.orbit, "ORBIT is withheld on the controller");
        assert!(
            verbs.stop && verbs.goto && verbs.lock,
            "the other verbs stay granted"
        );
    }

    /// Multiple DisableVerb modifications on one section ALL take effect. This
    /// pins the accumulation fix directly (a component is unique per entity, so
    /// inserting one `SectionDisableVerb` per verb would let the last win and drop
    /// the rest): the shakedown controller withholds GOTO+LOCK+ORBIT at once, and
    /// this asserts all three clear while STOP stays granted. Under a
    /// last-write-wins regression only ORBIT (the last) would clear and this fails.
    #[test]
    fn multiple_disable_verbs_all_apply() {
        let mut app = app_with_observers();
        let controller = app
            .world_mut()
            .spawn((
                base_section(BaseSectionConfig {
                    id: "controller".to_string(),
                    health: 100.0,
                    ..default()
                }),
                controller_section(ControllerSectionConfig::default()),
            ))
            .id();
        SectionModification::insert_all(
            &[
                SectionModification::DisableVerb(FlightVerb::Goto),
                SectionModification::DisableVerb(FlightVerb::Lock),
                SectionModification::DisableVerb(FlightVerb::Orbit),
            ],
            &mut app.world_mut().commands().entity(controller),
        );
        app.world_mut().flush();

        let verbs = app.world().get::<ControllerVerbs>(controller).unwrap();
        assert!(
            !verbs.goto && !verbs.lock && !verbs.orbit,
            "every listed verb is withheld, not just the last: {verbs:?}"
        );
        assert!(verbs.stop, "STOP (not disabled) stays granted");
    }

    /// The SAME DisableVerb component on a hull section (no ControllerVerbs) is
    /// inert: the observer matches no target and does nothing - no panic, and
    /// the hull carries no ControllerVerbs afterward.
    #[test]
    fn disable_verb_is_inert_on_a_hull() {
        let mut app = app_with_observers();
        let hull = app
            .world_mut()
            .spawn((
                base_section(BaseSectionConfig {
                    id: "hull".to_string(),
                    health: 100.0,
                    ..default()
                }),
                hull_section(HullSectionConfig::default()),
            ))
            .id();
        SectionModification::insert_all(
            &[SectionModification::DisableVerb(FlightVerb::Orbit)],
            &mut app.world_mut().commands().entity(hull),
        );
        app.world_mut().flush();

        assert!(
            app.world().get::<ControllerVerbs>(hull).is_none(),
            "a hull has no controller verbs to disable - the modification is inert"
        );
    }

    /// SetHealth overrides the section's Health (current and max) built by
    /// base_section.
    #[test]
    fn set_health_overrides_the_section_health() {
        let mut app = app_with_observers();
        let hull = app
            .world_mut()
            .spawn((
                base_section(BaseSectionConfig {
                    id: "hull".to_string(),
                    health: 100.0,
                    ..default()
                }),
                hull_section(HullSectionConfig::default()),
            ))
            .id();
        SectionModification::insert_all(
            &[SectionModification::SetHealth(42.0)],
            &mut app.world_mut().commands().entity(hull),
        );
        app.world_mut().flush();

        let health = app.world().get::<Health>(hull).unwrap();
        assert_eq!(health.current, 42.0);
        assert_eq!(health.max, 42.0);
    }

    /// End-to-end through the real spawn path: a controller section authored
    /// with `DisableVerb(Orbit)` as a modification on a ship spawned through
    /// `insert_spaceship_sections` has ORBIT withheld on its `ControllerVerbs`.
    /// This pins the spawn-order contract (the modification component is
    /// inserted alongside `ControllerVerbs` and its observer must see it) that
    /// the shakedown end-to-end relies on.
    #[test]
    fn disable_verb_applies_through_the_real_ship_spawn() {
        use nova_gameplay::prelude::{GameSections, SectionKind};

        use crate::objects::spaceship::{prelude::*, SectionSource};

        let mut app = App::new();
        app.add_plugins(crate::objects::spaceship::SpaceshipPlugin);
        app.init_resource::<GameSections>();

        let ship = app
            .world_mut()
            .spawn((
                Transform::default(),
                spaceship_scenario_object(SpaceshipConfig {
                    controller: SpaceshipController::None,
                    sections: vec![SpaceshipSectionConfig {
                        id: "controller".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        source: SectionSource::Inline(nova_gameplay::prelude::SectionConfig {
                            base: nova_gameplay::prelude::BaseSectionConfig {
                                id: "controller".to_string(),
                                health: 100.0,
                                ..default()
                            },
                            kind: SectionKind::Controller(
                                nova_gameplay::prelude::ControllerSectionConfig::default(),
                            ),
                        }),
                        modifications: vec![SectionModification::DisableVerb(FlightVerb::Orbit)],
                    }],
                }),
            ))
            .id();
        app.world_mut().flush();

        let mut q = app.world_mut().query::<(&ChildOf, &ControllerVerbs)>();
        let verbs = q
            .iter(app.world())
            .find(|(ChildOf(parent), _)| *parent == ship)
            .map(|(_, v)| *v)
            .expect("the ship has a controller section carrying ControllerVerbs");
        assert!(!verbs.orbit, "ORBIT is withheld by the modification");
        assert!(verbs.stop && verbs.goto && verbs.lock);
    }

    /// Rename sets the section entity's Name.
    #[test]
    fn rename_sets_the_entity_name() {
        let mut app = app_with_observers();
        let hull = app
            .world_mut()
            .spawn((
                base_section(BaseSectionConfig {
                    id: "hull".to_string(),
                    name: "Original".to_string(),
                    health: 100.0,
                    ..default()
                }),
                hull_section(HullSectionConfig::default()),
            ))
            .id();
        SectionModification::insert_all(
            &[SectionModification::Rename("Renamed".to_string())],
            &mut app.world_mut().commands().entity(hull),
        );
        app.world_mut().flush();

        let name = app.world().get::<Name>(hull).unwrap();
        assert_eq!(name.as_str(), "Renamed");
    }
}
