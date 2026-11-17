//! Consumables: hold items a pilot uses in flight for one timed or instant
//! effect.
//!
//! A [`ConsumableDesign`] is authored content keyed by the item it makes
//! usable. It names one effect from the closed [`ConsumableType`]: a repair
//! over time, a thrust boost over time, or a flare that pulls homing
//! torpedoes off the ship. Each effect type owns one flight key
//! ([`ConsumableSlotType`]), so the merged catalog holds at most one
//! consumable per type. The mod merge publishes the effective consumables as
//! [`GameConsumables`]; [`consumable_faults`] is the rule the merge and
//! content lint share.
//!
//! A use takes one item at once and starts the effect; [`plan_consumable_use`]
//! checks it first and a refusal takes nothing. `nova_ship` runs the timed
//! effects on the game clock and applies the flare. The pure rules a tick and
//! a flare use live here: [`plan_repair_split`], [`flare_lure_point`] and
//! [`flare_in_band`].

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;
use nova_events::prelude::Meters;

use crate::inventory::prelude::{GameItems, ItemDesignId, ItemRoleType};

/// Glob-import surface: `use nova_gameplay::consumable::prelude::*`.
pub mod prelude {
    pub use super::{
        consumable_faults, flare_in_band, flare_lure_point, plan_consumable_use, plan_repair_split,
        ConsumableDesign, ConsumableRefusalType, ConsumableSlotType, ConsumableType,
        ConsumableUseFacts, GameConsumables, CONSUMABLE_DURATION_MAX_SECS, FLARE_LURE_BLAST_FACTOR,
        THRUST_BOOST_FACTOR_MAX,
    };
}

/// The longest a timed effect may run, in game seconds.
pub const CONSUMABLE_DURATION_MAX_SECS: f32 = 60.0;

/// The largest thrust factor a boost may author. Above it the autopilot,
/// which plans with unboosted thrust, would hand back a ship moving far
/// faster than it ever flies.
pub const THRUST_BOOST_FACTOR_MAX: f32 = 3.0;

/// How many of the largest torpedo blast radius a flare lure must stand
/// beyond. A lured torpedo has no target and fuzes within half its blast
/// radius of the lure point, so its blast centre may stand half a radius
/// nearer the ship, and the blast reaches one radius past that: a lure past
/// 1.5 radii keeps the whole blast off the ship.
pub const FLARE_LURE_BLAST_FACTOR: f32 = 1.5;

/// One consumable effect and its numbers. Closed: each variant is code.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConsumableType {
    /// Heal `health` over `duration_secs`, each tick to the live, enabled
    /// section with the lowest health share. Never revives a lost section.
    Repair {
        /// Health points the whole effect restores.
        health: f32,
        /// Game seconds it runs.
        duration_secs: f32,
    },
    /// Multiply every thruster impulse by `factor` for `duration_secs`,
    /// while no autopilot verb flies the ship.
    ThrustBoost {
        /// The impulse factor, above 1 and at most
        /// [`THRUST_BOOST_FACTOR_MAX`].
        factor: f32,
        /// Game seconds it runs.
        duration_secs: f32,
    },
    /// Instant: every torpedo homing on the ship from `near_m` to `far_m`
    /// away drops its lock and flies to a point `lure_m` to one side of the
    /// ship.
    Flare {
        /// The nearest torpedo it pulls off. Nearer ones keep the lock.
        near_m: Meters,
        /// The farthest torpedo it pulls off.
        far_m: Meters,
        /// How far from the ship the lure point stands. Above
        /// [`FLARE_LURE_BLAST_FACTOR`] times the largest torpedo blast
        /// radius in the catalog, so a lured blast misses the ship.
        lure_m: Meters,
    },
}

impl ConsumableType {
    /// The slot, and so the flight key, this effect takes.
    pub fn slot(&self) -> ConsumableSlotType {
        match self {
            Self::Repair { .. } => ConsumableSlotType::Repair,
            Self::ThrustBoost { .. } => ConsumableSlotType::ThrustBoost,
            Self::Flare { .. } => ConsumableSlotType::Flare,
        }
    }

    /// Game seconds a timed effect runs; `None` for the instant flare.
    pub fn duration_secs(&self) -> Option<f32> {
        match *self {
            Self::Repair { duration_secs, .. } | Self::ThrustBoost { duration_secs, .. } => {
                Some(duration_secs)
            }
            Self::Flare { .. } => None,
        }
    }
}

