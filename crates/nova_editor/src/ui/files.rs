//! Save As and Open: the in-game menu that names a save slot, and the list
//! that picks one.
//!
//! No OS file dialog, on purpose. A save is a MOD - it lands in the same cache
//! a downloaded one does, under an id derived from the name - so what a
//! builder picks from is the set of ranges they have saved, not a directory
//! tree. It also means the same window works where there is no directory tree
//! to browse: the web has no local cache at all, and the list says so where
//! the rows would be.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use nova_ui::{
    prelude::{
        button, text_field, themed_button, ButtonSpec, TextFieldSpec, TextFieldValue, UiSkin,
        UiText,
    },
    theme,
};

use crate::{
    bundle::{
        bundle_id, saved_bundles, DocumentSlot, FileRequest, FileWindowKind, FileWindowRequest,
        NameProblem, SaveSlot,
    },
    config::EditorSays,
    gallery::GalleryState,
    node::{EditContext, ScenarioNode},
    ui::window::{window_frame, EditorWindowLayer, TOP_MARGIN, WINDOW_W},
};

#[cfg(test)]
mod tests;

/// How long a save name may be: room for a phrase a builder will recognise in
/// a list, and no more than an id derived from it can carry as a directory
/// name.
const NAME_CHARS: usize = 48;

/// An open file window, and what it is there to do.
///
/// One at a time, like every other window - and unlike the others it is not
/// bound to an Inspector row, so it stands until it is answered or closed.
#[derive(Component, Clone, Copy)]
pub(crate) struct FileWindow(pub(crate) FileWindowKind);

/// Anything whose press takes the window down with it.
#[derive(Component)]
pub(crate) struct FileAnswer;

/// The slots the window listed when it opened.
///
/// Read ONCE, at open. The list is disk, and a readout that re-read it every
/// frame would be a file system walk per frame to answer a question that only
/// changes when this window writes.
#[derive(Component)]
pub(crate) struct KnownBundles(pub(crate) Vec<SaveSlot>);

/// The name field of a Save As window.
#[derive(Component)]
pub(crate) struct SaveNameField;

/// The line under it: which slot that name lands in, and whether a save is
/// already there.
#[derive(Component)]
pub(crate) struct SaveIdReadout;

/// The button that writes, greyed while the name makes no id.
#[derive(Component)]
pub(crate) struct SaveConfirmButton;

/// One saved slot, offered as a row.
#[derive(Component, Clone)]
pub(crate) struct BundleRow(pub(crate) SaveSlot);

/// Put the asked-for window up.
pub(crate) fn open_file_window(
    mut commands: Commands,
    mut asked: ResMut<FileWindowRequest>,
    skin: Res<UiSkin>,
    layer: Option<Single<Entity, With<EditorWindowLayer>>>,
    open: Query<(), With<FileWindow>>,
    document: Res<DocumentSlot>,
    context: Res<EditContext>,
    q_settings: Query<&ScenarioNode>,
    screen: Option<Single<&Window>>,
    // Optional: a headless rig stands the layer up without the parts browser
    // behind it.
    gallery: Option<ResMut<GalleryState>>,
) {
    let mut gallery = gallery;
    let Some(kind) = asked.0.take() else {
        return;
    };
    let Some(layer) = layer else {
        return;
    };
    if !open.is_empty() {
        return;
    }
    // The gallery goes down first. It is a MODE, not a panel - the editor's
    // whole chrome, this layer included, is hidden while it is up - so a window
    // raised under one would be a window nobody could see or answer. Ctrl+S
    // from under the gallery on a document with no file is exactly that press.
    if let Some(gallery) = gallery.as_mut() {
        gallery.open = false;
        gallery.focused = false;
    }
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let at = Vec2::new(((size.x - WINDOW_W) * 0.5).max(8.0), TOP_MARGIN);
    let name = offered_name(&document, &context, &q_settings);
    let bundles = saved_bundles();
    commands
        .entity(*layer)
        .with_children(|layer| spawn_file_window(layer, kind, &name, bundles, at, *skin));
}

/// The name a Save As opens on: the one the document was last saved under, or
/// the range's own while it has never been saved.
///
/// The two are the same string in a document that has been saved once, so what
/// this really says is that a first save is offered the name the builder
/// already gave the range rather than a blank field.
fn offered_name(
    document: &DocumentSlot,
    context: &EditContext,
    q_settings: &Query<&ScenarioNode>,
) -> String {
    if let Some(slot) = document.0.as_ref() {
        return slot.name.clone();
    }
    context
        .scenario()
        .and_then(|scenario| q_settings.get(scenario).ok())
        .map(|settings| settings.name.clone())
        .unwrap_or_default()
}

