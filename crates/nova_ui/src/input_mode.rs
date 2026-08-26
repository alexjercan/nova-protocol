//! Who owns the keyboard.
//!
//! One arbiter decides, and every keyboard system asks it. Before this, each
//! system carried its own list of the things it must not fire under - a list
//! that only ever named the collisions somebody had already hit, so anything
//! new suppressed nothing.
//!
//! Two rules cover every consumer:
//!
//! - A VERB runs in [`InputMode::Normal`] alone
//!   (`.run_if(in_input_mode(InputMode::Normal))`). Keys are verbs there and
//!   nowhere else, so a mode that takes the keyboard takes it from verbs that
//!   have never heard of it.
//! - A MODE'S OWN system runs with [`owns_or_enters`], which answers in its
//!   mode and in Normal: the key that opens a mode is pressed before the mode
//!   exists, and the key that closes it is pressed inside. That leaves the
//!   owner suppressed by exactly the modes MORE exclusive than its own.
//!
//! [`InputMode`]'s declaration order IS that exclusivity, so who wins a
//! contested frame is one enum away rather than a rule per pair.
//!
//! A mode is CLAIMED, not set: whoever holds the state that defines a mode
//! writes a [`ClaimKeyboard`] each frame it holds, and [`InputModeSystems`]
//! keeps the most exclusive claim. A claimant lives with the state it reads -
//! the editor's gallery and rebind are not names this crate could know.

use bevy::prelude::*;

use crate::widget::TextFieldFocused;

/// Glob-import surface: the mode, the claim, the arbiter's ordering handle and
/// the two run conditions consumers gate on.
pub mod prelude {
    pub use super::{
        in_input_mode, in_input_mode_at_most, owns_or_enters, take_keyboard_now, ClaimKeyboard,
        InputMode, InputModeSystems,
    };
}

/// Who the keyboard belongs to this frame.
///
/// Ordered by EXCLUSIVITY, least first: a frame with several claims resolves to
/// the greatest one. Adding a mode is therefore a decision about where it sits
/// in this list, and nothing else.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputMode {
    /// Keys are verbs: place, delete, frame, save, back out.
    #[default]
    Normal,
    /// An overlay is up and answers the keyboard itself - the editor's parts
    /// gallery. Below [`InputMode::Insert`] because a field drawn over an
    /// overlay is still a field.
    Browse,
    /// A text field has the caret, so keys are characters.
    Insert,
    /// The next key is being captured as a binding. The most exclusive mode
    /// there is: the whole point of the gesture is that the key does NOT do
    /// what it usually does, including the keys that cannot be taken back.
    Bind,
}

/// A claim on the keyboard for this frame, written by whoever holds the state
/// that defines the mode.
///
/// Written EVERY frame the claim holds. There is no release: a mode that stops
/// claiming stops owning on the next resolve.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimKeyboard(pub InputMode);

/// Ordering handle for the arbiter. Claimants run `.before` it, in `PreUpdate`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputModeSystems;

/// Whether the keyboard is in `wanted`.
///
/// An app with no arbiter has no modes, and every consumer answers: a headless
/// test of one verb does not have to stand a mode machine up to prove the verb.
pub fn in_input_mode(wanted: InputMode) -> impl Fn(Option<Res<InputMode>>) -> bool + Clone {
    move |mode: Option<Res<InputMode>>| mode.is_none_or(|mode| *mode == wanted)
}

/// Whether the keyboard is in `mode` or in a QUIETER one.
///
/// For a verb no mode up to `mode` takes the key away from. A mode is a claim
/// on the keys it reads, not a claim on every key: the parts gallery takes
/// Browse so the arrows and the letters reach its grid, and Ctrl+S is neither.
/// Insert and Bind are different in kind - `S` is a letter a builder is typing,
/// and the whole chord is a key a rebind is entitled to capture.
pub fn in_input_mode_at_most(mode: InputMode) -> impl Fn(Option<Res<InputMode>>) -> bool + Clone {
    move |current: Option<Res<InputMode>>| current.is_none_or(|current| *current <= mode)
}

/// Whether `mode`'s own systems may act: the keyboard is in `mode`, or it is
/// free for `mode` to be entered.
///
/// The gesture that ENTERS a mode is pressed while the keyboard is still
/// Normal, so an owner gated on its mode alone could never open it. Gating on
/// both leaves the owner answering to precisely the modes above its own.
pub fn owns_or_enters(mode: InputMode) -> impl Fn(Option<Res<InputMode>>) -> bool + Clone {
    move |current: Option<Res<InputMode>>| {
        current.is_none_or(|current| *current == mode || *current == InputMode::Normal)
    }
}

/// Take `mode` for the REST OF THIS FRAME, ahead of the next resolve.
///
/// A mode entered by a CLICK cannot wait for the arbiter. The click lands in
/// `Update`, the claimant that reads the state it wrote runs in the next
/// `PreUpdate`, and every verb gated on `Normal` still runs in between - so one
/// Escape could cancel the capture the click had just armed AND take the rung
/// below it, in the same press.
///
/// The claim still has to be written every frame from the state, the way every
/// other claim is. This only closes the gap on the frame that opens the mode.
///
/// RAISES, never lowers: a frame already in a more exclusive mode keeps it, and
/// a mode ENDS by no longer being claimed.
pub fn take_keyboard_now(current: &mut InputMode, mode: InputMode) {
    if mode > *current {
        *current = mode;
    }
}

/// Keep the most exclusive claim of the frame, and Normal with none.
fn resolve_input_mode(mut claims: MessageReader<ClaimKeyboard>, mut mode: ResMut<InputMode>) {
    let claimed = claims
        .read()
        .map(|claim| claim.0)
        .max()
        .unwrap_or(InputMode::Normal);
    mode.set_if_neq(claimed);
}

/// A focused field claims [`InputMode::Insert`]. This crate's own claimant -
/// the field is a widget, so its mode is the widget layer's to declare.
fn a_focused_field_claims_the_keyboard(
    focused: Query<(), With<TextFieldFocused>>,
    mut claims: MessageWriter<ClaimKeyboard>,
) {
    if !focused.is_empty() {
        claims.write(ClaimKeyboard(InputMode::Insert));
    }
}

/// Wire the arbiter and this crate's claimant.
pub(crate) fn build(app: &mut App) {
    app.init_resource::<InputMode>();
    app.add_message::<ClaimKeyboard>();
    app.add_systems(
        PreUpdate,
        (
            a_focused_field_claims_the_keyboard.before(InputModeSystems),
            resolve_input_mode.in_set(InputModeSystems),
        ),
    );
}

#[cfg(test)]
mod tests;
