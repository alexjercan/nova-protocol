//! Consumables on a ship: a use takes one item from the hold and starts its
//! effect on the ship.
//!
//! A [`ConsumableUseRequest`] names a ship and the item; the Inventory
//! pane's `Use` button writes it. [`use_consumables`] checks it by
//! [`plan_consumable_use`] and, on a pass, takes one item at once (there is
//! no cancel). The effect then runs as the [`ShipConsumableEffect`] on the
//! ship root, one at a time. A refusal takes nothing. Every use and refusal
//! writes one [`ConsumableUsed`] for the cockpit deny cue.
//!
//! [`advance_consumable_effects`] runs the effect on the game clock, so the
//! TAB interface and the pause menu hold it. A repair gives its health evenly
//! over its duration, each frame's share to the live, enabled sections with
//! the lowest health share ([`plan_repair_split`]); it ends early when
//! nothing is damaged, and the rest is lost. A save keeps the running effect
//! by item: it holds no entity.
//!
//! [`plan_consumable_use`]: nova_gameplay::prelude::plan_consumable_use
//! [`plan_repair_split`]: nova_gameplay::prelude::plan_repair_split

use std::collections::HashSet;

use bevy::{
    ecs::{query::QueryFilter, system::SystemParam},
    prelude::*,
};
use nova_gameplay::prelude::{
    plan_consumable_use, plan_repair_split, ConsumableDesign, ConsumableRefusalType,
    ConsumableType, ConsumableUseFacts, GameConsumables, Health, IntegrityDisabledMarker,
    ItemDesignId, SectionMarker, ShipInventory, SpaceshipRootMarker,
};

use super::prelude::ShipRepairJobs;

/// The running effect, the use request and outcome, and their systems.
pub mod prelude {
    pub use super::{
        advance_consumable_effects, use_consumables, ConsumableFactsQuery, ConsumableOutcomeType,
        ConsumableSystems, ConsumableUseRequest, ConsumableUsed, ShipConsumableEffect,
    };
}

/// The effect running on a ship root: the item whose consumable it is and
/// the game seconds it has run, at least 0 and below its duration. Inserted
/// by a use, removed when the effect ends.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipConsumableEffect {
    /// The item; its consumable names the effect.
    pub item: ItemDesignId,
    /// Game seconds run.
    pub elapsed_secs: f32,
}

impl ShipConsumableEffect {
    /// Game seconds left of the effect.
    ///
    /// # Panics
    ///
    /// When the item has no loaded consumable; see
    /// [`GameConsumables::design`].
    pub fn remaining_secs(&self, consumables: &GameConsumables) -> f32 {
        let duration = consumables.design(&self.item).effect.duration_secs();
        (duration - self.elapsed_secs).max(0.0)
    }
}

/// Use one `item` on `ship`. Written by the Inventory pane.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ConsumableUseRequest {
    /// The ship root.
    pub ship: Entity,
    /// The consumable's item.
    pub item: ItemDesignId,
}

/// What one [`ConsumableUseRequest`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsumableOutcomeType {
    /// The effect started; one item was taken.
    Started,
    /// Refused; nothing was taken.
    Refused(ConsumableRefusalType),
}

