//! [`Names`]: what an authored string in a config REFERS TO.
//!
//! A handler is held together by strings. `SetAllegiance` names a ship,
//! `TimerCancel` names a timer, `NextScenario` names a scenario - and to the
//! type system all three are `String`, so a surface that shows a config to a
//! person can only offer a blank box and hope. The lint knows the difference
//! and has always known it, by reading the config type and the field name
//! together; that knowledge lived in one `match` arm per action and nowhere a
//! second reader could reach.
//!
//! This puts it on the FIELD, as a reflect custom attribute:
//!
//! ```ignore
//! #[reflect(@Names::Object)]
//! pub id: String,
//! ```
//!
//! Anything walking a config by reflection can then ask what a string names
//! and act on it - the editor's inspector offers the ids the document
//! actually spawns and marks a dangling one, rather than naming every field
//! it knows about in a list of its own that goes stale the day an action is
//! added.
//!
//! Touch this module when a config grows a string that names something.

use bevy::prelude::*;

/// Glob-import surface: `use nova_scenario::names::prelude::*` brings the
/// [`Names`] field attribute into scope.
pub mod prelude {
    pub use super::Names;
}

/// What a string field names.
///
/// The split between [`Names::Object`] and [`Names::NewObject`] is the one
/// that matters to a reader: a reference must already exist and is offered as
/// a choice, while a declaration mints an id and must not collide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum Names {
    /// A scenario object something ELSE spawned: a ship, a rock, a beacon, an
    /// area. Every one of these has to resolve against the scenario's own
    /// spawns or the handler fires at nothing.
    Object,
    /// The id a spawn DECLARES. Unique within the scenario; every
    /// [`Names::Object`] reference resolves against the set of these.
    NewObject,
    /// A scenario variable key.
    Variable,
    /// A scenario-local timer key.
    Timer,
    /// A scenario in the campaign, by its registered id.
    Scenario,
    /// A HUD objective, by the id it was posted under.
    Objective,
}
