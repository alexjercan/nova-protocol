use bevy::prelude::*;
use bevy_common_systems::modding::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        ConditionalFilterConfig, EntityFilterConfig, EventFilterConfig, ExpressionFilterConfig,
    };
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventFilterConfig {
    Entity(EntityFilterConfig),
    Conditional(ConditionalFilterConfig),
    Expression(ExpressionFilterConfig),
}

impl EventFilter<NovaEventWorld> for EventFilterConfig {
    fn filter(&self, world: &NovaEventWorld, info: &GameEventInfo) -> bool {
        match self {
            EventFilterConfig::Entity(config) => config.filter(world, info),
            EventFilterConfig::Conditional(config) => config.filter(world, info),
            EventFilterConfig::Expression(config) => config.filter(world, info),
        }
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityFilterConfig {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub id: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub type_name: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub other_id: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub other_type_name: Option<String>,
}

impl EventFilter<NovaEventWorld> for EntityFilterConfig {
    fn filter(&self, _: &NovaEventWorld, info: &GameEventInfo) -> bool {
        let Some(data) = &info.data else {
            return false;
        };

        let mut result = true;
        match &self.id {
            Some(id) => {
                let Some(id_value) = data.get(ENTITY_ID_COMPONENT_NAME).and_then(|v| v.as_str())
                else {
                    return false;
                };

                result &= id_value == id
            }
            None => result &= true,
        }

        match &self.type_name {
            Some(type_name) => {
                let Some(type_name_value) = data
                    .get(ENTITY_TYPE_NAME_COMPONENT_NAME)
                    .and_then(|v| v.as_str())
                else {
                    return false;
                };

                result &= type_name_value == type_name
            }
            None => result &= true,
        }

        match &self.other_id {
            Some(other_id) => {
                let Some(other_id_value) = data
                    .get(ENTITY_OTHER_ID_COMPONENT_NAME)
                    .and_then(|v| v.as_str())
                else {
                    return false;
                };

                result &= other_id_value == other_id
            }
            None => result &= true,
        }

        match &self.other_type_name {
            Some(other_type_name) => {
                let Some(other_type_name_value) = data
                    .get(ENTITY_OTHER_TYPE_NAME_COMPONENT_NAME)
                    .and_then(|v| v.as_str())
                else {
                    return false;
                };

                result &= other_type_name_value == other_type_name
            }
            None => result &= true,
        }

        result
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConditionalFilterConfig {
    Not(Box<EventFilterConfig>),
    Or(Box<EventFilterConfig>, Box<EventFilterConfig>),
    And(Box<EventFilterConfig>, Box<EventFilterConfig>),
}

impl ConditionalFilterConfig {
    pub fn not(inner: EventFilterConfig) -> Self {
        ConditionalFilterConfig::Not(Box::new(inner))
    }

    pub fn or(left: EventFilterConfig, right: EventFilterConfig) -> Self {
        ConditionalFilterConfig::Or(Box::new(left), Box::new(right))
    }

    pub fn and(left: EventFilterConfig, right: EventFilterConfig) -> Self {
        ConditionalFilterConfig::And(Box::new(left), Box::new(right))
    }
}

impl EventFilter<NovaEventWorld> for ConditionalFilterConfig {
    fn filter(&self, world: &NovaEventWorld, info: &GameEventInfo) -> bool {
        match self {
            ConditionalFilterConfig::Not(inner) => !inner.filter(world, info),
            ConditionalFilterConfig::Or(left, right) => {
                left.filter(world, info) || right.filter(world, info)
            }
            ConditionalFilterConfig::And(left, right) => {
                left.filter(world, info) && right.filter(world, info)
            }
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpressionFilterConfig(pub VariableConditionNode);

impl EventFilter<NovaEventWorld> for ExpressionFilterConfig {
    fn filter(&self, world: &NovaEventWorld, _: &GameEventInfo) -> bool {
        match self.0.evaluate(world) {
            Ok(result) => result,
            Err(e) => {
                error!(
                    "VariableFilterConfig: failed to evaluate condition: {:?}",
                    e
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy_common_systems::prelude::{
        CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
    };

    use super::*;
    use crate::prelude::*;

    // These pin the filter/action semantics every data-driven scenario leans
    // on - the exact patterns shipped content composes (gate guards, kill
    // counters) - with SYNTHETIC handlers through the real dispatch
    // (task 20260716-155830: per-mod behavior tests were removed, so the
    // machinery must be guaranteed here, not re-proven per mod). The bridges
    // that FIRE these events have their own pins (area.rs OnEnter,
    // asteroid.rs OnDestroyed); this module owns what happens after delivery.

    /// Headless dispatch rig: event plumbing only, no physics.
    fn dispatch_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app
    }

    /// Fire the scenario `OnDestroyed` for `id` - the same info the integrity
    /// bridge emits - and pump the handlers through.
    fn destroy(app: &mut App, id: &str) {
        let info = OnDestroyedEventInfo {
            id: id.to_string(),
            type_name: "asteroid".to_string(),
        };
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands.fire::<OnDestroyedEvent>(info.clone());
            })
            .expect("fire OnDestroyed");
        app.update();
        app.update();
    }

    /// An `OnDestroyed` handler from configs, built exactly as the loader
    /// builds one (`EventHandler::from` + `add_filter`/`add_action`).
    fn spawn_handler(
        app: &mut App,
        filters: Vec<EventFilterConfig>,
        actions: Vec<EventActionConfig>,
    ) {
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnDestroyed);
        for filter in filters {
            handler.add_filter(filter);
        }
        for action in actions {
            handler.add_action(action);
        }
        app.world_mut().spawn(handler);
    }

    fn id_filter(id: &str) -> EventFilterConfig {
        EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(id.to_string()),
            ..Default::default()
        })
    }

    /// `<name> == <n>` as the AST the RON authors (`Expression((Equal(..)))`).
    fn var_equals(name: &str, n: f64) -> EventFilterConfig {
        EventFilterConfig::Expression(ExpressionFilterConfig(VariableConditionNode::new_equals(
            var_expr(name),
            num_expr(n),
        )))
    }

    fn var_expr(name: &str) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_name(name),
        ))
    }

    fn num_expr(n: f64) -> VariableExpressionNode {
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(n)),
        ))
    }

    /// `key = <expression>` action.
    fn set_action(key: &str, expression: VariableExpressionNode) -> EventActionConfig {
        EventActionConfig::VariableSet(VariableSetActionConfig {
            key: key.to_string(),
            expression,
        })
    }

    fn set_number(app: &mut App, key: &str, n: f64) {
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable(key.to_string(), VariableLiteral::Number(n));
    }

    fn number(app: &App, key: &str) -> Option<f64> {
        match app.world().resource::<NovaEventWorld>().get_variable(key) {
            Some(VariableLiteral::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// A handler's filters must ALL pass (the gate-guard pattern: entity id
    /// AND `gate == N`); either side alone must keep the action inert. The
    /// unfiltered probe proves every dispatch was delivered.
    #[test]
    fn entity_and_expression_filters_must_both_pass() {
        let mut app = dispatch_app();
        set_number(&mut app, "gate", 0.0);
        set_number(&mut app, "advanced", 0.0);
        set_number(&mut app, "probe", 0.0);

        spawn_handler(
            &mut app,
            vec![id_filter("gate_1"), var_equals("gate", 1.0)],
            vec![set_action("advanced", num_expr(1.0))],
        );
        // Delivery probe: counts EVERY OnDestroyed, no filters.
        spawn_handler(
            &mut app,
            vec![],
            vec![set_action(
                "probe",
                VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name("probe")),
                    num_expr(1.0),
                ),
            )],
        );

        // Right id, wrong state.
        destroy(&mut app, "gate_1");
        assert_eq!(number(&app, "probe"), Some(1.0), "delivery guard");
        assert_eq!(
            number(&app, "advanced"),
            Some(0.0),
            "the expression guard must hold the action while gate != 1"
        );

        // Wrong id, right state.
        set_number(&mut app, "gate", 1.0);
        destroy(&mut app, "other_object");
        assert_eq!(number(&app, "probe"), Some(2.0), "delivery guard");
        assert_eq!(
            number(&app, "advanced"),
            Some(0.0),
            "the entity filter must hold the action for a non-matching id"
        );

        // Right id, right state.
        destroy(&mut app, "gate_1");
        assert_eq!(number(&app, "probe"), Some(3.0), "delivery guard");
        assert_eq!(
            number(&app, "advanced"),
            Some(1.0),
            "with every filter passing the action must run"
        );
    }

    /// An expression over an UNDEFINED variable fails closed (Err -> false):
    /// the reason shipped content must seed its variables in OnStart, and a
    /// missing seed soft-locks instead of misfiring.
    #[test]
    fn expression_filter_fails_closed_on_an_undefined_variable() {
        let mut app = dispatch_app();
        set_number(&mut app, "fired", 0.0);
        set_number(&mut app, "probe", 0.0);

        // `gate` is never defined.
        spawn_handler(
            &mut app,
            vec![id_filter("gate_1"), var_equals("gate", 1.0)],
            vec![set_action("fired", num_expr(1.0))],
        );
        spawn_handler(&mut app, vec![], vec![set_action("probe", num_expr(1.0))]);

        destroy(&mut app, "gate_1");
        assert_eq!(
            number(&app, "probe"),
            Some(1.0),
            "delivery guard: the event reached the handlers"
        );
        assert_eq!(
            number(&app, "fired"),
            Some(0.0),
            "an undefined variable in the guard must fail closed, not run the action"
        );
    }

    /// `n = n + 1` accumulates across events (the kill-counter pattern).
    #[test]
    fn variable_set_increment_accumulates_across_events() {
        let mut app = dispatch_app();
        set_number(&mut app, "destroyed", 0.0);

        spawn_handler(
            &mut app,
            vec![id_filter("target")],
            vec![set_action(
                "destroyed",
                VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name("destroyed")),
                    num_expr(1.0),
                ),
            )],
        );

        destroy(&mut app, "target");
        assert_eq!(number(&app, "destroyed"), Some(1.0));
        destroy(&mut app, "target");
        assert_eq!(
            number(&app, "destroyed"),
            Some(2.0),
            "each matching event must re-evaluate the expression against the current value"
        );
    }
}
