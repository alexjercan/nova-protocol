//! Nova's typed-damage layer: authored weapon damage, and the rule that decides
//! how far a round TRAVELS through what it hits.
//!
//! [`crate::integrity::health`] owns the HP store: one `on_damage` observer
//! subtracts `HealthApplyDamage.amount`, marks the node at zero, and propagates
//! up `ChildOf`. Damage reaching it is final - [`apply_damage`] is the single
//! place a weapon enters that store.
//!
//! A damage TYPE is not a set of multipliers. It is a way of travelling, which
//! is the thing a player can watch happen: a Kinetic slug punches (high per-hit,
//! and it carries on only through what it destroys), a Pierce round rakes (lower
//! per-hit, dealt in full to every layer it crosses, alive or dead). Closing
//! speed feeds each type's own resource - damage for Kinetic, power for Pierce.
//!
//! This module is the shared vocabulary every hit goes through - the turret, the
//! torpedo, a torpedo's blast, and a ram: [`DamageType`], the
//! [`ProjectileDamage`] a projectile carries, the [`SectionClass`] label a hit
//! lands on, the closing-speed curves, and the travel rule
//! ([`spend_piercing_damage`]) that decides whether a round survives what it
//! just hit.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::integrity::health::prelude::{Health, HealthApplyDamage};

/// The damage types and colours, blast spawning, the closing-speed curves, the
/// travel rule and `NovaDamagePlugin`.
pub mod prelude {
    pub use super::{
        apply_damage, closing_speed, damage_type_color, hit_bite, kinetic_damage_multiplier,
        nova_blast, pierce_power_multiplier, pierce_remainder, representative_kinetic_damage,
        spend_piercing_damage, DamageType, NovaBlast, NovaDamagePlugin, ProjectileDamage,
        SectionClass, MAX_PIERCE_LAYERS, NEUTRALIZED_BULLET_MASS, PIERCE_BASE_POWER,
        REFERENCE_CLOSING_SPEED,
    };
}

/// How a projectile hurts: two bullet types that differ in how they TRAVEL, plus
/// the torpedo's blast.
///
/// Neither bullet type is a multiplier on the other. A slug and a penetrator do
/// different things to a stack of sections, and that difference is visible from
/// the cockpit - which is the whole reason the resistance table it replaced is
/// gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DamageType {
    /// The punch. High per-hit damage, and closing speed makes it HARDER. It
    /// carries on only through what it DESTROYS, spending its damage as the
    /// budget - so it is the answer to one thin target, and it stops dead at
    /// anything it cannot kill.
    Kinetic,
    /// The rake. Lower per-hit damage, NOT scaled by speed, dealt in full to
    /// every layer it crosses whether that layer died or not. Closing speed
    /// feeds its POWER instead - how much thickness it gets through - so it is
    /// the answer to something deep.
    Pierce,
    /// Concussive area damage: the torpedo's blast. Not a bullet type - a blast
    /// has no line of flight, so no closing speed and no travel rule. Its
    /// identity is its radius and magnitude.
    Explosive,
}

/// What a projectile deals per hit, and what it has left to keep going with.
///
/// Making weapon damage AUTHORED (not emergent from bullet mass x velocity) is
/// the point of the typed pass: "a slug does X, a penetrator does Y" cannot come
/// out of one kinetic formula.
///
/// `amount` means the same thing to both types - the damage ONE hit deals,
/// before the Kinetic speed curve - but only Kinetic spends it: a slug's damage
/// IS its budget, and it shrinks as the round kills its way forward. A
/// penetrator's `amount` never changes; it pays for travel out of [`power`],
/// a wholly separate resource in HIT POINT units of section thickness.
///
/// [`power`]: ProjectileDamage::power
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct ProjectileDamage {
    /// Damage one hit deals, before the Kinetic speed curve. For a Kinetic round
    /// this doubles as the budget and decays; for a Pierce round it is flat.
    pub amount: f32,
    /// PIERCE ONLY: thickness the round can still get through, priced in the
    /// MAX health of each layer it crosses. Unused by the other types.
    pub power: f32,
    /// PIERCE ONLY: how many more layers the round may cross, whatever its
    /// power. The backstop against one round chaining down the long axis of a
    /// ship made of cheap plates.
    pub layers: u32,
    /// Which travel rule and which speed curve this projectile uses.
    pub kind: DamageType,
}

impl ProjectileDamage {
    /// A freshly fired round: full [`PIERCE_BASE_POWER`] and
    /// [`MAX_PIERCE_LAYERS`], which only a Pierce round reads.
    pub fn new(amount: f32, kind: DamageType) -> Self {
        Self {
            amount,
            power: PIERCE_BASE_POWER,
            layers: MAX_PIERCE_LAYERS,
            kind,
        }
    }
}

