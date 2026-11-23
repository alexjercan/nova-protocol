//! The fixed HUD panels a floating label keeps clear of.
//!
//! A fixed panel (the keybind dock, the comms stack, the objective stack, the
//! status bar, ...) carries [`HudKeepOutPanel`], and [`HudKeepOuts`] reads its
//! live rect from its last layout. Two panels own their rect instead: the HUD
//! notice, which hides by alpha and must claim its widest box on the frame its
//! line changes, and the target inset, which is placed in px from its own
//! `Node`. The label pass in `update_screen_indicators` and the two owner-fit
//! stacks (the flight chips and the lock readout) read the union.
//!
//! A box leaves most panels by the shortest way. The target inset is the
//! exception: it states an inward way out ([`KeepOutExitType::Toward`]), so a
//! label under it moves toward the screen centre, not along the screen edge
//! the inset hangs from.
//!
//! A laid-out rect is one frame stale: the readers run before this frame's
//! layout. A panel that moves is therefore cleared at last frame's place for
//! one frame.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_ui::prelude::StatusBarRootMarker;

use crate::{hud_notice::HudNoticeKeepOut, target_inset::TargetInsetKeepOut};

/// The keep-out marker and its plugin.
pub mod prelude {
    pub use super::{HudKeepOutPanel, HudKeepOutPlugin};
}

/// A fixed HUD panel that floating labels never draw over. Its rect is its
/// laid-out box while it is visible and not empty.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct HudKeepOutPanel;

/// Marks the panels this crate does not spawn: the `nova_ui` status bar.
#[derive(Default)]
pub struct HudKeepOutPlugin;

impl Plugin for HudKeepOutPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<StatusBarRootMarker, HudKeepOutPanel>();
    }
}

/// How a box that hits a [`KeepOut`] leaves it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum KeepOutExitType {
    /// The shortest axis move that clears it.
    Shortest,
    /// Only the axis move along this unit direction (UI px, y down). A panel
    /// that hangs from a screen edge names the way from that edge to the
    /// screen centre.
    Toward(Vec2),
}

/// A rect (UI px) a box keeps clear of, and how a box leaves it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KeepOut {
    pub(crate) rect: Rect,
    pub(crate) exit: KeepOutExitType,
}

impl From<Rect> for KeepOut {
    fn from(rect: Rect) -> Self {
        Self {
            rect,
            exit: KeepOutExitType::Shortest,
        }
    }
}

/// Every shown fixed panel as a keep-out in UI px.
#[derive(SystemParam)]
pub(crate) struct HudKeepOuts<'w, 's> {
    notice: HudNoticeKeepOut<'w, 's>,
    inset: TargetInsetKeepOut<'w, 's>,
    q_panel: Query<
        'w,
        's,
        (
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
        ),
        With<HudKeepOutPanel>,
    >,
}

impl HudKeepOuts<'_, '_> {
    /// The shown panels on a `view` of that size (UI px).
    pub(crate) fn shown(&self, view: Vec2) -> Vec<KeepOut> {
        let laid_out = self
            .q_panel
            .iter()
            .filter(|(computed, _, visible)| visible.get() && computed.size().min_element() > 0.0)
            .map(|(computed, transform, _)| {
                let scale = computed.inverse_scale_factor();
                KeepOut::from(Rect::from_center_size(
                    transform.translation * scale,
                    computed.size() * scale,
                ))
            });
        self.notice
            .rect(view)
            .map(KeepOut::from)
            .into_iter()
            .chain(self.inset.keep_out(view))
            .chain(laid_out)
            .collect()
    }
}
