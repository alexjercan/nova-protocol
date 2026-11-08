//! The small windows around a mod: name it, copy it, rename a scenario, say
//! where a package went, and pick a file by path.
//!
//! No OS file dialog, on purpose: a mod lives in the same cache a downloaded
//! one does, so naming is all a save needs. The path window is the one place a
//! builder reaches outside it, and it is plain directories and a typed path, so
//! it needs no native picker. A new use adds a [`PathPurposeType`] row.

use std::path::{Path, PathBuf};

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use nova_ship::prelude::GalleryState;
use nova_ui::{
    prelude::{
        button, text_field, themed_button, ButtonSpec, TextFieldSpec, TextFieldValue, UiText,
    },
    theme,
    theme::{density, ActiveUiTheme, UiColor},
    widget::{ButtonCueType, ThemedBorder, ThemedText},
};

use crate::{
    asset::{files::import_file, verbs::import_mesh},
    bundle::{FileWindowKind, FileWindowRequest},
    project::{
        ids::{is_taken, project_id, publish_id, NameProblem},
        package::import_package,
        store::local_ids,
        verbs::{name_project, pressed, save_copy},
        ModProject,
    },
    ui::window::{window_frame, EditorWindowLayer, TOP_MARGIN, WINDOW_W},
};

#[cfg(test)]
mod tests;

/// How long a name may be: room for a phrase a builder will recognise in a list,
/// and no more than an id derived from it can carry as a directory name.
pub(crate) const NAME_CHARS: usize = 48;

/// How long a typed path may be.
const PATH_CHARS: usize = 512;

/// What a path window picks a file for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathPurposeType {
    /// A mod package to import.
    Package,
    /// A model for the open section.
    Mesh,
    /// Any file, for the mod's folder.
    File,
}

impl PathPurposeType {
    /// The window's title.
    fn title(self) -> &'static str {
        match self {
            Self::Package => "Import Package",
            Self::Mesh => "Import Mesh",
            Self::File => "Import File",
        }
    }

    /// The file extension it lists. Empty lists every file.
    fn extension(self) -> &'static str {
        match self {
            Self::Package => "zip",
            Self::Mesh => "glb",
            Self::File => "",
        }
    }

    /// Use the picked file.
    fn run(self, world: &mut World, path: &str) {
        match self {
            Self::Package => import_package(world, path),
            Self::Mesh => import_mesh(world, path),
            Self::File => import_file(world, path),
        }
    }
}

/// An open file window, and what it is there to do.
///
/// One at a time, like every other window - and unlike the others it is not
/// bound to an Inspector row, so it stands until it is answered or closed.
#[derive(Component, Clone)]
pub(crate) struct FileWindow(pub(crate) FileWindowKind);

/// Anything whose press takes the window down with it.
#[derive(Component)]
pub(crate) struct FileAnswer;

/// The local ids the window saw when it opened.
///
/// Read ONCE, at open. The list is disk, and a readout that re-read it every
/// frame would be a file system walk per frame to answer a question that only
/// changes when this window writes.
#[derive(Component)]
pub(crate) struct KnownMods(pub(crate) Vec<String>);

/// The name field of a naming window.
#[derive(Component)]
pub(crate) struct SaveNameField;

/// The line under it: the id that name makes, and whether it is free.
#[derive(Component)]
pub(crate) struct SaveIdReadout;

/// The button that writes, greyed while the name cannot be used.
#[derive(Component)]
pub(crate) struct SaveConfirmButton;

/// The typed path of a path window.
#[derive(Component)]
pub(crate) struct PathField;

/// The list of a path window, and the folder it shows.
#[derive(Component, Default)]
pub(crate) struct PathList(Option<PathBuf>);

/// One row of that list.
#[derive(Component, Clone)]
pub(crate) struct PathRow(PathBuf);

/// The id `typed` makes, against `known`, as the line to show, its colour, and
/// whether it can be used. The hub's New Mod form says the same.
pub(crate) fn id_line(typed: &str, known: &[String]) -> (String, UiColor, bool) {
    match project_id(typed.trim()) {
        Err(NameProblem::Unusable) => (
            "A name needs a letter or a digit.".to_string(),
            UiColor::Danger,
            false,
        ),
        Err(NameProblem::Reserved) => (
            "That name is the game's own. Pick another.".to_string(),
            UiColor::Danger,
            false,
        ),
        Ok(id) if is_taken(&id, known) => (
            format!("{id} already exists. Pick another."),
            UiColor::Danger,
            false,
        ),
        Ok(id) => (
            format!("saves as {id}, publishes as {}", publish_id(&id)),
            UiColor::Label,
            true,
        ),
    }
}