/// Which section kind a collider belongs to: the ship computer's label for it.
///
/// A discriminant-only mirror of `nova_ship`'s `SectionKind` (which carries
/// per-kind config), inserted alongside each section's kind marker (see the
/// `*_section` bundles) so one query resolves what a section IS. `nova_os_ui`
/// reads it for section codes, glyphs and descriptions. Colliders without it
/// (asteroids, debris) are simply not sections; nothing in the
/// damage path branches on it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum SectionClass {
    /// Hull section: armored structure.
    Hull,
    /// Thruster section: propulsion.
    Thruster,
    /// Controller section: the command core.
    Controller,
    /// Turret section: the kinetic weapon mount.
    Turret,
    /// Torpedo section: the explosive weapon mount.
    Torpedo,
}

/// Physical mass given to a turret bullet so the emergent kinetic term rounds
/// to nothing.
///
/// [`on_impact_collision_deal_damage`](crate::integrity::core) computes damage
/// from `effective_mass = m_bullet * m_ship / (m_bullet + m_ship)` (~ `m_bullet`
/// since a ship is far heavier), so a near-zero bullet mass makes the impact
/// contribution negligible
/// next to nova's authored [`ProjectileDamage`], leaving the typed amount as
/// the only weapon damage. Gravity is unaffected: `gravity_well_system` applies
/// a mass-INDEPENDENT acceleration (`forces.apply_linear_acceleration`), and
/// Sensor bullets take no contact forces, so a tiny mass changes neither flight
/// nor knockback. Kept small but non-zero to avoid a zero-mass dynamic body.
pub const NEUTRALIZED_BULLET_MASS: f32 = 1.0e-6;

/// The closing speed both bullet curves read exactly 1.0 at, in units/second.
///
/// Chosen as the shipped PDC's `muzzle_speed` (100.0, on both PDC prototypes
/// and `better_turret_section`), which is also the speed
/// [`representative_kinetic_damage`] authored the historical per-hit against.
/// A round fired from a station-keeping ship at a station-keeping target closes
/// at exactly its muzzle speed, so the normal engagement multiplier is 1.0 and
/// every authored `bullet_damage` keeps the feel it was tuned for. Everything
/// speed adds or removes is therefore a deliberate departure from a
/// station-keeping duel, not a silent rebalance of the catalog.
pub const REFERENCE_CLOSING_SPEED: f32 = 100.0;

/// Floor on the Kinetic damage curve: a stern chase HURTS a slug, but a round
/// that catches its target from behind must still land a hit worth firing (and
/// a round overtaken by a faster ship, whose closing speed is negative, must
/// not deal zero or negative damage).
const KINETIC_DAMAGE_FLOOR: f32 = 0.25;

/// Ceiling on the Kinetic damage curve. A PDC compounds per-hit damage by fire
/// rate, so the head-on case is the one that can run away: 2.0 doubles a
/// charging pass and no more, which at the shipped 4.0/hit x 100 rounds/s is
/// ~800 DPS - punchy, still not a delete button.
const KINETIC_DAMAGE_CEILING: f32 = 2.0;

/// Floor on the Pierce power curve: a stern-chase penetrator arrives with half
/// its rated power and rakes half as deep, but it never becomes inert.
const PIERCE_POWER_FLOOR: f32 = 0.5;

/// Ceiling on the Pierce power curve: treble power, which is the difference
/// between raking a corvette's flank and raking it end to end. Above that,
/// [`MAX_PIERCE_LAYERS`] is the binding limit anyway.
const PIERCE_POWER_CEILING: f32 = 3.0;

/// Thickness a Pierce round spawns able to cross, in hit points of section MAX
/// health, at the reference closing speed.
///
/// Priced against the shipped catalog: a light hull is 60, a thruster 70, a
/// controller or torpedo bay 100, a turret 130 and a reinforced hull 200. 300
/// therefore rakes three or four light sections, or barely gets past ONE
/// reinforced hull block - which is the spaced-armour intuition the max-health
/// pricing exists for. The first playtest knob to turn if pierce feels weak or
/// oppressive.
pub const PIERCE_BASE_POWER: f32 = 300.0;

/// Hard cap on how many layers one Pierce round may cross, whatever its power.
///
/// Power alone does not bound a rake: thin plating is cheap by design, so a hull
/// faced with 5 hp panels would let one round chain through dozens of them.
/// Six is past any shipped craft's depth along a single line of fire, so it
/// binds only in the degenerate case it exists for.
pub const MAX_PIERCE_LAYERS: u32 = 6;

/// Closing speed of a round against what it is about to hit: the component of
/// their relative velocity along the round's own line of flight, in
/// units/second.
///
/// Positive means the two are converging. Projecting onto the ROUND's line (not
/// onto the line between the two bodies) is what makes this stable at the
/// impact site: at contact the two bodies are touching, so the vector between
/// them is near zero and its direction is noise, while a bullet's velocity is
/// never zero. It also gives the term the intended meaning - a target sliding
/// sideways is not fleeing, and does not change the round's arrival energy.
pub fn closing_speed(round_velocity: Vec3, target_velocity: Vec3) -> f32 {
    let Some(line) = round_velocity.try_normalize() else {
        return 0.0;
    };
    (round_velocity - target_velocity).dot(line)
}

