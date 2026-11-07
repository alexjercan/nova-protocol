//! Undo and redo: the document as it stood after each finished edit.
//!
//! The ECS node tree is the only live document, so history keeps snapshots of
//! it. A snapshot is taken at COMMIT time, once per user gesture, never per
//! frame. A verb calls [`commit_edit`] after its last document write. A commit
//! that finds the document unchanged adds nothing, so a verb can commit
//! without first asking whether it did anything.
//!
//! Restoring is a DIFF, not a reload. A node that exists on both sides keeps
//! its `Entity`, which is what keeps [`SelectedNode`], [`EditContext`], the
//! open inspector and its floating windows, [`Expanded`] and [`AutoLayout`]
//! pointing at the right thing. `document_content` and `fill_document` would
//! rebuild every entity and lose all of them.
//!
//! Three things never become steps: `reflow_auto_ships` (see
//! [`NodeParts::key`]), the view entities, and [`NextChildOrdinal`], which only
//! rises so an id is not minted twice across an undo.
//!
//! The stack is the PROJECT's. A snapshot also holds the [`ProjectData`] it stood
//! beside, so a step can change the mod as well as the nodes, and restoring one
//! reopens the asset that was open then through the same diff. The dirty mark
//! therefore means the mod.

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};

use bevy::{ecs::system::RunSystemOnce, prelude::*, ui_widgets::Activate};

use crate::{
    asset::{session::sync_project_sections, AssetNode, AssetPartType, SocketNode},
    clipboard::id_stem,
    config::{EditorSays, SelectedNode},
    event::{
        ActionNode, EventNode, Expanded, ExpressionNode, FilterNode, GateNode, ScriptNode, StepNode,
    },
    node::{
        default_allegiance, minted_name, rebuild_node_views, AutoLayout, EditContext, EditorNode,
        NextChildOrdinal, NodeId, NodeView, ObjectNode, ScenarioNode, SectionNode, ShipDriver,
        ShipNode,
    },
    problems::Problems,
    project::{
        verbs::{declare_used, note, refuse},
        ModProject, ProjectData,
    },
    ui::canvas::{close_block_popup, BlockPopup, PopupLeaveType},
};

/// How many undo steps the editor keeps. The oldest step goes first.
///
/// Steps share every node they did not change, so depth costs what the edits
/// touched rather than the document's size. 128 outlasts a working session's
/// memory of what it did without letting a long session grow for ever.
pub(crate) const HISTORY_DEPTH: usize = 128;

/// A node's place in the document: the ids from the root down.
///
/// `NodeId` is unique per parent only, so one id alone names nothing. A clash
/// the builder has not fixed yet gets `#1`, `#2` on the later siblings, because
/// a snapshot that folded the two into one key would lose a node.
type NodePath = Vec<String>;

/// Declares every document component once: the record, how it is read, and how
/// it is written back. A component missing here is one undo silently loses.
macro_rules! document_parts {
    ($($field:ident: $component:ty),* $(,)?) => {
        /// Every document component of one node. Absent means the node does
        /// not carry it: a script node has no pose, a section has no id
        /// counter of its own.
        #[derive(Debug, Clone, Default)]
        struct NodeParts {
            $($field: Option<$component>,)*
        }

        impl NodeParts {
            /// Copy the document components off `node`.
            fn of(node: EntityRef) -> Self {
                Self {
                    $($field: node.get::<$component>().cloned(),)*
                }
            }

            /// Write what differs from `live`, and only that. Touching an
            /// unchanged component would wake everything that watches it.
            fn write(&self, live: &Self, node: &mut EntityWorldMut) {
                $(
                    if format!("{:?}", self.$field) != format!("{:?}", live.$field) {
                        match &self.$field {
                            Some(part) => {
                                node.insert(part.clone());
                            }
                            None => {
                                node.remove::<$component>();
                            }
                        }
                    }
                )*
            }
        }
    };
}

document_parts! {
    id: NodeId,
    name: Name,
    transform: Transform,
    visibility: Visibility,
    auto_layout: AutoLayout,
    scenario: ScenarioNode,
    ship: ShipNode,
    section: SectionNode,
    object: ObjectNode,
    script: ScriptNode,
    event: EventNode,
    filter: FilterNode,
    expression: ExpressionNode,
    action: ActionNode,
    step: StepNode,
    gate: GateNode,
    asset: AssetNode,
    asset_part: AssetPartType,
    socket: SocketNode,
}