/// The window: a name to save under, the saves already there, and the answers.
fn spawn_file_window(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    kind: FileWindowKind,
    name: &str,
    bundles: Vec<SaveSlot>,
    at: Vec2,
    skin: UiSkin,
) {
    let title = match kind {
        FileWindowKind::SaveAs => "Save As",
        FileWindowKind::Open => "Open",
    };
    let listed = bundles.clone();
    let window = window_frame(
        layer,
        "File Window",
        title,
        at,
        skin,
        (FileWindow(kind), KnownBundles(bundles)),
        |body, _window| {
            if kind == FileWindowKind::Open {
                body.spawn((
                    Name::new("File Window Warning"),
                    UiText,
                    Text::new(
                        "This replaces everything on the stage with the range you pick. There is \
                         no undo.",
                    ),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR),
                ));
            }
            if kind == FileWindowKind::SaveAs {
                body.spawn((
                    Name::new("Save Name Label"),
                    UiText,
                    Text::new("Name"),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ));
                body.spawn((
                    Name::new("Save Name Field"),
                    SaveNameField,
                    text_field(
                        TextFieldSpec::new(name.to_string())
                            .max_chars(NAME_CHARS)
                            .placeholder("Name this range")
                            .dense(),
                    ),
                ));
                // Blank until `sync_save_name` fills it, which is the frame
                // after this one: the id belongs to what is TYPED, and there is
                // one place that works it out.
                body.spawn((
                    Name::new("Save Id Readout"),
                    SaveIdReadout,
                    UiText,
                    Text::new(String::new()),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ));
            }
            body.spawn((
                Name::new("Saved Ranges Label"),
                UiText,
                Text::new("SAVED RANGES"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::AMBER_NOVA),
                Node {
                    margin: UiRect::top(px(6)),
                    ..default()
                },
            ));
            if listed.is_empty() {
                body.spawn((
                    Name::new("Saved Ranges Empty"),
                    UiText,
                    Text::new(empty_line(kind)),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ));
            }
            for slot in &listed {
                let mut row = body.spawn((
                    Name::new(format!("Bundle Row {}", slot.id)),
                    BundleRow(slot.clone()),
                    // The rail's own row, like every other list the editor
                    // offers to pick from.
                    button(ButtonSpec::new(slot.name.clone()).block()),
                ));
                row.observe(on_bundle_row);
                // A row IS the answer when it opens one. In Save As it is a
                // name to write over, so the window stays up and the Save
                // button is still the press that writes.
                if kind == FileWindowKind::Open {
                    row.insert(FileAnswer);
                }
                // The id under the name, because the name alone does not tell
                // two rows apart: ids are derived, so a builder who saved
                // "My Range" and "My-Range!" has two files with one label on
                // them, and the id is the half that differs.
                body.spawn((
                    Name::new(format!("Bundle Row Id {}", slot.id)),
                    UiText,
                    Text::new(slot.id.clone()),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                    Node {
                        margin: UiRect::bottom(px(4)),
                        ..default()
                    },
                ));
            }
        },
    );
    // OUTSIDE the frame's scrolling body, on purpose: the list under the name
    // field grows with every range a builder saves, and answers that scrolled
    // away with it would leave a window with no visible way to press Save.
    layer
        .commands()
        .entity(window)
        .with_children(|frame| spawn_answers(frame, kind));
}

/// Cancel, and - in a Save As window - the press that writes.
fn spawn_answers(frame: &mut RelatedSpawnerCommands<ChildOf>, kind: FileWindowKind) {
    frame
        .spawn((
            Name::new("File Window Answers"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::all(px(10)),
                border: UiRect::top(px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::PHOSPHOR.with_alpha(0.16)),
        ))
        .with_children(|answers| {
            // The safe answer FIRST and named for what it does, the way the
            // confirm window orders its two.
            // A slot each, because `themed_button` is percent(100) wide - the
            // growing is the slot's job, and the marker and the observer belong
            // on the BUTTON, which is what emits the press.
            answers
                .spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|slot| {
                    slot.spawn((
                        Name::new("File Cancel Button"),
                        FileAnswer,
                        themed_button("Cancel"),
                    ));
                });
            if kind != FileWindowKind::SaveAs {
                return;
            }
            answers
                .spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|slot| {
                    slot.spawn((
                        Name::new("File Save Button"),
                        SaveConfirmButton,
                        FileAnswer,
                        themed_button("Save"),
                        observe(on_save),
                    ));
                });
        });
}

/// What the list says when it has no rows.
#[cfg(not(target_arch = "wasm32"))]
fn empty_line(kind: FileWindowKind) -> &'static str {
    match kind {
        FileWindowKind::SaveAs => "Nothing saved yet - the name above makes the first one.",
        FileWindowKind::Open => "Nothing saved yet.",
    }
}

