//! The gallery keyboard: opening, selection, paging, focus, the pick key and
//! the filter field.
//!
//! Letters reach the FILTER only while the filter field itself is focused.
//! Swallowing every keystroke made the field the only thing the keyboard could
//! do, which is the wrong trade for a browser you are meant to pass through:
//! unfocused, the letters are free for the shortcuts below. Change this module
//! when the browse keys change.

use bevy::{input::keyboard::KeyboardInput, picking::hover::Hovered, prelude::*};
use nova_ship::prelude::*;

use crate::{
    config::SectionChoice,
    gallery::{catalog, ui::GalleryCell, GalleryState, COLS, PAGE},
};

/// Opens and closes the gallery from the editor.
///
/// Tab, because that is already the game's "open the panel" key. It is free
/// here: the NOVA OS it opens in flight needs a ship to be the computer of, and
/// the editor's build mode has none (see `toggle_nova_os`).
pub(crate) const GALLERY_TOGGLE_KEY: KeyCode = KeyCode::Tab;

/// Arms whatever part is under the pointer - the same pipette the build view
/// uses on a placed section.
pub(crate) const GALLERY_PICK_KEY: KeyCode = KeyCode::KeyQ;

/// Puts the caret in the filter field, the way `/` does in every list that has
/// ever had a search box.
pub(crate) const GALLERY_FILTER_KEY: KeyCode = KeyCode::Slash;

/// Open or close the gallery. Runs whether or not it is up, unlike the rest of
/// this module.
pub(crate) fn toggle_gallery(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GalleryState>) {
    if !keys.just_pressed(GALLERY_TOGGLE_KEY) {
        return;
    }
    if state.open {
        *state = GalleryState {
            open: false,
            ..state.clone()
        };
    } else {
        // A fresh browse starts unfiltered, exactly as the rail row opens it.
        *state = GalleryState {
            open: true,
            category: state.category,
            ..default()
        };
    }
}