/// How much harder a Kinetic round hits at `closing_speed`: the speed ratio
/// against [`REFERENCE_CLOSING_SPEED`], clamped to
/// `[KINETIC_DAMAGE_FLOOR, KINETIC_DAMAGE_CEILING]`.
///
/// LINEAR in closing speed on purpose. The engine's own ram model
/// ([`impact_damage`](crate::integrity::core::impact_damage)) is an impulse
/// term (linear in speed) plus an absorbed-energy term (quadratic), and at
/// bullet speeds the quadratic half dominates so hard that the full curve reads
/// ~3.9x at twice the reference - which would turn the ~400 DPS PDC into ~1600
/// on a head-on pass. The impulse half keeps the same direction and the same
/// anchor with a magnitude a point-defense weapon can carry.
pub fn kinetic_damage_multiplier(closing_speed: f32) -> f32 {
    (closing_speed / REFERENCE_CLOSING_SPEED).clamp(KINETIC_DAMAGE_FLOOR, KINETIC_DAMAGE_CEILING)
}

/// How much deeper a Pierce round rakes at `closing_speed`: the same speed
/// ratio, clamped to `[PIERCE_POWER_FLOOR, PIERCE_POWER_CEILING]`.
///
/// Same linear shape as the Kinetic curve - the depth a rigid penetrator
/// reaches scales with the momentum it arrives with - but it buys an entirely
/// different resource. It prices what crossing a layer COSTS, never what the
/// layer takes, so a penetrator's per-hit damage is the number the weapon
/// authored no matter how the two ships are moving.
pub fn pierce_power_multiplier(closing_speed: f32) -> f32 {
    (closing_speed / REFERENCE_CLOSING_SPEED).clamp(PIERCE_POWER_FLOOR, PIERCE_POWER_CEILING)
}

/// The identifying color of a damage type, for HUD conveyance (the ammo readout
/// colors its pips by the loaded round's type). Opaque hue - callers apply
/// their own alpha (lit vs dim). Kinetic is the readout's historical amber so a
/// Kinetic weapon looks unchanged; the others are distinct hues (steel blue,
/// red-orange) that read on the dark HUD behind the pip outline.
pub fn damage_type_color(kind: DamageType) -> Color {
    match kind {
        // The original ammo-readout amber (LIT_COLOR's hue) - unchanged look.
        DamageType::Kinetic => Color::srgb(1.0, 0.75, 0.2),
        // Hardened penetrator: cold steel blue.
        DamageType::Pierce => Color::srgb(0.6, 0.75, 1.0),
        // Concussive: red-orange fire.
        DamageType::Explosive => Color::srgb(1.0, 0.4, 0.15),
    }
}

/// Spend `amount` hit points on `target`, attributed to `source`.
///
/// The single point at which a weapon enters the health store, so every weapon
/// - turret, torpedo blast, ram - lands identically. It is a plain trigger:
/// damage is one number now, and nothing between the weapon and
/// [`on_damage`](crate::integrity::health) reinterprets it.
pub fn apply_damage(commands: &mut Commands, target: Entity, source: Option<Entity>, amount: f32) {
    commands.trigger(HealthApplyDamage {
        entity: target,
        source,
        amount,
    });
}

/// The damage ONE hit delivers, before the health store clamps it to what the
/// target has left.
///
/// The whole per-hit difference between the two bullet types:
///
/// - Kinetic bites with its remaining budget scaled by
///   [`kinetic_damage_multiplier`] - hard on arrival, softer once it has spent
///   some of itself killing;
/// - Pierce bites its authored `amount`, flat. Not scaled by speed, and not
///   decayed by depth: the fifth layer takes exactly what the first did. A
///   decay curve would be more realistic and much harder to aim with.
pub fn hit_bite(damage: ProjectileDamage, closing_speed: f32) -> f32 {
    match damage.kind {
        DamageType::Kinetic => damage.amount * kinetic_damage_multiplier(closing_speed),
        DamageType::Pierce | DamageType::Explosive => damage.amount,
    }
}

