//! [`ActionContext`]: when a named action can fire at all, and
//! [`ActiveContexts`]: which contexts are live this frame.
//!
//! This exists because an action resolves to a KEY, and keys are reused across
//! surfaces on purpose. `G` is `autopilot_goto` in flight, `map_goto` in the
//! map viewer and `ship_mates` in the ship viewer. So pressing an action that
//! is not live does not fail quietly - it presses the key, and whatever IS
//! live reads it as its own action. A driver has to be told which actions can
//! fire THIS tick, not which ones exist.
//!
//! `nova_input` is a leaf and cannot see `PauseStates` or the terminal mode,
//! so the subsystem that owns a context is the one that raises and lowers it.
//! The context declared on an action and the run condition on the systems that
//! read it are two statements of one fact, and nothing in the type system ties
//! them together; [`crate::registry::InputBindings::conflicts`] is what catches
//! the case that matters, two actions that can be live together on one source.

use bevy::{platform::collections::HashSet, prelude::*};

/// When an action can fire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ActionContext {
    /// Live at every rung. The mode keys themselves, which have to work from
    /// wherever the player is - and the default, so an action nobody
    /// classified is over-reported rather than silently missing.
    #[default]
    Always,
    /// Live while the player flies: a player ship on the field, not paused.
    Flight,
    /// Live while any NOVA OS app owns the screen. NOT the prompt: there the
    /// keyboard is typing, which is a raw lane and not a vocabulary.
    Viewer,
    /// Live while ONE named NOVA OS app owns the screen. Carries the app's
    /// launch word - `map`, `ship`.
    ViewerApp(&'static str),
}

impl ActionContext {
    /// Can these two contexts be live at the same instant?
    ///
    /// The only nesting is the viewer one: a named app is live only while some
    /// app is, so [`Self::Viewer`] overlaps every [`Self::ViewerApp`]. Two
    /// DIFFERENT named apps never are - one app owns the screen.
    pub fn overlaps(self, other: Self) -> bool {
        match (self, other) {
            (Self::Always, _) | (_, Self::Always) => true,
            (Self::Viewer, Self::ViewerApp(_)) | (Self::ViewerApp(_), Self::Viewer) => true,
            _ => self == other,
        }
    }
}

/// The contexts that can fire right now.
///
/// Written by the subsystem that owns each context, read by every surface that
/// has to answer "what may be pressed": the process channel refusing a name,
/// the snapshot listing what a driver may send, a settings screen dimming a
/// row that cannot fire.
#[derive(Resource, Debug, Default)]
pub struct ActiveContexts(HashSet<ActionContext>);

impl ActiveContexts {
    /// Raise or lower one context. Call it only when the answer CHANGED:
    /// taking `&mut self` through a `ResMut` marks the resource changed, and
    /// surfaces that repaint on a change would then repaint every frame.
    pub fn set(&mut self, context: ActionContext, live: bool) {
        if live {
            self.0.insert(context);
        } else {
            self.0.remove(&context);
        }
    }

    /// Is this context live? [`ActionContext::Always`] is live by definition
    /// and nobody has to raise it.
    pub fn is_live(&self, context: ActionContext) -> bool {
        context == ActionContext::Always || self.0.contains(&context)
    }

    /// Raise `context` to `live` only if that is not what it already says.
    /// The change-detection-safe form of [`Self::set`], for a sync system that
    /// runs every frame.
    pub fn sync(this: &mut ResMut<'_, Self>, context: ActionContext, live: bool) {
        if this.is_live(context) != live {
            this.set(context, live);
        }
    }

    /// Every raised context, unordered. `Always` is not in here - it is not
    /// raised, it simply is.
    pub fn iter(&self) -> impl Iterator<Item = ActionContext> + '_ {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_is_live_without_anyone_raising_it() {
        let active = ActiveContexts::default();
        assert!(active.is_live(ActionContext::Always));
        assert!(!active.is_live(ActionContext::Flight));
    }

    #[test]
    fn a_raised_context_is_live_until_it_is_lowered() {
        let mut active = ActiveContexts::default();
        active.set(ActionContext::Flight, true);
        assert!(active.is_live(ActionContext::Flight));
        active.set(ActionContext::Flight, false);
        assert!(!active.is_live(ActionContext::Flight));
    }

    /// The viewer nesting is the only overlap rule, and it has to hold both
    /// ways round: the shared orbit keys and the map's own `G` ARE live
    /// together, so they share a conflict set.
    #[test]
    fn any_viewer_overlaps_a_named_one_but_two_named_ones_never_do() {
        use ActionContext::{Always, Flight, Viewer, ViewerApp};
        assert!(Viewer.overlaps(ViewerApp("map")));
        assert!(ViewerApp("map").overlaps(Viewer));
        assert!(!ViewerApp("map").overlaps(ViewerApp("ship")));
        assert!(ViewerApp("map").overlaps(ViewerApp("map")));
        assert!(!Flight.overlaps(Viewer));
        assert!(Always.overlaps(Flight), "the mode keys reach every rung");
        assert!(Flight.overlaps(Always));
    }
}