impl NodeParts {
    /// The parts as `reflow_auto_ships` will leave them.
    ///
    /// A pose that differs from the remembered row point means the ship has
    /// left the row. The reflow drops the marker a frame later; a commit in
    /// the same frame as the edit must not record the old answer.
    fn settled(mut self) -> Self {
        if let (Some(auto), Some(pose)) = (self.auto_layout, self.transform) {
            if pose.translation != auto.0 {
                self.auto_layout = None;
            }
        }
        self
    }

    /// The value of this node, as text.
    ///
    /// Text because the config types carry `Debug` and not `PartialEq`, and
    /// the enums among them hold most of the game's vocabulary.
    ///
    /// A ship in the automatic row leaves its X out. The layout owns that
    /// number and rewrites it whenever a neighbour grows, so a reflow can
    /// never read as an edit.
    ///
    /// Visibility is left out too: `sync_ship_focus` rewrites it every frame
    /// from where the builder stands, and no file holds it.
    fn key(&self) -> String {
        let mut laid = self.clone();
        laid.visibility = None;
        if laid.auto_layout.is_some() {
            laid.auto_layout = Some(AutoLayout(Vec3::ZERO));
            if let Some(pose) = &mut laid.transform {
                pose.translation.x = 0.0;
            }
        }
        format!("{laid:?}")
    }
}

/// One node as a snapshot holds it.
#[derive(Debug)]
struct NodeRecord {
    parts: NodeParts,
    /// [`NodeParts::key`], kept so a comparison never re-formats a snapshot.
    key: String,
    /// The node when this was recorded. Tried before the path, so a retype
    /// that changed a node's id still comes back on the same entity.
    entity: Entity,
    /// Folded or open. View state and no part of the value, but a deleted
    /// container should come back the way it was.
    expanded: bool,
    /// Whether the node mints ids, and how far it had got.
    ordinal: Option<u32>,
}

/// Every node of a document, by path.
type NodeMap = BTreeMap<NodePath, Arc<NodeRecord>>;

/// The mod a snapshot stood beside.
#[derive(Debug, Clone)]
struct ProjectMark {
    data: Arc<ProjectData>,
    /// The nodes `data` already holds for the open asset. A document equal to
    /// this needs no lowering, which is what keeps an unedited scenario from
    /// being rewritten.
    lowered: Arc<NodeMap>,
}

/// The whole document at one moment.
///
/// Sorted, so a parent always comes before its children. Nodes an edit did not
/// touch are shared with the snapshot before.
#[derive(Debug)]
struct Snapshot {
    /// Which document state this is. The dirty mark compares these.
    revision: u64,
    nodes: NodeMap,
    /// `None` while no project is open, which is a rig driving one document.
    project: Option<ProjectMark>,
}

/// One entry on a stack: what undoing or redoing it restores, and its name.
#[derive(Debug)]
struct Step {
    label: Cow<'static, str>,
    snapshot: Snapshot,
}

/// Which way a step goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Undo,
    Redo,
}

/// The document's undo and redo stacks, and whether it holds unsaved work.
#[derive(Resource, Debug, Default)]
pub(crate) struct EditHistory {
    /// Oldest first. Each entry is the document BEFORE the step of that name.
    undo: VecDeque<Step>,
    /// Next redo last. Each entry is the document AFTER the step of that name.
    redo: Vec<Step>,
    /// The document as of the last commit. `None` before any document exists.
    current: Option<Snapshot>,
    /// The revision that is on disk. A new document counts as saved: it holds
    /// nothing a builder could lose.
    saved: Option<u64>,
    /// The next revision number. Never reused, so a discarded redo branch
    /// cannot read as the saved state again.
    next_revision: u64,
    /// The highest ordinal each path ever reached.
    ///
    /// Outlives the node. Undoing a ship removes its counter, and a redo that
    /// started that ship's counter again would mint ids it already used.
    ordinals: HashMap<NodePath, u32>,
}

impl EditHistory {
    /// Whether a step can be undone.
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether a step can be redone.
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// How many steps can be undone.
    pub(crate) fn depth(&self) -> usize {
        self.undo.len()
    }