/// The three effect types, each with its own flight key and at most one
/// running effect per ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum ConsumableSlotType {
    /// The repair (key 1).
    Repair,
    /// The thrust boost (key 2).
    ThrustBoost,
    /// The flare (key 3).
    Flare,
}

impl ConsumableSlotType {
    /// Every slot, in key order.
    pub const ALL: [Self; 3] = [Self::Repair, Self::ThrustBoost, Self::Flare];

    /// The word the keybind and status chips read: `KIT`, `BOOST`, `FLARE`.
    pub fn word(self) -> &'static str {
        match self {
            Self::Repair => "KIT",
            Self::ThrustBoost => "BOOST",
            Self::Flare => "FLARE",
        }
    }

    /// The flight action that uses it.
    pub fn action(self) -> &'static str {
        match self {
            Self::Repair => "use_repair_kit",
            Self::ThrustBoost => "use_boost",
            Self::Flare => "use_flare",
        }
    }
}

/// One authored consumable: the item it makes usable and its effect. Every
/// field is required.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ConsumableDesign {
    /// The item a use takes one of; also the consumable's id.
    pub item: ItemDesignId,
    /// What a use does.
    pub effect: ConsumableType,
}

/// The merged consumables, by item. Inserted by the mod merge.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GameConsumables(pub BTreeMap<ItemDesignId, ConsumableDesign>);

impl GameConsumables {
    /// The catalog of `consumables`; a later one for an item replaces an
    /// earlier one.
    pub fn new(consumables: impl IntoIterator<Item = ConsumableDesign>) -> Self {
        Self(
            consumables
                .into_iter()
                .map(|consumable| (consumable.item.clone(), consumable))
                .collect(),
        )
    }

    /// The consumable of `item`, if the item is usable.
    pub fn get(&self, item: &ItemDesignId) -> Option<&ConsumableDesign> {
        self.0.get(item)
    }

    /// The consumable of `item`, for an item a running effect or a save
    /// check already resolved.
    ///
    /// # Panics
    ///
    /// When no consumable names the item: a running effect named an item the
    /// loaded content does not make usable, which the save check refuses
    /// first.
    pub fn design(&self, item: &ItemDesignId) -> &ConsumableDesign {
        self.get(item)
            .unwrap_or_else(|| panic!("item '{item}' has no consumable in the loaded catalog"))
    }

    /// The consumable in `slot`. The lint holds one per slot at most.
    pub fn in_slot(&self, slot: ConsumableSlotType) -> Option<&ConsumableDesign> {
        self.0
            .values()
            .find(|consumable| consumable.effect.slot() == slot)
    }

    /// Every consumable, in item id order.
    pub fn iter(&self) -> impl Iterator<Item = &ConsumableDesign> {
        self.0.values()
    }
}

/// Every fault of `consumable` on its own against `items`, keyed in the
/// message by its item: an unknown item, an item engine code already spends
/// ([`ItemRoleType`]), and a number out of range. `max_blast` is the largest
/// torpedo blast radius in the merged section catalog; a flare lure must
/// stand past [`FLARE_LURE_BLAST_FACTOR`] times it. The rule that two consumables never share a slot judges
/// the whole catalog and lives with the pack rule (`nova_assets::items`).
pub fn consumable_faults(
    consumable: &ConsumableDesign,
    items: &GameItems,
    max_blast: Meters,
) -> Vec<String> {
    let item = &consumable.item;
    let mut faults = Vec::new();
    if items.get(item).is_none() {
        faults.push(format!("the consumable '{item}' names no item"));
    }
    if ItemRoleType::ALL
        .iter()
        .any(|role| role.id() == item.as_str())
    {
        faults.push(format!(
            "the consumable '{item}' names an item the game already spends; use another item"
        ));
    }
    let mut duration = |duration_secs: f32| {
        if !duration_secs.is_finite()
            || duration_secs <= 0.0
            || duration_secs > CONSUMABLE_DURATION_MAX_SECS
        {
            faults.push(format!(
                "the consumable '{item}' runs {duration_secs} s; it must be above 0 and at most \
                 {CONSUMABLE_DURATION_MAX_SECS} s"
            ));
        }
    };
    match consumable.effect {
        ConsumableType::Repair {
            health,
            duration_secs,
        } => {
            duration(duration_secs);
            if !health.is_finite() || health <= 0.0 {
                faults.push(format!(
                    "the consumable '{item}' heals {health}; it must be finite and above 0"
                ));
            }
        }
        ConsumableType::ThrustBoost {
            factor,
            duration_secs,
        } => {
            duration(duration_secs);
            if !factor.is_finite() || factor <= 1.0 || factor > THRUST_BOOST_FACTOR_MAX {
                faults.push(format!(
                    "the consumable '{item}' boosts thrust by {factor}; it must be above 1 and at \
                     most {THRUST_BOOST_FACTOR_MAX}"
                ));
            }
        }
        ConsumableType::Flare {
            near_m,
            far_m,
            lure_m,
        } => {
            if !near_m.0.is_finite() || near_m.0 <= 0.0 {
                faults.push(format!(
                    "the consumable '{item}' has a near_m of {} m; it must be above 0",
                    near_m.0
                ));
            }
            if !far_m.0.is_finite() || far_m <= near_m {
                faults.push(format!(
                    "the consumable '{item}' has a far_m of {} m; it must be above its near_m of \
                     {} m",
                    far_m.0, near_m.0
                ));
            }
            let floor = max_blast.0 * FLARE_LURE_BLAST_FACTOR;
            if !lure_m.0.is_finite() || lure_m.0 <= floor {
                faults.push(format!(
                    "the consumable '{item}' lures torpedoes {} m away; it must be above \
                     {FLARE_LURE_BLAST_FACTOR} times the largest torpedo blast radius of {} m, \
                     {floor} m",
                    lure_m.0, max_blast.0
                ));
            }
        }
    }
    faults
}