/// What a naming window opens on.
fn offered_name(kind: &FileWindowKind, project: Option<&ModProject>) -> String {
    let Some(project) = project else {
        return String::new();
    };
    match kind {
        FileWindowKind::NameMod => project.data.meta.name.clone(),
        FileWindowKind::SaveCopy => format!("{} Copy", project.data.meta.name),
        FileWindowKind::Packaged(_) | FileWindowKind::PickPath(_) => String::new(),
    }
}

/// Put the asked-for window up.
pub(crate) fn open_file_window(
    mut commands: Commands,
    mut asked: ResMut<FileWindowRequest>,
    layer: Option<Single<Entity, With<EditorWindowLayer>>>,
    open: Query<(), With<FileWindow>>,
    project: Option<Res<ModProject>>,
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
    // from under the gallery on a mod with no id is exactly that press.
    if let Some(gallery) = gallery.as_mut() {
        gallery.open = false;
        gallery.focused = false;
    }
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let at = Vec2::new(((size.x - WINDOW_W) * 0.5).max(8.0), TOP_MARGIN);
    let name = offered_name(&kind, project.as_deref());
    commands
        .entity(*layer)
        .with_children(|layer| spawn_file_window(layer, kind, &name, at));
}

/// Small text under a field or above a list.
fn small_line(name: &str, line: &str, colour: UiColor) -> impl Bundle {
    (
        Name::new(name.to_string()),
        UiText,
        Text::new(line.to_string()),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
    )
}

/// The window, by kind.
fn spawn_file_window(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    kind: FileWindowKind,
    name: &str,
    at: Vec2,
) {
    let title = match &kind {
        FileWindowKind::NameMod => "Name This Mod",
        FileWindowKind::SaveCopy => "Save As Copy",
        FileWindowKind::Packaged(_) => "Packaged",
        FileWindowKind::PickPath(purpose) => purpose.title(),
    };
    let shown = kind.clone();
    let window = window_frame(
        layer,
        "File Window",
        title,
        at,
        (FileWindow(kind.clone()), KnownMods(local_ids())),
        |body, _window| match &shown {
            FileWindowKind::Packaged(path) => {
                body.spawn(small_line(
                    "Package Path Label",
                    "The package is at",
                    UiColor::Label,
                ));
                body.spawn(small_line("Package Path", path, UiColor::Primary));
            }
            FileWindowKind::PickPath(_) => {
                body.spawn(small_line("Path Label", "Path", UiColor::Label));
                body.spawn((
                    Name::new("Path Field"),
                    PathField,
                    text_field(
                        TextFieldSpec::new(start_folder().display().to_string())
                            .max_chars(PATH_CHARS)
                            .placeholder("Type a path")
                            .tool(),
                    ),
                ));
                body.spawn((
                    Name::new("Path List"),
                    PathList::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        row_gap: px(2),
                        ..default()
                    },
                ));
            }
            _ => {
                body.spawn(small_line("Save Name Label", "Name", UiColor::Label));
                body.spawn((
                    Name::new("Save Name Field"),
                    SaveNameField,
                    text_field(
                        TextFieldSpec::new(name.to_string())
                            .max_chars(NAME_CHARS)
                            .placeholder("Name it")
                            .tool(),
                    ),
                ));
                // Blank until `sync_save_name` fills it, which is the frame
                // after this one: the id belongs to what is TYPED, and there is
                // one place that works it out.
                body.spawn((
                    SaveIdReadout,
                    small_line("Save Id Readout", "", UiColor::Label),
                ));
            }
        },
    );
    // OUTSIDE the frame's scrolling body: a long folder list must not scroll the
    // answers away.
    layer
        .commands()
        .entity(window)
        .with_children(|frame| spawn_answers(frame, &kind));
}

/// Where a path window opens: the exports folder when it exists, because that
/// is where a package made here lands.
fn start_folder() -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir) = nova_assets::mod_cache::prelude::exports_dir().filter(|dir| dir.is_dir()) {
        return dir;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
}

/// Cancel, and the press that goes through with it.
fn spawn_answers(frame: &mut RelatedSpawnerCommands<ChildOf>, kind: &FileWindowKind) {
    let go = match kind {
        FileWindowKind::NameMod => Some("Save"),
        FileWindowKind::SaveCopy => Some("Save Copy"),
        FileWindowKind::PickPath(_) => Some("Pick"),
        FileWindowKind::Packaged(_) => None,
    };
    let keep = if go.is_some() { "Cancel" } else { "Close" };
    frame
        .spawn((
            Name::new("File Window Answers"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(density::GAP),
                padding: UiRect::all(px(density::PANEL_PAD)),
                border: UiRect::top(px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Primary, 0.16),
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
                        themed_button(keep),
                        ButtonCueType::Back,
                    ));
                });
            let Some(go) = go else {
                return;
            };
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
                        themed_button(go),
                        ButtonCueType::Owned,
                        observe(on_save),
                    ));
                });
        });
}

