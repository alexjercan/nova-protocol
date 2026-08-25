//! The editor's document: a tree of NODES, and the context you are inside.
//!
//! Everything the editor edits is a node entity. A [`ScenarioNode`] holds
//! [`ShipNode`]s, a ship holds [`SectionNode`]s, and each node carries its own
//! config as a component - so "the ship being built" is not a resource anywhere,
//! it is a subtree. Two ships cost nothing but two subtrees.
//!
//! MODEL AND VIEW ARE SEPARATE ENTITIES. A node is data and persists across
//! [`ExampleStates`]; its [`NodeView`] child carries the mesh, the collider and
//! the picking, and is `DespawnOnExit(Editor)`. That split is what lets Play
//! leave the editor and come back to the same document, and it is why a node can
//! never carry a collider into the flown scenario: the thing with the collider
//! is the thing that gets despawned.
//!
//! Picking therefore lands on a VIEW. Every pointer path maps it back with
//! [`node_of_view`].

use bevy::prelude::*;
use bevy_enhanced_input::prelude::Binding;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::{
    config::SelectedNode,
    gallery::EditorCamera,
    preview::{insert_preview_section, PreviewRole},
    ExampleStates,
};

/// How far apart two ship nodes sit on the stage. Wide enough that the biggest
/// hull anyone builds by hand does not reach its neighbour.
const SHIP_NODE_SPACING: f32 = 24.0;

/// Marker on every node of the document tree, at every depth.
#[derive(Component, Debug)]
pub(crate) struct EditorNode;

/// A node's stable id.
///
/// Minted once when the node is created and never re-derived, because it is what
/// a saved file and its input mapping are keyed by. Ids used to BE the live
/// entity id stringified, which made a saved file impossible: entity ids do not
/// survive a process restart, and they did not even survive leaving the editor
/// and coming back.
///
/// Unique within the PARENT, which is exactly the scope both consumers need -
/// `input_mapping` keys are per hull, and `BaseScenarioObjectConfig::id` is per
/// scenario.
#[derive(Component, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodeId(pub(crate) String);

/// The next ordinal this node hands out to a child id.
///
/// Monotonic, never reused: deleting `thruster_2` and placing another thruster
/// mints `thruster_3`. Reuse would let a stale reference to the old id silently
/// attach to the new section.
#[derive(Component, Debug, Default)]
pub(crate) struct NextChildOrdinal(pub(crate) u32);

/// The outermost node: the scenario being edited. One per document, and the
/// context the editor opens in.
#[derive(Component, Debug)]
pub(crate) struct ScenarioNode;

/// Who drives a ship once the scenario runs.
///
/// A property of the SHIP, not of the edit context: "which ship do I fly" is
/// answered by the document, so entering a ship to work on it cannot change
/// which one Play hands to the player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ShipDriver {
    /// The ship the player flies on Play.
    #[default]
    Player,
    /// Driven by the AI once the scenario runs.
    Ai,
}

/// A ship being built: everything about it that is not one of its sections.
#[derive(Component, Debug, Clone)]
pub(crate) struct ShipNode {
    /// Whether the ship wears its derived cladding - shown live in the build
    /// view (see [`crate::skin`]) and carried through to the flown ship, so what
    /// the builder sees is what they fly.
    pub(crate) skin: bool,
    /// The style id the cladding wears, or `None` for the first style the
    /// content merge loaded.
    pub(crate) style: Option<String>,
    /// Who drives it once the scenario runs.
    pub(crate) driver: ShipDriver,
}

impl Default for ShipNode {
    fn default() -> Self {
        Self {
            // Off, as the build state's `skin` was: a new ship starts bare.
            skin: false,
            style: None,
            driver: ShipDriver::Player,
        }
    }
}