/// What the round is after meeting `target` at `closing_speed`, or `None` when
/// it is expended there.
///
/// The travel rule, as pure arithmetic, one branch per type:
///
/// - KINETIC spends its damage. A hit that fails to destroy the target has by
///   definition put the whole bite into it, so the round dies; a hit that
///   destroys it costs only the health that was actually there, priced back
///   through the speed curve that scaled the bite, and the rest flies on. A
///   slug therefore can never deal more than it was fired with.
/// - PIERCE spends POWER, never damage. Crossing a layer costs that layer's MAX
///   health divided by [`pierce_power_multiplier`] - MAX, not remaining, for two
///   reasons: thin plating stays nearly free while a hull block is expensive
///   (the spaced-armour intuition), and softening a section with other fire
///   cannot make it cheaper to rake through. It crosses whether or not the layer
///   died, so its TOTAL damage legitimately exceeds what it was fired with.
///   [`MAX_PIERCE_LAYERS`] is the backstop under the power budget.
///
/// A target with no [`Health`] - an asteroid, a planetoid, a collider whose pool
/// lives on an ancestor - has no thickness this rule can price and nothing it
/// can prove destroyed, so it is a wall to both types at any speed.
pub fn pierce_remainder(
    damage: ProjectileDamage,
    target: Option<&Health>,
    closing_speed: f32,
) -> Option<ProjectileDamage> {
    let health = target?;
    match damage.kind {
        DamageType::Kinetic => {
            let scale = kinetic_damage_multiplier(closing_speed);
            if damage.amount * scale <= health.current {
                // Absorbed whole - including the exact kill, which leaves
                // nothing to carry on with anyway.
                return None;
            }
            let left = damage.amount - health.current / scale;
            (left > 0.0).then(|| ProjectileDamage {
                amount: left,
                ..damage
            })
        }
        DamageType::Pierce => {
            let layers = damage.layers.saturating_sub(1);
            let power = damage.power - health.max / pierce_power_multiplier(closing_speed);
            (layers > 0 && power > 0.0).then(|| ProjectileDamage {
                power,
                layers,
                ..damage
            })
        }
        // A blast does not travel; a round somehow carrying one is spent where
        // it lands.
        DamageType::Explosive => None,
    }
}

/// Deal a round's bite to `target` and report what the round has left: `Some` =
/// it flies on, `None` = it is expended here.
///
/// The generic travel seam for every projectile that wants it. Damage goes
/// through [`apply_damage`] whichever way it goes, so this adds no second health
/// pipeline - it only decides what happens to the ROUND. A Kinetic round leaves
/// real geometry behind it (it only continues through what it destroyed); a
/// Pierce round needs no geometry at all, because a sensor round was never
/// stopped by the section it crossed - only by this rule.
///
/// `target_health` is read at the callsite, one flush before the health store
/// actually subtracts. Two rounds landing on the same section inside one flush
/// therefore both see it alive; that is a frame of optimism about who gets the
/// kill, not a way for a slug to exceed its budget.
pub fn spend_piercing_damage(
    commands: &mut Commands,
    target: Entity,
    source: Option<Entity>,
    target_health: Option<&Health>,
    damage: ProjectileDamage,
    closing_speed: f32,
) -> Option<ProjectileDamage> {
    apply_damage(commands, target, source, hit_bite(damage, closing_speed));
    pierce_remainder(damage, target_health, closing_speed)
}

/// The per-hit kinetic damage the emergent impact model deals for a bullet of `mass`
/// striking at relative speed `speed`, approximating `effective_mass ~ mass`
/// (a target ship is far heavier than a bullet, so the effective mass is within
/// a few percent of the bullet mass across every ship).
///
/// Used to AUTHOR the turret's fixed Kinetic `amount` so the typed system
/// preserves the old feel at a representative engagement speed (which is also
/// [`REFERENCE_CLOSING_SPEED`], where the Kinetic curve reads 1.0).
///
/// It IS the ram formula - [`impact_damage`](crate::integrity::core::impact_damage),
/// which nova owns - so the two can no longer drift. It used to be a hand copy
/// of the same constants out of a third-party crate, carrying an apology for
/// duplicating them.
pub fn representative_kinetic_damage(mass: f32, speed: f32) -> f32 {
    crate::integrity::core::impact_damage(mass, speed)
}

/// A radial blast volume: a static sensor sphere that damages everything it
/// overlaps, falling off linearly to zero at `radius`.
///
/// The falloff IS the blast's shape: a torpedo detonates outside a hull, so the
/// outer sections are nearer and take more, which buys an exterior-to-interior
/// gradient without any occlusion rule. There is deliberately none - light plating
/// gives no cover against a blast, and that is what makes torpedoes the counter
/// to armour a bullet cannot rake through. Pair it with a short `TempEntity` so
/// the volume cleans itself up after the frame it fires.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct NovaBlast {
    /// Bodies beyond this take no damage.
    pub radius: f32,
    /// Damage at the blast centre (distance 0).
    pub max_damage: f32,
    /// The blast's damage type (Explosive for torpedoes).
    pub kind: DamageType,
}

/// Bundle for a nova typed blast volume: a Static sensor sphere that owns its
/// collision events, so it raises `CollisionStart` against every overlapped
/// collider, and routes damage through `on_nova_blast_collision`. Spawn with a `Transform` at the centre
/// and a short `TempEntity` so it cleans itself up.
pub fn nova_blast(radius: f32, max_damage: f32, kind: DamageType) -> impl Bundle {
    (
        Name::new("NovaBlastArea"),
        NovaBlast {
            radius,
            max_damage,
            kind,
        },
        RigidBody::Static,
        Collider::sphere(radius),
        Sensor,
        CollisionEventsEnabled,
        Visibility::Visible,
    )
}

/// Linear falloff to zero at `radius`.
fn blast_falloff(distance: f32, radius: f32, max_damage: f32) -> f32 {
    if distance >= radius {
        0.0
    } else {
        max_damage * (1.0 - distance / radius)
    }
}

