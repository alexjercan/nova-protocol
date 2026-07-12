//! Nova's typed-damage layer: authored weapon damage scaled by a per-section
//! resistance table, applied by OWNING the `HealthApplyDamage` trigger.
//!
//! - Architecture: docs/spikes/20260712-133135-weapon-and-damage-type-variety.md
//! - Types + table: docs/spikes/20260712-160505-damage-and-bullet-type-taxonomy.md
//!
//! bevy_common_systems (bcs) owns the generic HP + integrity store: its single
//! `on_damage` observer subtracts `HealthApplyDamage.amount`, marks the node at
//! zero, and re-propagates up `ChildOf`. bcs carries NO damage type, and Bevy
//! 0.19 gives no ordering between observers of one event - so a nova observer
//! that tried to scale `amount` would race bcs's subtractor and lose half the
//! time (spike, rejected option C). Instead nova owns the trigger: it computes
//! the already-resistance-scaled amount at the weapon-hit callsite and only THEN
//! triggers `HealthApplyDamage`, so bcs just subtracts what nova decided. This
//! module is the shared vocabulary the turret and torpedo callsites use:
//! [`DamageType`], the [`ProjectileDamage`] a projectile carries, the
//! [`SectionDamageClass`] a hit resolves to, the [`resistance`] table, and the
//! one [`apply_typed_damage`] application helper.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::HealthApplyDamage;

pub mod prelude {
    pub use super::{
        apply_typed_damage, damage_type_color, nova_blast, representative_kinetic_damage,
        resistance, DamageType, NovaBlast, NovaDamagePlugin, ProjectileDamage, SectionDamageClass,
        NEUTRALIZED_BULLET_MASS,
    };
}

/// How a projectile hurts: the typed axis, orthogonal to raw hit points.
///
/// Kinetic is the reference type - [`resistance`] is 1.0 for it against every
/// section, so a Kinetic weapon behaves exactly as the pre-typed model did. The
/// three others each have one clear best target (spike taxonomy).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum DamageType {
    /// Plain slug / mass driver (the turret). Generalist; never exploits a
    /// weakness, never wasted.
    Kinetic,
    /// Dense penetrator: strong vs armor (turret, hull), wasted on soft targets.
    ArmorPiercing,
    /// Anti-electronics: crushes the command core, inert vs raw structure.
    Emp,
    /// Concussive area damage (the torpedo): shreds the exposed, bounces off
    /// hardened armor.
    Explosive,
}

/// Authored damage a projectile carries, set from its weapon (and, later, the
/// loaded bullet type). `amount` is the PRE-resistance magnitude; the weapon-hit
/// callsite scales it by [`resistance`] before triggering damage. Making weapon
/// damage authored (not emergent from bullet mass x velocity) is the point of
/// the pass: "AP does X, EMP does Y" cannot come out of one kinetic formula.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct ProjectileDamage {
    /// Pre-resistance damage at the point of impact.
    pub amount: f32,
    /// Which resistance column this projectile is scaled against.
    pub kind: DamageType,
}

/// Which section kind a hit landed on, for the resistance lookup.
///
/// A discriminant-only mirror of [`SectionKind`](crate::sections::prelude::SectionKind)
/// (which carries per-kind config this table does not need), inserted alongside
/// each section's kind marker (see the `*_section` bundles) so a single query
/// resolves a hit collider's class. Targets WITHOUT this component (asteroids,
/// debris) take the raw amount - resistance defaults to 1.0.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum SectionDamageClass {
    Hull,
    Thruster,
    Controller,
    Turret,
    Torpedo,
}

/// Physical mass given to a turret bullet so bcs's emergent kinetic term rounds
/// to nothing.
///
/// bcs's `on_impact_collision_deal_damage` computes damage from
/// `effective_mass = m_bullet * m_ship / (m_bullet + m_ship)` (~ `m_bullet`
/// since a ship is far heavier), so a near-zero bullet mass makes bcs's
/// contribution negligible next to nova's authored [`ProjectileDamage`], leaving
/// the typed amount as the only weapon damage. Gravity is unaffected:
/// `gravity_well_system` applies a mass-INDEPENDENT acceleration
/// (`forces.apply_linear_acceleration`), and Sensor bullets take no contact
/// forces, so a tiny mass changes neither flight nor knockback. Kept small but
/// non-zero to avoid a zero-mass dynamic body. See task 20260712-133343.
pub const NEUTRALIZED_BULLET_MASS: f32 = 1.0e-6;

