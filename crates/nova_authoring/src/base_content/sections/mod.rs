//! Built-in section-prototype content.
//!
//! Every built-in prototype is a generic mountable module: the base fleet is
//! block-built, so no prototype here is a body part cut off one named craft.
//! A mod that brings modelled craft brings their prototypes with them (The
//! Ledger, task 20260824-125959).
//!
//! CLADDING is not here and is not a prototype at all. A ship's skin is DERIVED
//! from the structure it wraps - see `nova_ship`'s `shell_skin` - so no id names
//! a plate and nothing places one by hand.
//!
//! `ordnance` is the odd one out: a torpedo TYPE is not a section, it is what a
//! torpedo bay loads, so it lives beside the bays that author it.

use nova_ship::prelude::SectionConfig;

use super::assets::BaseContentAssets;

pub(crate) mod ordnance;
mod standard;

/// Generic hull, controller, thruster, turret, and torpedo prototypes.
pub(crate) fn standard_section_prototypes(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    standard::standard_section_prototypes(assets)
}

/// Complete built-in prototype catalog in stable generated-content order.
pub(crate) fn section_catalog(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    standard_section_prototypes(assets)
}

#[cfg(test)]
mod range_tests {
    use nova_events::prelude::*;
    use nova_ship::prelude::{SectionKind, AI_FIRE_RANGE_FACTOR, AI_STANDOFF_OUTER_EDGE};

    use super::*;

    /// Every authored gun must reach the far edge of the orbit band its own
    /// AI flies, or the ship engages correctly and never pulls the trigger -
    /// no error, no warning, just silence. Reach is
    /// `muzzle_speed * projectile_lifetime` (a turret has no range field),
    /// and the AI fires inside `AI_FIRE_RANGE_FACTOR` of it, so this is the
    /// standing constraint on any lifetime edit - including the shorter
    /// lifetime the 1,000 m/s guns carry and the longer one the 600 m/s
    /// scavenger guns need to compensate.
    #[test]
    fn every_authored_turret_reaches_past_the_standoff_band() {
        let assets = BaseContentAssets::from_paths();
        let turrets: Vec<(String, Meters)> = section_catalog(&assets)
            .into_iter()
            .filter_map(|section| match section.kind {
                SectionKind::Turret(turret) => Some((
                    section.base.id,
                    turret.muzzle_speed.over(turret.projectile_lifetime) * AI_FIRE_RANGE_FACTOR,
                )),
                _ => None,
            })
            .collect();
        assert!(!turrets.is_empty(), "the catalog carries turrets");
        for (id, gate) in turrets {
            // The standoff band is engine-side AI tuning, so the authored
            // reach crosses to meet it.
            assert!(
                gate.to_engine() > AI_STANDOFF_OUTER_EDGE,
                "'{id}' fires out to {:.0} m but the standoff band reaches \
                 {:.0} m - a ship carrying it would orbit outside its own \
                 reach and never fire",
                gate.get(),
                Meters::from_engine(AI_STANDOFF_OUTER_EDGE).get()
            );
        }
    }
}

#[cfg(test)]
mod ordnance_tests {
    use bevy::platform::collections::HashMap;
    use nova_events::prelude::*;
    use nova_ship::prelude::{SectionKind, TorpedoSectionConfig, TorpedoTypeConfig};

    use super::*;

    /// Every torpedo bay in the catalog, as `(id, config)`.
    fn bays(assets: &BaseContentAssets) -> Vec<(String, TorpedoSectionConfig)> {
        section_catalog(assets)
            .into_iter()
            .filter_map(|section| match section.kind {
                SectionKind::Torpedo(bay) => Some((section.base.id, bay)),
                _ => None,
            })
            .collect()
    }

    /// The format break, pinned to the shipped catalog: a bay now AUTHORS its
    /// blast in meters, and the engine still receives the world-unit radius it
    /// received when the same bay authored `blast_radius: 30`. If a conversion
    /// is ever added, doubled, or dropped between the file and the collider,
    /// this is the assertion that fails.
    #[test]
    fn an_authored_three_hundred_meter_blast_reaches_the_engine_as_thirty_units() {
        let assets = BaseContentAssets::from_paths();
        let bays = bays(&assets);
        assert!(!bays.is_empty(), "the catalog carries torpedo bays");

        let (id, bay) = bays
            .iter()
            .find(|(id, _)| id == "torpedo_section")
            .expect("the standard assault bay ships in the catalog");
        assert_eq!(
            bay.blast_radius,
            Meters(300.0),
            "'{id}' authors its blast in meters"
        );
        assert_eq!(
            bay.blast_radius.to_engine(),
            30.0,
            "'{id}' hands the engine the radius it held before the break"
        );
    }

