use bevy::prelude::*;

pub mod prelude {
    pub use super::{
        objectives_hud, GameObjectivesHud, ObjectiveActionConfig, ObjectiveHudMarker,
        ObjectiveRootHudConfig, ObjectiveRootHudMarker, ObjectivesHudPlugin,
    };
}

#[derive(Clone, Debug)]
pub struct ObjectiveActionConfig {
    pub id: String,
    pub message: String,
}

impl ObjectiveActionConfig {
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct GameObjectivesHud {
    pub objectives: Vec<ObjectiveActionConfig>,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveRootHudMarker;

#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveHudMarker;

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct ObjectiveHudId(pub String);

#[derive(Clone, Debug, Default)]
pub struct ObjectiveRootHudConfig {}

pub fn objectives_hud(config: ObjectiveRootHudConfig) -> impl Bundle {
    debug!("objective_hud: config {:?}", config);

    (
        Name::new("ObjectiveHUD"),
        ObjectiveRootHudMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            right: px(5),
            ..default()
        },
    )
}

#[derive(Default)]
pub struct ObjectivesHudPlugin;

impl Plugin for ObjectivesHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameObjectivesHud>();

        app.add_systems(
            Update,
            update_text_hud
                .run_if(resource_changed::<GameObjectivesHud>)
                .in_set(super::NovaHudSystems),
        );
    }
}

fn update_text_hud(
    mut commands: Commands,
    q_hud: Single<(Entity, Option<&Children>), With<ObjectiveRootHudMarker>>,
    game_objectives: Res<GameObjectivesHud>,
) {
    trace!("update_text_hud: game_objectives {:?}", *game_objectives);
    let (entity, children) = q_hud.into_inner();

    let new_children = game_objectives
        .objectives
        .iter()
        .map(|objective| {
            commands
                .spawn((
                    Name::new(format!("Objective {}", objective.id)),
                    ObjectiveHudMarker,
                    ObjectiveHudId(objective.id.clone()),
                    Text::new(objective.message.clone()),
                    TextShadow::default(),
                    TextLayout::justify(Justify::Center),
                ))
                .id()
        })
        .collect::<Vec<_>>();

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    commands.entity(entity).replace_children(&new_children);
}