/// One section placed on a ship. Its POSE is the node's `Transform` rather than
/// a field, because the node is an entity and that is where an entity keeps its
/// pose.
#[derive(Component, Debug, Clone)]
pub(crate) struct SectionNode {
    /// Where the section's config comes from: inline, or a catalog prototype by
    /// id. The editor writes `Inline` today; `Prototype` resolves through
    /// [`SectionNode::resolve`] rather than being dropped, which is what the old
    /// rebuild path did to it.
    pub(crate) source: SectionSource,
    /// Data-only deltas applied at spawn. Carried so a lifted section keeps
    /// them; the editor authors none yet.
    pub(crate) modifications: Vec<SectionModification>,
    /// The inputs this section fires on. Document data rather than a component
    /// on the view: the view is render-only, and a binding that lived out there
    /// would be a second copy to keep in step across a despawn.
    pub(crate) binds: Vec<Binding>,
}

impl SectionNode {
    /// The section's config, or `None` when a mod overlay dropped the prototype
    /// it names.
    pub(crate) fn resolve<'a>(
        &'a self,
        sections: Option<&'a GameSections>,
    ) -> Option<&'a SectionConfig> {
        match &self.source {
            SectionSource::Inline(config) => Some(config),
            SectionSource::Prototype(id) => sections?.get_section(id),
        }
    }

    /// The catalog id this section was built from - what the pipette arms and
    /// what a minted id is named after.
    pub(crate) fn prototype(&self) -> &str {
        match &self.source {
            SectionSource::Inline(config) => &config.base.id,
            SectionSource::Prototype(id) => id,
        }
    }

    /// Whether this kind of section takes an input binding at all. Hull and
    /// controller sections do not.
    pub(crate) fn bindable(&self, sections: Option<&GameSections>) -> bool {
        self.resolve(sections).is_some_and(|config| {
            !matches!(
                config.kind,
                SectionKind::Hull(_) | SectionKind::Controller(_)
            )
        })
    }
}

/// The render half of a node: mesh, collider and picking. Spawned on entering
/// the editor and despawned on leaving it, so the flown scenario never contains
/// one.
#[derive(Component, Debug)]
pub(crate) struct NodeView;

/// The node the editor is INSIDE, as a path from the scenario node down.
///
/// A path rather than one handle so exiting has somewhere to return to: enter
/// pushes, exit pops, and the root can never be popped. Empty until the document
/// exists.
#[derive(Resource, Debug, Default)]
pub(crate) struct EditContext {
    pub(crate) path: Vec<Entity>,
}

impl EditContext {
    /// The scenario node, or `None` before the document exists.
    pub(crate) fn scenario(&self) -> Option<Entity> {
        self.path.first().copied()
    }

    /// The node the editor is inside.
    pub(crate) fn current(&self) -> Option<Entity> {
        self.path.last().copied()
    }

    /// The ship being edited, or `None` out in the scenario context. Depth is
    /// the test because only a ship can be entered today.
    pub(crate) fn ship(&self) -> Option<Entity> {
        (self.path.len() >= 2).then(|| self.path[1])
    }

    /// Go inside `node`.
    ///
    /// A no-op before the document exists: there is nothing to be inside OF,
    /// and pushing onto an empty path would leave a ship node answering as the
    /// scenario node.
    pub(crate) fn enter(&mut self, node: Entity) {
        if self.path.is_empty() || self.current() == Some(node) {
            return;
        }
        // Entering a sibling is an enter, not a nest: drop back to the scenario
        // first so the path is always a real ancestry.
        self.path.truncate(1);
        self.path.push(node);
    }

    /// Come back out one level. The scenario node is the floor.
    pub(crate) fn exit(&mut self) {
        if self.path.len() > 1 {
            self.path.pop();
        }
    }

    /// Straight back to the scenario node, however deep the path is - what
    /// clicking the tree's root row means.
    pub(crate) fn to_root(&mut self) {
        self.path.truncate(1);
    }
}

/// Every section node on `ship`, in id order.
///
/// SORTED rather than in query order: an archetype walk hands the same ship over
/// in whatever order its entities were spawned, and both the lowering (whose
/// output is a file) and the solver (whose refusals name section indices) need
/// one answer per ship. This is the sort `sync_editor_skin` used to do by
/// position, for the same reason and against a worse key.
pub(crate) fn sections_of<'a>(
    ship: Entity,
    nodes: &'a SectionNodes,
) -> Vec<(Entity, &'a NodeId, &'a SectionNode, &'a Transform)> {
    let mut found: Vec<_> = nodes
        .iter()
        .filter(|(_, child_of, ..)| child_of.parent() == ship)
        .map(|(entity, _, id, section, transform)| (entity, id, section, transform))
        .collect();
    found.sort_unstable_by(|a, b| a.1.cmp(b.1));
    found
}