    /// The owner's rule, made structural: the two assault types "both deal the
    /// same blast damage but one is more evasive than the other". A type
    /// decides how the ordnance FLIES and nothing else, so any second
    /// difference between the two bays is a balance change nobody asked for -
    /// and a difference in blast, reach or magazine would quietly turn the
    /// choice into a straight upgrade.
    ///
    /// `max_speed` is deliberately NOT in the list below: it lives on the type
    /// now and is the evasive type's price (see `sections::ordnance`). It is
    /// how the ordnance flies, which is exactly what a type is allowed to
    /// change.
    ///
    /// Checked over every pair of bays sharing a `blast_damage`, so the
    /// semantic cargo-B pods are graded next to their own `_lance` twin the
    /// same way the standalone bays are, and a third type inherits the rule.
    #[test]
    fn torpedo_types_differ_only_in_how_the_ordnance_flies() {
        let assets = BaseContentAssets::from_paths();
        let bays = bays(&assets);
        assert!(!bays.is_empty(), "the catalog carries torpedo bays");

        let mut pairs = 0usize;
        for (id, bay) in &bays {
            for (other_id, other) in &bays {
                if id >= other_id || bay.blast_damage != other.blast_damage {
                    continue;
                }
                pairs += 1;
                let stats = |bay: &TorpedoSectionConfig| {
                    (
                        bay.blast_damage,
                        bay.blast_radius,
                        bay.linear_damping,
                        bay.nav_constant,
                        bay.projectile_lifetime,
                        bay.projectile_health,
                        bay.fire_rate,
                        bay.ammo_capacity,
                        bay.reload.map(|reload| (reload.delay, reload.amount)),
                    )
                };
                assert_eq!(
                    stats(bay),
                    stats(other),
                    "'{id}' and '{other_id}' load different torpedo types, so \
                     everything else about them must be identical"
                );
            }
        }
        assert!(pairs > 0, "the catalog carries bays to compare");
    }

    /// A type is a NAME plus a look plus a flight, and all three have to agree
    /// across the catalog: one name must always mean the same ordnance, and two
    /// names must always be tellable apart in flight. Otherwise "which torpedo
    /// is that" has no answer a player can read, which is the whole reason the
    /// type carries a tint at all.
    #[test]
    fn every_torpedo_type_is_one_thing_and_looks_unlike_the_others() {
        let assets = BaseContentAssets::from_paths();
        let mut types: HashMap<String, (TorpedoTypeConfig, String)> = HashMap::new();
        for (id, bay) in bays(&assets) {
            let torpedo_type = bay.torpedo_type;
            if let Some((known, known_id)) = types.get(&torpedo_type.name) {
                assert_eq!(
                    known, &torpedo_type,
                    "'{id}' and '{known_id}' both load a '{}' but authored it \
                     differently",
                    torpedo_type.name
                );
                continue;
            }
            types.insert(torpedo_type.name.clone(), (torpedo_type, id));
        }
        assert!(
            types.len() >= 2,
            "the catalog must offer a CHOICE of torpedo type, got {:?}",
            types.keys().collect::<Vec<_>>()
        );
        let listed: Vec<_> = types.values().collect();
        for (index, (torpedo_type, id)) in listed.iter().enumerate() {
            for (other, other_id) in &listed[index + 1..] {
                assert_ne!(
                    torpedo_type.tint, other.tint,
                    "'{}' ({id}) and '{}' ({other_id}) fly in the same colour, \
                     so nothing in the frame says which one is inbound",
                    torpedo_type.name, other.name
                );
            }
        }
    }
}

#[cfg(test)]
mod ammunition_tests {
    use nova_ship::prelude::{SectionKind, SectionReloadConfig, TurretJoint};

    use super::*;

    /// Point-defense rounds one WEAVING torpedo costs a mount to stop, measured
    /// by `point_defense_cost_tests` (nova_ship, torpedo projectile) against the
    /// real lead solve and the real fire-alignment gate across the shipped
    /// 1,500 m envelope. A straight torpedo costs 116; the terminal weave is what
    /// makes it 369, and that tripling is why the bay's reload rate had to be
    /// re-derived rather than restored to what it was before the weave.
    const ROUNDS_PER_WEAVING_INTERCEPT: f32 = 369.0;

