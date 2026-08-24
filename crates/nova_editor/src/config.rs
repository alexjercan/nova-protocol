//! The editor's placement state and screen furniture: which tool is active
//! (`SectionChoice`), what a click would build (`PlacementPreview`), and the
//! markers the rail and the ghost are found by.
//!
//! The SHIP is not here. What the player is assembling lives in
//! [`crate::node`] as a tree of node entities, one config component per node.

use bevy::prelude::*;

/// The active placement tool, driven by the rail tools and the component cards
/// through `button_on_setting::<SectionChoice>`.
#[derive(Resource, Default, Debug, PartialEq, Eq, Clone, Reflect)]
pub(crate) enum SectionChoice {
    /// Select / rebind mode: clicking a bindable section arms a keybind capture.
    #[default]
    None,
    /// Place the section with this catalog id.
    Section(String),
    /// Delete the clicked section.
    Delete,
}

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
/// [`PlayerSpaceshipConfig::skin`] changes.
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