/// Apply nova blast damage to every body a [`NovaBlast`] sensor overlaps.
///
/// The blast is the `body1`/self side of the event - it owns the collision
/// events (see [`nova_blast`]), so avian raises `CollisionStart` against every
/// collider it overlaps regardless of the target's own configuration. The
/// swapped `{body1 = target}` ordering is ignored because `q_blast.get(blast)`
/// fails on the target side, so each overlap deals damage exactly once and never
/// double-dips. `source` is the blast collider, so the AI threat model resolves
/// it to the shooter through the blast entity's `ProjectileOwner`.
fn on_nova_blast_collision(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_blast: Query<(&Transform, &NovaBlast)>,
    q_body: Query<&Transform, With<RigidBody>>,
) {
    let blast_collider = collision.collider1;
    let target_collider = collision.collider2;
    let Some(blast) = collision.body1 else {
        return;
    };
    let Some(target) = collision.body2 else {
        return;
    };

    // Only act when this side of the event is the blast; the swapped ordering is
    // handled by its own event (or ignored entirely).
    let Ok((blast_transform, blast_config)) = q_blast.get(blast) else {
        return;
    };
    let Ok(target_transform) = q_body.get(target) else {
        return;
    };

    let distance = blast_transform
        .translation
        .distance(target_transform.translation);
    let amount = blast_falloff(distance, blast_config.radius, blast_config.max_damage);
    if amount <= f32::EPSILON {
        return;
    }

    apply_damage(&mut commands, target_collider, Some(blast_collider), amount);
}

/// Registers the typed-damage reflection types and the nova blast observer.
///
/// The application HELPER ([`apply_damage`]) is called from the weapon-hit
/// callsites in their own modules (turret `resolve_bullet_hit`, torpedo
/// detonate); this plugin owns only the nova-blast observer and type
/// registration.
pub struct NovaDamagePlugin;