    /// The name of the step Undo would take back.
    pub(crate) fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|step| step.label.as_ref())
    }

    /// The name of the step Redo would put back.
    pub(crate) fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|step| step.label.as_ref())
    }

    /// The revision the document stands at, once history has seen it.
    pub(crate) fn revision(&self) -> Option<u64> {
        self.current.as_ref().map(|current| current.revision)
    }

    /// Whether the document differs from what is on disk.
    pub(crate) fn dirty(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|current| Some(current.revision) != self.saved)
    }

    fn mint_revision(&mut self) -> u64 {
        self.next_revision += 1;
        self.next_revision
    }

    /// Read the live document, sharing every record `current` already holds.
    fn read(&mut self, world: &mut World) -> NodeMap {
        let mut nodes = BTreeMap::new();
        for (path, entity) in document_paths(world) {
            let held = self
                .current
                .as_ref()
                .and_then(|current| current.nodes.get(&path));
            let record = read_record(world, entity, held);
            if let Some(ordinal) = record.ordinal {
                let high = self.ordinals.entry(path.clone()).or_default();
                *high = (*high).max(ordinal);
            }
            nodes.insert(path, record);
        }
        nodes
    }

    /// Record a step. `lowered` says the project already holds the nodes as they
    /// stand, which a project verb has made true and a document edit has not.
    fn commit(&mut self, world: &mut World, label: Cow<'static, str>, lowered: bool) {
        let nodes = self.read(world);
        self.declare_what_changed(world, &nodes);
        let data = live_project(world);
        let unchanged = self.current.as_ref().is_some_and(|current| {
            same_document(&current.nodes, &nodes) && same_project(current, data.as_ref())
        });
        if unchanged {
            return;
        }
        let held = self
            .current
            .as_ref()
            .and_then(|current| current.project.as_ref());
        let project = data.map(|data| ProjectMark {
            data,
            lowered: match held {
                Some(held) if !lowered => Arc::clone(&held.lowered),
                _ if lowered => Arc::new(nodes.clone()),
                _ => Arc::default(),
            },
        });
        let revision = self.mint_revision();
        let after = Snapshot {
            revision,
            nodes,
            project,
        };
        let Some(before) = self.current.replace(after) else {
            // Every founding path calls `forget_history`, so a commit here
            // means a new founding path missed it.
            error!("editor: '{label}' committed before any baseline - it cannot be undone");
            return;
        };
        self.undo.push_back(Step {
            label,
            snapshot: before,
        });
        if self.undo.len() > HISTORY_DEPTH {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    /// Fold every step since `revision` into one named `label`. A lone step
    /// keeps its own name. Answers whether `revision` is still on the stack.
    fn fold_since(&mut self, revision: u64, label: Cow<'static, str>) -> bool {
        let Some(first) = self
            .undo
            .iter()
            .position(|step| step.snapshot.revision == revision)
        else {
            return false;
        };
        if self.undo.len() > first + 1 {
            self.undo.truncate(first + 1);
            self.undo[first].label = label;
        }
        true
    }

    /// Hand the commit's new records to the project, which declares any mod they
    /// reach into. Only what changed: a commit costs what the edit touched.
    fn declare_what_changed(&self, world: &mut World, nodes: &NodeMap) {
        let Some(current) = &self.current else {
            return;
        };
        let fresh: Vec<&NodeRecord> = nodes
            .iter()
            .filter(|(path, record)| {
                !current
                    .nodes
                    .get(*path)
                    .is_some_and(|held| held.key == record.key)
            })
            .map(|(_, record)| record.as_ref())
            .collect();
        if fresh.is_empty() {
            return;
        }
        let keys: Vec<&str> = fresh.iter().map(|record| record.key.as_str()).collect();
        let parts: Vec<String> = fresh
            .iter()
            .filter_map(|record| record.parts.section.as_ref())
            .map(|section| section.prototype().to_string())
            .collect();
        declare_used(world, &keys, &parts);
    }

    /// Drop every step and take the document as it stands as the saved start.
    fn forget(&mut self, world: &mut World) {
        self.undo.clear();
        self.redo.clear();
        self.ordinals.clear();
        self.current = None;
        let nodes = self.read(world);
        let revision = self.mint_revision();
        let project = live_project(world).map(|data| ProjectMark {
            data,
            lowered: Arc::new(nodes.clone()),
        });
        self.current = Some(Snapshot {
            revision,
            nodes,
            project,
        });
        self.saved = Some(revision);
    }

    /// Whether the project holds the document as it stands.
    fn is_lowered(&mut self, world: &mut World) -> bool {
        let nodes = self.read(world);
        self.current
            .as_ref()
            .and_then(|current| current.project.as_ref())
            .is_some_and(|mark| same_document(&mark.lowered, &nodes))
    }

    /// Say that the project now holds the document, without making a step.
    fn lowered_now(&mut self, world: &mut World) {
        let nodes = self.read(world);
        let Some(data) = live_project(world) else {
            return;
        };
        if let Some(current) = &mut self.current {
            current.project = Some(ProjectMark {
                data,
                lowered: Arc::new(nodes),
            });
        }
    }

    /// Say that another asset stands in the document now, without making a step.
    ///
    /// The revision stays. What is on disk is no nearer or further for having
    /// looked at another scenario, so the dirty mark does not move.
    fn switched_now(&mut self, world: &mut World) {
        let nodes = self.read(world);
        let Some(data) = live_project(world) else {
            return;
        };
        if let Some(current) = &mut self.current {
            current.project = Some(ProjectMark {
                data,
                lowered: Arc::new(nodes.clone()),
            });
            current.nodes = nodes;
        }
    }

    /// Say that the document as it stands is what is on disk.
    ///
    /// Compared rather than assumed: a save during a held drag writes a pose
    /// no snapshot holds, and undoing to the last commit must not read clean.
    fn saved_now(&mut self, world: &mut World) {
        let nodes = self.read(world);
        self.saved = self
            .current
            .as_ref()
            .filter(|current| same_document(&current.nodes, &nodes))
            .map(|current| current.revision);
    }

    fn step(&mut self, world: &mut World, direction: Direction) {
        let taken = match direction {
            Direction::Undo => self.undo.pop_back(),
            Direction::Redo => self.redo.pop(),
        };
        let Some(Step { label, snapshot }) = taken else {
            return;
        };
        let was_open = open_asset_of(world);
        // The data first: a restored ship's body may be a section only the
        // restored mod holds, and the catalog has to carry it by then.
        if let (Some(mark), Some(mut project)) =
            (&snapshot.project, world.get_resource_mut::<ModProject>())
        {
            project.data = Arc::clone(&mark.data);
        }
        sync_project_sections(world);
        restore(world, &snapshot, &self.ordinals);
        let now_open = open_asset_of(world);
        if now_open != was_open {
            if let Some(name) = now_open {
                note(world, format!("reopened {name} for that step"));
            }
        }
        let Some(left) = self.current.replace(snapshot) else {
            return;
        };
        let opposite = Step {
            label,
            snapshot: left,
        };
        match direction {
            Direction::Undo => self.redo.push(opposite),
            Direction::Redo => self.undo.push_back(opposite),
        }
    }
}

/// The project's data as it stands, if a project is open.
fn live_project(world: &World) -> Option<Arc<ProjectData>> {
    world
        .get_resource::<ModProject>()
        .map(|project| Arc::clone(&project.data))
}

/// The name of the asset open in the project.
fn open_asset_of(world: &World) -> Option<String> {
    world.get_resource::<ModProject>()?.data.open_name()
}

/// Whether `data` is the mod `snapshot` stood beside.
///
/// By identity. Every project verb writes through [`Arc::make_mut`] only once it
/// knows it is changing something.
fn same_project(snapshot: &Snapshot, data: Option<&Arc<ProjectData>>) -> bool {
    match (&snapshot.project, data) {
        (Some(mark), Some(data)) => Arc::ptr_eq(&mark.data, data),
        (None, None) => true,
        _ => false,
    }
}

/// Run `work` against the history, with the world beside it.
///
/// Initialized here because a verb reaches this through `Commands`, so a
/// headless rig driving one verb does not have to know history exists.
fn with_history(world: &mut World, work: impl FnOnce(&mut EditHistory, &mut World)) {
    world.init_resource::<EditHistory>();
    world.resource_scope(|world, mut history: Mut<EditHistory>| work(&mut history, world));
}

/// Record the finished edit as one undo step named `label`.
///
/// This is the whole interface a verb needs. Call it once per user gesture,
/// after the last document write and on the success path only. It is queued,
/// so it sees everything the same `Commands` queued before it, and it adds no
/// step if the document did not change.
pub(crate) fn commit_edit(commands: &mut Commands, label: impl Into<Cow<'static, str>>) {
    let label = label.into();
    commands.queue(move |world: &mut World| commit_edit_now(world, label));
}

/// [`commit_edit`] for a verb that already holds the world.
pub(crate) fn commit_edit_now(world: &mut World, label: impl Into<Cow<'static, str>>) {
    let label = label.into();
    with_history(world, |history, world| history.commit(world, label, false));
}

/// [`commit_edit_now`] for a project verb, which has left the project holding the
/// nodes as they stand.
pub(crate) fn commit_project_now(world: &mut World, label: impl Into<Cow<'static, str>>) {
    let label = label.into();
    with_history(world, |history, world| history.commit(world, label, true));
}

/// Whether the project holds the document as it stands.
pub(crate) fn document_is_lowered(world: &mut World) -> bool {
    let mut lowered = false;
    with_history(world, |history, world| lowered = history.is_lowered(world));
    lowered
}

/// See [`EditHistory::lowered_now`].
pub(crate) fn mark_document_lowered(world: &mut World) {
    with_history(world, EditHistory::lowered_now);
}

/// See [`EditHistory::switched_now`].
pub(crate) fn mark_document_switched(world: &mut World) {
    with_history(world, EditHistory::switched_now);
    if let Some(mut problems) = world.get_resource_mut::<Problems>() {
        problems.document_switched();
    }
}

/// Make the steps since `revision` one undo step named `label`.
pub(crate) fn fold_edits_since(
    world: &mut World,
    revision: u64,
    label: impl Into<Cow<'static, str>>,
) {
    let label = label.into();
    with_history(world, |history, _| {
        history.fold_since(revision, label);
    });
}

/// Take back the steps since `revision`, leaving nothing to redo. A history too
/// deep to reach `revision` keeps the edits, and says so.
pub(crate) fn cancel_edits_since(world: &mut World, revision: u64) {
    let mut lost = false;
    with_history(world, |history, world| {
        if history.fold_since(revision, Cow::Borrowed("")) {
            history.step(world, Direction::Undo);
            history.redo.pop();
        } else {
            lost = history.revision() != Some(revision);
        }
    });
    if lost {
        refuse(
            world,
            "the edit stayed: the undo history no longer holds its start",
        );
    }
}

/// Put the document back as the last commit left it, for a step that failed
/// half way. No step is recorded.
pub(crate) fn revert_to_last_commit(world: &mut World) {
    with_history(world, |history, world| {
        if let Some(current) = &history.current {
            restore(world, current, &history.ordinals);
        }
    });
}

/// [`forget_history`] for a verb that already holds the world.
pub(crate) fn forget_history_now(world: &mut World) {
    with_history(world, EditHistory::forget);
}

/// [`mark_saved`] for a verb that already holds the world.
pub(crate) fn saved_now(world: &mut World) {
    with_history(world, EditHistory::saved_now);
}

/// Start history over from the document as it will stand once `commands` has
/// run. For every path that founds, loads or ends a document: a step from one
/// document can never be restored onto another.
pub(crate) fn forget_history(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        with_history(world, EditHistory::forget);
    });
}

