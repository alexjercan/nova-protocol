//! The action TABLE: one row per action, and the macro that turns the table
//! into everything the rest of the engine reads off it.
//!
//! Before this existed, one action was fifteen edits, nine of them exhaustive
//! matches over the same enum in two crates. A row here now generates the enum
//! arm, the dispatch, the creative-map class, the reflected payload accessor
//! and the whole authoring-menu surface, so what is left to write by hand is
//! the payload struct, the editor's stock value, the lint and the docs - four
//! places, each of which carries information the table cannot.
//!
//! The row is deliberately not a derive. A derive would put the label and the
//! class on the payload struct, and the payload struct is shared vocabulary; a
//! table read top to bottom is also the only place the whole action set is
//! visible at once.
//!
//! [`scenario_actions`] expands at its invocation site in
//! [`super`](crate::actions), and reads `EventAction`, `GameEventInfo`,
//! `NovaEventWorld` and every payload type from that scope.

/// Whether an action's payload has an inspector.
///
/// `Reflect` is the answer for every action but one: the editor draws its
/// fields from the reflection. `Opaque` is `Sequence`, whose payload is a beat
/// chain the editor lifts into its own tree of nodes and draws itself.
macro_rules! action_payload {
    (Reflect, $config:expr) => {
        Some($config as &dyn ::bevy::reflect::PartialReflect)
    };
    (Opaque, $config:expr) => {{
        let _ = $config;
        None
    }};
}

/// The `&mut` half of [`action_payload`].
macro_rules! action_payload_mut {
    (Reflect, $config:expr) => {
        Some($config as &mut dyn ::bevy::reflect::PartialReflect)
    };
    (Opaque, $config:expr) => {{
        let _ = $config;
        None
    }};
}

/// An action's own creative-map class, as the name the badge reports.
///
/// The three answers are the whole question a new action has to answer:
/// `Bookkeeping` touches only scenario state and presentation, `Injection`
/// reaches into the simulation in a way playing could not have produced, and
/// `Nested` means the class is whatever the actions it schedules are worth.
macro_rules! action_injection {
    (Bookkeeping, $name:expr) => {
        None
    };
    (Nested, $name:expr) => {
        None
    };
    (Injection, $name:expr) => {
        Some($name)
    };
}

/// Declare the closed action vocabulary.
///
/// Rows are written in the order an authoring menu lists them: the mission
/// surface first (what a player is told to do), then the world, then the ships
/// in it, then the run's own flow, and the authoring aids last. That order is
/// [`ActionTag::ALL`], so the menu has no list of its own to drift from this
/// one.
///
/// Generates [`EventActionConfig`](crate::actions::EventActionConfig), its
/// dispatch, [`ActionTag`](crate::actions::ActionTag) and the four things every
/// reader wants off a row: its RON name, its menu label, the stem a minted id
/// is named after, and its injection class.
macro_rules! scenario_actions {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident($payload:ty) {
                label: $label:literal,
                stem: $stem:literal,
                effect: $effect:ident,
                inspect: $inspect:ident,
            }
        ),* $(,)?
    ) => {
        /// What a handler does when it fires: one entry in the RON `actions`
        /// list, run in order after every filter passes.
        ///
        /// Declared by [`scenario_actions`] in
        /// [`registry`](crate::actions::registry); add an action there.
        #[derive(Clone, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum EventActionConfig {
            $(
                $(#[$meta])*
                $variant($payload),
            )*
        }

        impl EventAction<NovaEventWorld> for EventActionConfig {
            fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
                match self {
                    $(
                        EventActionConfig::$variant(config) => config.action(world, info),
                    )*
                }
            }
        }

        /// Which action, without the payload: the row of the table, as a value.
        ///
        /// Everything an authoring surface wants to know about an action
        /// BEFORE it has one - what to call it in a menu, what to name an id it
        /// mints, what order to list it in - hangs off here, so a new action is
        /// listed, labelled and named by the same row that declares it.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
        pub enum ActionTag {
            $(
                $(#[$meta])*
                $variant,
            )*
        }

        impl ActionTag {
            /// How many actions there are.
            pub const COUNT: usize = [$( stringify!($variant), )*].len();

            /// Every action a handler can be given, in menu order.
            pub const ALL: [ActionTag; Self::COUNT] = [
                $( ActionTag::$variant, )*
            ];

            /// The variant name, which is also the RON tag.
            pub fn name(self) -> &'static str {
                match self {
                    $( ActionTag::$variant => stringify!($variant), )*
                }
            }

            /// The row label an authoring menu lists this action under.
            pub fn label(self) -> &'static str {
                match self {
                    $( ActionTag::$variant => $label, )*
                }
            }

            /// The stem a minted id is named after.
            pub fn stem(self) -> &'static str {
                match self {
                    $( ActionTag::$variant => $stem, )*
                }
            }

            /// This action's OWN injection name, ignoring anything it
            /// schedules; `None` for bookkeeping and for a nesting action whose
            /// class is its steps'.
            pub fn injection(self) -> Option<&'static str> {
                match self {
                    $(
                        ActionTag::$variant => $crate::actions::registry::action_injection!(
                            $effect,
                            stringify!($variant)
                        ),
                    )*
                }
            }

            /// The action named by its RON tag, or `None` for a name no row
            /// declares.
            pub fn named(name: &str) -> Option<ActionTag> {
                ActionTag::ALL.iter().copied().find(|tag| tag.name() == name)
            }
        }

        impl EventActionConfig {
            /// Which action this is.
            pub fn tag(&self) -> ActionTag {
                match self {
                    $( EventActionConfig::$variant(_) => ActionTag::$variant, )*
                }
            }

            /// This action's payload as a reflected value, for an inspector to
            /// draw; `None` for an action the editor draws itself.
            pub fn payload(&self) -> Option<&dyn ::bevy::reflect::PartialReflect> {
                match self {
                    $(
                        EventActionConfig::$variant(config) =>
                            $crate::actions::registry::action_payload!($inspect, config),
                    )*
                }
            }

            /// The same payload, for writing.
            pub fn payload_mut(&mut self) -> Option<&mut dyn ::bevy::reflect::PartialReflect> {
                match self {
                    $(
                        EventActionConfig::$variant(config) =>
                            $crate::actions::registry::action_payload_mut!($inspect, config),
                    )*
                }
            }
        }
    };
}

pub(crate) use action_injection;
pub(crate) use action_payload;
pub(crate) use action_payload_mut;
pub(crate) use scenario_actions;