/// Drive the open gallery from the keyboard.
pub(crate) fn gallery_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    sections: Res<GameSections>,
    hovered: Query<(&GalleryCell, &Hovered)>,
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

    // Escape backs out one step at a time: out of the filter field, then out of
    // the focus card, then out of the gallery.
    if keys.just_pressed(KeyCode::Escape) {
        if next.filter_focused {
            next.filter_focused = false;
        } else if next.focused {
            next.focused = false;
        } else {
            next.open = false;
        }
    } else if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        if next.filter_focused {
            // Enter in a search box means "the top hit", so it leaves the field
            // AND opens what the filter narrowed to. Leaving the field alone
            // would cost a second Enter to see the one part you just typed the
            // name of.
            next.filter_focused = false;
            next.focused = !listed.is_empty();
        } else if next.focused {
            if let Some(id) = next.selected_id(&sections) {
                *choice = SectionChoice::Section(id);
            }
            next.open = false;
            next.focused = false;
        } else if !listed.is_empty() {
            next.focused = true;
        }
    }

    // The pipette: point at a tile, take that part, go back to building. The
    // pointer already says which part you mean, so this is the whole gesture -
    // no focus card, no Place button, no second click.
    if !next.filter_focused && keys.just_pressed(GALLERY_PICK_KEY) {
        if let Some(cell) = hovered
            .iter()
            .find(|(_, hovered)| hovered.get())
            .map(|(cell, _)| cell.listed)
        {
            next.selected = cell;
            if let Some(id) = next.selected_id(&sections) {
                *choice = SectionChoice::Section(id);
                next.open = false;
                next.focused = false;
            }
        }
    }

    if !next.filter_focused && keys.just_pressed(GALLERY_FILTER_KEY) {
        next.filter_focused = true;
    }

    // Navigation stays live while the filter is focused: arrows carry no text,
    // so nothing is ambiguous, and narrowing then arrowing is one gesture.
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

    // Drained every frame, focused or not: an unread queue would replay the
    // keys pressed while browsing into the field the moment it took focus.
    let pressed: Vec<_> = typed
        .read()
        .filter(|event| event.state.is_pressed())
        .map(|event| (event.key_code, event.text.clone()))
        .collect();
    // Gated on the focus the field had BEFORE this frame, so the `/` that
    // focuses it is not also the first character typed into it.
    if state.filter_focused {
        let pressed = pressed.iter().map(|(key, text)| (*key, text.as_deref()));
        if edit_filter(&mut next, pressed) {
            // A narrower list can leave the selection past its end.
            let listed = catalog::browsable(&sections, next.category, &next.filter);
            next.step(0, listed.len());
        }
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

    /// A world with the gallery's keyboard wired to it, so the tests below
    /// drive the real system rather than its parts.
    fn gallery_app(state: GalleryState) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_message::<KeyboardInput>();
        app.insert_resource(state);
        app.insert_resource(SectionChoice::None);
        app.insert_resource(GameSections(vec![
            SectionConfig {
                base: BaseSectionConfig {
                    id: "hull".to_string(),
                    name: "Hull".to_string(),
                    ..default()
                },
                kind: SectionKind::Hull(HullSectionConfig::default()),
            },
            SectionConfig {
                base: BaseSectionConfig {
                    id: "thruster".to_string(),
                    name: "Thruster".to_string(),
                    ..default()
                },
                kind: SectionKind::Thruster(ThrusterSectionConfig::default()),
            },
        ]));
        app.add_systems(Update, (toggle_gallery, gallery_keyboard).chain());
        app
    }

    fn tap(app: &mut App, key: KeyCode, text: Option<&str>) {
        if let Some(text) = text {
            app.world_mut().write_message(KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Character(text.into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some(text.into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    /// Tab opens the gallery from the build view and closes it again. It is the
    /// game's "open the panel" key, and it is free in the editor because the
    /// NOVA OS it opens in flight needs a ship to be the computer of.
    #[test]
    fn tab_opens_and_closes_the_gallery() {
        let mut app = gallery_app(GalleryState::default());

        tap(&mut app, KeyCode::Tab, None);
        assert!(app.world().resource::<GalleryState>().open);

        tap(&mut app, KeyCode::Tab, None);
        assert!(!app.world().resource::<GalleryState>().open);
    }

    /// Letters are SHORTCUTS until the filter field is focused. Typing into a
    /// grid that swallows every keystroke leaves no room for a pick key, which
    /// is the whole reason the field takes focus at all.
    #[test]
    fn letters_reach_the_filter_only_once_the_field_is_focused() {
        let mut app = gallery_app(GalleryState {
            open: true,
            ..default()
        });

        tap(&mut app, KeyCode::KeyH, Some("h"));
        assert_eq!(
            app.world().resource::<GalleryState>().filter,
            "",
            "an unfocused field must not eat the keyboard"
        );

        // `/` takes focus, and is not itself typed into the field.
        tap(&mut app, KeyCode::Slash, Some("/"));
        assert!(app.world().resource::<GalleryState>().filter_focused);
        assert_eq!(
            app.world().resource::<GalleryState>().filter,
            "",
            "the key that focuses the field is not a character in it"
        );

        tap(&mut app, KeyCode::KeyH, Some("h"));
        assert_eq!(app.world().resource::<GalleryState>().filter, "h");

        // Escape backs out of the field first, and only then out of the gallery.
        tap(&mut app, KeyCode::Escape, None);
        let state = app.world().resource::<GalleryState>();
        assert!(!state.filter_focused);
        assert!(
            state.open,
            "the first Escape leaves the field, not the browse"
        );
    }

    /// The pipette: point at a tile, press Q, and you are back in the build view
    /// holding that part. No focus card, no Place button, no second click.
    #[test]
    fn q_takes_the_part_under_the_pointer_and_leaves() {
        let mut app = gallery_app(GalleryState {
            open: true,
            ..default()
        });
        // The second tile is under the pointer, not the selected one - so a
        // pass that read the SELECTION instead of the hover would take "hull".
        app.world_mut()
            .spawn((GalleryCell { listed: 1 }, Hovered(true)));
        app.world_mut()
            .spawn((GalleryCell { listed: 0 }, Hovered(false)));

        tap(&mut app, KeyCode::KeyQ, Some("q"));

        assert_eq!(
            *app.world().resource::<SectionChoice>(),
            SectionChoice::Section("thruster".to_string()),
            "Q must arm the part under the pointer"
        );
        assert!(
            !app.world().resource::<GalleryState>().open,
            "and hand the builder straight back to placing it"
        );
    }

    /// Q is a character too: with the caret in the filter it types, and the
    /// gallery stays up.
    #[test]
    fn q_types_while_the_filter_has_the_caret() {
        let mut app = gallery_app(GalleryState {
            open: true,
            filter_focused: true,
            ..default()
        });
        app.world_mut()
            .spawn((GalleryCell { listed: 1 }, Hovered(true)));

        tap(&mut app, KeyCode::KeyQ, Some("q"));

        assert_eq!(app.world().resource::<GalleryState>().filter, "q");
        assert_eq!(
            *app.world().resource::<SectionChoice>(),
            SectionChoice::None
        );
        assert!(app.world().resource::<GalleryState>().open);
    }
}
