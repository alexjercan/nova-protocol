//! The editor's placement state and screen furniture: which tool is active
//! (`SectionChoice`), what a click would build (`PlacementPreview`), and the
//! markers the rail and the ghost are found by.
//!
//! The SHIP is not here. What the player is assembling lives in
//! [`crate::node`] as a tree of node entities, one config component per node.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_ui::theme;

/// The group every immediate-mode line in the editor draws in.
///
/// One group for the whole editor, so [`EDITOR_LINE_WIDTH`] settles the weight
/// of the grid, the trigger volumes, the plumb line, the ship heading and the
/// link points together - and settles it for the editor alone, which sharing
/// Bevy's default group would not.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub(crate) struct EditorGizmos;

/// Line width (px) for those lines. Under Bevy's 2 px default: everything
/// drawn here is drawn AROUND the subject, and at the default the floor read
/// as heavily as the thing standing on it.
const EDITOR_LINE_WIDTH: f32 = 1.0;

/// The editor's gizmo config, applied at plugin registration.
pub(crate) fn editor_gizmo_config() -> GizmoConfig {
    GizmoConfig {
        line: GizmoLineConfig {
            width: EDITOR_LINE_WIDTH,
            ..default()
        },
        ..default()
    }
}

/// The active placement tool: what the next click on the ship does. Set by the
/// gallery when a part is armed.
///
/// TWO states, because there is one tool. Delete acts on the SELECTION, so it
/// is a verb rather than a mode and needs nothing in this enum.
#[derive(Resource, Default, Debug, PartialEq, Eq, Clone, Reflect)]
pub(crate) enum SectionChoice {
    /// Select mode: clicking a node in the world selects it, exactly as its
    /// tree row would.
    #[default]
    None,
    /// Place the section with this catalog id.
    Section(String),
}

/// The node the Scene tree has selected, or `None`.
///
/// SELECTION IS NOT CONTEXT. Selecting a node marks it in the tree so an
/// inspector (and the Rebind action) has something to act on; the context is
/// which CONTAINER the editor is inside. A section is selected, a ship is
/// entered.
#[derive(Resource, Default, Debug)]
pub(crate) struct SelectedNode(pub(crate) Option<Entity>);

/// The node the pointer is resting on, wherever it rests.
///
/// HOVER IS NOT SELECTION. Nothing acts on this - it is what the rail and the
/// stage light up so the same node can be found on both surfaces without
/// clicking, which would move the camera and the selection to find out.
/// Filled by [`crate::highlight::sync_hovered_node`] from whichever surface
/// the pointer is over.
#[derive(Resource, Default, Debug)]
pub(crate) struct HoveredNode(pub(crate) Option<Entity>);

/// How long after a click a second one on the same node still reads as a
/// double.
///
/// The window desktops have used for decades, and generous on purpose: the
/// gesture it guards is not destructive - the worst a late second click does
/// is select what was already selected.
pub(crate) const DOUBLE_CLICK_SECS: f32 = 0.5;

/// The Scene row the last click landed on, and when.
///
/// Clicking a DIFFERENT row restarts the count: a fast click on one ship and
/// then another is two selections, not an entry.
///
/// The TREE only. On the stage a second press on the same ship is far more
/// often the start of a drag than a request to go inside it, and a press
/// cannot yet know which - see `crate::placement::on_click_spaceship_section`.
#[derive(Resource, Default, Debug)]
pub(crate) struct LastClick {
    node: Option<Entity>,
    at: f32,
}

impl LastClick {
    /// Record a click on `node` at `now` seconds, and say whether it completes
    /// a double.
    ///
    /// A completed double is FORGOTTEN rather than kept: three fast clicks are
    /// one double and one single, not two doubles, so a builder drumming on a
    /// ship row enters it once.
    pub(crate) fn press(&mut self, node: Entity, now: f32) -> bool {
        let double = self.node == Some(node) && now - self.at <= DOUBLE_CLICK_SECS;
        self.node = (!double).then_some(node);
        self.at = now;
        double
    }
}

/// Which half of the document the rail's tree is showing.
///
/// One tree and two tabs rather than two stacked trees: a range holds twenty
/// objects and a script holds as many handlers, and a 150px rail cannot show
/// both at once without the half you came for being off the bottom edge.
#[derive(Resource, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) enum RailTab {
    /// The world: the range, the ships on it and the objects around them.
    #[default]
    Scene,
    /// The script: the handlers, their filters and what they do.
    Events,
}

/// One tab of the rail's tree header, carrying the half it switches to.
#[derive(Component, Clone, Copy)]
pub(crate) struct RailTabButton(pub(crate) RailTab);

/// The Scene block's row container, emptied and refilled by
/// `crate::ui::sync_scene_list`.
#[derive(Component)]
pub(crate) struct SceneList;

/// One Scene row, carrying the node it points at. The scenario root is a row
/// like any other: clicking it is how the tree leaves a ship.
#[derive(Component)]
pub(crate) struct SceneRow(pub(crate) Entity);

