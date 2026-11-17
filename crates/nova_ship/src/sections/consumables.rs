//! Consumables in flight: a use takes one item from the hold and starts its
//! effect on the ship.
//!
//! A [`ConsumableUseRequest`] names a ship and a [`ConsumableSlotType`]; the
//! flight keys and the Inventory pane's `Use` button write it.
//! [`use_consumables`] checks it by [`plan_consumable_use`] and, on a pass,
//! takes one item at once (there is no cancel). A repair or a boost then
//! runs as a [`RunningConsumable`] in [`ShipConsumableEffects`] on the ship
//! root, one per slot; a flare acts at once: every torpedo homing on the
//! ship inside its band drops the lock and flies to its lure point, where it
//! fuzes as a torpedo whose target died does. A refusal takes nothing. Every
//! use and refusal writes one [`ConsumableUsed`] for the flare notice and
//! the cockpit deny cue.
//!
//! [`advance_consumable_effects`] runs the timed effects on the game clock,
//! so the TAB interface and the pause menu hold them. A repair gives its
//! health evenly over its duration, each frame's share to the live, enabled
//! sections with the lowest health share ([`plan_repair_split`]); it ends
//! early when nothing is damaged, and the rest is lost. A boost multiplies
//! every thruster impulse by its factor while no autopilot verb flies the
//! ship (`thruster_impulse_system`); its clock runs either way. A save keeps
//! the running effects by item: they hold no entity.
//!
//! [`plan_consumable_use`]: nova_gameplay::prelude::plan_consumable_use
//! [`plan_repair_split`]: nova_gameplay::prelude::plan_repair_split

use std::collections::{HashMap, HashSet};

use bevy::{
    ecs::{query::QueryFilter, system::SystemParam},
    prelude::*,
};
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{
    flare_in_band, flare_lure_point, plan_consumable_use, plan_repair_split, ConsumableDesign,
    ConsumableRefusalType, ConsumableSlotType, ConsumableType, ConsumableUseFacts, GameConsumables,
    Health, IntegrityDisabledMarker, ItemDesignId, SectionMarker, ShipInventory,
    SpaceshipRootMarker, ThrusterSectionMarker, TorpedoProjectileMarker,
};

use super::prelude::{DockedShip, ShipRepairJobs, TorpedoTargetEntity, TorpedoTargetPosition};

/// The running effects, the use request and outcome, and their systems.
pub mod prelude {
    pub use super::{
        advance_consumable_effects, use_consumables, ConsumableFactsQuery, ConsumableOutcomeType,
        ConsumableSystems, ConsumableUseRequest, ConsumableUsed, RunningConsumable,
        ShipConsumableEffects,
    };
}

/// One timed effect running on a ship: the item whose consumable it is and
/// the game seconds it has run, at least 0 and below its duration.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RunningConsumable {
    /// The item; its consumable is a repair or a boost.
    pub item: ItemDesignId,
    /// Game seconds run.
    pub elapsed_secs: f32,
}

/// The timed effects running on a ship root, at most one per slot. Inserted
/// by the first use, removed when the last effect ends.
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct ShipConsumableEffects(pub Vec<RunningConsumable>);

impl ShipConsumableEffects {
    /// The effect running in `slot` and its consumable.
    pub fn in_slot<'a>(
        &'a self,
        consumables: &'a GameConsumables,
        slot: ConsumableSlotType,
    ) -> Option<(&'a RunningConsumable, &'a ConsumableDesign)> {
        self.0.iter().find_map(|running| {
            let design = consumables.get(&running.item)?;
            (design.effect.slot() == slot).then_some((running, design))
        })
    }

    /// Game seconds left of the effect in `slot`, if one runs.
    pub fn remaining_secs(
        &self,
        consumables: &GameConsumables,
        slot: ConsumableSlotType,
    ) -> Option<f32> {
        let (running, design) = self.in_slot(consumables, slot)?;
        let duration = design.effect.duration_secs()?;
        Some((duration - running.elapsed_secs).max(0.0))
    }

    /// The thrust factor a running boost gives; 1 without one.
    pub fn thrust_factor(&self, consumables: &GameConsumables) -> f32 {
        match self.in_slot(consumables, ConsumableSlotType::ThrustBoost) {
            Some((
                _,
                ConsumableDesign {
                    effect: ConsumableType::ThrustBoost { factor, .. },
                    ..
                },
            )) => *factor,
            _ => 1.0,
        }
    }
}