/// On the web both lists are empty for one reason, and it is not that the
/// builder has saved nothing.
#[cfg(target_arch = "wasm32")]
fn empty_line(_kind: FileWindowKind) -> &'static str {
    "Saving is not available on the web yet."
}

/// Say where the typed name lands, and grey Save while it lands nowhere.
///
/// Unconditional rather than `Changed`-gated: at most one window is open, the
/// work is a slug of forty-eight characters, and the readout has to be right
/// on the frame the window opens as well as on the frames it is typed into.
/// Nothing is written unless it differs, so an untouched field costs no
/// re-layout.
pub(crate) fn sync_save_name(
    mut commands: Commands,
    fields: Query<&TextFieldValue, With<SaveNameField>>,
    known: Query<&KnownBundles>,
    mut readouts: Query<(&mut Text, &mut TextColor), With<SaveIdReadout>>,
    buttons: Query<(Entity, Has<InteractionDisabled>), With<SaveConfirmButton>>,
) {
    let (Some(typed), Some(bundles)) = (fields.iter().next(), known.iter().next()) else {
        return;
    };
    let id = bundle_id(typed.0.trim());
    let (line, colour) = match id.as_deref() {
        Err(NameProblem::Unusable) => ("A name needs a letter or a digit.".to_string(), theme::RED),
        // The one derived id the editor keeps for itself. Said here because
        // the list below CANNOT say it: the sandbox is a registered scenario
        // rather than a file, so it is in no row to collide with.
        Err(NameProblem::Reserved) => (
            "That name is the editor's own. Pick another.".to_string(),
            theme::RED,
        ),
        Ok(id) => match bundles.0.iter().find(|slot| slot.id == id) {
            // The collision, said before the press rather than after it: two
            // names that differ only in punctuation derive one id, and the
            // second one written silently replaces the first.
            Some(taken) => (
                format!("overwrites \"{}\" ({id})", taken.name),
                theme::AMBER_NOVA,
            ),
            None => (format!("saves as {id}"), theme::PHOSPHOR_MUTED),
        },
    };
    for (mut text, mut paint) in &mut readouts {
        if text.0 != line {
            text.0.clone_from(&line);
        }
        if paint.0 != colour {
            paint.0 = colour;
        }
    }
    for (button, greyed) in &buttons {
        match (id.is_ok(), greyed) {
            (true, true) => {
                commands.entity(button).remove::<InteractionDisabled>();
            }
            (false, false) => {
                commands.entity(button).insert(InteractionDisabled);
            }
            _ => {}
        }
    }
}

/// A row was pressed: it is the answer in an Open window, and a name to write
/// over in a Save As one.
pub(crate) fn on_bundle_row(
    activate: On<Activate>,
    rows: Query<&BundleRow>,
    windows: Query<&FileWindow>,
    mut fields: Query<&mut TextFieldValue, With<SaveNameField>>,
    mut request: ResMut<FileRequest>,
) {
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    match window.0 {
        FileWindowKind::SaveAs => {
            for mut value in &mut fields {
                value.0.clone_from(&row.0.name);
            }
        }
        FileWindowKind::Open => *request = FileRequest::Open(row.0.id.clone()),
    }
}

/// Write the document into the slot the typed name derives.
///
/// The name goes onto the RANGE as well. A document has one name - the one the
/// Scenarios picker will list it under and the one this window offers back
/// next time - and keeping a file name and a scenario name in step by hand is
/// bookkeeping the editor can do.
pub(crate) fn on_save(
    _activate: On<Activate>,
    fields: Query<&TextFieldValue, With<SaveNameField>>,
    context: Res<EditContext>,
    mut settings: Query<&mut ScenarioNode>,
    mut request: ResMut<FileRequest>,
    mut says: EditorSays,
) {
    let Some(typed) = fields.iter().next() else {
        return;
    };
    let name = typed.0.trim().to_string();
    let id = match bundle_id(&name) {
        Ok(id) => id,
        Err(NameProblem::Unusable) => {
            says.refuse("a name needs a letter or a digit");
            return;
        }
        Err(NameProblem::Reserved) => {
            says.refuse("that name is the editor's own - pick another");
            return;
        }
    };
    if let Some(mut node) = context
        .scenario()
        .and_then(|scenario| settings.get_mut(scenario).ok())
    {
        node.name.clone_from(&name);
    }
    *request = FileRequest::SaveAs(SaveSlot { id, name });
}

/// Take the window down, whichever answer was pressed.
///
/// Central, so a row's own observer never has to know it was in a window.
pub(crate) fn close_file_window(
    activate: On<Activate>,
    answers: Query<(), With<FileAnswer>>,
    windows: Query<Entity, With<FileWindow>>,
    mut commands: Commands,
) {
    if !answers.contains(activate.entity) {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
}
