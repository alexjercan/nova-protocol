//! The gallery keyboard: selection, paging, focus, and the filter field.
//!
//! While the overlay is up it owns the keyboard - the free-fly rig is parked
//! (`scene`) and the editor's rebind capture is gated off - so typing filters
//! instead of flying. Change this module when the browse keys change.

use bevy::{input::keyboard::KeyboardInput, prelude::*};
use nova_ship::prelude::*;

use crate::{
    config::SectionChoice,
    gallery::{catalog, GalleryState, COLS, PAGE},
};

/// Drive the gallery from the keyboard.
pub(crate) fn gallery_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    sections: Res<GameSections>,
    mut state: ResMut<GalleryState>,
    mut choice: ResMut<SectionChoice>,
) {
    if !state.open {
        // Drain, so a keypress made before the gallery opened is not replayed
        // into the filter on the frame it opens.
        typed.clear();
        return;
    }

    let listed = catalog::browsable(&sections, state.category, &state.filter);
    let mut next = state.clone();

    if keys.just_pressed(KeyCode::Escape) {
        if next.focused {
            next.focused = false;
        } else {
            next.open = false;
        }
    } else if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        if next.focused {
            if let Some(id) = next.selected_id(&sections) {
                *choice = SectionChoice::Section(id);
            }
            next.open = false;
            next.focused = false;
        } else if !listed.is_empty() {
            next.focused = true;
        }
    }

    if !next.focused {
        for (key, delta) in [
            (KeyCode::ArrowLeft, -1),
            (KeyCode::ArrowRight, 1),
            (KeyCode::ArrowUp, -(COLS as isize)),
            (KeyCode::ArrowDown, COLS as isize),
            (KeyCode::PageUp, -(PAGE as isize)),
            (KeyCode::PageDown, PAGE as isize),
        ] {
            if keys.just_pressed(key) {
                next.step(delta, listed.len());
            }
        }
    } else {
        // The focus card cycles through the same filtered list.
        for (key, delta) in [(KeyCode::ArrowLeft, -1), (KeyCode::ArrowRight, 1)] {
            if keys.just_pressed(key) {
                next.step(delta, listed.len());
            }
        }
    }

    let pressed = typed
        .read()
        .filter(|event| event.state.is_pressed())
        .map(|event| (event.key_code, event.text.as_deref()));
    if edit_filter(&mut next, pressed) {
        // A narrower list can leave the selection past its end.
        let listed = catalog::browsable(&sections, next.category, &next.filter);
        next.step(0, listed.len());
    }

    if *state != next {
        *state = next;
    }
}

/// Apply typed characters to the filter, returning whether it changed.
///
/// Takes `(key, produced text)` pairs rather than the events themselves: the
/// text is what a keypress MEANS under the player's layout, and the pure form
/// is what the tests drive. Control keys (arrows, Enter, Esc) produce no text
/// and the navigation above already owns them.
fn edit_filter<'a>(
    state: &mut GalleryState,
    keys: impl Iterator<Item = (KeyCode, Option<&'a str>)>,
) -> bool {
    let mut changed = false;
    for (key, text) in keys {
        if key == KeyCode::Backspace {
            changed |= state.filter.pop().is_some();
            continue;
        }
        let Some(text) = text else {
            continue;
        };
        for character in text.chars().filter(|character| !character.is_control()) {
            state.filter.push(character);
            changed = true;
        }
    }
    if changed {
        // A new filter renumbers the list; keep the selection on the first
        // match rather than on whatever now sits at the old index.
        state.selected = 0;
        state.focused = false;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typing builds the filter and returns the selection to the first match;
    /// backspace takes it back. Control keys carry no text and must not land in
    /// the field.
    #[test]
    fn typing_edits_the_filter_and_resets_the_selection() {
        let mut state = GalleryState {
            selected: 5,
            focused: true,
            ..default()
        };
        let typed = [(KeyCode::KeyR, Some("r")), (KeyCode::KeyA, Some("a"))];
        assert!(edit_filter(&mut state, typed.into_iter()));
        assert_eq!(state.filter, "ra");
        assert_eq!(state.selected, 0, "a new filter re-seeds the selection");
        assert!(!state.focused, "and drops back to the grid");

        let navigation = [(KeyCode::Escape, None), (KeyCode::ArrowLeft, None)];
        assert!(
            !edit_filter(&mut state, navigation.into_iter()),
            "a text-less key is navigation, not typing"
        );
        assert_eq!(state.filter, "ra");

        let backspace = [(KeyCode::Backspace, None)];
        assert!(edit_filter(&mut state, backspace.into_iter()));
        assert_eq!(state.filter, "r");
    }

    /// Backspace on an empty field is not a change - it must not re-seed the
    /// selection out from under the player.
    #[test]
    fn backspace_on_an_empty_filter_changes_nothing() {
        let mut state = GalleryState {
            selected: 3,
            ..default()
        };
        let backspace = [(KeyCode::Backspace, None)];
        assert!(!edit_filter(&mut state, backspace.into_iter()));
        assert_eq!(state.selected, 3);
    }
}
