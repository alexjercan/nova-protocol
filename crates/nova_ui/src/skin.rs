//! The UI skin: which visual language the shared widgets render in.
//!
//! Every themed widget ([`crate::widget`]) reads the [`UiSkin`] resource and
//! renders from the same NOVA OS palette in one of two skins:
//!
//! - [`UiSkin::Phosphor`] (default): the CLI-drawn terminal look - flat
//!   phosphor-on-black fills, 1px phosphor borders, inverted selection, 2px
//!   corners. Widgets read as elements a terminal drew.
//! - [`UiSkin::Hardware`]: the light-3D moulded casing - case-gradient faces,
//!   rim/undercut/drop `BoxShadow` bevels, amber selection, 7px corners.
//!
//! The resource change drives a live restyle of every spawned widget (see
//! [`crate::widget`]'s reconciler), so flipping the skin in Settings reskins the
//! whole UI in place. Settings persists the choice through the `ui_skin` field
//! of its settings store.

/// Glob-import surface for the skin selector.
pub mod prelude {
    pub use super::UiSkin;
}

use bevy::prelude::*;

/// Which visual language the shared widgets render in.
///
/// `Resource` (component-backed on Bevy 0.19, so it doubles as the `Component`
/// that [`crate::widget::button_on_setting`] needs for a `ButtonValue<UiSkin>`
/// settings row - deriving `Component` too would conflict) + `PartialEq`/`Clone`
/// so a settings button can carry and set it exactly like the editor's
/// `SectionChoice` (also Resource-only).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UiSkin {
    /// The phosphor terminal look - the primary, default skin.
    #[default]
    Phosphor,
    /// The light-3D hardware casing look.
    Hardware,
}

impl UiSkin {
    /// True for the phosphor terminal skin.
    pub fn is_phosphor(self) -> bool {
        matches!(self, UiSkin::Phosphor)
    }

    /// True for the hardware casing skin.
    pub fn is_hardware(self) -> bool {
        matches!(self, UiSkin::Hardware)
    }
}