/// Damage multiplier for a `(section class, damage type)` pair. `> 1.0` = the
/// section takes MORE, `< 1.0` = less. Kinetic is 1.0 on every section (the
/// feel-preserving reference). The full table and its per-row intent live in
/// docs/spikes/20260712-160505-damage-and-bullet-type-taxonomy.md; keep the
/// intent (which type beats which section) even if playtest moves the numbers.
pub const fn resistance(class: SectionDamageClass, kind: DamageType) -> f32 {
    use DamageType::*;
    use SectionDamageClass::*;
    match (class, kind) {
        // Kinetic: the 1.0 reference on every section - preserves today's feel.
        (_, Kinetic) => 1.0,

        // ArmorPiercing: peaks vs armor (Turret, Hull); penalised on the thin
        // exposed Thruster (over-penetration); neutral vs the electronics kinds.
        (Hull, ArmorPiercing) => 1.5,
        (Thruster, ArmorPiercing) => 0.75,
        (Controller, ArmorPiercing) => 1.0,
        (Turret, ArmorPiercing) => 1.75,
        (Torpedo, ArmorPiercing) => 1.0,

        // Emp: crushes the command core; near-inert vs dumb structure and raw
        // mechanism; mild vs the launcher's electronics.
        (Hull, Emp) => 0.1,
        (Thruster, Emp) => 0.25,
        (Controller, Emp) => 3.0,
        (Turret, Emp) => 1.5,
        (Torpedo, Emp) => 1.25,

        // Explosive: shreds the exposed Thruster; bounces off the hardened
        // Turret mount; neutral vs Hull/Controller.
        (Hull, Explosive) => 1.0,
        (Thruster, Explosive) => 1.5,
        (Controller, Explosive) => 1.0,
        (Turret, Explosive) => 0.5,
        (Torpedo, Explosive) => 1.25,
    }
}

/// The identifying color of a damage type, for HUD conveyance (the ammo readout
/// colors its pips by the loaded round's type; task 20260712-133349). Opaque
/// hue - callers apply their own alpha (lit vs dim). Kinetic is the readout's
/// historical amber so a Kinetic weapon looks unchanged; the others are distinct
/// hues (steel blue, cyan, red-orange) that read on the dark HUD behind the pip
/// outline.
pub fn damage_type_color(kind: DamageType) -> Color {
    match kind {
        // The original ammo-readout amber (LIT_COLOR's hue) - unchanged look.
        DamageType::Kinetic => Color::srgb(1.0, 0.75, 0.2),
        // Hardened penetrator: cold steel blue.
        DamageType::ArmorPiercing => Color::srgb(0.6, 0.75, 1.0),
        // Anti-electronics: electric cyan.
        DamageType::Emp => Color::srgb(0.3, 0.9, 1.0),
        // Concussive: red-orange fire.
        DamageType::Explosive => Color::srgb(1.0, 0.4, 0.15),
    }
}

/// The pre-scaled amount a hit deals: `damage.amount x resistance`, with an
/// unknown class (non-section target) defaulting to a 1.0 multiplier.
pub fn scaled_amount(class: Option<SectionDamageClass>, damage: ProjectileDamage) -> f32 {
    let multiplier = match class {
        Some(class) => resistance(class, damage.kind),
        None => 1.0,
    };
    damage.amount * multiplier
}

/// Compute the resistance-scaled amount and OWN the `HealthApplyDamage` trigger.
///
/// `target` is the hit collider (whose [`SectionDamageClass`], if any, keys the
/// table); `source` is the projectile/blast collider for threat attribution.
/// bcs's `on_damage` then subtracts the pre-scaled `amount` and drives the
/// destruction pipeline unchanged - no observer race, because nova already did
/// the scaling before triggering. This is the single application point, so every
/// weapon scales identically.
pub fn apply_typed_damage(
    commands: &mut Commands,
    target: Entity,
    source: Option<Entity>,
    class: Option<SectionDamageClass>,
    damage: ProjectileDamage,
) {
    let amount = scaled_amount(class, damage);
    commands.trigger(HealthApplyDamage {
        entity: target,
        source,
        amount,
    });
}

/// The per-hit kinetic damage bcs's emergent model dealt for a bullet of `mass`
/// striking at relative speed `speed`, approximating `effective_mass ~ mass`
/// (a target ship is far heavier than a bullet, so the effective mass is within
/// a few percent of the bullet mass across every ship).
///
/// Used to AUTHOR the turret's fixed Kinetic `amount` so the typed system
/// preserves the old feel at a representative engagement speed (the design
/// deliberately trades velocity-dependent damage for a fixed authored amount -
/// spike). Mirrors bcs integrity/plugin.rs:143-150 (RESTITUTION 0.5,
/// IMPULSE_MOD 0.1, ENERGY_MOD 0.05); nova must not modify bcs, so the constants
/// are duplicated here with this citation.
pub fn representative_kinetic_damage(mass: f32, speed: f32) -> f32 {
    const RESTITUTION: f32 = 0.5;
    const IMPULSE_MOD: f32 = 0.1;
    const ENERGY_MOD: f32 = 0.05;
    let impulse = mass * (1.0 + RESTITUTION) * speed;
    let energy = 0.5 * mass * (1.0 - RESTITUTION * RESTITUTION) * speed * speed;
    impulse * IMPULSE_MOD + energy * ENERGY_MOD
}

