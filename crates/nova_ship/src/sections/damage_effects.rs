//! WHICH damage effects a section wears, authored in content.
//!
//! Damage is two readings - how far gone a body is
//! ([`DamageLevel`](nova_gameplay::prelude::DamageLevel)) and where it was hit
//! ([`DamageMarks`](nova_gameplay::prelude::DamageMarks)) - and neither says
//! what a section should LOOK like. That is each effect's business, and which
//! effects a section carries is a content decision, not an engine one.
//!
//! # One list, one translation
//!
//! A section authors [`DamageEffects`]: a list of [`DamageEffect`], each of
//! which this module turns into exactly one component. Nothing else translates,
//! and no effect system reads the list - each reads only its own component. So
//! the authored list is the CONTENT vocabulary, the components are the RUNTIME
//! vocabulary, and a Rust mod that wants a look nobody authored inserts its own
//! component and never touches either.
//!
//! # Why a list and not a section kind
//!
//! Because a turret and a thruster spark for the same reason and a hull does
//! not, and none of that follows from the kind. Keying effects off
//! `SectionKind` would put the decision in the engine, where a mod cannot reach
//! it, and would force every new look to become a new kind.
//!
//! # The default is SCORCH, not nothing
//!
//! An omitted list means `[Scorch]`, which is what every section did before it
//! was authorable. Nothing in the shipped catalog or in a third-party mod gets
//! quieter for not knowing about this, and a section that genuinely wants no
//! damage look at all authors the empty list and says so.
//!
//! The rule the vocabulary is kept honest by: ONLY NON-FUNCTIONAL MATERIAL IS
//! REMOVED. A turret that has taken a beating still has to point and shoot, so
//! it sparks rather than losing geometry, while a hull is material and nothing
//! else and can afford to be bitten into ([`DamageEffect::Carve`]). There is no
//! effect for shedding whole pieces because the sections that could afford to
//! shed are the ones already wearing skin - a plate is shot off and stays off.

use bevy::prelude::*;
use nova_gameplay::prelude::SectionMarker;

use crate::sections::{
    damage_carve::prelude::DamageCarve, damage_plume::prelude::DamagePlume,
    damage_sparks::prelude::DamageSparks, damage_tint::prelude::DamageScorch,
};

/// `DamageEffect`, `DamageEffects` and `DamageEffectsPlugin`.
pub mod prelude {
    pub use super::{DamageEffect, DamageEffects, DamageEffectsPlugin};
}

/// One authorable damage look.
///
/// Each variant maps to exactly one component - see
/// [`fit_damage_effects`]. Adding a look means a variant here, a component, and
/// one arm; nothing else in the engine learns its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DamageEffect {
    /// The section's paint reddens, darkens and finally burns out.
    Scorch,
    /// The section throws sparks, faster the worse it is, and keeps every part
    /// of itself.
    Sparks,
    /// The section's exhaust plume guts and flickers.
    Plume,
    /// The section loses real geometry where it was hit: a crater in the drawn
    /// mesh, not a mark on it.
    ///
    /// The one effect that costs the section its authored silhouette - the
    /// remesh draws it at the mesher's fidelity from then on - so it is for
    /// sections that are MATERIAL. A hull is a slab and survives that; a turret
    /// is a mechanism whose shape is the read, and it sparks instead.
    Carve,
}

/// The damage looks a section wears, authored in its content.
///
/// Both the authored value (a field of
/// [`BaseSectionConfig`](super::base_section::BaseSectionConfig)) and the live
/// component, so what a section is wearing can be read straight off it rather
/// than inferred from which components happen to be present.
///
/// [`Default`] is `[Scorch]` and not the empty list - see the module docs.
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DamageEffects(pub Vec<DamageEffect>);

impl Default for DamageEffects {
    fn default() -> Self {
        Self(vec![DamageEffect::Scorch])
    }
}

impl DamageEffects {
    /// A section that wears nothing at all: no colour grading, no sparks, no
    /// plume. Authored rather than defaulted, because "I want none" and "I did
    /// not say" are different statements.
    pub fn none() -> Self {
        Self(Vec::new())
    }

    /// Whether this list carries `effect`.
    pub fn wears(&self, effect: DamageEffect) -> bool {
        self.0.contains(&effect)
    }

