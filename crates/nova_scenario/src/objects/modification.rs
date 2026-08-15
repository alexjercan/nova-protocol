//! Section modifications: small, closed, data-only deltas an authored ship
//! section applies on top of a resolved `SectionConfig` (whether the config
//! was inlined or pulled from the section-prototype catalog by id).
//!
//! The model (spike 20260714, user direction): each authored
//! [`SectionModification`] is inserted at spawn as a distinct COMPONENT on the
//! resolved section entity, and a small `On<Add, _>` observer per component
//! applies it WHERE RELEVANT (it queries for the target component) and is INERT
//! elsewhere. The one exception is `DisableVerb`, whose target component
//! ([`WithheldVerbs`]) IS the withheld-verb state itself: instead of a marker +
//! observer, the accumulated `DisableVerb` set is written straight onto the
//! section entity as a [`WithheldVerbs`] component. A `WithheldVerbs` on a
//! non-controller section (a hull) is simply never read by the flight gate, so
//! it stays inert. Extending the model with a one-shot delta is a new variant +
//! component + observer, no central match to grow.

use bevy::{ecs::system::EntityCommands, platform::collections::HashSet, prelude::*};
use nova_gameplay::prelude::Health;
use nova_ship::prelude::{FlightVerb, SectionAmmo, SectionReload, WithheldVerbs};

/// `SectionModification` with the rename, health and ammo overrides it applies.
pub mod prelude {
    pub use super::{
        SectionAmmoOverride, SectionHealthOverride, SectionModification, SectionRename,
    };
}

/// A single, closed, data-only delta applied to a ship section at spawn on top
/// of its resolved `SectionConfig`. A small starter set - each variant is
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
    /// Give this weapon section a HARD magazine of exactly this many rounds:
    /// the magazine is overridden AND the auto-reload is stripped, so when the
    /// rounds are gone the section is dry for good. The knob for a ship that
    /// must eventually be overrun (a display scene's doomed defender, a
    /// mission ship whose ammo is the clock). Inert on a section with no
    /// magazine (an unlimited weapon, a hull).
    SetAmmo(u32),
}

impl SectionModification {
    /// Insert a whole authored modification list onto the section entity as
    /// components; the per-variant observers then apply them where relevant.
    ///
    /// `DisableVerb` modifications ACCUMULATE into a single [`WithheldVerbs`]
    /// component carrying every listed verb (the set naturally dedups). A
    /// component type can exist only once per entity, so writing one insert per
    /// DisableVerb would let the last verb win and silently drop the others (the
    /// shakedown controller withholds GOTO+LOCK+ORBIT at once). `WithheldVerbs`
    /// is the live state the flight gate reads directly - no marker + observer
    /// hop - so on a non-controller section (a hull) it is simply never read.
    /// The other variants are one component each with an apply-on-add observer.
    pub fn insert_all(modifications: &[SectionModification], entity: &mut EntityCommands) {
        let mut withheld: HashSet<FlightVerb> = HashSet::new();
        for modification in modifications {
            match modification {
                SectionModification::DisableVerb(verb) => {
                    withheld.insert(*verb);
                }
                SectionModification::SetHealth(health) => {
                    entity.insert(SectionHealthOverride(*health));
                }
                SectionModification::Rename(name) => {
                    entity.insert(SectionRename(name.clone()));
                }
                SectionModification::SetAmmo(rounds) => {
                    entity.insert(SectionAmmoOverride(*rounds));
                }
            }
        }
        if !withheld.is_empty() {
            entity.insert(WithheldVerbs(withheld));
        }
    }
}

/// Marker/data component: override this section's starting health.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionHealthOverride(pub f32);

/// Marker/data component: rename this section entity.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionRename(pub String);

/// Marker/data component: hard-magazine override (rounds; reload stripped).
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionAmmoOverride(pub u32);