/// The Play button, so `crate::ui::sync_play_button` can disable it outside the
/// scenario node.
#[derive(Component)]
pub(crate) struct PlayButton;

/// The top bar's breadcrumb: where in the document the editor is, as a path of
/// pressable steps. Refilled by `crate::ui::sync_breadcrumb`.
#[derive(Component)]
pub(crate) struct ContextBreadcrumb;

/// What the editor stage DRAWS on top of the document, toggled from the View
/// menu.
///
/// Overlays rather than settings: none of this is saved with the scenario and
/// none of it changes what Play spawns. They live in one resource so the menu
/// has one thing to read back and the gizmo systems have one thing to check.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct EditorOverlays {
    /// The footer's key line.
    pub(crate) key_legend: bool,
    /// The socket rings a ship draws while a part is armed.
    pub(crate) link_points: bool,
    /// The ground plane the range is laid out on, and the plumb line under the
    /// selection.
    pub(crate) world_grid: bool,
    /// The volumes and aims an object carries but has no body to show: a
    /// trigger sphere, a lamp's reach, a sun's direction.
    pub(crate) object_volumes: bool,
    /// Every field a node's config declares, instead of the ones its kind is
    /// authored through. Off: the inspector is a first screen, and this is the
    /// way past it.
    pub(crate) all_fields: bool,
    /// The tree draws each node's ID rather than its name. Off: a name is what
    /// a builder called the thing, and this is for the times the id is what
    /// they need - an event's filter names nodes by id, not by name.
    pub(crate) ids: bool,
}

impl Default for EditorOverlays {
    /// All ON. The legend is how the editor teaches its keys, the sockets are
    /// the only thing placement snaps to - an editor that opened with them off
    /// would be an editor whose first build is guesswork - and the grid is the
    /// only thing on the stage that says how far apart two objects are.
    fn default() -> Self {
        Self {
            key_legend: true,
            link_points: true,
            world_grid: true,
            object_volumes: true,
            // The exceptions, both of which trade a reading for a listing:
            // the curated inspector IS the editor's answer to "what does this
            // thing do", and a tree of ids is a tree nobody named.
            all_fields: false,
            ids: false,
        }
    }
}

/// The rail's ship settings block (skin, style, attitude). Shown only inside a
/// ship: they are properties of the ship being edited, and there is none at
/// the scenario node.
#[derive(Component)]
pub(crate) struct ShipSettings;

/// The Rebind row (Ship > Rebind Key), greyed unless the selection is a
/// bindable section of the edited ship.
#[derive(Component)]
pub(crate) struct RebindButton;

/// The ghost part that previews where a placed section will land.
#[derive(Component)]
pub(crate) struct SectionPreviewMarker;

/// The builder's two placement choices: which of the armed part's sockets does
/// the mating, and how far the part is rolled about the mating axis.
///
/// A mate fixes everything else - the two sockets are coincident and their
/// normals opposed - so these are the only degrees of freedom left, and
/// neither can be derived from the ship.
#[derive(Resource, Default, Debug, Clone, Copy, Reflect)]
pub(crate) struct PlacementPose {
    /// Socket index on the part. Wraps, so a caller can just count up.
    pub(crate) source: usize,
    /// Quarter turns about the mating axis, 0..4.
    pub(crate) roll: u32,
}

/// What a click would build right now, solved once per frame from the section
/// under the pointer so the ghost and the click cannot disagree.
#[derive(Resource, Default)]
pub(crate) struct PlacementPreview {
    /// `None` when no part is armed or nothing is under the pointer.
    pub(crate) placement: Option<Placement>,
}

/// One solved placement: the armed prototype, the section it mates onto, and
/// the pose plus verdict the solver returned.
pub(crate) struct Placement {
    /// Catalog id of the armed prototype.
    pub(crate) prototype: String,
    /// The preview section under the pointer.
    pub(crate) target_section: Entity,
    /// Pose and refusal.
    pub(crate) solve: crate::snap::Placement,
}

/// The ghost's identity, so a pose change MOVES it and a part change rebuilds
/// it - a respawned scene every frame would flicker.
#[derive(Component)]
pub(crate) struct SectionGhost {
    /// Catalog id the ghost is showing.
    pub(crate) prototype: String,
    /// Socket index the ghost mates with.
    pub(crate) source: usize,
    /// The ship node it hangs on. Part of its identity because a solve is in
    /// SHIP-LOCAL space: entering another ship with the same part still in hand
    /// keeps the prototype and the socket, and a ghost kept across that switch
    /// would draw the new ship's pose on the old ship.
    pub(crate) ship: Entity,
}

/// The editor's status line: the one place it says something back.
#[derive(Component)]
pub(crate) struct PlacementStatus;