/// Read-only access to every section node and its pose.
pub(crate) type SectionNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static NodeId,
        &'static SectionNode,
        &'static Transform,
    ),
>;

/// Read-only access to every ship node.
pub(crate) type ShipNodes<'w, 's> =
    Query<'w, 's, (Entity, &'static ChildOf, &'static NodeId, &'static ShipNode)>;

/// One node the current edit context contains.
pub(crate) struct ContextNode<'a> {
    pub(crate) entity: Entity,
    pub(crate) id: &'a NodeId,
}

/// Everything the edit context CONTAINS, in id order.
///
/// At the scenario node that is its ships; inside a ship it is that ship's
/// sections. This lives here rather than in the rail that draws it because it
/// is a question about the document, and the probe has to answer it too - a
/// driven run that read the answer off the UI tree would be testing the rail
/// rather than the tree.
pub(crate) fn context_nodes<'a>(
    context: &EditContext,
    q_ships: &'a ShipNodes,
    nodes: &'a SectionNodes,
) -> Vec<ContextNode<'a>> {
    let Some(scenario) = context.scenario() else {
        return Vec::new();
    };
    let Some(ship) = context.ship() else {
        let mut ships: Vec<_> = q_ships
            .iter()
            .filter(|(_, owner, ..)| owner.parent() == scenario)
            .map(|(entity, _, id, _)| ContextNode { entity, id })
            .collect();
        ships.sort_unstable_by(|a, b| a.id.cmp(b.id));
        return ships;
    };
    sections_of(ship, nodes)
        .into_iter()
        .map(|(entity, id, ..)| ContextNode { entity, id })
        .collect()
}

/// The id of the node the editor is inside, or `None` at the scenario node.
pub(crate) fn inside_id<'a>(context: &EditContext, q_ships: &'a ShipNodes) -> Option<&'a NodeId> {
    let ship = context.ship()?;
    q_ships.get(ship).ok().map(|(_, _, id, _)| id)
}