/// What a use reads off the ship, gathered by the caller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConsumableUseFacts {
    /// Units of the item in the hold, less what running repair jobs hold
    /// back.
    pub held: u32,
    /// Whether an effect of the same slot runs on the ship.
    pub running: bool,
    /// Whether a live, enabled section is below its full health.
    pub damaged: bool,
    /// Whether the ship is docked.
    pub docked: bool,
    /// Whether the ship has a live thruster.
    pub thrusters: bool,
    /// Torpedoes homing on the ship inside the flare band.
    pub torpedoes: usize,
}

/// Why a use takes nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum ConsumableRefusalType {
    /// The hold carries none of the item.
    NoneHeld,
    /// An effect of the same slot runs.
    Running,
    /// A repair, and no live section is damaged.
    NothingToRepair,
    /// A boost, and the ship is docked.
    Docked,
    /// A boost, and the ship has no live thruster.
    NoThruster,
    /// A flare, and no torpedo in its band homes on the ship.
    NoTorpedo,
}

/// Check a use of `consumable` against `facts`, in
/// [`ConsumableRefusalType`] order. A pass takes one item; a refusal takes
/// nothing.
pub fn plan_consumable_use(
    consumable: &ConsumableDesign,
    facts: &ConsumableUseFacts,
) -> Result<(), ConsumableRefusalType> {
    if facts.held == 0 {
        return Err(ConsumableRefusalType::NoneHeld);
    }
    if facts.running {
        return Err(ConsumableRefusalType::Running);
    }
    match consumable.effect {
        ConsumableType::Repair { .. } if !facts.damaged => {
            Err(ConsumableRefusalType::NothingToRepair)
        }
        ConsumableType::ThrustBoost { .. } if facts.docked => Err(ConsumableRefusalType::Docked),
        ConsumableType::ThrustBoost { .. } if !facts.thrusters => {
            Err(ConsumableRefusalType::NoThruster)
        }
        ConsumableType::Flare { .. } if facts.torpedoes == 0 => {
            Err(ConsumableRefusalType::NoTorpedo)
        }
        _ => Ok(()),
    }
}

/// Share `amount` health over `sections`, each `(current, max)`: the section
/// with the lowest share `current / max` first, filled to its max, then the
/// next, until `amount` runs out. A section at 0 health (lost) or at its max
/// takes nothing. Ties go to the earlier section. Returns `(index, new
/// current)` for each section that took health, in the order they took it.
/// What no section takes is lost.
pub fn plan_repair_split(amount: f32, sections: &[(f32, f32)]) -> Vec<(usize, f32)> {
    let mut order: Vec<usize> = (0..sections.len())
        .filter(|&index| {
            let (current, max) = sections[index];
            current > 0.0 && max > 0.0 && current < max
        })
        .collect();
    let share = |index: usize| sections[index].0 / sections[index].1;
    order.sort_by(|a, b| share(*a).total_cmp(&share(*b)).then(a.cmp(b)));
    let mut left = amount.max(0.0);
    let mut healed = Vec::new();
    for index in order {
        if left <= 0.0 {
            break;
        }
        let (current, max) = sections[index];
        let given = left.min(max - current);
        left -= given;
        healed.push((index, current + given));
    }
    healed
}

