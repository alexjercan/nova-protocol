//! Scenario writes to a ship's purse, and the refusal an action raises when
//! it cannot run whole.
//!
//! A write here is all or nothing. An action that cannot run changes
//! nothing, logs why, and raises `OnActionRefused` so a handler can answer
//! it.

use std::fmt::Display;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Why a scenario action refused to run.
///
/// The text of a variant is the `reason` field of `OnActionRefused`. Mods
/// compare against it, so a shipped variant keeps its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum ActionRefusalType {
    /// No live scoped ship or station carries the id.
    UnknownShip,
    /// A charge is larger than the balance.
    InsufficientCredits,
    /// A grant would pass the `u32` limit of a balance.
    CreditOverflow,
}

impl ActionRefusalType {
    /// The reason text an `OnActionRefused` payload carries.
    pub fn as_str(self) -> &'static str {
        match self {
            ActionRefusalType::UnknownShip => "unknown_ship",
            ActionRefusalType::InsufficientCredits => "insufficient_credits",
            ActionRefusalType::CreditOverflow => "credit_overflow",
        }
    }
}

/// Refuse `action` on the target `id`: log why and raise `OnActionRefused`.
///
/// `action` is the action's RON name. The caller has changed nothing and
/// returns after the call. `detail` is for the log only: the numbers that
/// made the action impossible.
pub fn refuse_action(
    world: &mut World,
    action: &str,
    id: &str,
    refusal: ActionRefusalType,
    detail: impl Display,
) {
    let reason = refusal.as_str();
    error!("{action}: refused on '{id}' ({reason}): {detail}");
    world.trigger(GameEvent::new(
        OnActionRefusedEvent::name(),
        ActionRefusedEventInfo {
            action: action.to_string(),
            id: id.to_string(),
            reason: reason.to_string(),
        }
        .into(),
    ));
}

/// Grant credits to one ship or station, or charge it.
///
/// A positive `amount` grants and a negative one charges. The balance stays
/// in `0..=u32::MAX`: a charge past zero is refused with
/// `insufficient_credits`, a grant past the limit with `credit_overflow`,
/// and an id no live ship carries with `unknown_ship`. A refused action
/// moves no credits. A write that goes through raises `OnCreditsChanged`
/// with cause `Scenario`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrantCreditsActionConfig {
    /// The `EntityId` of the scoped ship or station.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// Credits to add. Negative charges. Zero is a lint error.
    pub amount: i32,
}

impl EventAction<NovaEventWorld> for GrantCreditsActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        let amount = self.amount;
        debug!("GrantCredits: '{id}' {amount:+} cr");

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| grant_credits(world, &id, amount));
        });
    }
}

