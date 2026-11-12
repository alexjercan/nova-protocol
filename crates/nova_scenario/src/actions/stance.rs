//! The scenario write to a civilization's stance toward the player.
//!
//! A civilization has no authored id, so the action names a body and the
//! civilization is the one that body belongs to. The seed's draw lives in
//! the open world crate, above this one: the action resolves the body and
//! its civilization, refuses what it cannot resolve, and raises a
//! [`SetStanceRequest`] the open world applies.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Set the stance one civilization holds toward the player.
///
/// `body` names a live body of the civilization: a living generated station
/// or an intact generated ship. No author can write such an id, so `body` is
/// `$id` or `$other_id`, bound from the firing event: `$id` under `OnDocked`
/// is the station docked with. The civilization takes
/// `stance` at once, with nothing pending, and `OnStanceChanged` fires. A
/// civilization already on `stance` is left as it is and fires nothing.
///
/// Refused with `unknown_body` when the event carries no such field or no
/// live body wears the id, and with `no_civilization` when the body belongs
/// to none: an authored station, a rock, a dead hull. A refused action moves
/// no stance.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetStanceActionConfig {
    /// `$id` or `$other_id`: the firing event's subject or other party, a
    /// body of the civilization to move.
    #[reflect(@Names::Object)]
    pub body: String,
    /// The stance the civilization takes.
    pub stance: StanceType,
}

impl EventAction<NovaEventWorld> for SetStanceActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        let reference = self.body.clone();
        let bound = bind_object_reference(&self.body, info);
        let stance = self.stance;
        debug!("SetStance: '{reference}' ({bound:?}) -> {stance:?}");

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| set_stance(world, &reference, bound, stance));
        });
    }
}

/// Ask the open world to move the civilization of the body `bound` names, or
/// refuse and change nothing. Both refusals are found here, before the
/// request: the observer that applies it has nothing left to refuse.
fn set_stance(world: &mut World, reference: &str, bound: Option<String>, stance: StanceType) {
    const ACTION: &str = "SetStance";

    let Some(id) = bound else {
        refuse_action(
            world,
            ACTION,
            reference,
            ActionRefusalType::UnknownBody,
            "the firing event carries no such field",
        );
        return;
    };
    let body = match id.parse::<GeneratedBodyId>() {
        Ok(generated) => generated_entity(world, &generated),
        Err(_) => scoped_entity(world, &id),
    };
    let Some(body) = body else {
        refuse_action(
            world,
            ACTION,
            &id,
            ActionRefusalType::UnknownBody,
            "no live body wears the id",
        );
        return;
    };
    let Some(civilization) = world
        .get::<TraderMarket>(body)
        .map(|market| market.civilization)
    else {
        refuse_action(
            world,
            ACTION,
            &id,
            ActionRefusalType::NoCivilization,
            "the body carries no market, so it belongs to no civilization",
        );
        return;
    };
    world.trigger(SetStanceRequest {
        civilization,
        stance,
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// Every stance request and every refusal raised, in order.
    #[derive(Resource, Default)]
    struct Raised {
        requests: Vec<SetStanceRequest>,
        refusals: Vec<(String, String)>,
    }

    fn civilization() -> CivilizationId {
        CivilizationId {
            world_seed: 7,
            node: [1, 0, -2],
        }
    }

    fn station_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "cluster_0_0_0_station", 0)
    }

    /// A world that records requests and refusals, with one generated
    /// station of [`civilization`], one generated rock and one authored
    /// station `dock`.
    fn world_with_bodies() -> World {
        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        world.init_resource::<Raised>();
        world.add_observer(
            |request: On<SetStanceRequest>, mut raised: ResMut<Raised>| {
                raised.requests.push(*request);
            },
        );
        world.add_observer(|event: On<GameEvent>, mut raised: ResMut<Raised>| {
            if event.name() != OnActionRefusedEvent::name() {
                return;
            }
            let text = |field: &str| match event.info().field(field) {
                Some(GameEventField::Text(text)) => text.to_string(),
                other => panic!("the refusal carries the text field '{field}': {other:?}"),
            };
            raised.refusals.push((
                text(ENTITY_ID_COMPONENT_NAME),
                text(REFUSAL_REASON_FIELD_NAME),
            ));
        });
        world.spawn((
            ScenarioScopedMarker,
            station_id(),
            TraderMarket {
                cluster: ClusterType::RockOnly,
                civilization: civilization(),
                role_preference: [1.0; 4],
                target: BTreeMap::new(),
                profile: None,
            },
        ));
        world.spawn((ScenarioScopedMarker, rock_id()));
        world.spawn((
            ScenarioScopedMarker,
            ScenarioAddressableMarker,
            EntityId::new("dock".to_string()),
        ));
        world
    }

    fn rock_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "background_rock", 3)
    }

    /// Run `SetStance` on `body` under an `OnDocked` of the player with the
    /// hull `docked_with`.
    fn run(world: &mut World, body: &str, docked_with: &str, stance: StanceType) {
        let info = GameEventInfo::from_data(DockingEventInfo {
            id: docked_with.to_string(),
            other_id: "player".to_string(),
            ..default()
        });
        let action = SetStanceActionConfig {
            body: body.to_string(),
            stance,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &info);
        NovaEventWorld::state_to_world_system(world);
        world.flush();
    }

    #[test]
    fn a_bound_body_with_a_market_asks_for_its_civilization_s_stance() {
        let mut world = world_with_bodies();

        run(
            &mut world,
            "$id",
            &station_id().to_string(),
            StanceType::Enemy,
        );

        let raised = world.resource::<Raised>();
        assert_eq!(
            raised.requests,
            [SetStanceRequest {
                civilization: civilization(),
                stance: StanceType::Enemy,
            }]
        );
        assert!(raised.refusals.is_empty(), "{:?}", raised.refusals);
    }

    #[test]
    fn a_body_nobody_wears_or_with_no_civilization_is_refused_and_asks_nothing() {
        let retired = GeneratedBodyId::new(SectorCoord::new(4, 0, 0), "cluster_0_0_0_station", 0);
        for (docked_with, reason) in [
            (retired.to_string(), "unknown_body"),
            ("ghost".to_string(), "unknown_body"),
            (rock_id().to_string(), "no_civilization"),
            ("dock".to_string(), "no_civilization"),
        ] {
            let mut world = world_with_bodies();

            run(&mut world, "$id", &docked_with, StanceType::Ally);

            let raised = world.resource::<Raised>();
            assert!(
                raised.requests.is_empty(),
                "{reason}: {:?}",
                raised.requests
            );
            assert_eq!(raised.refusals, [(docked_with, reason.to_string())]);
        }
    }

    /// A binding the event does not carry is refused under the reference as
    /// written: there is no id to name.
    #[test]
    fn a_binding_the_event_does_not_carry_is_refused_as_an_unknown_body() {
        let mut world = world_with_bodies();
        let action = SetStanceActionConfig {
            body: "$other_id".to_string(),
            stance: StanceType::Ally,
        };

        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        world.flush();

        let raised = world.resource::<Raised>();
        assert!(raised.requests.is_empty());
        assert_eq!(
            raised.refusals,
            [("$other_id".to_string(), "unknown_body".to_string())]
        );
    }
}