/// FOCUS the entered ship: inside one, every other ship leaves the stage;
/// at the scenario node everything is back.
///
/// Two writes because hiding is two facts. `Visibility` takes the meshes off
/// screen, and the views' `Pickable` follows it because the picking ray does
/// not care what renders - an invisible collider would still eat clicks, and
/// the founding click ("nothing under the pointer") most of all.
pub(crate) fn sync_ship_focus(
    mut commands: Commands,
    context: Res<EditContext>,
    mut ships: Query<(Entity, &mut Visibility), With<ShipNode>>,
    q_sections: Query<&ChildOf, With<SectionNode>>,
    views: Query<(Entity, &ChildOf, Has<Pickable>), With<NodeView>>,
) {
    let entered = context.ship();
    let hidden = |ship: Entity| entered.is_some_and(|edited| edited != ship);
    for (ship, mut visibility) in &mut ships {
        let wanted = if hidden(ship) {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    // Views carry no `Pickable` of their own, so its presence IS the "this
    // ship is off the stage" mark and removal restores the default.
    for (view, owner, ignored) in &views {
        let Ok(section_owner) = q_sections.get(owner.parent()) else {
            continue;
        };
        match (hidden(section_owner.parent()), ignored) {
            (true, false) => {
                commands.entity(view).insert(Pickable::IGNORE);
            }
            (false, true) => {
                commands.entity(view).remove::<Pickable>();
            }
            _ => {}
        }
    }
}

/// Snap the stage camera to the edit context: entering a ship frames that
/// ship, and the scenario node frames everything the document holds.
///
/// The free-fly rig rewrites the camera `Transform` every frame from private
/// state, so a bare pose write is gone by the next frame. The controller is
/// therefore removed and re-inserted around the write - the same move the
/// gallery's camera parking makes - and its setup re-reads the transform this
/// system just set. Keyed on the PATH (compared, not change-ticked: `exit()`
/// at the floor marks the resource changed without moving it) plus a fresh
/// camera, because a second editor visit spawns a new camera at the stock pose
/// while the context still points wherever it pointed.
pub(crate) fn sync_camera_focus(
    mut commands: Commands,
    context: Res<EditContext>,
    fresh: Query<(), Added<EditorCamera>>,
    ships: Query<&Transform, (With<ShipNode>, Without<EditorCamera>)>,
    camera: Option<Single<(Entity, &mut Transform), (With<EditorCamera>, Without<ShipNode>)>>,
    mut shown: Local<Option<Vec<Entity>>>,
) {
    if fresh.is_empty() && shown.as_ref() == Some(&context.path) {
        return;
    }
    // The move is recorded only once it lands: an entered ship or the camera
    // can be a frame away (both are spawned through commands), and a change
    // swallowed while they settle would leave the camera wherever it was.
    let Some(camera) = camera else {
        return;
    };
    let (entity, mut transform) = camera.into_inner();
    let pose = match context.ship() {
        Some(ship) => {
            let Ok(target) = ships.get(ship) else {
                return;
            };
            frame_stage(target.translation, 0.0)
        }
        None => {
            let positions: Vec<Vec3> = ships.iter().map(|ship| ship.translation).collect();
            let centre = positions.iter().sum::<Vec3>() / positions.len().max(1) as f32;
            let spread = positions
                .iter()
                .map(|position| position.distance(centre))
                .fold(0.0, f32::max);
            frame_stage(centre, spread)
        }
    };
    *shown = Some(context.path.clone());
    *transform = pose;
    commands
        .entity(entity)
        .remove::<WASDCameraController>()
        .insert(WASDCameraController);
}

/// The stock editor view over `target`: the spawn pose slid onto it, backed
/// off by `spread` so a stage of several ships fits the frame. Zero spread IS
/// the spawn pose, which is what keeps the driven walks' fixed screen points
/// meaning what they measured.
fn frame_stage(target: Vec3, spread: f32) -> Transform {
    Transform::from_translation(target + Vec3::new(0.0, 5.0 + 0.75 * spread, 10.0 + 1.5 * spread))
        .looking_at(target, Vec3::Y)
}

/// Delete the document on the way out of the session (Back to Main Menu).
///
/// The nodes survive every INNER state change on purpose - Play leaves and
/// returns to the same ships - but a session ends at the main menu, and a
/// document that outlived it read as the editor never resetting (owner,
/// 2026-08-25). Despawning the scenario node takes the whole tree with it;
/// the context and selection are cleared so nothing points into the rubble.
pub(crate) fn teardown_document(
    mut commands: Commands,
    mut context: ResMut<EditContext>,
    mut selected: ResMut<SelectedNode>,
    roots: Query<Entity, With<ScenarioNode>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !context.path.is_empty() {
        context.path.clear();
    }
    if selected.0.is_some() {
        selected.0 = None;
    }
}

/// The node a picking hit belongs to: hits land on views, and the document is
/// the parent.
pub(crate) fn node_of_view(
    view: Entity,
    views: &Query<&ChildOf, With<NodeView>>,
) -> Option<Entity> {
    views.get(view).ok().map(ChildOf::parent)
}

/// Mint the next id under `parent`, named after `prototype`.
fn mint_id(ordinals: &mut Query<&mut NextChildOrdinal>, parent: Entity, prototype: &str) -> NodeId {
    let ordinal = match ordinals.get_mut(parent) {
        Ok(mut next) => {
            next.0 += 1;
            next.0
        }
        // A parent with no counter cannot mint a unique id, and a duplicate id
        // is a silently broken save. Loud and non-fatal.
        Err(_) => {
            error!("editor: node {parent} has no id counter - falling back to ordinal 0");
            0
        }
    };
    NodeId(format!("{prototype}_{ordinal}"))
}

/// Create the document if there is none: one empty scenario node, entered.
///
/// Lazily rather than at plugin build so a headless rig that never opens the
/// editor never grows a tree. The document is NOT `DespawnOnExit` of any inner
/// state: Play and F1 round-trip it. It dies with the SESSION instead - Back
/// to Main Menu runs [`teardown_document`] (owner decision, 2026-08-25), so a
/// later Sandbox entry starts on a fresh scenario.
pub(crate) fn ensure_document(mut commands: Commands, mut context: ResMut<EditContext>) {
    if context.scenario().is_some() {
        return;
    }
    let scenario = commands
        .spawn((
            EditorNode,
            ScenarioNode,
            NodeId("scenario".to_string()),
            NextChildOrdinal::default(),
            Name::new("Scenario Node"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();
    context.path = vec![scenario];
}

/// Add a BLANK ship to the document and go inside it.
///
/// Additive: a second "Add Ship" is one more subtree standing beside the first
/// rather than a reset. Ships are spaced along +X so two of them are two things
/// on the stage rather than one pile. Blank on purpose - which part a ship
/// starts from is the builder's first decision, and the empty ship's founding
/// click (see `crate::placement::found_empty_ship`) is where they make it.
pub(crate) fn spawn_ship_node(
    commands: &mut Commands,
    ordinals: &mut Query<&mut NextChildOrdinal>,
    context: &mut EditContext,
    ships: usize,
    driver: ShipDriver,
) -> Option<Entity> {
    let scenario = context.scenario()?;
    let id = mint_id(ordinals, scenario, "ship");
    let ship = commands
        .spawn((
            EditorNode,
            ShipNode {
                driver,
                ..default()
            },
            Name::new(format!("Ship Node {}", id.0)),
            id,
            NextChildOrdinal::default(),
            Transform::from_xyz(ships as f32 * SHIP_NODE_SPACING, 0.0, 0.0),
            Visibility::Visible,
            ChildOf(scenario),
        ))
        .id();
    context.enter(ship);
    Some(ship)
}

/// Add a section to `ship` at `transform`, and give it a view to be seen by.
pub(crate) fn spawn_section_node(
    commands: &mut Commands,
    ordinals: &mut Query<&mut NextChildOrdinal>,
    ship: Entity,
    config: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) -> Entity {
    let id = mint_id(ordinals, ship, &config.base.id);
    insert_section_node(commands, ship, id, config, transform, binds)
}

/// The spawn itself, for callers that already know the id.
fn insert_section_node(
    commands: &mut Commands,
    ship: Entity,
    id: NodeId,
    config: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) -> Entity {
    let node = commands
        .spawn((
            EditorNode,
            SectionNode {
                source: SectionSource::Inline(config.clone()),
                modifications: vec![],
                binds,
            },
            Name::new(format!("Section Node {}", id.0)),
            id,
            transform,
            // INHERITED, never `Visible`: an explicit `Visible` overrides a
            // hidden ancestor, so every section of a ship `sync_ship_focus`
            // hid would stay on screen while its picking was off.
            Visibility::Inherited,
            ChildOf(ship),
        ))
        .id();
    spawn_node_view(commands, node, config);
    node
}

/// Give `node` the mesh, collider and picking that make it visible and clickable.
pub(crate) fn spawn_node_view(commands: &mut Commands, node: Entity, config: &SectionConfig) {
    commands.entity(node).with_children(|parent| {
        let mut view = parent.spawn((
            DespawnOnExit(ExampleStates::Editor),
            NodeView,
            Transform::default(),
            Visibility::Inherited,
        ));
        insert_preview_section(&mut view, config, PreviewRole::Section);
    });
}

/// Rebuild every node's view on entering the editor.
///
/// The views are `DespawnOnExit(Editor)` and the nodes are not, so a second
/// visit finds a document with no bodies and gives it new ones. This replaces
/// the old re-keying rebuild, which had to re-derive both maps onto whatever
/// entities it happened to spawn - and which DROPPED any section it could not
/// inline.
pub(crate) fn rebuild_node_views(
    mut commands: Commands,
    sections: Option<Res<GameSections>>,
    nodes: Query<(Entity, &NodeId, &SectionNode)>,
) {
    for (node, id, section) in &nodes {
        let Some(config) = section.resolve(sections.as_deref()) else {
            warn!(
                "editor: section '{}' names prototype '{}', which is not in the catalog - \
                 it stays in the document but has nothing to show",
                id.0,
                section.prototype()
            );
            continue;
        };
        spawn_node_view(&mut commands, node, config);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn hull(id: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A ship node with one section node on it, at the given source.
    fn document(world: &mut World, source: SectionSource) -> (Entity, Entity) {
        let ship = world
            .spawn((
                EditorNode,
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                NextChildOrdinal(1),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        let section = world
            .spawn((
                EditorNode,
                SectionNode {
                    source,
                    modifications: vec![],
                    binds: vec![],
                },
                NodeId("hull_1".to_string()),
                Transform::from_xyz(1.0, 2.0, 3.0),
                Visibility::Inherited,
                ChildOf(ship),
            ))
            .id();
        (ship, section)
    }

    fn views(world: &mut World) -> usize {
        world
            .query_filtered::<(), With<NodeView>>()
            .iter(world)
            .count()
    }

    /// The point of the whole split: leaving the editor takes the BODIES and
    /// leaves the document, so a second visit rebuilds the same ship under the
    /// same ids. The old path re-keyed two maps onto whatever entities it
    /// happened to spawn, which is why a saved file was impossible.
    #[test]
    fn a_document_outlives_its_views_and_keeps_its_ids() {
        let mut world = World::new();
        let (_, section) = document(&mut world, SectionSource::Inline(hull("hull")));

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");
        assert_eq!(views(&mut world), 1, "the section got a body");

        // What DespawnOnExit(Editor) does on the way out to the scenario.
        let view: Vec<Entity> = world
            .query_filtered::<Entity, With<NodeView>>()
            .iter(&world)
            .collect();
        for entity in view {
            world.despawn(entity);
        }
        assert_eq!(views(&mut world), 0);

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");

        assert_eq!(views(&mut world), 1, "and a new one on the way back in");
        assert_eq!(
            world.get::<NodeId>(section),
            Some(&NodeId("hull_1".to_string())),
            "the id is the same id, not a re-derived one"
        );
        assert_eq!(
            world.get::<Transform>(section).map(|t| t.translation),
            Some(Vec3::new(1.0, 2.0, 3.0)),
            "and so is the pose"
        );
    }

    /// A section that names a catalog prototype is REBUILT rather than dropped.
    /// The old rebuild inlined-or-discarded, which is the direct blocker on ever
    /// loading a file that references the catalog.
    #[test]
    fn a_prototype_sourced_section_is_rebuilt_from_the_catalog() {
        let mut world = World::new();
        world.insert_resource(GameSections(vec![hull("hull")]));
        document(&mut world, SectionSource::Prototype("hull".to_string()));

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");

        assert_eq!(views(&mut world), 1, "the prototype resolved to a body");
    }

    /// A prototype a mod overlay dropped leaves the section in the document with
    /// nothing to show, rather than deleting the player's work.
    #[test]
    fn a_missing_prototype_keeps_the_section_and_shows_nothing() {
        let mut world = World::new();
        world.insert_resource(GameSections(vec![]));
        let (_, section) = document(&mut world, SectionSource::Prototype("gone".to_string()));

        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");

        assert_eq!(views(&mut world), 0);
        assert!(
            world.get::<SectionNode>(section).is_some(),
            "the section is still in the document"
        );
    }

    /// Entering a ship takes every other ship off the stage - the meshes AND
    /// the picking, because an invisible collider would still eat clicks.
    #[test]
    fn entering_a_ship_hides_and_unpicks_its_siblings() {
        let mut world = World::new();
        let (first, _) = document(&mut world, SectionSource::Inline(hull("hull")));
        let (second, second_section) = document(&mut world, SectionSource::Inline(hull("hull")));
        world
            .run_system_once(rebuild_node_views)
            .expect("the view rebuild runs");
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, first],
        });

        let view_of = |world: &mut World, section: Entity| {
            world
                .query_filtered::<(Entity, &ChildOf), With<NodeView>>()
                .iter(world)
                .find(|(_, owner)| owner.parent() == section)
                .map(|(view, _)| view)
                .expect("the section grew a view")
        };

        world
            .run_system_once(sync_ship_focus)
            .expect("the focus sync runs");
        assert_eq!(world.get::<Visibility>(first), Some(&Visibility::Visible));
        assert_eq!(
            world.get::<Visibility>(second),
            Some(&Visibility::Hidden),
            "the sibling leaves the stage"
        );
        let hidden_view = view_of(&mut world, second_section);
        assert_eq!(
            world.get::<Pickable>(hidden_view),
            Some(&Pickable::IGNORE),
            "and its colliders leave the pointer's way"
        );

        world.resource_mut::<EditContext>().exit();
        world
            .run_system_once(sync_ship_focus)
            .expect("the focus sync runs");
        assert_eq!(world.get::<Visibility>(second), Some(&Visibility::Visible));
        assert_eq!(
            world.get::<Pickable>(hidden_view),
            None,
            "back at the scenario node the default picking is restored"
        );
    }

    /// A placed section INHERITS its ship's visibility. `Visibility::Visible`
    /// overrides a hidden ancestor in bevy, so sections spawned with it stayed
    /// on screen after `sync_ship_focus` hid their ship - the focus tests all
    /// passed by reading the ship's own component while the live stage showed
    /// every ship at once.
    #[test]
    fn a_placed_section_inherits_its_ships_visibility() {
        let mut world = World::new();
        let ship = world
            .spawn((ShipNode::default(), NextChildOrdinal::default()))
            .id();
        let section = world
            .run_system_once(move |mut commands: Commands| {
                insert_section_node(
                    &mut commands,
                    ship,
                    NodeId("hull_1".to_string()),
                    &hull("hull"),
                    Transform::default(),
                    vec![],
                )
            })
            .expect("the section spawner runs");

        assert_eq!(
            world.get::<Visibility>(section),
            Some(&Visibility::Inherited),
            "an explicit Visible would override the hidden ship above it"
        );
    }

    /// Back to Main Menu ends the session, and the session owns the document:
    /// the whole tree goes, and nothing keeps pointing into it.
    #[test]
    fn leaving_the_session_deletes_the_document() {
        let mut world = World::new();
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        world
            .run_system_once(ensure_document)
            .expect("the document is created");
        let scenario = world
            .resource::<EditContext>()
            .scenario()
            .expect("the document exists");
        let ship = world
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                NextChildOrdinal::default(),
                ChildOf(scenario),
            ))
            .id();
        world.resource_mut::<EditContext>().enter(ship);
        world.resource_mut::<SelectedNode>().0 = Some(ship);

        world
            .run_system_once(teardown_document)
            .expect("the teardown runs");

        assert_eq!(
            world.query::<&EditorNode>().iter(&world).count(),
            0,
            "the scenario node takes its whole subtree with it"
        );
        assert!(world.resource::<EditContext>().path.is_empty());
        assert_eq!(world.resource::<SelectedNode>().0, None);

        // And the next entry starts a fresh document rather than resuming.
        world
            .run_system_once(ensure_document)
            .expect("the document check runs");
        assert_ne!(
            world.resource::<EditContext>().scenario(),
            Some(scenario),
            "a later Sandbox entry founds a new scenario"
        );
    }

    /// Enter/exit is a path, so exiting has somewhere to return to - and the
    /// scenario node is the floor, because there is nothing outside the
    /// document to back out into.
    #[test]
    fn the_context_enters_exits_and_never_pops_the_scenario_node() {
        let scenario = Entity::from_raw_u32(1).expect("a test entity id");
        let first = Entity::from_raw_u32(2).expect("a test entity id");
        let second = Entity::from_raw_u32(3).expect("a test entity id");

        let mut context = EditContext {
            path: vec![scenario],
        };
        assert_eq!(context.scenario(), Some(scenario));
        assert_eq!(context.ship(), None, "the document opens outside any ship");

        context.enter(first);
        assert_eq!(context.ship(), Some(first));
        assert_eq!(context.current(), Some(first));

        // Entering a SIBLING is an enter, not a nest: a ship is never inside
        // another ship.
        context.enter(second);
        assert_eq!(context.path, vec![scenario, second]);
        assert_eq!(context.ship(), Some(second));

        context.exit();
        assert_eq!(context.current(), Some(scenario));
        assert_eq!(context.ship(), None);

        context.exit();
        assert_eq!(
            context.path,
            vec![scenario],
            "the scenario node is the floor"
        );
    }
}
