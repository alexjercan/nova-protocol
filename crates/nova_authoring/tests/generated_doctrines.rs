//! Generated ships across many civilizations from the real base catalog:
//! every civilization's build doctrine lays out every role, its wrecks and
//! its escort raiders through the world's own `generate_ship` and
//! `generate_wreck`, with no failure and inside the section budget.
//!
//! Both sweeps print a report (redraw rate, largest section count, turn-rate
//! medians per role and advancement) for the doctrine ranges to be judged
//! against. Run them with
//! `cargo test -p nova_authoring --test generated_doctrines -- --ignored --nocapture`.

use std::{collections::BTreeMap, thread};

use bevy::math::{Quat, Vec3};
use nova_authoring::{generation::build_palettes, lint_walk::repo_ship_part_packs};
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{escort_raider_size, CivilizationId, ShipRoleType, ShipSizeType};
use nova_ship::prelude::{hull_mass_properties, structural_arm, AttitudeEnvelope};
use nova_world_base::prelude::{
    generate_ship, generate_wreck, ShipLayout, ShipLayoutRequest, ShipPart, ShipPartSnapshot,
    ShipSpareDriveType, MAX_SHIP_SECTIONS,
};

/// The world seeds the civilizations come from.
const WORLDS: std::ops::RangeInclusive<u32> = 1..=8;

/// How far the lattice reaches from the origin on each of its two axes: 5 x 5
/// nodes per world, so 200 civilizations over [`WORLDS`].
const REACH: i32 = 2;

/// The advancement steps every role is built at.
const ADVANCEMENTS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// The hull seeds per civilization, role and advancement.
const SEEDS: u32 = 32;

/// Every fourth hull seed is also ruined.
const WRECK_EVERY: u32 = 4;

/// The advancements each escort raider band is built at: its start, middle
/// and end. The band from 0.75 builds at full advancement size and is swept
/// above.
const RAIDER_ADVANCEMENTS: [[f32; 3]; 3] =
    [[0.0, 0.12, 0.24], [0.25, 0.37, 0.49], [0.5, 0.62, 0.74]];

/// The hull seeds per raider civilization and advancement.
const RAIDER_SEEDS: u32 = 8;

/// The scoped threads a sweep splits its civilizations over.
const THREADS: usize = 12;

/// Every civilization the sweeps build for.
fn civilizations() -> Vec<CivilizationId> {
    WORLDS
        .flat_map(|world_seed| {
            (-REACH..=REACH).flat_map(move |x| {
                (-REACH..=REACH).map(move |y| CivilizationId {
                    world_seed,
                    node: [x, y, 0],
                })
            })
        })
        .collect()
}

/// The hull seed of the `seed`-th hull of `civilization`: spread so two
/// civilizations do not build the same seed list.
fn hull_seed(civilization: CivilizationId, seed: u32) -> u32 {
    seed.wrapping_mul(2_654_435_761) ^ civilization.node[0] as u32 ^ civilization.world_seed
}

fn snapshot() -> ShipPartSnapshot {
    ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]), &build_palettes())
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"))
}

/// The sustained turn rate of `layout` in deg/s, `sqrt(LOAD_LIMIT / arm)`,
/// by the same arithmetic as the editor's read-out: every generated hull is
/// structure-bound.
fn turn_rate(parts: &BTreeMap<&str, &ShipPart>, layout: &ShipLayout) -> f32 {
    let posed: Vec<(Vec3, Quat, &ShipPart)> = layout
        .design
        .sections
        .iter()
        .map(|section| {
            let id = section.source.prototype_id();
            let part = parts
                .get(id)
                .unwrap_or_else(|| panic!("'{id}' is a base part"));
            (section.position, section.rotation, *part)
        })
        .collect();
    let properties = hull_mass_properties(posed.iter().map(|(position, rotation, part)| {
        (
            *position,
            *rotation,
            part.config.base.collider.unwrap_or_default().to_collider(),
            1.0,
        )
    }));
    let arm = structural_arm(
        properties.center_of_mass,
        posed.iter().map(|(position, rotation, part)| {
            (
                *position,
                *rotation,
                part.config
                    .base
                    .collider
                    .unwrap_or_default()
                    .aabb_half_extents(),
            )
        }),
    );
    AttitudeEnvelope::new(0.0, 0.0, Meters::from_engine(arm))
        .sustained_turn_rate()
        .to_degrees()
}