    /// Rounds per second when every returned batch is fired immediately: one
    /// idle delay plus the time needed to spend that batch.
    fn sustained_per_second(_capacity: u32, spend_rate: f32, reload: &SectionReloadConfig) -> f32 {
        let amount = reload.amount as f32;
        let batch_fire_time = reload.amount.saturating_sub(1) as f32 / spend_rate;
        amount / (reload.delay + batch_fire_time)
    }

    /// Every muzzle in a turret's joint tree, summed: the rate the mount
    /// actually spends its magazine at.
    fn tree_fire_rate(joint: &TurretJoint) -> f32 {
        joint.muzzle.as_ref().map_or(0.0, |muzzle| muzzle.fire_rate)
            + joint.children.iter().map(tree_fire_rate).sum::<f32>()
    }

    /// Ammunition here is a RATE LIMIT, not a budget. A weapon that carries a
    /// magazine and no way to refill it leaves a ship alive with nothing to
    /// fight with, which strands an engagement rather than resolving it.
    /// Unlimited (`ammo_capacity: None`) is a separate, deliberate authoring
    /// choice and is not what this grades.
    #[test]
    fn every_authored_magazine_refills() {
        let assets = BaseContentAssets::from_paths();
        let mut graded = 0;
        for section in section_catalog(&assets) {
            let (capacity, reload) = match &section.kind {
                SectionKind::Turret(turret) => (turret.ammo_capacity, turret.reload),
                SectionKind::Torpedo(bay) => (bay.ammo_capacity, bay.reload),
                SectionKind::Railgun(lance) => (lance.ammo_capacity, lance.reload),
                _ => continue,
            };
            let Some(capacity) = capacity else { continue };
            graded += 1;
            assert!(
                reload.is_some(),
                "'{}' carries {capacity} rounds and no reload - once they are \
                 gone the section is dead weight that still flies",
                section.base.id
            );
        }
        assert!(graded > 0, "the catalog carries weapons with magazines");
    }

    /// No torpedo bay may out-supply the point defense meant to answer it.
    ///
    /// The exchange is meant to be decided by SATURATION - more bays than the
    /// defender has mounts, spent in one burst - and never by patience. A bay
    /// that regrows torpedoes faster than a mount sustains intercepts wins by
    /// waiting, whatever else is tuned. One bay against one mount is the
    /// scale-free form of that: it says exactly what two bays against two
    /// mounts says.
    ///
    /// Graded against the BEST mount in the catalog, since that is the point
    /// defense a defender would choose to carry. Every input is read from the
    /// authored catalog, so a change to a PDC's magazine, fire rate or reload
    /// re-derives the ceiling here instead of silently moving it.
    #[test]
    fn no_torpedo_bay_out_sustains_a_point_defense_mount() {
        let assets = BaseContentAssets::from_paths();
        let catalog = section_catalog(&assets);

        let best_mount = catalog
            .iter()
            .filter_map(|section| {
                let SectionKind::Turret(turret) = &section.kind else {
                    return None;
                };
                let rounds = sustained_per_second(
                    turret.ammo_capacity?,
                    tree_fire_rate(&turret.root),
                    &turret.reload?,
                );
                Some((
                    section.base.id.as_str(),
                    rounds / ROUNDS_PER_WEAVING_INTERCEPT,
                ))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("the catalog carries point-defense mounts");

        let mut graded = 0;
        for section in &catalog {
            let SectionKind::Torpedo(bay) = &section.kind else {
                continue;
            };
            // Unlimited authoring or testing bays have no sustained reload
            // cadence to grade here.
            let (Some(capacity), Some(reload)) = (bay.ammo_capacity, bay.reload) else {
                continue;
            };
            graded += 1;
            let launches = sustained_per_second(capacity, bay.fire_rate, &reload);
            assert!(
                launches < best_mount.1,
                "'{}' sustains {launches:.3} torpedoes/s but '{}' - the best \
                 mount in the catalog - only answers {:.3}/s at \
                 {ROUNDS_PER_WEAVING_INTERCEPT:.0} rounds an intercept, so the \
                 attacker wins by waiting instead of by saturating",
                section.base.id,
                best_mount.0,
                best_mount.1,
            );
        }
        assert!(
            graded > 0,
            "the catalog carries torpedo bays with magazines"
        );
    }
}