/// Use the consumable of `slot` on `ship`. Written by the flight keys and
/// the Inventory pane.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsumableUseRequest {
    /// The ship root.
    pub ship: Entity,
    /// The effect type, so the key.
    pub slot: ConsumableSlotType,
}

/// What one [`ConsumableUseRequest`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumableOutcomeType {
    /// A repair or a boost started; one item was taken.
    Started,
    /// A flare pulled `torpedoes` torpedoes off; one item was taken.
    Decoyed {
        /// How many torpedoes dropped the lock.
        torpedoes: usize,
    },
    /// Refused; nothing was taken.
    Refused(ConsumableRefusalType),
}

/// One use or refusal, for the flare notice and the cockpit deny cue.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct ConsumableUsed {
    /// The ship root.
    pub ship: Entity,
    /// The consumable's item.
    pub item: ItemDesignId,
    /// What the use did.
    pub outcome: ConsumableOutcomeType,
}

/// System set for the consumable use and tick, in `Update`. After
/// `CraftJobSystems`: all three write the hold, and a use counts what the
/// repair jobs hold back. The use runs before the torpedo target tracking
/// (`update_target_position`), so a flared torpedo turns the frame of the
/// press.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConsumableSystems;

/// The live, enabled sections under `children`, each `(entity, current,
/// max)`.
fn live_sections<F: QueryFilter>(
    children: Option<&Children>,
    q_section: &Query<(&Health, Has<IntegrityDisabledMarker>), F>,
) -> Vec<(Entity, f32, f32)> {
    children
        .into_iter()
        .flatten()
        .filter_map(|child| {
            let (health, disabled) = q_section.get(*child).ok()?;
            (!disabled && health.current > 0.0).then_some((*child, health.current, health.max))
        })
        .collect()
}

/// What a use reads off a ship, shared by [`use_consumables`] and the
/// flight hints, so a lit chip and an answered key cannot disagree.
#[derive(SystemParam)]
pub struct ConsumableFactsQuery<'w, 's> {
    ships: Query<
        'w,
        's,
        (
            Option<&'static ShipRepairJobs>,
            Has<DockedShip>,
            &'static Transform,
            Option<&'static Children>,
        ),
        With<SpaceshipRootMarker>,
    >,
    sections: Query<'w, 's, (&'static Health, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
    thrusters: Query<'w, 's, (), With<ThrusterSectionMarker>>,
    torpedoes: Query<
        'w,
        's,
        (Entity, &'static Transform, &'static TorpedoTargetEntity),
        With<TorpedoProjectileMarker>,
    >,
}

impl ConsumableFactsQuery<'_, '_> {
    /// The facts for a use of `design` on `ship`, whose hold is `hold`, and
    /// the torpedoes a flare would pull off with where each flies now.
    /// `running` says an effect of the same slot runs. `None` when `ship` is
    /// no ship root.
    pub fn facts(
        &self,
        ship: Entity,
        design: &ConsumableDesign,
        hold: &ShipInventory,
        running: bool,
    ) -> Option<(ConsumableUseFacts, Vec<(Entity, Vec3)>)> {
        let (repairs, docked, pose, children) = self.ships.get(ship).ok()?;
        let held = match repairs {
            Some(repairs) => repairs.free_hold(hold).count(&design.item),
            None => hold.count(&design.item),
        };
        let sections = live_sections(children, &self.sections);
        let homing: Vec<(Entity, Vec3)> = match design.effect {
            ConsumableType::Flare { near_m, far_m, .. } => self
                .torpedoes
                .iter()
                .filter(|(_, _, target)| ***target == ship)
                .map(|(torpedo, at, _)| (torpedo, at.translation))
                .filter(|(_, at)| {
                    let distance = Meters::from_engine(at.distance(pose.translation));
                    flare_in_band(distance, near_m, far_m)
                })
                .collect(),
            _ => Vec::new(),
        };
        let facts = ConsumableUseFacts {
            held,
            running,
            damaged: sections.iter().any(|(_, current, max)| current < max),
            docked,
            thrusters: sections
                .iter()
                .any(|(section, _, _)| self.thrusters.contains(*section)),
            torpedoes: homing.len(),
        };
        Some((facts, homing))
    }