/// A nova-owned radial blast volume, the typed replacement for bcs's
/// `blast_damage`.
///
/// bcs's blast observer applies its blast UNTYPED; a torpedo wants Explosive
/// typing, so nova spawns THIS instead and applies the falloff through the typed
/// path. It carries NO bcs `BlastDamageMarker`, so bcs's blast observer stays
/// dormant for it and the damage is never double-counted.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct NovaBlast {
    /// Bodies beyond this take no damage.
    pub radius: f32,
    /// Damage at the blast centre (distance 0), before resistance.
    pub max_damage: f32,
    /// The blast's damage type (Explosive for torpedoes).
    pub kind: DamageType,
}

/// Bundle for a nova typed blast volume. Mirrors bcs's `blast_damage` collider
/// setup - a Static sensor sphere that owns its collision events so it raises
/// `CollisionStart` against every overlapped collider - but routes damage
/// through [`on_nova_blast_collision`]. Spawn with a `Transform` at the centre
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

/// Linear falloff to zero at `radius`, mirroring bcs `calculate_blast_damage`
/// (integrity/plugin.rs:229).
fn blast_falloff(distance: f32, radius: f32, max_damage: f32) -> f32 {
    if distance >= radius {
        0.0
    } else {
        max_damage * (1.0 - distance / radius)
    }
}

/// Apply nova typed blast damage to every body a [`NovaBlast`] sensor overlaps.
///
/// Mirrors bcs `on_blast_collision_deal_damage`: the blast is the `body1`/self
/// side (it carries the events), and the swapped `{body1 = target}` ordering is
/// ignored because `q_blast.get(blast)` fails on the target side - so each
/// overlap deals damage exactly once. Unlike bcs it scales by [`resistance`] and
/// triggers the TYPED damage. `source` is the blast collider, matching bcs, so
/// the AI threat model still resolves it to the shooter via the blast entity's
/// `ProjectileOwner`.
fn on_nova_blast_collision(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_blast: Query<(&Transform, &NovaBlast)>,
    q_body: Query<&Transform, With<RigidBody>>,
    q_class: Query<&SectionDamageClass>,
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

    let class = q_class.get(target_collider).ok().copied();
    apply_typed_damage(
        &mut commands,
        target_collider,
        Some(blast_collider),
        class,
        ProjectileDamage {
            amount,
            kind: blast_config.kind,
        },
    );
}

/// Registers the typed-damage reflection types and the nova blast observer.
///
/// The application HELPER ([`apply_typed_damage`]) is called from the weapon-hit
/// callsites in their own modules (turret `despawn_bullet_on_hit`, torpedo
/// detonate); this plugin owns only the nova-blast observer and type
/// registration.
pub struct NovaDamagePlugin;

impl Plugin for NovaDamagePlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaDamagePlugin: build");
        app.register_type::<DamageType>()
            .register_type::<ProjectileDamage>()
            .register_type::<SectionDamageClass>()
            .register_type::<NovaBlast>();
        app.add_observer(on_nova_blast_collision);
    }
}

#[cfg(test)]
mod tests {
    use bevy_common_systems::prelude::Health;

    use super::*;
    use crate::integrity::test_support::{integrity_physics_app, settle};

    fn health(app: &App, entity: Entity) -> f32 {
        app.world().get::<Health>(entity).unwrap().current
    }