impl Plugin for NovaDamagePlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaDamagePlugin: build");
        app.register_type::<DamageType>()
            .register_type::<ProjectileDamage>()
            .register_type::<SectionClass>()
            .register_type::<NovaBlast>();
        app.add_observer(on_nova_blast_collision);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        integrity::health::prelude::Health,
        test_support::{integrity_physics_app, settle},
    };

    fn health(app: &App, entity: Entity) -> f32 {
        app.world().get::<Health>(entity).unwrap().current
    }

    /// Every travel-rule test that is about the RULE and not about speed runs at
    /// the anchor, where both curves read 1.0.
    const REFERENCE: f32 = REFERENCE_CLOSING_SPEED;

    /// A layer of `max` hit points, `current` of them left.
    fn worn(current: f32, max: f32) -> Health {
        Health { current, max }
    }

    /// A ship-shaped target: a RigidBody parent with a single child collider that
    /// carries the Health (and optional section class), mirroring how nova ships
    /// hold section colliders under a root body. Returns `(body, collider)`; avian
    /// reports the parent as `body*` and the child as `collider*` in a
    /// CollisionStart, and damage lands on (and health lives on) the child.
    fn spawn_target(
        app: &mut App,
        at: Vec3,
        hp: f32,
        class: Option<SectionClass>,
    ) -> (Entity, Entity) {
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::from_translation(at)))
            .id();
        let mut collider = app.world_mut().spawn((
            ChildOf(body),
            Collider::sphere(1.0),
            ColliderDensity(1.0),
            Health::new(hp),
        ));
        if let Some(class) = class {
            collider.insert(class);
        }
        (body, collider.id())
    }

    #[test]
    fn a_kinetic_round_that_fails_to_destroy_its_target_is_expended_on_it() {
        // The slug's rule, unchanged: it carries on only through what it KILLS,
        // and a dent stops it - including the hit that kills the target outright
        // with nothing to spare.
        let kinetic = ProjectileDamage::new(20.0, DamageType::Kinetic);
        assert!(pierce_remainder(kinetic, Some(&Health::new(100.0)), REFERENCE).is_none());
        assert!(pierce_remainder(kinetic, Some(&Health::new(20.0)), REFERENCE).is_none());
    }

    #[test]
    fn a_kinetic_round_that_destroys_its_target_carries_the_rest_on() {
        // 100 damage into a 20 hp plate leaves 80 for whatever is behind it.
        let kinetic = ProjectileDamage::new(100.0, DamageType::Kinetic);
        let left = pierce_remainder(kinetic, Some(&Health::new(20.0)), REFERENCE)
            .expect("a 100-point slug destroys a 20 hp plate and flies on");
        assert_eq!(left.amount, 80.0);
    }

    #[test]
    fn an_indestructible_target_stops_every_round_however_fast_it_closes() {
        // No health pool (an asteroid, a planetoid, a collider whose health
        // lives on an ancestor) means no thickness to price and nothing this
        // rule can prove destroyed, so it is a wall to both types.
        let kinetic = ProjectileDamage::new(1000.0, DamageType::Kinetic);
        let pierce = ProjectileDamage::new(1000.0, DamageType::Pierce);
        assert!(pierce_remainder(kinetic, None, REFERENCE).is_none());
        assert!(pierce_remainder(pierce, None, 10.0 * REFERENCE).is_none());
    }

    #[test]
    fn a_spent_target_costs_a_slug_nothing_and_a_penetrator_its_thickness() {
        // A section already at zero (dead, not yet swept away) absorbs nothing,
        // so a slug must not be charged for it either. A penetrator still pays:
        // the wreck is as thick as it ever was, which is exactly why power is
        // priced on MAX health - softening a section first must not open a
        // cheaper hole through it.
        let kinetic = ProjectileDamage::new(100.0, DamageType::Kinetic);
        let left = pierce_remainder(kinetic, Some(&worn(0.0, 100.0)), REFERENCE)
            .expect("a corpse absorbs nothing, so the slug is undiminished");
        assert_eq!(left.amount, 100.0);

        let pierce = ProjectileDamage::new(2.0, DamageType::Pierce);
        let raked = pierce_remainder(pierce, Some(&worn(0.0, 100.0)), REFERENCE)
            .expect("the penetrator still gets through");
        assert_eq!(
            raked.power,
            PIERCE_BASE_POWER - 100.0,
            "a spent layer costs its FULL max health in power"
        );
    }

    #[test]
    fn pierce_power_is_priced_on_max_health_not_on_what_is_left() {
        // The degenerate loop this rule exists to close: soften a section with
        // other fire and it must NOT become cheaper to rake through.
        let pierce = ProjectileDamage::new(2.0, DamageType::Pierce);
        let fresh = pierce_remainder(pierce, Some(&Health::new(120.0)), REFERENCE).unwrap();
        let softened = pierce_remainder(pierce, Some(&worn(5.0, 120.0)), REFERENCE).unwrap();
        assert_eq!(fresh.power, softened.power);
    }

    #[test]
    fn a_slug_conserves_its_damage_down_a_row_of_plates() {
        // The conservation property the Kinetic budget exists for, and which is
        // now KINETIC-ONLY: whatever the plates are worth, the sum of what they
        // absorb is exactly what the round carried.
        let (absorbed, _) = spend_down_plates(
            ProjectileDamage::new(100.0, DamageType::Kinetic),
            REFERENCE,
            &[20.0, 30.0, 25.0, 40.0],
        );
        assert!((absorbed - 100.0).abs() < 1e-3, "absorbed {absorbed}");
    }

    /// Walk a round down a row of plates of `plate_hp` each (full health), and
    /// report `(total absorbed, layers hit)`.
    fn spend_down_plates(
        mut damage: ProjectileDamage,
        closing: f32,
        plates: &[f32],
    ) -> (f32, usize) {
        let mut absorbed = 0.0;
        let mut hits = 0;
        for &plate_hp in plates {
            absorbed += hit_bite(damage, closing).min(plate_hp);
            hits += 1;
            match pierce_remainder(damage, Some(&Health::new(plate_hp)), closing) {
                Some(left) => damage = left,
                None => break,
            }
        }
        (absorbed, hits)
    }

    #[test]
    fn a_penetrators_total_deliberately_exceeds_what_it_was_fired_with() {
        // The opposite of the slug invariant, and the point of the rake: a
        // Pierce round pays for travel out of POWER, so its damage does not
        // deplete. Four 50 hp layers take 2 each from a round authored at 2.
        let pierce = ProjectileDamage::new(2.0, DamageType::Pierce);
        let (absorbed, hits) = spend_down_plates(pierce, REFERENCE, &[50.0; 8]);
        assert_eq!(hits, 6, "MAX_PIERCE_LAYERS caps the rake at six layers");
        assert!(
            absorbed > pierce.amount,
            "a rake's total must exceed its authored per-hit, got {absorbed}"
        );
        assert_eq!(
            absorbed, 12.0,
            "six layers x 2 damage, undiminished by depth"
        );
    }

    #[test]
    fn pierce_power_stops_a_rake_through_thick_armour_long_before_the_layer_cap() {
        // The power budget, not the backstop, is what normally ends a rake: at
        // 200 hp a reinforced hull block costs two thirds of a fresh round's
        // power, so it gets through one and dies in the second.
        let pierce = ProjectileDamage::new(2.0, DamageType::Pierce);
        let (_, hits) = spend_down_plates(pierce, REFERENCE, &[200.0; 6]);
        assert_eq!(
            hits, 2,
            "300 power buys one 200 hp block and stops inside the next"
        );
    }

    #[test]
    fn the_layer_cap_bounds_a_rake_through_near_free_plating() {
        // Power alone cannot bound a rake, because thin plating is cheap on purpose.
        // Twenty 1 hp panels cost 20 of 300 power; the cap is what stops
        // the round.
        let pierce = ProjectileDamage::new(2.0, DamageType::Pierce);
        let (_, hits) = spend_down_plates(pierce, REFERENCE, &[1.0; 20]);
        assert_eq!(hits, MAX_PIERCE_LAYERS as usize);
    }

    #[test]
    fn both_speed_curves_read_exactly_one_at_the_reference_closing_speed() {
        // The anchor the whole pass is built on: at a station-keeping
        // engagement every authored bullet_damage lands exactly as authored, and
        // a penetrator gets exactly its rated thickness.
        assert_eq!(kinetic_damage_multiplier(REFERENCE_CLOSING_SPEED), 1.0);
        assert_eq!(pierce_power_multiplier(REFERENCE_CLOSING_SPEED), 1.0);
        for kind in [DamageType::Kinetic, DamageType::Pierce] {
            let round = ProjectileDamage::new(20.0, kind);
            assert!(
                (hit_bite(round, REFERENCE_CLOSING_SPEED) - 20.0).abs() < 1e-4,
                "{kind:?} must bite its authored 20.0 at the reference speed"
            );
        }
        // And the anchored power cost of a layer is that layer's max health.
        let pierce = ProjectileDamage::new(2.0, DamageType::Pierce);
        let left = pierce_remainder(pierce, Some(&Health::new(100.0)), REFERENCE).unwrap();
        assert_eq!(left.power, PIERCE_BASE_POWER - 100.0);
    }

    #[test]
    fn closing_speed_rises_on_a_charge_and_falls_on_a_flight() {
        // The term the curves read. A round flying -Z at 100 into a target at
        // rest closes at 100; a target running away closes slower, one charging
        // in closes faster, and one sliding sideways is neither.
        let round = Vec3::NEG_Z * 100.0;
        assert!((closing_speed(round, Vec3::ZERO) - 100.0).abs() < 1e-4);
        assert!((closing_speed(round, Vec3::NEG_Z * 40.0) - 60.0).abs() < 1e-4);
        assert!((closing_speed(round, Vec3::Z * 40.0) - 140.0).abs() < 1e-4);
        assert!((closing_speed(round, Vec3::X * 40.0) - 100.0).abs() < 1e-4);
        // A round with no velocity has no line of flight to project onto.
        assert_eq!(closing_speed(Vec3::ZERO, Vec3::X), 0.0);
    }

    #[test]
    fn a_kinetic_round_closing_fast_bites_harder_than_one_closing_slow() {
        // Kinetic's identity: the same authored round is a different weapon on
        // a charge than on a run.
        let round = ProjectileDamage::new(20.0, DamageType::Kinetic);
        let charging = hit_bite(round, 1.5 * REFERENCE);
        let anchored = hit_bite(round, REFERENCE);
        let fleeing = hit_bite(round, 0.5 * REFERENCE);
        assert!(
            fleeing < anchored && anchored < charging,
            "kinetic bite must rise with closing speed: {fleeing} / {anchored} / {charging}"
        );
        assert!((charging - 30.0).abs() < 1e-4, "1.5x closing = 1.5x damage");
    }

    #[test]
    fn a_pierce_round_closing_fast_rakes_deeper_without_biting_harder() {
        // Pierce's identity, the exact contrast with the test above: speed buys
        // POWER, never damage. Same bite, more layers crossed.
        let round = ProjectileDamage::new(20.0, DamageType::Pierce);
        assert_eq!(
            hit_bite(round, 3.0 * REFERENCE),
            hit_bite(round, REFERENCE),
            "a pierce round must not deal more per hit for closing faster"
        );
        // 200 hp blocks: 300 power buys one at the anchor and three at treble.
        let (_, anchored) = spend_down_plates(round, REFERENCE, &[200.0; 6]);
        let (_, fast) = spend_down_plates(round, 3.0 * REFERENCE, &[200.0; 6]);
        assert!(
            fast > anchored,
            "a fast rake must cross more layers: {fast} vs {anchored}"
        );
    }

    #[test]
    fn both_speed_curves_are_clamped_at_absurd_and_near_zero_closing_speeds() {
        // Neither end may run away: a head-on charge is bounded, and a stern
        // chase (or a round the shooter has overtaken, whose closing speed goes
        // NEGATIVE) still lands something rather than nothing.
        for absurd in [50.0 * REFERENCE, f32::MAX] {
            assert_eq!(kinetic_damage_multiplier(absurd), KINETIC_DAMAGE_CEILING);
            assert_eq!(pierce_power_multiplier(absurd), PIERCE_POWER_CEILING);
        }
        for crawling in [0.0, 1.0, -200.0] {
            assert_eq!(kinetic_damage_multiplier(crawling), KINETIC_DAMAGE_FLOOR);
            assert_eq!(pierce_power_multiplier(crawling), PIERCE_POWER_FLOOR);
        }
        // The floors are what keep a tail chase from being free: a slug still
        // bites, and a penetrator still gets through something.
        assert!(hit_bite(ProjectileDamage::new(20.0, DamageType::Kinetic), -200.0) > 0.0);
        let crawling = ProjectileDamage::new(2.0, DamageType::Pierce);
        assert!(pierce_remainder(crawling, Some(&Health::new(50.0)), -200.0).is_some());
    }

    #[test]
    fn damage_type_color_is_distinct_per_type_and_kinetic_is_the_readout_amber() {
        let colors = [
            damage_type_color(DamageType::Kinetic),
            damage_type_color(DamageType::Pierce),
            damage_type_color(DamageType::Explosive),
        ];
        // Every pair distinct, so the ammo readout reads a different color per
        // loaded type (would fail if two types shared a hue).
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "types {i} and {j} share a color");
            }
        }
        // Kinetic keeps the historical readout amber, so a Kinetic weapon looks
        // exactly as it did before typed ammo.
        assert_eq!(
            damage_type_color(DamageType::Kinetic),
            Color::srgb(1.0, 0.75, 0.2)
        );
    }

    #[test]
    fn authored_turret_amounts_reproduce_the_old_emergent_kinetic() {
        // Pins the authored `bullet_damage` values in nova_assets/sections.rs to
        // the historical emergent per-hit (better turret mass 0.1 @ 100 u/s;
        // light turret mass 0.05 @ 60 u/s), so the Kinetic anchor is genuinely
        // feel-preserving. If these move, the config values must move with them.
        assert!((representative_kinetic_damage(0.1, 100.0) - 20.25).abs() < 1e-3);
        assert!((representative_kinetic_damage(0.05, 60.0) - 3.825).abs() < 1e-3);
    }

    #[test]
    fn neutralized_bullet_mass_makes_the_emergent_kinetic_negligible() {
        // Drive the REAL impact observer against a neutralized-mass bullet
        // and confirm the emergent kinetic it deals is negligible, then A/B the
        // same rig at the old 0.1 mass to prove the test can fail (the old mass
        // deals ~20). This is the neutralization the typed path depends on.
        fn emergent_impact_damage(bullet_mass: f32) -> f32 {
            let mut app = integrity_physics_app();
            let (target_body, target_collider) = spawn_target(&mut app, Vec3::ZERO, 1000.0, None);
            let bullet = app
                .world_mut()
                .spawn((
                    RigidBody::Dynamic,
                    Collider::sphere(0.05),
                    Sensor,
                    Mass(bullet_mass),
                    Transform::from_xyz(10.0, 0.0, 0.0),
                ))
                .id();
            settle(&mut app);
            // Bullet closing at 100 u/s head-on; target at rest.
            app.world_mut().get_mut::<LinearVelocity>(bullet).unwrap().0 =
                Vec3::new(-100.0, 0.0, 0.0);
            app.world_mut()
                .get_mut::<LinearVelocity>(target_body)
                .unwrap()
                .0 = Vec3::ZERO;
            // Target is collider1/body1 so the impact lands on the section.
            app.world_mut().trigger(CollisionStart {
                collider1: target_collider,
                collider2: bullet,
                body1: Some(target_body),
                body2: Some(bullet),
            });
            app.update();
            1000.0 - health(&app, target_collider)
        }

        let neutralized = emergent_impact_damage(NEUTRALIZED_BULLET_MASS);
        let old = emergent_impact_damage(0.1);
        assert!(
            neutralized < 1.0e-2,
            "neutralized bullet must deal ~0 emergent kinetic, got {neutralized}"
        );
        assert!(
            old > 15.0,
            "A/B guard: the old 0.1 mass must deal real emergent kinetic (got {old}), \
             else this test proves nothing"
        );
    }

    #[test]
    fn the_health_store_subtracts_exactly_what_the_weapon_decided() {
        // The one-application-point contract end to end: a weapon triggers
        // HealthApplyDamage through `apply_damage` and on_damage subtracts
        // exactly that - nothing in between reinterprets the number.
        let mut app = integrity_physics_app();
        let (_body, target) = spawn_target(&mut app, Vec3::ZERO, 100.0, Some(SectionClass::Turret));
        settle(&mut app);
        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 17.5,
        });
        app.update();
        assert!(
            (health(&app, target) - 82.5).abs() < 1e-3,
            "health should drop by exactly 17.5, got {}",
            health(&app, target)
        );
    }

    #[test]
    fn nova_blast_deals_its_falloff_once() {
        // A real sensor overlap fires the nova blast observer, which applies the
        // linear falloff once. The typed blast is the only blast path in the
        // app - so the drop is exactly the single falloff amount, not doubled.
        let mut app = integrity_physics_app();
        // `integrity_physics_app` deliberately does NOT include NovaDamagePlugin,
        // so this is the ONLY registration of the blast observer. That matters:
        // a second registration would fire the observer twice and double the
        // damage, silently masking a real double-count regression this test
        // exists to catch.
        app.add_observer(on_nova_blast_collision);
        let radius = 30.0;
        let max_damage = 100.0;
        // distance 15 of 30 -> falloff 0.5.
        let (_body, target) = spawn_target(
            &mut app,
            Vec3::new(15.0, 0.0, 0.0),
            1000.0,
            Some(SectionClass::Turret),
        );
        app.world_mut().spawn((
            nova_blast(radius, max_damage, DamageType::Explosive),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        settle(&mut app);
        assert!(
            (health(&app, target) - 950.0).abs() < 1e-1,
            "nova blast should deal a single 50.0, got drop {}",
            1000.0 - health(&app, target)
        );
    }
}