/// Whether a torpedo `distance` from the ship is inside the flare band
/// `near..=far`.
pub fn flare_in_band(distance: Meters, near: Meters, far: Meters) -> bool {
    distance >= near && distance <= far
}

/// The point a flare sends a torpedo at `torpedo` to: `lure` from the ship
/// at `ship`, at right angles to the torpedo's approach, along
/// `approach x up`. When the approach runs along `up` the side is `right`.
/// Engine units in, engine units out; `up` and `right` are unit vectors of
/// the ship's frame.
pub fn flare_lure_point(ship: Vec3, torpedo: Vec3, up: Vec3, right: Vec3, lure: f32) -> Vec3 {
    let approach = (ship - torpedo).normalize_or_zero();
    let side = approach
        .cross(up)
        .try_normalize()
        .unwrap_or_else(|| right.normalize_or_zero());
    ship + side * lure
}

impl fmt::Display for ConsumableSlotType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.word())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_items;

    fn kit(health: f32, duration_secs: f32) -> ConsumableDesign {
        ConsumableDesign {
            item: ItemDesignId::from("RepairKit"),
            effect: ConsumableType::Repair {
                health,
                duration_secs,
            },
        }
    }

    fn faults(consumable: &ConsumableDesign) -> Vec<String> {
        consumable_faults(consumable, &test_items(), Meters(300.0))
    }

    #[test]
    fn the_base_numbers_have_no_faults() {
        let boost = ConsumableDesign {
            item: ItemDesignId::from("BoostCell"),
            effect: ConsumableType::ThrustBoost {
                factor: 1.5,
                duration_secs: 8.0,
            },
        };
        let flare = ConsumableDesign {
            item: ItemDesignId::from("DecoyFlare"),
            effect: ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(800.0),
            },
        };
        for consumable in [kit(80.0, 10.0), boost, flare] {
            assert_eq!(faults(&consumable), Vec::<String>::new(), "{consumable:?}");
        }
    }

    #[test]
    fn an_unknown_or_engine_item_is_refused() {
        let mut unknown = kit(80.0, 10.0);
        unknown.item = ItemDesignId::from("NoSuchItem");
        assert_eq!(faults(&unknown).len(), 1);
        assert!(faults(&unknown)[0].contains("names no item"));
        let mut role = kit(80.0, 10.0);
        role.item = ItemDesignId::from(ItemRoleType::HullPlate.id());
        assert_eq!(faults(&role).len(), 1);
        assert!(faults(&role)[0].contains("already spends"));
    }

    #[test]
    fn bad_numbers_are_refused() {
        assert_eq!(faults(&kit(0.0, 10.0)).len(), 1);
        assert_eq!(faults(&kit(f32::NAN, 10.0)).len(), 1);
        assert_eq!(faults(&kit(80.0, 0.0)).len(), 1);
        assert_eq!(faults(&kit(80.0, 61.0)).len(), 1);
        assert!(faults(&kit(80.0, 60.0)).is_empty());
        let boost = |factor| ConsumableDesign {
            item: ItemDesignId::from("BoostCell"),
            effect: ConsumableType::ThrustBoost {
                factor,
                duration_secs: 8.0,
            },
        };
        assert_eq!(faults(&boost(1.0)).len(), 1);
        assert_eq!(faults(&boost(3.5)).len(), 1);
        assert!(faults(&boost(3.0)).is_empty());
        let flare = |near, far, lure| ConsumableDesign {
            item: ItemDesignId::from("DecoyFlare"),
            effect: ConsumableType::Flare {
                near_m: Meters(near),
                far_m: Meters(far),
                lure_m: Meters(lure),
            },
        };
        assert_eq!(faults(&flare(0.0, 3000.0, 800.0)).len(), 1);
        assert_eq!(faults(&flare(400.0, 400.0, 800.0)).len(), 1);
    }

    #[test]
    fn a_lure_inside_the_largest_blast_is_refused() {
        let flare = ConsumableDesign {
            item: ItemDesignId::from("DecoyFlare"),
            effect: ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(300.0),
            },
        };
        let found = faults(&flare);
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("largest torpedo blast"), "{found:?}");
        assert!(consumable_faults(&flare, &test_items(), Meters(199.0)).is_empty());
    }

    /// A lured torpedo fuzes up to half a blast radius short of the lure
    /// point, so the lure must clear one and a half radii, not one.
    #[test]
    fn a_lure_within_one_and_a_half_blast_radii_is_refused() {
        let flare = |lure| ConsumableDesign {
            item: ItemDesignId::from("DecoyFlare"),
            effect: ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(lure),
            },
        };
        let blast = Meters(300.0);
        assert_eq!(
            consumable_faults(&flare(400.0), &test_items(), blast).len(),
            1
        );
        assert_eq!(
            consumable_faults(&flare(450.0), &test_items(), blast).len(),
            1
        );
        assert!(consumable_faults(&flare(451.0), &test_items(), blast).is_empty());
    }

    #[test]
    fn a_use_is_refused_in_check_order() {
        let repair = kit(80.0, 10.0);
        let facts = ConsumableUseFacts {
            held: 0,
            running: true,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&repair, &facts),
            Err(ConsumableRefusalType::NoneHeld)
        );
        let facts = ConsumableUseFacts {
            held: 1,
            running: true,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&repair, &facts),
            Err(ConsumableRefusalType::Running)
        );
        let facts = ConsumableUseFacts {
            held: 1,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&repair, &facts),
            Err(ConsumableRefusalType::NothingToRepair)
        );
        let damaged = ConsumableUseFacts {
            held: 1,
            damaged: true,
            ..default()
        };
        assert_eq!(plan_consumable_use(&repair, &damaged), Ok(()));

        let boost = ConsumableDesign {
            item: ItemDesignId::from("BoostCell"),
            effect: ConsumableType::ThrustBoost {
                factor: 1.5,
                duration_secs: 8.0,
            },
        };
        let docked = ConsumableUseFacts {
            held: 1,
            docked: true,
            thrusters: false,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&boost, &docked),
            Err(ConsumableRefusalType::Docked)
        );
        let bare = ConsumableUseFacts {
            held: 1,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&boost, &bare),
            Err(ConsumableRefusalType::NoThruster)
        );

        let flare = ConsumableDesign {
            item: ItemDesignId::from("DecoyFlare"),
            effect: ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(800.0),
            },
        };
        assert_eq!(
            plan_consumable_use(&flare, &bare),
            Err(ConsumableRefusalType::NoTorpedo)
        );
        let homing = ConsumableUseFacts {
            held: 1,
            torpedoes: 2,
            ..default()
        };
        assert_eq!(plan_consumable_use(&flare, &homing), Ok(()));
    }

    #[test]
    fn a_repair_heals_the_lowest_share_first_and_carries_at_max() {
        // Shares: 0.5, 0.25, lost, full, 0.9.
        let sections = [
            (50.0, 100.0),
            (50.0, 200.0),
            (0.0, 100.0),
            (100.0, 100.0),
            (90.0, 100.0),
        ];
        assert_eq!(plan_repair_split(10.0, &sections), vec![(1, 60.0)]);
        // 150 fills section 1 and carries 0 to the rest.
        assert_eq!(plan_repair_split(150.0, &sections), vec![(1, 200.0)]);
        // 210 fills 1 (150), then 0 (50), then 10 into 4; nothing reaches the
        // lost or full ones.
        assert_eq!(
            plan_repair_split(210.0, &sections),
            vec![(1, 200.0), (0, 100.0), (4, 100.0)]
        );
        assert!(plan_repair_split(10.0, &[(0.0, 100.0), (100.0, 100.0)]).is_empty());
    }

    #[test]
    fn a_flare_band_includes_its_edges() {
        let (near, far) = (Meters(400.0), Meters(3000.0));
        assert!(!flare_in_band(Meters(399.0), near, far));
        assert!(flare_in_band(Meters(400.0), near, far));
        assert!(flare_in_band(Meters(3000.0), near, far));
        assert!(!flare_in_band(Meters(3001.0), near, far));
    }

    #[test]
    fn a_lure_point_stands_aside_of_the_approach() {
        let ship = Vec3::new(10.0, 0.0, 0.0);
        let torpedo = Vec3::new(10.0, 0.0, -200.0);
        let lure = flare_lure_point(ship, torpedo, Vec3::Y, Vec3::X, 80.0);
        assert!((lure.distance(ship) - 80.0).abs() < 1e-4);
        assert!((lure - ship).dot(torpedo - ship).abs() < 1e-3);
        // Approach along up: the side falls back to right.
        let above = Vec3::new(10.0, 200.0, 0.0);
        let lure = flare_lure_point(ship, above, Vec3::Y, Vec3::X, 80.0);
        assert!(lure.abs_diff_eq(ship + Vec3::X * 80.0, 1e-4));
    }
}