/// Delete `node` and its subtree as one step named after what it was.
pub(crate) fn delete_node(commands: &mut Commands, node: Entity) {
    commands.queue(move |world: &mut World| {
        // Two presses in one frame queue this twice, and the second finds nothing.
        let Ok(found) = world.get_entity(node) else {
            return;
        };
        let label = format!("Delete {}", kind_of(found));
        world.entity_mut(node).despawn();
        commit_edit_now(world, label);
    });
}

/// Delete `nodes` and their subtrees as one step.
pub(crate) fn delete_nodes(commands: &mut Commands, nodes: Vec<Entity>) {
    commands.queue(move |world: &mut World| {
        let found: Vec<Entity> = nodes
            .into_iter()
            .filter(|node| world.get_entity(*node).is_ok())
            .collect();
        let label = format!("Delete {} nodes", found.len());
        for node in found {
            // A node under another selected node went with its parent.
            if let Ok(node) = world.get_entity_mut(node) {
                node.despawn();
            }
        }
        commit_edit_now(world, label);
    });
}

/// A node and everything under it, held apart from the document.
///
/// The clipboard's unit, and built from the same parts a snapshot holds, so a
/// component that joins [`document_parts!`] comes along.
#[derive(Debug, Clone)]
pub(crate) struct NodeCopy {
    parts: NodeParts,
    expanded: bool,
    ordinal: Option<u32>,
    children: Vec<NodeCopy>,
}

