//! How far gone a destructible body LOOKS, as one number derived from its own
//! health.
//!
//! # Geometry follows health, never the other way round
//!
//! Damage is untouched by anything here. A bullet, a blast and a ram deal
//! exactly what they authored, health is still the kill gate, and
//! [`DamageLevel`] is a READING of health taken afterwards. Nothing in this
//! module can change what a hit costs.
//!
//! That direction is the whole design. Because the level is a function of
//! health rather than an accumulator beside it, a body at half health always
//! looks the same amount of wrecked - the picture cannot drift from the number,
//! there is nothing extra to save (health is already persisted, so a reload
//! restores the look for free), and damage from a source with no geometry to
//! speak of - blast pressure, a ram, a scripted `destroy` - shows up exactly
//! like damage from a shot.
//!
//! # One level, many consumers
//!
//! This module owns the NUMBER and nothing else. What the number means visually
//! is decided by DAMAGE EFFECTS: components a body carries, each turning the
//! same scalar into its own kind of "more or less". A hull cell erodes, a
//! turret sheds cowling and throws sparks, a thruster's plume goes ragged.
//!
//! Effects are components rather than a mode enum on purpose. They COMPOSE (a
//! thruster is sparks and a failing plume at once), a mod adds a look by adding
//! a component instead of extending a variant list the engine owns, and each
//! one can be dropped on its own if it does not survive contact with the art.
//! "Undamaged-looking" is not a mode here: it is a body with no effect
//! attached.
//!
//! The rule that keeps effects honest: ONLY NON-FUNCTIONAL MATERIAL IS REMOVED.
//! A turret that has lost most of its mass still shoots, because what it lost
//! was cowling and the barrel was never expendable.

use bevy::prelude::*;

use super::health::prelude::Health;

/// `DamageLevel` and `DamageLevelPlugin`.
pub mod prelude {
    pub use super::{DamageLevel, DamageLevelPlugin};
}

/// How far gone a body looks: `0.0` pristine, `1.0` destroyed.
///
/// Derived from the entity's OWN [`Health`], never from an aggregate. That
/// matters on a ship: a skin plate carries its own pool and is
/// `HealthIsolated`, so a stripped plate reads as stripped while the hull it
/// was bolted to still reads as untouched, which is exactly what happened.
///
/// Read it, do not write it - [`derive_damage_level`] overwrites the value
/// whenever health moves.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct DamageLevel(pub f32);

impl DamageLevel {
    /// The level a pool of `current` out of `max` reads at.
    ///
    /// A pool with no capacity is pristine rather than destroyed: `0/0` is a
    /// body nothing can damage, and calling it wrecked would erode every
    /// zero-health prop in a scene on its first frame.
    pub fn of(health: &Health) -> Self {
        if health.max <= f32::EPSILON {
            return Self(0.0);
        }
        Self((1.0 - health.current / health.max).clamp(0.0, 1.0))
    }
}

/// Derives [`DamageLevel`] from [`Health`], and nothing else.
pub struct DamageLevelPlugin;

impl Plugin for DamageLevelPlugin {
    fn build(&self, app: &mut App) {
        debug!("DamageLevelPlugin: build");

        app.register_type::<DamageLevel>();
        app.add_systems(Update, derive_damage_level);
    }
}

/// Keep every health pool's [`DamageLevel`] in step with it.
///
/// `Changed<Health>` covers the first frame too - an added component counts as
/// changed - so a body is readable the frame it spawns rather than the frame it
/// is first hit. Effects can then assume the level is always present on
/// anything with health, instead of each carrying its own "not yet" case.
fn derive_damage_level(
    mut commands: Commands,
    mut q_health: Query<(Entity, &Health, Option<&mut DamageLevel>), Changed<Health>>,
) {
    for (entity, health, level) in &mut q_health {
        let derived = DamageLevel::of(health);
        match level {
            // Written through change detection so an effect can watch
            // `Changed<DamageLevel>` and do its work only when the reading
            // actually moved, rather than every frame of a long fight.
            Some(mut level) => {
                level.set_if_neq(derived);
            }
            None => {
                commands.entity(entity).try_insert(derived);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DamageLevelPlugin);
        app
    }

    fn level_of(app: &App, entity: Entity) -> f32 {
        app.world()
            .get::<DamageLevel>(entity)
            .expect("every health pool gets a level")
            .0
    }

    /// The reading is a pure function of the pool, at both ends and in between.
    #[test]
    fn the_level_reads_the_share_of_health_that_is_gone() {
        let mut app = level_app();
        let full = app.world_mut().spawn(Health::new(200.0)).id();
        let half = app
            .world_mut()
            .spawn(Health {
                current: 100.0,
                max: 200.0,
            })
            .id();
        let dead = app
            .world_mut()
            .spawn(Health {
                current: 0.0,
                max: 200.0,
            })
            .id();
        app.update();

        assert_eq!(level_of(&app, full), 0.0);
        assert_eq!(level_of(&app, half), 0.5);
        assert_eq!(level_of(&app, dead), 1.0);
    }

    /// A body is readable the frame it spawns, so no effect needs a "the level
    /// has not arrived yet" case of its own.
    #[test]
    fn a_body_has_a_level_before_anything_has_hit_it() {
        let mut app = level_app();
        let body = app.world_mut().spawn(Health::new(50.0)).id();
        app.update();

        assert_eq!(level_of(&app, body), 0.0);
    }

    /// Health moves, the level follows.
    #[test]
    fn the_level_follows_health_down() {
        let mut app = level_app();
        let body = app.world_mut().spawn(Health::new(100.0)).id();
        app.update();
        assert_eq!(level_of(&app, body), 0.0);

        app.world_mut().get_mut::<Health>(body).unwrap().current = 25.0;
        app.update();

        assert_eq!(level_of(&app, body), 0.75);
    }

    /// An effect that watches `Changed<DamageLevel>` must not be woken every
    /// frame of a long fight, so an unmoved reading is not a write.
    #[test]
    fn a_reading_that_did_not_move_is_not_a_change() {
        let mut app = level_app();
        let body = app.world_mut().spawn(Health::new(100.0)).id();
        app.update();

        // Touch health without changing it: `Changed<Health>` fires, and the
        // level must still decline to report a change of its own.
        app.world_mut().get_mut::<Health>(body).unwrap().current = 100.0;
        app.update();

        let level = app
            .world_mut()
            .query::<Ref<DamageLevel>>()
            .single(app.world())
            .expect("the body has a level");
        assert!(!level.is_changed(), "an unmoved reading is not a write");
    }

    /// A pool nothing can damage reads pristine, not destroyed. `0/0` would
    /// otherwise erode every zero-health prop in a scene on its first frame.
    #[test]
    fn a_body_with_no_health_at_all_reads_pristine() {
        let mut app = level_app();
        let prop = app
            .world_mut()
            .spawn(Health {
                current: 0.0,
                max: 0.0,
            })
            .id();
        app.update();

        assert_eq!(level_of(&app, prop), 0.0);
    }
}