    /// The ship's pose, for the flare's lure points.
    fn pose(&self, ship: Entity) -> Option<&Transform> {
        self.ships.get(ship).ok().map(|(_, _, pose, _)| pose)
    }
}

/// Answer every [`ConsumableUseRequest`] of this frame. See the module docs.
///
/// A request for a ship that is gone, or for a slot no loaded consumable
/// takes, is dropped with a debug line: the keys write one for the player
/// ship whatever the catalog holds.
pub fn use_consumables(
    mut requests: MessageReader<ConsumableUseRequest>,
    consumables: Option<Res<GameConsumables>>,
    mut commands: Commands,
    mut q_ship: Query<
        (&mut ShipInventory, Option<&mut ShipConsumableEffects>),
        With<SpaceshipRootMarker>,
    >,
    q_facts: ConsumableFactsQuery,
    mut used: MessageWriter<ConsumableUsed>,
) {
    // Effects started this frame on a ship that had none: its component is
    // inserted by command, so a second request this frame reads them here.
    let mut started: HashMap<Entity, Vec<RunningConsumable>> = HashMap::new();
    // Torpedoes a flare lured this frame: the lure lands by command, so they
    // still read as homing, and a second flare would be spent on them.
    let mut lured: HashSet<Entity> = HashSet::new();
    for request in requests.read() {
        let Some(consumables) = consumables.as_deref() else {
            debug!("use_consumables: no consumable catalog loaded");
            continue;
        };
        let Some(design) = consumables.in_slot(request.slot) else {
            debug!(
                "use_consumables: no consumable takes the {} slot",
                request.slot
            );
            continue;
        };
        let Ok((mut hold, effects)) = q_ship.get_mut(request.ship) else {
            debug!("use_consumables: ship {:?} is gone", request.ship);
            continue;
        };
        let pending = started.get(&request.ship);
        let running = effects
            .as_deref()
            .is_some_and(|effects| effects.in_slot(consumables, request.slot).is_some())
            || pending.is_some_and(|pending| {
                pending.iter().any(|running| {
                    consumables
                        .get(&running.item)
                        .is_some_and(|other| other.effect.slot() == request.slot)
                })
            });
        let Some((mut facts, mut homing)) = q_facts.facts(request.ship, design, &hold, running)
        else {
            debug!("use_consumables: ship {:?} is gone", request.ship);
            continue;
        };
        homing.retain(|(torpedo, _)| !lured.contains(torpedo));
        facts.torpedoes = homing.len();
        let outcome = match plan_consumable_use(design, &facts) {
            Err(refusal) => ConsumableOutcomeType::Refused(refusal),
            Ok(()) => {
                hold.remove(&design.item, 1);
                match design.effect {
                    ConsumableType::Flare { lure_m, .. } => {
                        let pose = q_facts
                            .pose(request.ship)
                            .expect("use_consumables: the facts just read this ship's pose");
                        let (up, right) = (pose.up().as_vec3(), pose.right().as_vec3());
                        for (torpedo, at) in &homing {
                            let lure = flare_lure_point(
                                pose.translation,
                                *at,
                                up,
                                right,
                                lure_m.to_engine(),
                            );
                            commands
                                .entity(*torpedo)
                                .remove::<TorpedoTargetEntity>()
                                .insert(TorpedoTargetPosition(lure));
                            lured.insert(*torpedo);
                        }
                        ConsumableOutcomeType::Decoyed {
                            torpedoes: homing.len(),
                        }
                    }
                    ConsumableType::Repair { .. } | ConsumableType::ThrustBoost { .. } => {
                        let entry = RunningConsumable {
                            item: design.item.clone(),
                            elapsed_secs: 0.0,
                        };
                        match effects {
                            Some(mut effects) => effects.0.push(entry),
                            None => {
                                let pending = started.entry(request.ship).or_default();
                                pending.push(entry);
                                commands
                                    .entity(request.ship)
                                    .insert(ShipConsumableEffects(pending.clone()));
                            }
                        }
                        ConsumableOutcomeType::Started
                    }
                }
            }
        };
        debug!(
            "use_consumables: {} on {:?}: {outcome:?}",
            design.item, request.ship
        );
        used.write(ConsumableUsed {
            ship: request.ship,
            item: design.item.clone(),
            outcome,
        });
    }
}

/// Run every ship's timed effects for this frame's game time. See the
/// module docs.
///
/// # Panics
///
/// When an effect names an item the loaded consumables lack, or runs before
/// they load: a use starts one only from a loaded consumable, and the save
/// check refuses an unknown one.
pub fn advance_consumable_effects(
    time: Res<Time>,
    consumables: Option<Res<GameConsumables>>,
    mut commands: Commands,
    mut q_ship: Query<(Entity, &mut ShipConsumableEffects, Option<&Children>)>,
    mut q_health: Query<(&mut Health, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (ship, mut effects, children) in &mut q_ship {
        let consumables = consumables
            .as_deref()
            .expect("consumable effects: an effect runs only after the catalog loads");
        effects.0.retain_mut(|running| {
            let design = consumables.design(&running.item);
            let Some(duration) = design.effect.duration_secs() else {
                return false;
            };
            let step = dt.min(duration - running.elapsed_secs).max(0.0);
            running.elapsed_secs += step;
            if let ConsumableType::Repair {
                health,
                duration_secs,
            } = design.effect
            {
                let mut sections = live_sections(children, &q_health.as_readonly());
                let pools: Vec<(f32, f32)> = sections.iter().map(|(_, c, m)| (*c, *m)).collect();
                for (index, current) in plan_repair_split(health * step / duration_secs, &pools) {
                    sections[index].1 = current;
                    if let Ok((mut section, _)) = q_health.get_mut(sections[index].0) {
                        section.current = current;
                    }
                }
                if sections.iter().all(|(_, current, max)| current >= max) {
                    debug!(
                        "consumable effects: {} on {ship:?} has nothing left to repair",
                        running.item
                    );
                    return false;
                }
            }
            running.elapsed_secs < duration
        });
        if effects.0.is_empty() {
            commands.entity(ship).remove::<ShipConsumableEffects>();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use nova_gameplay::test_support::test_items;

    use super::*;

    fn consumables() -> GameConsumables {
        GameConsumables::new([
            ConsumableDesign {
                item: ItemDesignId::from("RepairKit"),
                effect: ConsumableType::Repair {
                    health: 80.0,
                    duration_secs: 10.0,
                },
            },
            ConsumableDesign {
                item: ItemDesignId::from("BoostCell"),
                effect: ConsumableType::ThrustBoost {
                    factor: 1.5,
                    duration_secs: 8.0,
                },
            },
        ])
    }

    /// A ship with `kits` repair kits and one section at `health` of 100,
    /// its clock stepping one game second a frame after the warm-up frame.
    fn effect_app(kits: u32, health: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(60));
        app.insert_resource(consumables());
        app.add_message::<ConsumableUseRequest>();
        app.add_message::<ConsumableUsed>();
        app.add_systems(
            Update,
            (use_consumables, advance_consumable_effects).chain(),
        );
        let items = test_items();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Transform::default(),
                ShipInventory::new(&items, 1_000_000, [(ItemDesignId::from("RepairKit"), kits)]),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: health,
                    max: 100.0,
                },
                ChildOf(ship),
            ))
            .id();
        app.update();
        (app, ship, section)
    }