/// Say where the typed name lands, and grey the answer while it lands nowhere.
///
/// Unconditional rather than `Changed`-gated: at most one window is open, the
/// work is a slug of forty-eight characters, and the readout has to be right
/// on the frame the window opens as well as on the frames it is typed into.
/// Nothing is written unless it differs, so an untouched field costs no
/// re-layout.
pub(crate) fn sync_save_name(
    mut commands: Commands,
    theme: Res<ActiveUiTheme>,
    fields: Query<&TextFieldValue, With<SaveNameField>>,
    known: Query<&KnownMods, With<FileWindow>>,
    mut readouts: Query<(&mut Text, &mut TextColor), With<SaveIdReadout>>,
    buttons: Query<(Entity, Has<InteractionDisabled>), With<SaveConfirmButton>>,
) {
    let (Some(typed), Some(mods)) = (fields.iter().next(), known.iter().next()) else {
        return;
    };
    let (line, colour, usable) = id_line(&typed.0, &mods.0);
    let colour = theme.color(colour);
    for (mut text, mut paint) in &mut readouts {
        if text.0 != line {
            text.0.clone_from(&line);
        }
        if paint.0 != colour {
            paint.0 = colour;
        }
    }
    for (button, greyed) in &buttons {
        match (usable, greyed) {
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

/// The folders and matching files of `dir`, folders first. An empty
/// `extension` matches every file.
fn folder_rows(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let (mut folders, mut files): (Vec<PathBuf>, Vec<PathBuf>) = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_dir()
                || extension.is_empty()
                || path
                    .extension()
                    .is_some_and(|found| found.eq_ignore_ascii_case(extension))
        })
        .partition(|path| path.is_dir());
    folders.sort();
    files.sort();
    folders.extend(files);
    folders
}

/// Keep the path window's list on the folder the typed path stands in.
pub(crate) fn sync_path_window(
    mut commands: Commands,
    windows: Query<&FileWindow>,
    fields: Query<&TextFieldValue, With<PathField>>,
    mut lists: Query<(Entity, &mut PathList)>,
) {
    let (Some(typed), Ok((entity, mut list))) = (fields.iter().next(), lists.single_mut()) else {
        return;
    };
    let Some(FileWindowKind::PickPath(purpose)) = windows.iter().next().map(|window| &window.0)
    else {
        return;
    };
    let typed = Path::new(typed.0.trim());
    let folder = if typed.is_dir() {
        typed.to_path_buf()
    } else {
        typed.parent().map_or_else(PathBuf::new, Path::to_path_buf)
    };
    if list.0.as_ref() == Some(&folder) {
        return;
    }
    let rows = folder_rows(&folder, purpose.extension());
    let up = folder.parent().map(Path::to_path_buf);
    list.0 = Some(folder);
    commands.entity(entity).despawn_related::<Children>();
    commands.entity(entity).with_children(|list| {
        if let Some(up) = up {
            list.spawn((
                Name::new("Path Row Up"),
                PathRow(up),
                button(ButtonSpec::new("..").dense().block()),
                observe(on_path_row),
            ));
        }
        for row in rows {
            let label = row
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            let label = if row.is_dir() {
                format!("{label}/")
            } else {
                label
            };
            list.spawn((
                Name::new(format!("Path Row {label}")),
                PathRow(row),
                button(ButtonSpec::new(label).dense().block()),
                observe(on_path_row),
            ));
        }
    });
}

/// A path row: its path goes in the field, which moves the list if it is a
/// folder.
fn on_path_row(
    activate: On<Activate>,
    rows: Query<&PathRow>,
    mut fields: Query<&mut TextFieldValue, With<PathField>>,
) {
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    for mut value in &mut fields {
        value.0 = row.0.display().to_string();
    }
}

/// The press that goes through with the open window.
pub(crate) fn on_save(
    _activate: On<Activate>,
    mut commands: Commands,
    names: Query<&TextFieldValue, With<SaveNameField>>,
    paths: Query<&TextFieldValue, With<PathField>>,
    windows: Query<&FileWindow>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let kind = window.0.clone();
    let typed = match kind {
        FileWindowKind::PickPath(_) => paths.iter().next(),
        _ => names.iter().next(),
    };
    let Some(typed) = typed.map(|typed| typed.0.trim().to_string()) else {
        return;
    };
    commands.queue(move |world: &mut World| match kind {
        FileWindowKind::NameMod => name_project(world, &typed),
        FileWindowKind::SaveCopy => save_copy(world, &typed),
        FileWindowKind::PickPath(purpose) => pressed(world, |world| purpose.run(world, &typed)),
        FileWindowKind::Packaged(_) => {}
    });
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
