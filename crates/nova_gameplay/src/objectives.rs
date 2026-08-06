//! Mission objectives: the [`GameObjectives`] list the game reasons about, the
//! [`objectives_panel`] that renders it, and the conveyance tags
//! ([`ObjectiveMarkerTarget`], [`ItemHighlight`]) the scenario side attaches to
//! world entities.
//!
//! Nova owns this because objectives are mission state, not a widget: the
//! scenario loader writes [`GameObjectives`], and the HUD reads it from three
//! places (the objective stack, the NOVA OS monitor and the objective-change
//! feedback). The conveyance tags live here - not in nova_scenario with the
//! actions that insert them - because the HUD chip modules
//! (`hud/objective_markers.rs`, `hud/item_highlights.rs`) query them and the
//! crate dependency runs nova_scenario -> nova_gameplay, the same split as
//! `BeaconMarker`.
//!
//! ```
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn setup(mut commands: Commands, mut objectives: ResMut<GameObjectives>) {
//! commands.spawn(objectives_panel(ObjectivesPanelConfig::default()));
//! objectives.objectives = vec![
//!     Objective::new("reach_exit", "Reach the exit"),
//!     Objective::new("collect_keys", "Collect 3 keys"),
//! ];
//! # }
//! ```

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::objectives::prelude::*` re-exports
/// the public API of this module.
pub mod prelude {
    pub use super::{
        objectives_panel, GameObjectives, ItemHighlight, Objective, ObjectiveMarker,
        ObjectiveMarkerTarget, ObjectivesPanelConfig, ObjectivesPanelMarker, ObjectivesPlugin,
        ObjectivesPluginSystems,
    };
}

/// A single objective line: an opaque `id` for game code to address, and the `message` shown.
#[derive(Clone, Debug)]
pub struct Objective {
    /// Opaque identifier for game code (not shown).
    pub id: String,
    /// The text shown for this objective.
    pub message: String,
}

impl Objective {
    /// Convenience constructor from string slices.
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

/// The current objectives. Replace the `objectives` vec to change what the panel shows.
#[derive(Resource, Clone, Debug, Default)]
pub struct GameObjectives {
    /// The objectives, rendered top to bottom.
    pub objectives: Vec<Objective>,
}

/// Marker for the objectives panel root node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectivesPanelMarker;

/// Marker for a single objective text line under the panel.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveMarker;

/// The `id` of the objective a line was rendered from.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct ObjectiveId(pub String);

/// Configuration for [`objectives_panel`]. Empty for now; a hook for future layout options.
#[derive(Clone, Debug, Default)]
pub struct ObjectivesPanelConfig {}

/// Bundle for the objectives panel root. Spawn one; [`ObjectivesPlugin`] fills it with a text
/// line per [`GameObjectives`] entry. Override the [`Node`] afterwards to reposition it.
pub fn objectives_panel(config: ObjectivesPanelConfig) -> impl Bundle {
    debug!("objectives_panel: config {:?}", config);

    (
        Name::new("ObjectivesPanel"),
        ObjectivesPanelMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(50.0),
            right: Val::Px(5.0),
            ..default()
        },
    )
}

/// System set for the objectives rebuild, so games can order around it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObjectivesPluginSystems {
    /// Rebuilds the panel's text lines when [`GameObjectives`] changes.
    Sync,
}

/// Plugin that renders [`GameObjectives`] under the [`objectives_panel`] node.
#[derive(Default)]
pub struct ObjectivesPlugin;

impl Plugin for ObjectivesPlugin {
    fn build(&self, app: &mut App) {
        debug!("ObjectivesPlugin: build");
        app.init_resource::<GameObjectives>();

        app.add_systems(
            Update,
            rebuild_lines
                .run_if(resource_changed::<GameObjectives>)
                .in_set(ObjectivesPluginSystems::Sync),
        );
    }
}

fn rebuild_lines(
    mut commands: Commands,
    q_panel: Single<(Entity, Option<&Children>), With<ObjectivesPanelMarker>>,
    objectives: Res<GameObjectives>,
) {
    trace!("rebuild_lines: objectives {:?}", *objectives);
    let (entity, children) = q_panel.into_inner();

    let new_children = objectives
        .objectives
        .iter()
        .map(|objective| {
            commands
                .spawn((
                    Name::new(format!("Objective {}", objective.id)),
                    ObjectiveMarker,
                    ObjectiveId(objective.id.clone()),
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

/// Marks an entity as the current objective: attaching this grows a gold
/// HUD marker chip (label + distance, edge-clamped as a direction cue) via
/// the objective-markers observer; removing it (or despawning the entity)
/// takes the chip down. Named `ObjectiveMarkerTarget`, not
/// [`ObjectiveMarker`] - that name belongs to the objectives panel's text
/// lines, which live in this module now that nova owns them. Renaming it is
/// a public API change across nova_scenario and the HUD, deferred as its own
/// task.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ObjectiveMarkerTarget {
    /// The short name the marker chip shows next to the distance.
    pub label: String,
}

impl ObjectiveMarkerTarget {
    /// Construct from a string slice.
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

/// Marks an interactable/collectible prop the player is meant to notice:
/// the item-highlights observer grows a bracket chip over it that tracks
/// the prop's on-screen size (hidden off-screen - pointing at off-screen
/// items is the objective marker's job). Spawned intrinsically by pickup
/// objects (salvage crates); a pickup that does not advertise itself is a
/// bug, not a policy.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ItemHighlight {
    /// The prop's VISIBLE bounding-sphere radius (world units) - what the
    /// bracket sizes to. Authored, not collider-derived: a pickup's only
    /// collider is its oversized sensor sphere, which would balloon the
    /// bracket to the trigger volume.
    pub world_radius: f32,
}

impl ItemHighlight {
    /// Construct from the visible bounding radius.
    pub fn new(world_radius: f32) -> Self {
        Self { world_radius }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Including when the list shrinks: the panel rebuilds down, despawning the
    /// lines it no longer needs.
    #[test]
    fn the_panel_renders_one_line_per_objective() {
        let mut app = App::new();
        app.add_plugins(ObjectivesPlugin);
        let panel = app.world_mut().spawn(objectives_panel(default())).id();

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("a", "Alpha"), Objective::new("b", "Bravo")];
        app.update();

        let children = app.world().get::<Children>(panel).expect("panel children");
        assert_eq!(children.len(), 2);

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("a", "Alpha")];
        app.update();
        assert_eq!(app.world().get::<Children>(panel).unwrap().len(), 1);
    }
}