/// One use or refusal, for the cockpit deny cue.
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
/// repair jobs hold back.
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
/// Inventory pane's Use button, so an enabled button and a passed use
/// cannot disagree.
#[derive(SystemParam)]
pub struct ConsumableFactsQuery<'w, 's> {
    ships: Query<
        'w,
        's,
        (
            Option<&'static ShipRepairJobs>,
            Has<ShipConsumableEffect>,
            Option<&'static Children>,
        ),
        With<SpaceshipRootMarker>,
    >,
    sections: Query<'w, 's, (&'static Health, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
}

impl ConsumableFactsQuery<'_, '_> {
    /// The facts for a use of `design` on `ship`, whose hold is `hold`.
    /// `None` when `ship` is no ship root.
    pub fn facts(
        &self,
        ship: Entity,
        design: &ConsumableDesign,
        hold: &ShipInventory,
    ) -> Option<ConsumableUseFacts> {
        let (repairs, running, children) = self.ships.get(ship).ok()?;
        let held = match repairs {
            Some(repairs) => repairs.free_hold(hold).count(&design.item),
            None => hold.count(&design.item),
        };
        let sections = live_sections(children, &self.sections);
        Some(ConsumableUseFacts {
            held,
            running,
            damaged: sections.iter().any(|(_, current, max)| current < max),
        })
    }
}

/// Answer every [`ConsumableUseRequest`] of this frame. See the module docs.
///
/// A request for a ship that is gone, or for an item no loaded consumable
/// takes, is dropped with a debug line.
pub fn use_consumables(
    mut requests: MessageReader<ConsumableUseRequest>,
    consumables: Option<Res<GameConsumables>>,
    mut commands: Commands,
    mut q_hold: Query<&mut ShipInventory, With<SpaceshipRootMarker>>,
    q_facts: ConsumableFactsQuery,
    mut used: MessageWriter<ConsumableUsed>,
) {
    // Ships an effect started on this frame: the component lands by command,
    // so a second request this frame would not see it run.
    let mut started: HashSet<Entity> = HashSet::new();
    for request in requests.read() {
        let Some(consumables) = consumables.as_deref() else {
            debug!("use_consumables: no consumable catalog loaded");
            continue;
        };
        let Some(design) = consumables.get(&request.item) else {
            debug!("use_consumables: no consumable takes '{}'", request.item);
            continue;
        };
        let Ok(mut hold) = q_hold.get_mut(request.ship) else {
            debug!("use_consumables: ship {:?} is gone", request.ship);
            continue;
        };
        let Some(mut facts) = q_facts.facts(request.ship, design, &hold) else {
            debug!("use_consumables: ship {:?} is gone", request.ship);
            continue;
        };
        facts.running |= started.contains(&request.ship);
        let outcome = match plan_consumable_use(design, &facts) {
            Err(refusal) => ConsumableOutcomeType::Refused(refusal),
            Ok(()) => {
                hold.remove(&design.item, 1);
                commands.entity(request.ship).insert(ShipConsumableEffect {
                    item: design.item.clone(),
                    elapsed_secs: 0.0,
                });
                started.insert(request.ship);
                ConsumableOutcomeType::Started
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

/// Run every ship's effect for this frame's game time. See the module docs.
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
    mut q_ship: Query<(Entity, &mut ShipConsumableEffect, Option<&Children>)>,
    mut q_health: Query<(&mut Health, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (ship, mut running, children) in &mut q_ship {
        let consumables = consumables
            .as_deref()
            .expect("consumable effects: an effect runs only after the catalog loads");
        let design = consumables.design(&running.item);
        let duration = design.effect.duration_secs();
        let step = dt.min(duration - running.elapsed_secs).max(0.0);
        running.elapsed_secs += step;
        let ended = match design.effect {
            ConsumableType::Repair { health, .. } => {
                let mut sections = live_sections(children, &q_health.as_readonly());
                let pools: Vec<(f32, f32)> = sections.iter().map(|(_, c, m)| (*c, *m)).collect();
                for (index, current) in plan_repair_split(health * step / duration, &pools) {
                    sections[index].1 = current;
                    if let Ok((mut section, _)) = q_health.get_mut(sections[index].0) {
                        section.current = current;
                    }
                }
                let repaired = sections.iter().all(|(_, current, max)| current >= max);
                if repaired {
                    debug!(
                        "consumable effects: {} on {ship:?} has nothing left to repair",
                        running.item
                    );
                }
                repaired
            }
        };
        if ended || running.elapsed_secs >= duration {
            commands.entity(ship).remove::<ShipConsumableEffect>();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use nova_gameplay::test_support::test_items;

    use super::*;

    const KIT: &str = "RepairKit";

    /// A ship with `kits` repair kits and one section at `health` of 100,
    /// its clock stepping one game second a frame after the warm-up frame.
    fn effect_app(kits: u32, health: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(60));
        app.insert_resource(GameConsumables::new([ConsumableDesign {
            item: ItemDesignId::from(KIT),
            effect: ConsumableType::Repair {
                health: 80.0,
                duration_secs: 10.0,
            },
        }]));
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
                ShipInventory::new(&items, 1_000_000, [(ItemDesignId::from(KIT), kits)]),
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
            item: ItemDesignId::from(KIT),
        });
    }

    fn held(app: &App, ship: Entity) -> u32 {
        app.world()
            .get::<ShipInventory>(ship)
            .unwrap()
            .count(&ItemDesignId::from(KIT))
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
        assert_eq!(held(&app, ship), 1);
        for _ in 0..12 {
            app.update();
        }
        assert!((health(&app) - 90.0).abs() < 1e-3, "{}", health(&app));
        assert!(app.world().get::<ShipConsumableEffect>(ship).is_none());
    }

    /// Two uses in one frame: the first starts, the second sees it run and
    /// keeps its kit, though the effect lands by command.
    #[test]
    fn a_second_kit_in_the_same_frame_is_refused_and_kept() {
        let (mut app, ship, _) = effect_app(2, 10.0);
        use_kit(&mut app, ship);
        use_kit(&mut app, ship);
        app.update();
        assert_eq!(
            outcomes(&mut app),
            [
                ConsumableOutcomeType::Started,
                ConsumableOutcomeType::Refused(ConsumableRefusalType::Running),
            ]
        );
        assert_eq!(held(&app, ship), 1);
    }

    /// A kit ends once nothing is damaged; the rest of its health is lost.
    #[test]
    fn a_repair_kit_ends_early_when_nothing_is_damaged() {
        let (mut app, ship, section) = effect_app(1, 90.0);
        use_kit(&mut app, ship);
        app.update();
        app.update();
        assert_eq!(app.world().get::<Health>(section).unwrap().current, 100.0);
        assert!(app.world().get::<ShipConsumableEffect>(ship).is_none());
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
        assert_eq!(held(&app, ship), 1);
    }
}