impl NodeCopy {
    /// Read `node` and its document children. `None` if it has gone.
    pub(crate) fn of(world: &World, node: Entity) -> Option<Self> {
        let found = world.get_entity(node).ok()?;
        let children = found
            .get::<Children>()
            .into_iter()
            .flatten()
            .filter(|child| world.get::<EditorNode>(**child).is_some())
            .filter_map(|child| Self::of(world, *child))
            .collect();
        Some(Self {
            parts: NodeParts::of(found),
            expanded: found.contains::<Expanded>(),
            ordinal: found.get::<NextChildOrdinal>().map(|next| next.0),
            children,
        })
    }

    /// Spawn this under `parent` with its subtree, and nothing else about it
    /// changed.
    fn spawn_under(&self, world: &mut World, parent: Entity) -> Entity {
        let mut node = world.spawn((EditorNode, ChildOf(parent)));
        self.parts.write(&NodeParts::default(), &mut node);
        if self.expanded {
            node.insert(Expanded);
        }
        if let Some(ordinal) = self.ordinal {
            node.insert(NextChildOrdinal(ordinal));
        }
        let spawned = node.id();
        for child in &self.children {
            child.spawn_under(world, spawned);
        }
        spawned
    }

    /// Stand a copy of a script block under `parent` on a fresh ordinal. The
    /// ids below it stay: they are unique per parent, and the parent is new.
    pub(crate) fn join_script(&self, world: &mut World, parent: Entity) -> Entity {
        let mut copy = self.clone();
        let stem = copy.parts.id.as_ref().map_or("node", id_stem);
        let id = free_id(world, parent, stem);
        let word = copy
            .parts
            .name
            .as_ref()
            .and_then(|name| name.as_str().split(" Node ").next())
            .unwrap_or("Script")
            .to_string();
        copy.parts.name = Some(Name::new(format!("{word} Node {}", id.0)));
        copy.parts.id = Some(id);
        copy.spawn_under(world, parent)
    }