    fn outcomes(app: &mut App) -> Vec<ConsumableOutcomeType> {
        app.world_mut()
            .resource_mut::<Messages<ConsumableUsed>>()
            .drain()
            .map(|used| used.outcome)
            .collect()
    }

    fn use_kit(app: &mut App, ship: Entity) {
        app.world_mut().write_message(ConsumableUseRequest {
            ship,
            slot: ConsumableSlotType::Repair,
        });
    }

    /// A kit heals 8 HP a second for 10 s; a second use while it runs is
    /// refused and keeps the item.
    #[test]
    fn a_repair_kit_heals_over_its_duration_and_a_second_use_is_refused() {
        let (mut app, ship, section) = effect_app(2, 10.0);
        use_kit(&mut app, ship);
        app.update();
        assert_eq!(outcomes(&mut app), [ConsumableOutcomeType::Started]);
        let health = |app: &App| app.world().get::<Health>(section).unwrap().current;
        assert!((health(&app) - 18.0).abs() < 1e-3, "{}", health(&app));
        use_kit(&mut app, ship);
        app.update();
        assert_eq!(
            outcomes(&mut app),
            [ConsumableOutcomeType::Refused(
                ConsumableRefusalType::Running
            )]
        );
        let held = app
            .world()
            .get::<ShipInventory>(ship)
            .unwrap()
            .count(&ItemDesignId::from("RepairKit"));
        assert_eq!(held, 1);
        for _ in 0..12 {
            app.update();
        }
        assert!((health(&app) - 90.0).abs() < 1e-3, "{}", health(&app));
        assert!(app.world().get::<ShipConsumableEffects>(ship).is_none());
    }

