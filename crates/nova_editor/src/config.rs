//! The editor's placement state and screen furniture: which tool is active
//! (`SectionChoice`), what a click would build (`PlacementPreview`), and the
//! markers the rail and the ghost are found by.
//!
//! The SHIP is not here. What the player is assembling lives in
//! [`crate::node`] as a tree of node entities, one config component per node.

use bevy::prelude::*;

/// The active placement tool, driven by the top bar's tool buttons and the
/// gallery cards through `button_on_setting::<SectionChoice>`.
#[derive(Resource, Default, Debug, PartialEq, Eq, Clone, Reflect)]
pub(crate) enum SectionChoice {
    /// Select mode: clicking a node in the world selects it, exactly as its
    /// tree row would.
    #[default]
    None,
    /// Place the section with this catalog id.
    Section(String),
    /// Delete the clicked section.
    Delete,
}

/// The node the Scene tree has selected, or `None`.
///
/// SELECTION IS NOT CONTEXT. Selecting a node marks it in the tree so an
/// inspector (and the Rebind action) has something to act on; the context is
/// which CONTAINER the editor is inside. A section is selected, a ship is
/// entered.
#[derive(Resource, Default, Debug)]
pub(crate) struct SelectedNode(pub(crate) Option<Entity>);

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

/// The top bar's breadcrumb: where in the document the editor is, as a path.
#[derive(Component)]
pub(crate) struct ContextBreadcrumb;

/// The top bar's scenario-context action group (Add Ship, Delete). Shown only
/// at the scenario node; each context gets its own verbs.
#[derive(Component)]
pub(crate) struct ScenarioActions;

/// The Delete action's button, greyed unless the selection is a node the
/// scenario can lose.
#[derive(Component)]
pub(crate) struct DeleteNodeButton;

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
}

impl Default for EditorOverlays {
    /// Both ON. The legend is how the editor teaches its keys, and the sockets
    /// are the only thing placement snaps to - an editor that opened with them
    /// off would be an editor whose first build is guesswork.
    fn default() -> Self {
        Self {
            key_legend: true,
            link_points: true,
        }
    }
}

/// The top bar's ship-context action group (Parts, Delete, Rebind). Shown only
/// inside a ship.
#[derive(Component)]
pub(crate) struct ShipActions;

/// The rail's ship settings block (skin, look, attitude). Shown only inside a
/// ship: they are properties of the ship being edited, and there is none at
/// the scenario node.
#[derive(Component)]
pub(crate) struct ShipSettings;

/// The Rebind action's button, greyed unless the selection is a bindable
/// section of the edited ship.
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

/// The editor's placement status line: why the ghost is refused.
#[derive(Component)]
pub(crate) struct PlacementStatus;

/// The rail's attitude readout: what the hull under construction would turn
/// like, and which of the two ceilings says so. Repainted from the build state
/// by `crate::attitude::sync_attitude_readout`.
#[derive(Component)]
pub(crate) struct AttitudeReadout;

/// The editor's key legend, bottom-left. Its text follows the armed tool, so
/// the keys that do nothing in the current mode are not listed and the line
/// stays short enough to read rather than long enough to ignore.
#[derive(Component)]
pub(crate) struct EditorKeyLegend;

/// The box of the cladding toggle in the Tools block, repainted in place when
/// the edited ship's [`ShipNode::skin`](crate::node::ShipNode::skin) changes.
#[derive(Component)]
pub(crate) struct SkinToggleCheckbox;

/// The list of looks under the cladding toggle. Shown only while the ship is
/// clad, because a look is a property of a skin that is on.
#[derive(Component)]
pub(crate) struct StyleList;

/// One row of that list, carrying the style id it picks - the same shape the
/// tool rows use, so the shared `Selected` highlight marks the active look.
#[derive(Component)]
pub(crate) struct StyleChoice(pub(crate) String);