    /// Stand a new node up under `scenario`, `by` away from this one, with a
    /// fresh id.
    ///
    /// A copy is its own node from here on: it leaves the auto layout row, and
    /// it cannot also be the player's ship, because a scenario flies one.
    pub(crate) fn stand_up(&self, world: &mut World, scenario: Entity, by: Vec3) -> Entity {
        let mut copy = self.clone();
        let stem = copy.parts.id.as_ref().map_or("node", id_stem);
        let id = free_id(world, scenario, stem);
        if let Some(pose) = &mut copy.parts.transform {
            pose.translation += by;
        }
        copy.parts.auto_layout = None;
        if let Some(ship) = &mut copy.parts.ship {
            ship.name = minted_name(&id.0);
            if ship.driver == ShipDriver::Player {
                ship.driver = ShipDriver::Ai;
                ship.allegiance = default_allegiance(ShipDriver::Ai);
            }
            copy.parts.name = Some(Name::new(format!("Ship Node {}", id.0)));
        }
        if let Some(object) = &mut copy.parts.object {
            object.name = minted_name(&id.0);
        }
        copy.parts.id = Some(id);
        let node = copy.spawn_under(world, scenario);
        // A ship's sections get their bodies here. An object's are the frame's
        // `sync_object_views`.
        if let Err(fault) = world.run_system_cached(rebuild_node_views) {
            error!("editor: a copy's bodies were not rebuilt: {fault}");
        }
        node
    }
}

/// The next id under `parent` with this stem, past any sibling that holds it.
///
/// Counted on the parent's own ordinal, but a seeded id never came from it, so
/// the ordinal alone can land on a sibling.
fn free_id(world: &mut World, parent: Entity, stem: &str) -> NodeId {
    let taken: HashSet<String> = world
        .get::<Children>(parent)
        .into_iter()
        .flatten()
        .filter_map(|child| world.get::<NodeId>(*child))
        .map(|id| id.0.clone())
        .collect();
    let mut next = world
        .get::<NextChildOrdinal>(parent)
        .map_or(0, |next| next.0);
    let id = loop {
        next += 1;
        let id = format!("{stem}_{next}");
        if !taken.contains(&id) {
            break id;
        }
    };
    if let Some(mut ordinal) = world.get_mut::<NextChildOrdinal>(parent) {
        ordinal.0 = next;
    }
    NodeId(id)
}

/// What a builder calls this kind of node.
fn kind_of(node: EntityRef) -> &'static str {
    if node.contains::<SectionNode>() {
        "Section"
    } else if node.contains::<ShipNode>() {
        "Ship"
    } else if node.contains::<ObjectNode>() {
        "Object"
    } else if node.contains::<EventNode>() {
        "Event"
    } else if node.contains::<FilterNode>() {
        "Filter"
    } else if node.contains::<ExpressionNode>() {
        "Expression"
    } else if node.contains::<ActionNode>() {
        "Action"
    } else if node.contains::<StepNode>() {
        "Step"
    } else if node.contains::<GateNode>() {
        "Gate"
    } else if node.contains::<SocketNode>() {
        "Socket"
    } else {
        "Node"
    }
}

/// Every document node with its path, parents first.
fn document_paths(world: &mut World) -> Vec<(NodePath, Entity)> {
    let roots: Vec<(Entity, String)> = world
        .query_filtered::<(Entity, &NodeId), With<ScenarioNode>>()
        .iter(world)
        .map(|(root, id)| (root, id.0.clone()))
        .collect();
    let mut found = Vec::new();
    for (root, id) in roots {
        walk_document(world, root, vec![id], &mut found);
    }
    found
}