/// What one sweep cell (a role or band and an advancement) built.
#[derive(Default)]
struct Cell {
    hulls: usize,
    fails: Vec<String>,
    redrawn: usize,
    most_sections: usize,
    largest_clearance: f32,
    turn_rates: Vec<f32>,
    wrecks: usize,
    wreck_fails: Vec<String>,
}

impl Cell {
    fn merge(&mut self, other: Cell) {
        self.hulls += other.hulls;
        self.fails.extend(other.fails);
        self.redrawn += other.redrawn;
        self.most_sections = self.most_sections.max(other.most_sections);
        self.largest_clearance = self.largest_clearance.max(other.largest_clearance);
        self.turn_rates.extend(other.turn_rates);
        self.wrecks += other.wrecks;
        self.wreck_fails.extend(other.wreck_fails);
    }

    fn record(&mut self, parts: &BTreeMap<&str, &ShipPart>, built: Result<ShipLayout, String>) {
        self.hulls += 1;
        match built {
            Ok(layout) => {
                self.redrawn += usize::from(layout.attempt > 0);
                self.most_sections = self.most_sections.max(layout.design.sections.len());
                self.largest_clearance = self.largest_clearance.max(layout.clearance.0);
                self.turn_rates.push(turn_rate(parts, &layout));
            }
            Err(failure) => self.fails.push(failure),
        }
    }

    fn median_turn(&self) -> f32 {
        let mut rates = self.turn_rates.clone();
        rates.sort_by(f32::total_cmp);
        rates.get(rates.len() / 2).copied().unwrap_or(f32::NAN)
    }
}

/// Build `cells` for every civilization over [`THREADS`] threads, keyed by
/// whatever `sweep` keys each civilization's cells by, and print them.
fn sweep(
    title: &str,
    snapshot: &ShipPartSnapshot,
    sweep: impl Fn(&BTreeMap<&str, &ShipPart>, CivilizationId) -> BTreeMap<(String, u32), Cell> + Sync,
) -> BTreeMap<(String, u32), Cell> {
    let parts: BTreeMap<&str, &ShipPart> = snapshot
        .parts()
        .iter()
        .map(|part| (part.id(), part))
        .collect();
    let civilizations = civilizations();
    let chunk = civilizations.len().div_ceil(THREADS);
    let mut cells: BTreeMap<(String, u32), Cell> = BTreeMap::new();
    thread::scope(|scope| {
        let handles: Vec<_> = civilizations
            .chunks(chunk)
            .map(|chunk| {
                let (parts, sweep) = (&parts, &sweep);
                scope.spawn(move || {
                    let mut cells: BTreeMap<(String, u32), Cell> = BTreeMap::new();
                    for civilization in chunk {
                        for (key, cell) in sweep(parts, *civilization) {
                            cells.entry(key).or_default().merge(cell);
                        }
                    }
                    cells
                })
            })
            .collect();
        for handle in handles {
            for (key, cell) in handle.join().expect("a sweep thread finishes") {
                cells.entry(key).or_default().merge(cell);
            }
        }
    });

    println!(
        "{title}: {} civilizations\nrow\tadvancement\thulls\tfails\tredrawn %\tmost sections\tclearance max m\tturn median deg/s\twrecks\twreck fails",
        civilizations.len()
    );
    let mut total = Cell::default();
    for ((row, advancement), cell) in &cells {
        println!(
            "{row}\t{:.2}\t{}\t{}\t{:.1}\t{}\t{:.0}\t{:.0}\t{}\t{}",
            *advancement as f32 / 100.0,
            cell.hulls,
            cell.fails.len(),
            100.0 * cell.redrawn as f32 / cell.hulls as f32,
            cell.most_sections,
            cell.largest_clearance,
            cell.median_turn(),
            cell.wrecks,
            cell.wreck_fails.len(),
        );
        total.hulls += cell.hulls;
        total.redrawn += cell.redrawn;
        total.most_sections = total.most_sections.max(cell.most_sections);
        total.wrecks += cell.wrecks;
        total.fails.extend(cell.fails.iter().cloned());
        total.wreck_fails.extend(cell.wreck_fails.iter().cloned());
    }
    println!(
        "{title} total: {} hulls, {} fails, redrawn {:.1}%, most sections {}, {} wrecks, {} wreck fails",
        total.hulls,
        total.fails.len(),
        100.0 * total.redrawn as f32 / total.hulls as f32,
        total.most_sections,
        total.wrecks,
        total.wreck_fails.len(),
    );
    for failure in total.fails.iter().chain(&total.wreck_fails).take(20) {
        println!("  {failure}");
    }
    cells
}