/// Register the modification components and their apply-on-add observers. Called
/// from `SpaceshipPlugin::build`.
pub(crate) fn register_section_modifications(app: &mut App) {
    app.register_type::<SectionHealthOverride>()
        .register_type::<SectionRename>()
        .register_type::<SectionAmmoOverride>();

    app.add_observer(apply_section_health_override);
    app.add_observer(apply_section_rename);
    app.add_observer(apply_section_ammo_override);
    app.add_observer(apply_section_ammo_override_on_ammo);
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

/// Apply [`SectionAmmoOverride`] when the override lands after the weapon's
/// own [`SectionAmmo`]: overwrite the magazine and strip the reload. Inert
/// when the entity has no magazine yet - the twin observer below covers the
/// build order where the weapon's ammo arrives later (weapon components are
/// inserted by deferred setup observers, so either order is real).
fn apply_section_ammo_override(
    add: On<Add, SectionAmmoOverride>,
    mut commands: Commands,
    q_override: Query<&SectionAmmoOverride>,
    mut q_ammo: Query<&mut SectionAmmo>,
) {
    let entity = add.entity;
    let Ok(over) = q_override.get(entity) else {
        return;
    };
    let Ok(mut ammo) = q_ammo.get_mut(entity) else {
        return;
    };
    *ammo = SectionAmmo::new(over.0);
    commands.entity(entity).remove::<SectionReload>();
}

/// The twin of [`apply_section_ammo_override`], keyed on the magazine's own
/// arrival: a weapon whose `SectionAmmo` is inserted by its deferred setup
/// observer AFTER the modification components must still end up overridden.
fn apply_section_ammo_override_on_ammo(
    add: On<Add, SectionAmmo>,
    mut commands: Commands,
    q_override: Query<&SectionAmmoOverride>,
    mut q_ammo: Query<&mut SectionAmmo>,
) {
    let entity = add.entity;
    let Ok(over) = q_override.get(entity) else {
        return;
    };
    let Ok(mut ammo) = q_ammo.get_mut(entity) else {
        return;
    };
    *ammo = SectionAmmo::new(over.0);
    commands.entity(entity).remove::<SectionReload>();
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
    use nova_gameplay::prelude::ControllerSectionMarker;
    use nova_ship::prelude::{
        base_section, controller_section, hull_section, BaseSectionConfig, ControllerSectionConfig,
        HullSectionConfig, WithheldVerbs,
    };

    use super::*;

    fn app_with_observers() -> App {
        let mut app = App::new();
        register_section_modifications(&mut app);
        app
    }

    /// DisableVerb(Orbit) on a controller section withholds the orbit verb via
    /// a WithheldVerbs component while leaving the other verbs granted - the
    /// accumulated set is written straight onto the section entity.
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

        let withheld = app.world().get::<WithheldVerbs>(controller).unwrap();
        assert!(
            !withheld.granted(FlightVerb::Orbit),
            "ORBIT is withheld on the controller"
        );
        assert!(
            withheld.granted(FlightVerb::Stop)
                && withheld.granted(FlightVerb::Goto)
                && withheld.granted(FlightVerb::Lock),
            "the other verbs stay granted"
        );
    }

    /// The RCS fine-adjust verb threads through the same DisableVerb plumbing:
    /// withholding it leaves the maneuver verbs granted, so a scenario can ship
    /// a hull that flies the autopilot but cannot fine-adjust.
    #[test]
    fn disable_verb_clears_rcs_on_a_controller() {
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
            &[SectionModification::DisableVerb(FlightVerb::Rcs)],
            &mut app.world_mut().commands().entity(controller),
        );
        app.world_mut().flush();

        let withheld = app.world().get::<WithheldVerbs>(controller).unwrap();
        assert!(
            !withheld.granted(FlightVerb::Rcs),
            "RCS is withheld on the controller"
        );
        assert!(
            withheld.granted(FlightVerb::Stop)
                && withheld.granted(FlightVerb::Goto)
                && withheld.granted(FlightVerb::Orbit)
                && withheld.granted(FlightVerb::Lock),
            "the other verbs stay granted"
        );
    }

    /// Multiple DisableVerb modifications on one section ALL take effect. This
    /// pins the accumulation directly (a component is unique per entity, so
    /// writing one `WithheldVerbs` per verb would let the last win and drop the
    /// rest): the shakedown controller withholds GOTO+LOCK+ORBIT at once, and
    /// this asserts all three are withheld while STOP stays granted. Under a
    /// last-write-wins regression only ORBIT (the last) would apply and this fails.
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

        let withheld = app.world().get::<WithheldVerbs>(controller).unwrap();
        assert!(
            !withheld.granted(FlightVerb::Goto)
                && !withheld.granted(FlightVerb::Lock)
                && !withheld.granted(FlightVerb::Orbit),
            "every listed verb is withheld, not just the last: {withheld:?}"
        );
        assert!(
            withheld.granted(FlightVerb::Stop),
            "STOP (not disabled) stays granted"
        );
    }

    /// A DisableVerb on a hull section writes a `WithheldVerbs` component too,
    /// but nothing on a hull reads it - the flight gate only queries controller
    /// sections. This pins that the modification never panics on a non-controller
    /// and the withheld set carries the listed verb (inert-but-present).
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

        let withheld = app.world().get::<WithheldVerbs>(hull).unwrap();
        assert!(
            !withheld.granted(FlightVerb::Orbit),
            "the withheld set carries ORBIT, but no gate on a hull reads it - inert"
        );

        // The actual inertness guarantee: every verb-availability gate filters
        // `With<ControllerSectionMarker>`, and a hull has no controller marker, so
        // the hull's WithheldVerbs is never read. Pin that the hull is not a
        // controller section rather than trusting the readers' filters elsewhere.
        let mut q_controllers = app
            .world_mut()
            .query_filtered::<Entity, With<ControllerSectionMarker>>();
        assert!(
            !q_controllers.iter(app.world()).any(|e| e == hull),
            "a hull is not a controller section, so no verb gate ever reads its WithheldVerbs"
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

    /// SetAmmo is a HARD magazine: the rounds are overridden and the
    /// auto-reload is stripped, in BOTH build orders - weapon components are
    /// inserted by deferred setup observers, so the override can land either
    /// before or after the magazine it modifies.
    #[test]
    fn set_ammo_hard_magazine_in_both_build_orders() {
        use nova_ship::prelude::{SectionAmmo, SectionReload, SectionReloadConfig};

        let reload = || {
            SectionReload::from_config(SectionReloadConfig {
                reload_time: 3.0,
                rounds_per_cycle: 500,
                only_when_empty: true,
            })
        };

        // Override lands after the magazine (inline build).
        let mut app = app_with_observers();
        let weapon = app
            .world_mut()
            .spawn((SectionAmmo::new(500), reload()))
            .id();
        SectionModification::insert_all(
            &[SectionModification::SetAmmo(1800)],
            &mut app.world_mut().commands().entity(weapon),
        );
        app.world_mut().flush();
        let ammo = app.world().get::<SectionAmmo>(weapon).unwrap();
        assert_eq!((ammo.rounds, ammo.capacity), (1800, 1800));
        assert!(
            app.world().get::<SectionReload>(weapon).is_none(),
            "a hard magazine never refills"
        );

        // Magazine lands after the override (deferred setup observer order).
        let mut app = app_with_observers();
        let weapon = app.world_mut().spawn_empty().id();
        SectionModification::insert_all(
            &[SectionModification::SetAmmo(1800)],
            &mut app.world_mut().commands().entity(weapon),
        );
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(weapon)
            .insert((SectionAmmo::new(500), reload()));
        app.world_mut().flush();
        let ammo = app.world().get::<SectionAmmo>(weapon).unwrap();
        assert_eq!((ammo.rounds, ammo.capacity), (1800, 1800));
        assert!(app.world().get::<SectionReload>(weapon).is_none());
    }

    /// End-to-end through the real spawn path: a controller section authored
    /// with `DisableVerb(Orbit)` as a modification on a ship spawned through
    /// `insert_spaceship_sections` has ORBIT withheld on its `WithheldVerbs`.
    /// This pins the spawn contract (the `WithheldVerbs` component is inserted
    /// straight onto the controller section) that the shakedown e2e relies on.
    #[test]
    fn disable_verb_applies_through_the_real_ship_spawn() {
        use nova_ship::prelude::{GameSections, SectionKind};

        use crate::objects::spaceship::{prelude::*, SectionSource};

        let mut app = App::new();
        app.add_plugins(crate::objects::spaceship::SpaceshipPlugin);
        app.init_resource::<GameSections>();

        let ship = app
            .world_mut()
            .spawn((
                Transform::default(),
                spaceship_scenario_object(SpaceshipConfig {
                    collapse_threshold: None,
                    skin: false,
                    style: None,
                    controller: SpaceshipController::None,
                    allegiance: None,
                    sections: vec![SpaceshipSectionConfig {
                        id: "controller".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        source: SectionSource::Inline(nova_ship::prelude::SectionConfig {
                            base: nova_ship::prelude::BaseSectionConfig {
                                id: "controller".to_string(),
                                health: 100.0,
                                ..default()
                            },
                            kind: SectionKind::Controller(
                                nova_ship::prelude::ControllerSectionConfig::default(),
                            ),
                        }),
                        modifications: vec![SectionModification::DisableVerb(FlightVerb::Orbit)],
                    }],
                }),
            ))
            .id();
        app.world_mut().flush();

        let mut q = app.world_mut().query::<(&ChildOf, &WithheldVerbs)>();
        let withheld = q
            .iter(app.world())
            .find(|(ChildOf(parent), _)| *parent == ship)
            .map(|(_, w)| w.clone())
            .expect("the ship has a controller section carrying WithheldVerbs");
        assert!(
            !withheld.granted(FlightVerb::Orbit),
            "ORBIT is withheld by the modification"
        );
        assert!(
            withheld.granted(FlightVerb::Stop)
                && withheld.granted(FlightVerb::Goto)
                && withheld.granted(FlightVerb::Lock)
        );
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