fn walk_document(world: &World, node: Entity, path: NodePath, found: &mut Vec<(NodePath, Entity)>) {
    found.push((path.clone(), node));
    let Some(children) = world.get::<Children>(node) else {
        return;
    };
    let mut seen: HashMap<&str, u32> = HashMap::new();
    for child in children.iter() {
        // A view is a child too, and carries no id.
        let Some(id) = world.get::<NodeId>(child) else {
            continue;
        };
        let earlier = seen.entry(id.0.as_str()).or_default();
        let segment = match *earlier {
            0 => id.0.clone(),
            clash => format!("{}#{clash}", id.0),
        };
        *earlier += 1;
        let mut below = path.clone();
        below.push(segment);
        walk_document(world, child, below, found);
    }
}

/// Read one node, reusing `held` if the node has not changed.
fn read_record(world: &World, entity: Entity, held: Option<&Arc<NodeRecord>>) -> Arc<NodeRecord> {
    let node = world.entity(entity);
    let parts = NodeParts::of(node).settled();
    let key = parts.key();
    let expanded = node.contains::<Expanded>();
    let ordinal = node.get::<NextChildOrdinal>().map(|next| next.0);
    match held {
        Some(held)
            if held.key == key
                && held.entity == entity
                && held.expanded == expanded
                && held.ordinal == ordinal =>
        {
            Arc::clone(held)
        }
        _ => Arc::new(NodeRecord {
            parts,
            key,
            entity,
            expanded,
            ordinal,
        }),
    }
}

/// Whether two snapshots hold the same document.
///
/// By value alone. Folding a row or minting an id is not an edit.
fn same_document(a: &NodeMap, b: &NodeMap) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|((a_path, a), (b_path, b))| a_path == b_path && a.key == b.key)
}

/// Make the live document match `target`, moving as little as it can.
fn restore(world: &mut World, target: &Snapshot, ordinals: &HashMap<NodePath, u32>) {
    let live = document_paths(world);
    let by_path: HashMap<&[String], Entity> = live
        .iter()
        .map(|(path, entity)| (path.as_slice(), *entity))
        .collect();
    let mut placed: HashMap<&[String], Entity> = HashMap::new();
    let mut kept: HashSet<Entity> = HashSet::new();

    for (path, record) in &target.nodes {
        let same_entity = Some(record.entity).filter(|node| world.get::<NodeId>(*node).is_some());
        let surviving = same_entity
            .into_iter()
            .chain(by_path.get(path.as_slice()).copied())
            .find(|node| !kept.contains(node));
        let node = match surviving {
            Some(node) => {
                update_node(world, node, record);
                node
            }
            None => spawn_node(world, record),
        };
        if let Some(parent) = path
            .split_last()
            .and_then(|(_, above)| placed.get(above).copied())
        {
            if world.get::<ChildOf>(node).map(ChildOf::parent) != Some(parent) {
                world.entity_mut(node).insert(ChildOf(parent));
            }
        }
        if let Some(mut next) = world.get_mut::<NextChildOrdinal>(node) {
            let high = ordinals.get(path).copied().unwrap_or_default();
            if next.0 < high {
                next.0 = high;
            }
        }
        placed.insert(path.as_slice(), node);
        kept.insert(node);
    }

    // Before the despawn: the way up from a doomed node is about to go.
    repoint_selection(world, &kept);
    for (_, node) in &live {
        if !kept.contains(node) && world.get_entity(*node).is_ok() {
            world.entity_mut(*node).despawn();
        }
    }
    // Sections get a body here because nothing else will: the pass runs on
    // entering the editor, and a reconciler would rebuild them every frame.
    // Objects have `sync_object_views`.
    if let Err(fault) = world.run_system_cached(rebuild_node_views) {
        error!("editor: restored sections could not get views: {fault}");
    }
}

/// Bring a surviving node to `record`, leaving a matching one untouched.
fn update_node(world: &mut World, node: Entity, record: &NodeRecord) {
    let live = NodeParts::of(world.entity(node)).settled();
    if live.key() == record.key {
        return;
    }
    let reshaped = live.section.as_ref().map(SectionNode::prototype)
        != record.parts.section.as_ref().map(SectionNode::prototype)
        || format!("{:?}", live.object) != format!("{:?}", record.parts.object);
    record.parts.write(&live, &mut world.entity_mut(node));
    if reshaped {
        drop_views(world, node);
    }
}

/// Despawn `node`'s body, for the view passes to build a new one.
fn drop_views(world: &mut World, node: Entity) {
    let Some(children) = world.get::<Children>(node) else {
        return;
    };
    let views: Vec<Entity> = children
        .iter()
        .filter(|child| world.get::<NodeView>(*child).is_some())
        .collect();
    for view in views {
        world.entity_mut(view).despawn();
    }
}

