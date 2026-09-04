//! first_shift_rcs: First Shift's four-mark RCS briefing on the shared map.
//!
//! Cutter starts physically stopped at the work mark. The complete A-B-C-D
//! box appears under the same wide Cutter-relative briefing pose as mainline;
//! the camera and controls return together, then the player can fly the route
//! with Shift plus mouse and scroll.
//!
//! ```text
//! cargo run --example first_shift_rcs --features debug
//! ```

#[path = "shared/first_shift_stage.rs"]
mod stage;

use bevy::prelude::*;
use nova_protocol::prelude::*;

const CUTTER_POS: Meters3 = Meters3::new(-500.0, 80.0, 900.0);
const ROUTE_CENTRE: Meters3 = Meters3::new(-350.0, 190.0, 900.0);
const BRIEFING_OFFSET: Meters3 = Meters3::new(450.0, 300.0, 650.0);
const ROUTE: [(&str, &str, Meters3); 4] = [
    ("trim_a", "TRIM A", Meters3::new(-200.0, 80.0, 900.0)),
    ("trim_b", "TRIM B", Meters3::new(-200.0, 300.0, 900.0)),
    ("trim_c", "TRIM C", Meters3::new(-500.0, 300.0, 900.0)),
    ("trim_d", "TRIM D", CUTTER_POS),
];

fn main() -> bevy::app::AppExit {
    AppBuilder::new()
        .with_game_plugins(rcs_plugin)
        .build()
        .run()
}

fn rcs_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load);
}

fn load(mut commands: Commands, assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(scenario(&assets)));
}

fn ship(
    id: &str,
    name: &str,
    position: Meters3,
    prototype: &str,
    controller: SpaceshipController,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            hull: ShipSource::Prototype(prototype.to_string()),
            ..default()
        }),
    }
}

fn number(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

fn variable(name: &str) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(VariableFactorNode::new_name(
        name,
    )))
}

fn set_step(value: f64) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: "trim_step".to_string(),
        expression: number(value),
    })
}

fn mark(id: &str, label: &str, position: Meters3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: label.to_string(),
            radius: Meters(20.0),
            color: Color::srgb(0.3, 0.9, 1.0),
            area_radius: Some(Meters(100.0)),
            lock_signature: None,
        }),
    }
}

fn route_step(index: usize) -> ScenarioEventConfig {
    let (id, _, _) = ROUTE[index];
    let next = ROUTE.get(index + 1);
    let mut actions = vec![
        set_step((index + 2) as f64),
        EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig::new(id)),
    ];
    if let Some((next_id, next_label, _)) = next {
        actions.push(EventActionConfig::ObjectiveMarkerAttach(
            ObjectiveMarkerAttachActionConfig::new(next_id, next_label),
        ));
    } else {
        actions.push(EventActionConfig::ObjectiveComplete(
            ObjectiveCompleteActionConfig {
                id: "trim_route".to_string(),
            },
        ));
    }

    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnEnter,
        once: true,
        filters: vec![
            EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(id.to_string()),
                other_id: Some("cutter".to_string()),
                ..default()
            }),
            EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_equals(
                    variable("trim_step"),
                    number((index + 1) as f64),
                ),
            )),
        ],
        actions,
    }
}

fn scenario(assets: &GameAssets) -> ScenarioConfig {
    let mut objects = stage::belt(&assets.asteroid_texture);
    objects.extend(
        ThreePointRig::around("first_shift", Meters3::new(0.0, 0.0, -2_000.0), 25.0).objects(),
    );
    objects.extend([
        ship(
            "cutter",
            "Cutter",
            CUTTER_POS,
            "block_cutter",
            SpaceshipController::Player(PlayerControllerConfig {
                speed_cap: Some(MetersPerSecond(150.0)),
                ..default()
            }),
        ),
        ship(
            "carrier",
            "ICV Meridian",
            stage::CARRIER_POS,
            "block_carrier",
            SpaceshipController::None,
        ),
    ]);
    objects.extend(
        ROUTE
            .into_iter()
            .map(|(id, label, position)| mark(id, label, position)),
    );

    let briefing = SequenceActionConfig {
        key: "rcs_briefing".to_string(),
        steps: vec![SequenceStepConfig {
            after: Some(6.0),
            actions: vec![
                EventActionConfig::ReleaseCamera(ReleaseCameraActionConfig),
                EventActionConfig::ResumePlayerControl(ResumePlayerControlActionConfig),
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "trim_route",
                    "Fly TRIM A, B, C, then D with short RCS taps.",
                )),
                EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
                    "trim_a", "TRIM A",
                )),
                set_step(1.0),
            ],
            ..default()
        }],
    };

    let mut events = vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .chain([
                    set_step(0.0),
                    EventActionConfig::SuspendPlayerControl(SuspendPlayerControlActionConfig),
                    EventActionConfig::SetCameraAnchor(SetCameraAnchorActionConfig {
                        anchor: "cutter".to_string(),
                        offset: BRIEFING_OFFSET,
                        frame: CameraOffsetFrame::World,
                        look_at: CameraLookAtConfig::Point(ROUTE_CENTRE),
                    }),
                    EventActionConfig::StoryMessage(StoryMessageActionConfig {
                        speaker: "Copilot".to_string(),
                        text: "Hold Shift and move the mouse: short taps, no turning. The velocity ball goes violet while RCS has the ship.".to_string(),
                        dwell: None,
                        icon: None,
                    }),
                    EventActionConfig::Sequence(briefing),
                ])
                .collect(),
        }];
    events.extend((0..ROUTE.len()).map(route_step));

    ScenarioConfig {
        description: "First Shift's four-mark RCS briefing and route".to_string(),
        events,
        ..ScenarioConfig::new(
            "first_shift_rcs".to_string(),
            "First Shift RCS".to_string(),
            assets.cubemap.clone().into(),
        )
    }
}
