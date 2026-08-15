//! The editor's build-state resources and preview markers: what the player is
//! assembling (`PlayerSpaceshipConfig`), which placement tool is active
//! (`SectionChoice`), and the non-physics preview entities.

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::Binding;
use nova_scenario::prelude::*;

/// The ship the player is building, in the exact serialized shape the scenario
/// consumes on hand-off. `sections` is keyed by the live preview entity so a
/// delete/rebind can find its config; `inputs` mirrors each bindable section's
/// keybinds (what the scenario's `PlayerControllerConfig` reads).
#[derive(Resource, Debug, Clone, Default, Reflect)]
pub(crate) struct PlayerSpaceshipConfig {
    pub(crate) sections: HashMap<Entity, SpaceshipSectionConfig>,
    pub(crate) inputs: HashMap<Entity, Vec<Binding>>,
    /// Whether the ship wears its derived cladding - shown live in the build
    /// view (see `crate::skin`) and carried through to the flown ship, so what
    /// the builder sees is what they fly.
    ///
    /// Part of the BUILD STATE rather than a view setting of its own: it is a
    /// property of the ship, it has to survive a trip out to the scenario and
    /// back, and one resource changing is one thing for the preview to watch.
    pub(crate) skin: bool,
    /// The style id the ship's cladding wears, or `None` for the first style
    /// the content merge loaded.
    ///
    /// Part of the build state for the same reason `skin` is: it travels out to
    /// the scenario with the ship. There is no picker for it yet, so the build
    /// view falls back to the first AUTHORED style rather than to a hard-coded
    /// id - a mod that ships one look is then what the editor shows.
    pub(crate) style: Option<String>,
}

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

/// The root of the editor's preview ship. Deliberately distinct from the gameplay
/// `SpaceshipRootMarker`: the preview is a static, pickable visual used only to build a
/// `PlayerSpaceshipConfig`, so it must not trigger `insert_spaceship_sections` or any of the
/// integrity/health systems that key on `SpaceshipRootMarker`. The real ship is built from
/// the config when entering the scenario.
#[derive(Component)]
pub(crate) struct SpaceshipPreviewMarker;

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
}

/// The editor's placement status line: why the ghost is refused.
#[derive(Component)]
pub(crate) struct PlacementStatus;

/// The editor's key legend, bottom-left. Its text follows the armed tool, so
/// the keys that do nothing in the current mode are not listed and the line
/// stays short enough to read rather than long enough to ignore.
#[derive(Component)]
pub(crate) struct EditorKeyLegend;

/// The box of the cladding toggle in the Tools block, repainted in place when
/// [`PlayerSpaceshipConfig::skin`] changes.
#[derive(Component)]
pub(crate) struct SkinToggleCheckbox;
