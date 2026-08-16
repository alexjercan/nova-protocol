//! Built-in section-prototype content.
//!
//! `standard` owns generic mountable modules; semantic body-part prototypes
//! live with their complete craft definitions under `ships`. This module joins
//! both sources into the one generated section catalog.
//!
//! CLADDING is not here and is not a prototype at all. A ship's skin is DERIVED
//! from the structure it wraps - see `nova_ship`'s `shell_skin` - so no id names
//! a plate and nothing places one by hand.

use nova_ship::prelude::SectionConfig;

use super::{assets::BaseContentAssets, ships};

mod standard;

pub(crate) use standard::{turret_joint_tree, UNIT_TURRET_MOUNT, UNIT_TURRET_SCALE};

/// Generic hull, controller, thruster, turret, and torpedo prototypes.
pub(crate) fn standard_section_prototypes(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    standard::standard_section_prototypes(assets)
}

/// Complete built-in prototype catalog in stable generated-content order.
pub(crate) fn section_catalog(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    let mut sections = standard_section_prototypes(assets);
    sections.extend(ships::semantic_part_prototypes(assets));
    sections
}

#[cfg(test)]
mod range_tests {
    use nova_ship::prelude::{SectionKind, AI_FIRE_RANGE_FACTOR, AI_STANDOFF_OUTER_EDGE};

    use super::*;

    /// Every authored gun must reach the far edge of the orbit band its own
    /// AI flies, or the ship engages correctly and never pulls the trigger -
    /// no error, no warning, just silence. Reach is
    /// `muzzle_speed * projectile_lifetime` (a turret has no range field),
    /// and the AI fires inside `AI_FIRE_RANGE_FACTOR` of it, so this is the
    /// standing constraint on any lifetime edit - including the shorter
    /// lifetime the 100 u/s guns carry and the longer one the 60 u/s
    /// scavenger guns need to compensate.
    #[test]
    fn every_authored_turret_reaches_past_the_standoff_band() {
        let assets = BaseContentAssets::from_paths();
        let turrets: Vec<(String, f32)> = section_catalog(&assets)
            .into_iter()
            .filter_map(|section| match section.kind {
                SectionKind::Turret(turret) => Some((
                    section.base.id,
                    turret.muzzle_speed * turret.projectile_lifetime * AI_FIRE_RANGE_FACTOR,
                )),
                _ => None,
            })
            .collect();
        assert!(!turrets.is_empty(), "the catalog carries turrets");
        for (id, gate) in turrets {
            assert!(
                gate > AI_STANDOFF_OUTER_EDGE,
                "'{id}' fires out to {gate:.0}u but the standoff band reaches \
                 {AI_STANDOFF_OUTER_EDGE:.0}u - a ship carrying it would orbit \
                 outside its own reach and never fire"
            );
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
    /// 150 u envelope. A straight torpedo costs 116; the terminal weave is what
    /// makes it 369, and that tripling is why the bay's regen rate had to be
    /// re-derived rather than restored to what it was before the weave.
    const ROUNDS_PER_WEAVING_INTERCEPT: f32 = 369.0;

    /// Rounds per second a magazine sustains indefinitely.
    ///
    /// Continuous regen (`only_when_empty: false`) IS the refill rate - firing
    /// faster only empties the rack sooner and then waits on it. A discrete
    /// reload-on-empty is a duty cycle instead: `capacity` rounds spent at
    /// `spend_rate`, then one `reload_time` with the weapon silent.
    fn sustained_per_second(capacity: u32, spend_rate: f32, reload: &SectionReloadConfig) -> f32 {
        let capacity = capacity as f32;
        if reload.only_when_empty {
            capacity / (capacity / spend_rate + reload.reload_time)
        } else {
            reload.rounds_per_cycle as f32 / reload.reload_time
        }
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
            // An unlimited bay is scene dressing (the siege battery), not a
            // combat participant, and the AI launch cadence is what paces it.
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