/// Every role of 200 civilizations at five advancements and 32 seeds lays
/// out under its civilization's doctrine, every fourth one also as a wreck,
/// with no failure and none past [`MAX_SHIP_SECTIONS`].
#[test]
#[ignore = "160,000 layouts: minutes of CPU, run by hand when the doctrine or the generator's draws change"]
fn every_civilization_builds_every_role_and_its_wrecks_within_the_section_budget() {
    let snapshot = snapshot();
    let cells = sweep("doctrine sweep", &snapshot, |parts, civilization| {
        let mut cells: BTreeMap<(String, u32), Cell> = BTreeMap::new();
        for advancement in ADVANCEMENTS {
            for role in snapshot.eligible_roles(civilization, advancement) {
                let cell = cells
                    .entry((format!("{role:?}"), (advancement * 100.0).round() as u32))
                    .or_default();
                for seed in 0..SEEDS {
                    let request = ShipLayoutRequest {
                        seed: hull_seed(civilization, seed),
                        civilization,
                        role,
                        advancement,
                        size: ShipSizeType::Advancement,
                        spare_drive: ShipSpareDriveType::Without,
                    };
                    cell.record(
                        parts,
                        generate_ship(&snapshot, request).map_err(|failure| failure.to_string()),
                    );
                    if seed % WRECK_EVERY == 0 {
                        cell.wrecks += 1;
                        if let Err(failure) = generate_wreck(&snapshot, request) {
                            cell.wreck_fails.push(failure.to_string());
                        }
                    }
                }
            }
        }
        cells
    });
    for ((role, advancement), cell) in &cells {
        assert!(
            cell.fails.is_empty() && cell.wreck_fails.is_empty(),
            "{role} at {advancement}: {:?} {:?}",
            cell.fails,
            cell.wreck_fails
        );
        assert!(
            cell.most_sections <= MAX_SHIP_SECTIONS,
            "{role} at {advancement}: {} sections",
            cell.most_sections
        );
    }
}

/// Every escort raider band of 200 civilizations, at three advancements and
/// eight seeds each, lays out a scavenger inside its section budget: a job
/// raider that fails panics.
#[test]
#[ignore = "14,400 layouts: run by hand with the doctrine sweep"]
fn every_civilization_builds_its_escort_raiders_in_every_band() {
    let snapshot = snapshot();
    let cells = sweep("raider sweep", &snapshot, |parts, civilization| {
        let mut cells: BTreeMap<(String, u32), Cell> = BTreeMap::new();
        for (band, advancements) in RAIDER_ADVANCEMENTS.iter().enumerate() {
            for advancement in *advancements {
                // A civilization with no open weapon posts no raider.
                if !snapshot
                    .eligible_roles(civilization, advancement)
                    .contains(&ShipRoleType::Scavenger)
                {
                    continue;
                }
                let size = escort_raider_size(advancement);
                assert!(matches!(size, ShipSizeType::Budget { .. }), "{advancement}");
                let cell = cells
                    .entry((format!("band {band}"), (advancement * 100.0).round() as u32))
                    .or_default();
                for seed in 0..RAIDER_SEEDS {
                    let request = ShipLayoutRequest {
                        seed: hull_seed(civilization, seed),
                        civilization,
                        role: ShipRoleType::Scavenger,
                        advancement,
                        size,
                        spare_drive: ShipSpareDriveType::Without,
                    };
                    cell.record(
                        parts,
                        generate_ship(&snapshot, request).map_err(|failure| failure.to_string()),
                    );
                }
            }
        }
        cells
    });
    for ((band, advancement), cell) in &cells {
        assert!(
            cell.fails.is_empty(),
            "{band} at {advancement}: {:?}",
            cell.fails
        );
    }
}