    /// A kit ends once nothing is damaged; the rest of its health is lost.
    #[test]
    fn a_repair_kit_ends_early_when_nothing_is_damaged() {
        let (mut app, ship, section) = effect_app(1, 90.0);
        use_kit(&mut app, ship);
        app.update();
        app.update();
        assert_eq!(app.world().get::<Health>(section).unwrap().current, 100.0);
        assert!(app.world().get::<ShipConsumableEffects>(ship).is_none());
    }

    /// An undamaged ship refuses a kit and keeps it.
    #[test]
    fn a_kit_with_nothing_to_repair_is_refused() {
        let (mut app, ship, _) = effect_app(1, 100.0);
        use_kit(&mut app, ship);
        app.update();
        assert_eq!(
            outcomes(&mut app),
            [ConsumableOutcomeType::Refused(
                ConsumableRefusalType::NothingToRepair
            )]
        );
    }

    /// Two flare presses in one frame: the first lures the torpedo, the
    /// second finds none left homing and keeps its flare.
    #[test]
    fn a_second_flare_in_the_same_frame_is_refused_and_kept() {
        let flare = ItemDesignId::from("DecoyFlare");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameConsumables::new([ConsumableDesign {
            item: flare.clone(),
            effect: ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(800.0),
            },
        }]));
        app.add_message::<ConsumableUseRequest>();
        app.add_message::<ConsumableUsed>();
        app.add_systems(Update, use_consumables);
        let items = test_items();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Transform::default(),
                ShipInventory::new(&items, 1_000_000, [(flare.clone(), 2)]),
            ))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                // 100 engine units: 1 km, inside the 400 m to 3 km band.
                Transform::from_xyz(0.0, 0.0, -100.0),
                TorpedoTargetEntity(ship),
            ))
            .id();
        app.update();
        for _ in 0..2 {
            app.world_mut().write_message(ConsumableUseRequest {
                ship,
                slot: ConsumableSlotType::Flare,
            });
        }
        app.update();
        assert_eq!(
            outcomes(&mut app),
            [
                ConsumableOutcomeType::Decoyed { torpedoes: 1 },
                ConsumableOutcomeType::Refused(ConsumableRefusalType::NoTorpedo),
            ]
        );
        assert_eq!(
            app.world()
                .get::<ShipInventory>(ship)
                .unwrap()
                .count(&flare),
            1
        );
        assert!(app.world().get::<TorpedoTargetEntity>(torpedo).is_none());
    }

    #[test]
    fn a_running_boost_scales_thrust_by_its_factor() {
        let mut effects = ShipConsumableEffects::default();
        assert_eq!(effects.thrust_factor(&consumables()), 1.0);
        effects.0.push(RunningConsumable {
            item: ItemDesignId::from("BoostCell"),
            elapsed_secs: 3.0,
        });
        assert_eq!(effects.thrust_factor(&consumables()), 1.5);
        assert_eq!(
            effects.remaining_secs(&consumables(), ConsumableSlotType::ThrustBoost),
            Some(5.0)
        );
    }
}