/// What that line says.
///
/// TWO slots, because two kinds of thing want it. The READOUT is the placement
/// preview's, rewritten every frame while a part is in hand and gone the moment
/// it is put down. What a VERB SAID - a save wrote, a load failed - has to
/// outlast the frame it happened in, or the builder never sees it, so it holds
/// the line for [`SAID_HOLD_SECS`] and the readout waits.
#[derive(Resource, Default)]
pub(crate) struct EditorStatus {
    /// The placement readout, or `None` when nothing is being placed.
    readout: Option<(String, Color)>,
    /// The last thing a verb said, and the moment it stops holding the line.
    said: Option<(String, Color, f64)>,
}

/// How long a verb holds the line before the readout gets it back.
const SAID_HOLD_SECS: f64 = 4.0;

impl EditorStatus {
    /// Write the placement readout, or clear it.
    pub(crate) fn report(&mut self, line: Option<(String, Color)>) {
        self.readout = line;
    }

    /// Say something that has to be read: it holds the line for a few seconds.
    pub(crate) fn say(&mut self, message: impl Into<String>, tint: Color, now: f64) {
        self.said = Some((message.into(), tint, now + SAID_HOLD_SECS));
    }

    /// Drop what a verb said once its hold is over.
    ///
    /// The clock lives HERE rather than in the readers, so the line, the probe
    /// and anything else that comes along all see one answer to "is this still
    /// being said" - and so a reader needs no `Time` of its own.
    pub(crate) fn expire(&mut self, now: f64) {
        if self
            .said
            .as_ref()
            .is_some_and(|(_, _, until)| now >= *until)
        {
            self.said = None;
        }
    }

    /// What the line shows right now.
    pub(crate) fn line(&self) -> Option<(&str, Color)> {
        self.said
            .as_ref()
            .map(|(message, tint, _)| (message.as_str(), *tint))
            .or_else(|| {
                self.readout
                    .as_ref()
                    .map(|(message, tint)| (message.as_str(), *tint))
            })
    }
}

/// The editor speaking: the one line, and the clock a message is stamped with.
///
/// One `SystemParam` rather than two, because a refusal that has to reach for
/// `Time` as well as [`EditorStatus`] is a refusal that gets written as a
/// `warn!` instead. Every verb in the editor that can say no takes this.
#[derive(SystemParam)]
pub(crate) struct EditorSays<'w> {
    status: ResMut<'w, EditorStatus>,
    time: Res<'w, Time>,
}

impl EditorSays<'_> {
    /// Say no, in red. The message is the REASON, phrased as the way out where
    /// there is one - a builder who is told what to do next does not have to
    /// work out what just happened.
    pub(crate) fn refuse(&mut self, message: impl Into<String>) {
        let now = self.time.elapsed_secs_f64();
        self.status.say(message, theme::RED, now);
    }

    /// Say what happened, in phosphor. For a thing the editor did FOR you and
    /// would otherwise do silently.
    pub(crate) fn note(&mut self, message: impl Into<String>) {
        let now = self.time.elapsed_secs_f64();
        self.status.say(message, theme::PHOSPHOR, now);
    }
}

/// The rail's engineer readout: what the hull under construction weighs,
/// pushes with, survives and would turn like. Repainted from the build state
/// by `crate::readout::sync_ship_readout`.
#[derive(Component)]
pub(crate) struct ShipReadout;

/// The line under that block: which of the two attitude ceilings holds the
/// turn rate down, and the one thing that would raise it.
#[derive(Component)]
pub(crate) struct ShipReadoutNote;

/// One pressable step of the breadcrumb, carrying the node it goes to.
#[derive(Component)]
pub(crate) struct CrumbStep(pub(crate) Entity);

/// The breadcrumb's selection chip, carrying the node it names.
#[derive(Component)]
pub(crate) struct CrumbSelection(pub(crate) Entity);

/// The editor's key legend, bottom-left. Its text follows the armed tool, so
/// the keys that do nothing in the current mode are not listed and the line
/// stays short enough to read rather than long enough to ignore.
#[derive(Component)]
pub(crate) struct EditorKeyLegend;

/// The box of the skin toggle in the Tools block, repainted in place when
/// the edited ship's [`ShipNode::skin`](crate::node::ShipNode::skin) changes.
#[derive(Component)]
pub(crate) struct SkinToggleCheckbox;

/// The list of styles under the skin toggle. Always shown, and greyed while
/// the ship wears no skin: the greyed list is what advertises that the toggle
/// leads somewhere.
#[derive(Component)]
pub(crate) struct StyleList;

/// The colour block on a style row, carrying the paint that style puts on a
/// hull's top surface - so the row can be dimmed and given its colour back
/// without reading the catalog a second time.
#[derive(Component)]
pub(crate) struct StyleSwatch(pub(crate) Color);

/// One row of that list, carrying the style id it picks - the same shape the
/// tool rows use, so the shared `Selected` highlight marks the active style.
#[derive(Component)]
pub(crate) struct StyleChoice(pub(crate) String);