/// Stand a node back up from its record.
fn spawn_node(world: &mut World, record: &NodeRecord) -> Entity {
    let mut node = world.spawn(EditorNode);
    record.parts.write(&NodeParts::default(), &mut node);
    if record.expanded {
        node.insert(Expanded);
    }
    if let Some(ordinal) = record.ordinal {
        node.insert(NextChildOrdinal(ordinal));
    }
    node.id()
}

/// Move the selection and the entered path off any node that is about to go.
fn repoint_selection(world: &mut World, kept: &HashSet<Entity>) {
    let primary = world
        .get_resource::<SelectedNode>()
        .and_then(SelectedNode::primary)
        .filter(|node| !kept.contains(node));
    if let Some(mut selected) = world.get_resource_mut::<SelectedNode>() {
        selected.retain(|node| kept.contains(node));
    }
    // A step that took the whole selection hands the mark to what held it.
    if let Some(gone) = primary.filter(|_| world.resource::<SelectedNode>().is_empty()) {
        let mut above = world.get::<ChildOf>(gone).map(ChildOf::parent);
        while let Some(node) = above.filter(|node| !kept.contains(node)) {
            above = world.get::<ChildOf>(node).map(ChildOf::parent);
        }
        world.resource_mut::<SelectedNode>().set(above);
    }
    let Some(mut context) = world.get_resource_mut::<EditContext>() else {
        return;
    };
    if let Some(gone) = context.path.iter().position(|node| !kept.contains(node)) {
        context.path.truncate(gone);
    }
}

/// Ask for one step, and say what happened.
///
/// Refused while a pointer button is held: a drag owns the node under it and
/// commits on release, so a step taken inside one would be overwritten by the
/// gesture it interrupted.
fn take_step(
    commands: &mut Commands,
    history: &EditHistory,
    says: &mut EditorSays,
    buttons: Option<&ButtonInput<MouseButton>>,
    direction: Direction,
) {
    let (word, label) = match direction {
        Direction::Undo => ("undid", history.undo_label()),
        Direction::Redo => ("redid", history.redo_label()),
    };
    let Some(label) = label else {
        return;
    };
    if buttons.is_some_and(|buttons| buttons.get_pressed().next().is_some()) {
        says.refuse("let go of the pointer first");
        return;
    }
    says.note(format!("{word} {}", label.to_lowercase()));
    commands.queue(move |world: &mut World| {
        with_history(world, |history, world| history.step(world, direction));
    });
}

/// Ctrl+Z undoes; Ctrl+Shift+Z and Ctrl+Y redo.
///
/// Gated like Ctrl+S: a focused text field owns its own keys.
pub(crate) fn history_keys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    history: Res<EditHistory>,
    popup: Res<BlockPopup>,
    mut says: EditorSays,
) {
    if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let direction =
        if keys.just_pressed(KeyCode::KeyY) || (shift && keys.just_pressed(KeyCode::KeyZ)) {
            Direction::Redo
        } else if keys.just_pressed(KeyCode::KeyZ) {
            Direction::Undo
        } else {
            return;
        };
    if popup.is_open() {
        commands.queue(move |world: &mut World| step_past_popup(world, direction));
        return;
    }
    take_step(
        &mut commands,
        &history,
        &mut says,
        buttons.as_deref(),
        direction,
    );
}

/// Close the pop-up, then step. A step taken under it would move the document
/// behind the revision its Esc takes back to.
fn step_past_popup(world: &mut World, direction: Direction) {
    close_block_popup(world, PopupLeaveType::Keep);
    let stepped = world.run_system_once(
        move |mut commands: Commands,
              history: Res<EditHistory>,
              mut says: EditorSays,
              buttons: Option<Res<ButtonInput<MouseButton>>>| {
            take_step(
                &mut commands,
                &history,
                &mut says,
                buttons.as_deref(),
                direction,
            );
        },
    );
    if let Err(fault) = stepped {
        warn!("the history key did not run: {fault}");
    }
}

/// Edit > Undo.
pub(crate) fn undo_item(
    _activate: On<Activate>,
    mut commands: Commands,
    history: Res<EditHistory>,
    mut says: EditorSays,
) {
    take_step(&mut commands, &history, &mut says, None, Direction::Undo);
}

/// Edit > Redo.
pub(crate) fn redo_item(
    _activate: On<Activate>,
    mut commands: Commands,
    history: Res<EditHistory>,
    mut says: EditorSays,
) {
    take_step(&mut commands, &history, &mut says, None, Direction::Redo);
}

#[cfg(test)]
pub(crate) mod tests;
