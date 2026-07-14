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

/// The translucent cube that previews where a placed/deleted section will land.
#[derive(Component)]
pub(crate) struct SectionPreviewMarker;