    /// A ship-shaped target: a RigidBody parent with a single child collider that
    /// carries the Health (and optional damage class), mirroring how nova ships
    /// hold section colliders under a root body. Returns `(body, collider)`; bcs
    /// reports the parent as `body*` and the child as `collider*` in a
    /// CollisionStart, and damage lands on (and health lives on) the child.
    fn spawn_target(
        app: &mut App,
        at: Vec3,
        hp: f32,
        class: Option<SectionDamageClass>,
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
    fn kinetic_resistance_is_one_on_every_section() {
        // The feel-preserving invariant: a Kinetic weapon is never scaled, so
        // the pre-typed durability tuning is untouched. Would fail if any
        // Kinetic cell drifted off 1.0.
        for class in [
            SectionDamageClass::Hull,
            SectionDamageClass::Thruster,
            SectionDamageClass::Controller,
            SectionDamageClass::Turret,
            SectionDamageClass::Torpedo,
        ] {
            assert_eq!(resistance(class, DamageType::Kinetic), 1.0);
        }
    }

    #[test]
    fn resistance_table_matches_the_spike() {
        use DamageType::*;
        use SectionDamageClass::*;
        // One spot per non-Kinetic column, including the extremes that carry the
        // design intent: EMP annihilates the Controller and barely dents Hull;
        // AP peaks on the armored Turret; Explosive bounces off it.
        assert_eq!(resistance(Turret, ArmorPiercing), 1.75);
        assert_eq!(resistance(Thruster, ArmorPiercing), 0.75);
        assert_eq!(resistance(Controller, Emp), 3.0);
        assert_eq!(resistance(Hull, Emp), 0.1);
        assert_eq!(resistance(Thruster, Explosive), 1.5);
        assert_eq!(resistance(Turret, Explosive), 0.5);
    }

    #[test]
    fn scaled_amount_applies_the_multiplier_and_defaults_unknown_to_one() {
        let ap = ProjectileDamage {
            amount: 10.0,
            kind: DamageType::ArmorPiercing,
        };
        // Turret x AP = 1.75.
        assert_eq!(scaled_amount(Some(SectionDamageClass::Turret), ap), 17.5);
        // Unknown target (asteroid): raw amount, no resistance.
        assert_eq!(scaled_amount(None, ap), 10.0);
    }

    #[test]
    fn damage_type_color_is_distinct_per_type_and_kinetic_is_the_readout_amber() {
        let colors = [
            damage_type_color(DamageType::Kinetic),
            damage_type_color(DamageType::ArmorPiercing),
            damage_type_color(DamageType::Emp),
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
        // light turret mass 0.05 @ 60 u/s), so "Kinetic at 1.0" is genuinely
        // feel-preserving. If these move, the config values must move with them.
        assert!((representative_kinetic_damage(0.1, 100.0) - 20.25).abs() < 1e-3);
        assert!((representative_kinetic_damage(0.05, 60.0) - 3.825).abs() < 1e-3);
    }

    #[test]
    fn neutralized_bullet_mass_makes_bcs_emergent_kinetic_negligible() {
        // Drive the REAL bcs impact observer against a neutralized-mass bullet
        // and confirm the emergent kinetic it deals is negligible, then A/B the
        // same rig at the old 0.1 mass to prove the test can fail (the old mass
        // deals ~20). This is the neutralization the typed path depends on.
        fn bcs_impact_damage(bullet_mass: f32) -> f32 {
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
            // Target is collider1/body1 so bcs applies the impact to the section.
            app.world_mut().trigger(CollisionStart {
                collider1: target_collider,
                collider2: bullet,
                body1: Some(target_body),
                body2: Some(bullet),
            });
            app.update();
            1000.0 - health(&app, target_collider)
        }

        let neutralized = bcs_impact_damage(NEUTRALIZED_BULLET_MASS);
        let old = bcs_impact_damage(0.1);
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
    fn bcs_subtracts_the_prescaled_amount_nova_triggers() {
        // The own-the-trigger contract end to end: nova triggers a HealthApplyDamage
        // carrying the ALREADY-scaled amount, and bcs's on_damage subtracts exactly
        // that - no second scaling, no race.
        let mut app = integrity_physics_app();
        let (_body, target) = spawn_target(
            &mut app,
            Vec3::ZERO,
            100.0,
            Some(SectionDamageClass::Turret),
        );
        settle(&mut app);
        let dmg = ProjectileDamage {
            amount: 10.0,
            kind: DamageType::ArmorPiercing,
        };
        let expected = scaled_amount(Some(SectionDamageClass::Turret), dmg); // 17.5
        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: expected,
        });
        app.update();
        assert!(
            (health(&app, target) - (100.0 - expected)).abs() < 1e-3,
            "health should drop by the pre-scaled 17.5, got {}",
            health(&app, target)
        );
    }

    #[test]
    fn nova_blast_deals_typed_falloff_once() {
        // A real sensor overlap fires the nova blast observer, which scales the
        // linear falloff by the Explosive column and applies it once. The target
        // is a Turret section (Explosive x0.5), and because the nova blast has no
        // bcs BlastDamageMarker, bcs's blast observer never fires - so the drop is
        // exactly the single typed amount, not doubled.
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
            Some(SectionDamageClass::Turret),
        );
        app.world_mut().spawn((
            nova_blast(radius, max_damage, DamageType::Explosive),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        settle(&mut app);
        // falloff = 100 * (1 - 15/30) = 50; Explosive vs Turret = 0.5 -> 25.
        let expected = 50.0 * resistance(SectionDamageClass::Turret, DamageType::Explosive);
        assert!(
            (health(&app, target) - (1000.0 - expected)).abs() < 1e-1,
            "nova blast should deal a single typed 25.0, got drop {}",
            1000.0 - health(&app, target)
        );
    }
}