/// Move `amount` credits onto the ship `id`, or refuse and change nothing.
fn grant_credits(world: &mut World, id: &str, amount: i32) {
    const ACTION: &str = "GrantCredits";

    let Some(ship) = scoped_ship(world, id) else {
        refuse_action(
            world,
            ACTION,
            id,
            ActionRefusalType::UnknownShip,
            "no live scoped ship carries the id",
        );
        return;
    };
    let mut purse = world.query::<(&mut ShipCredits, &mut ShipInventoryReports)>();
    let (mut credits, mut reports) = purse
        .get_mut(world, ship)
        .expect("a ship root requires ShipCredits and ShipInventoryReports");
    let held = credits.0;
    let wanted = i64::from(held) + i64::from(amount);
    let Ok(current_cr) = u32::try_from(wanted) else {
        let refusal = if wanted < 0 {
            ActionRefusalType::InsufficientCredits
        } else {
            ActionRefusalType::CreditOverflow
        };
        refuse_action(
            world,
            ACTION,
            id,
            refusal,
            format_args!("it holds {held} cr and the action adds {amount} cr"),
        );
        return;
    };
    reports.change_credits(&mut credits, current_cr, CreditChangeCauseType::Scenario);
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::loader::trackers::track_ship_inventory_reports;

    /// Every scenario event raised, as its name and its payload fields.
    #[derive(Resource, Default)]
    struct Fired(Vec<(&'static str, Vec<(String, String)>)>);

    impl Fired {
        fn named(&self, name: &str) -> Vec<Vec<(String, String)>> {
            self.0
                .iter()
                .filter(|(fired, _)| *fired == name)
                .map(|(_, fields)| fields.clone())
                .collect()
        }
    }

    fn fields(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    /// A world that records every scenario event, with one scoped ship
    /// `freighter` holding `credits`.
    fn world_with_a_freighter(credits: u32) -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        world.init_resource::<Fired>();
        world.add_observer(|event: On<GameEvent>, mut fired: ResMut<Fired>| {
            let info = event.info();
            let fields = info
                .field_names()
                .into_iter()
                .map(|name| {
                    let value = match info.field(name).expect("a listed field reads") {
                        GameEventField::Text(text) => text.to_string(),
                        GameEventField::Number(number) => number.to_string(),
                        GameEventField::Boolean(flag) => flag.to_string(),
                    };
                    (name.to_string(), value)
                })
                .collect();
            fired.0.push((event.name(), fields));
        });
        let ship = world
            .spawn((
                ScenarioScopedMarker,
                ScenarioAddressableMarker,
                SpaceshipRootMarker,
                EntityId::new("freighter".to_string()),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                ShipCredits(credits),
            ))
            .id();
        (world, ship)
    }

    /// Run `action` the way a handler does, then drain the hold reports the
    /// way the fixed tick does.
    fn run(world: &mut World, action: impl EventAction<NovaEventWorld>) {
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(world);
        world
            .run_system_once(track_ship_inventory_reports)
            .expect("the tracker runs");
        world.flush();
    }

    fn grant(ship: &str, amount: i32) -> GrantCreditsActionConfig {
        GrantCreditsActionConfig {
            ship: ship.to_string(),
            amount,
        }
    }

    #[test]
    fn a_grant_and_a_charge_move_the_balance_and_name_the_scenario_as_cause() {
        let (mut world, ship) = world_with_a_freighter(100);

        run(&mut world, grant("freighter", 250));
        run(&mut world, grant("freighter", -350));

        assert_eq!(world.get::<ShipCredits>(ship).map(|held| held.0), Some(0));
        let changed = |previous: &str, current: &str, delta: &str| {
            fields(&[
                (CREDIT_CAUSE_FIELD_NAME, "Scenario"),
                (CURRENT_CREDITS_FIELD_NAME, current),
                (DELTA_CREDITS_FIELD_NAME, delta),
                (ENTITY_ID_COMPONENT_NAME, "freighter"),
                (PREVIOUS_CREDITS_FIELD_NAME, previous),
                (ENTITY_TYPE_NAME_COMPONENT_NAME, SPACESHIP_TYPE_NAME),
            ])
        };
        let fired = world.resource::<Fired>();
        assert_eq!(
            fired.named(OnCreditsChangedEvent::name()),
            [changed("100", "350", "250"), changed("350", "0", "-350")]
        );
        assert!(fired.named(OnActionRefusedEvent::name()).is_empty());
    }

    #[test]
    fn a_refused_grant_moves_no_credits_and_fires_one_refusal() {
        for (held, ship, amount, reason) in [
            (100, "freighter", -101, "insufficient_credits"),
            (u32::MAX - 1, "freighter", 2, "credit_overflow"),
            (100, "ghost", 5, "unknown_ship"),
        ] {
            let (mut world, freighter) = world_with_a_freighter(held);

            run(&mut world, grant(ship, amount));

            assert_eq!(
                world.get::<ShipCredits>(freighter).map(|credits| credits.0),
                Some(held),
                "{reason}"
            );
            let fired = world.resource::<Fired>();
            assert_eq!(
                fired.named(OnActionRefusedEvent::name()),
                [fields(&[
                    (REFUSED_ACTION_FIELD_NAME, "GrantCredits"),
                    (ENTITY_ID_COMPONENT_NAME, ship),
                    (REFUSAL_REASON_FIELD_NAME, reason),
                ])]
            );
            assert!(
                fired.named(OnCreditsChangedEvent::name()).is_empty(),
                "{reason}"
            );
        }
    }
}