    /// `skip_serializing_if` predicate: omit the field when it says exactly
    /// what an omitted field would have meant, so authored content stays clean
    /// and a catalog dump does not grow a line per section.
    #[cfg(feature = "serde")]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Turns each authored [`DamageEffect`] into its component.
///
/// SPLIT, like the skin's: the authored list is data a section carries whether
/// or not anybody is watching (a headless server loads and saves it), while
/// every component it names is read only by a render system. So the type is
/// always registered and the fitting only happens when sections draw.
#[derive(Default, Clone, Debug)]
pub struct DamageEffectsPlugin {
    /// Whether authored effects are fitted into their components.
    pub render: bool,
}

impl Plugin for DamageEffectsPlugin {
    fn build(&self, app: &mut App) {
        debug!("DamageEffectsPlugin: build");

        app.register_type::<DamageEffects>();
        if self.render {
            app.add_observer(fit_damage_effects);
        }
    }
}

/// Fit a section with the components its authored effects name.
///
/// Keyed on the SECTION rather than on [`DamageEffects`], so a section built by
/// hand - a test rig, a scenario fixture, anything not coming through
/// [`base_section`](super::base_section::base_section) - is fitted with the
/// default list instead of silently wearing nothing.
///
/// Insertion is one-way: nothing here removes a component a section already
/// carries, so a mod that inserted its own [`DamageSparks`] before this ran
/// keeps it.
///
/// [`DamageSparks`]: super::damage_sparks::DamageSparks
fn fit_damage_effects(
    add: On<Add, SectionMarker>,
    mut commands: Commands,
    q_effects: Query<&DamageEffects>,
) {
    let section = add.entity;
    // An unauthored section wears the default list, which is what it wore
    // before any of this was authorable.
    let default = DamageEffects::default();
    let effects = q_effects.get(section).unwrap_or(&default);

    // `try_insert` throughout: a section can be chain-destroyed the same frame
    // it spawns (a ship exploding as it loads), despawning it before the
    // command buffer applies.
    for effect in &effects.0 {
        let mut section = commands.entity(section);
        match effect {
            DamageEffect::Scorch => section.try_insert(DamageScorch),
            DamageEffect::Sparks => section.try_insert(DamageSparks::default()),
            DamageEffect::Plume => section.try_insert(DamagePlume),
            DamageEffect::Carve => section.try_insert(DamageCarve),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DamageEffectsPlugin { render: true });
        app
    }

    /// The claim the default exists for: nothing gets quieter for not knowing
    /// about this. A section built without an authored list still scorches,
    /// exactly as every section did before.
    #[test]
    fn an_unauthored_section_still_scorches() {
        let mut app = effects_app();
        let section = app.world_mut().spawn(SectionMarker).id();

        assert!(
            app.world().get::<DamageScorch>(section).is_some(),
            "an unauthored section wears the default list"
        );
        assert!(app.world().get::<DamageSparks>(section).is_none());
        assert!(app.world().get::<DamagePlume>(section).is_none());
    }

    /// An authored list is what the section wears, and nothing else.
    #[test]
    fn an_authored_list_is_fitted_exactly() {
        let mut app = effects_app();
        let turret = app
            .world_mut()
            .spawn((
                SectionMarker,
                DamageEffects(vec![DamageEffect::Scorch, DamageEffect::Sparks]),
            ))
            .id();

        assert!(app.world().get::<DamageScorch>(turret).is_some());
        assert!(app.world().get::<DamageSparks>(turret).is_some());
        assert!(
            app.world().get::<DamagePlume>(turret).is_none(),
            "an effect that was not authored is not fitted"
        );
    }

    /// "I want none" has to be sayable, and it has to differ from saying
    /// nothing - otherwise a section that should stay pristine cannot.
    #[test]
    fn a_section_can_author_no_damage_look_at_all() {
        let mut app = effects_app();
        let glass = app
            .world_mut()
            .spawn((SectionMarker, DamageEffects::none()))
            .id();

        assert!(app.world().get::<DamageScorch>(glass).is_none());
        assert!(app.world().get::<DamageSparks>(glass).is_none());
        assert!(app.world().get::<DamagePlume>(glass).is_none());
    }

    /// The empty list and the default list are different values, which is the
    /// whole reason `none()` is spelled out rather than being `Vec::new()`.
    #[test]
    fn saying_nothing_and_saying_none_are_different() {
        assert_ne!(DamageEffects::default(), DamageEffects::none());
        assert!(DamageEffects::default().wears(DamageEffect::Scorch));
        assert!(!DamageEffects::none().wears(DamageEffect::Scorch));
    }
}
