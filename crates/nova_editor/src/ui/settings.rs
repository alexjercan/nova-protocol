//! Mod Settings: the manifest's fields, its icon, its screenshots and its
//! dependencies.
//!
//! Every change is a step on the one undo stack, so the window repaints from
//! the project whenever the history moves. A field the builder is typing in is
//! left alone. The screenshots and the dependencies are list editors: a row per
//! entry with a remove `x`, and a search dropdown under the last row.

use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*, ui_widgets::observe};
use nova_assets::mod_cache::prelude::is_safe_id;
use nova_modding::prelude::{ModMeta, BASE_MOD_ID};
use nova_ui::{
    prelude::{
        list_add, list_editor, list_entry_row, list_remove, select, text_field, ListChoice,
        ListEditor, ListEntryAdded, ListEntryRemoved, Select, SelectChanged, SelectLabel,
        TextFieldFocused, TextFieldSpec, TextFieldSubmitted, TextFieldValue, UiText,
    },
    theme::UiColor,
    widget::ThemedText,
};

use crate::{
    asset_index::AssetIndex,
    history::EditHistory,
    project::{
        store,
        verbs::{add_dependency, edit_meta, pressed, refuse, remove_dependency},
        ModProject,
    },
    ui::{
        layer,
        window::{window_frame, EditorWindowLayer, TOP_MARGIN, WINDOW_W},
    },
};

/// How long a one-line field may be.
const FIELD_CHARS: usize = 96;
/// How long the description may be.
const DESCRIPTION_CHARS: usize = 400;

/// The window.
#[derive(Component)]
pub(crate) struct SettingsWindow;

/// One typed manifest field.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MetaFieldType {
    /// What players see it called.
    Name,
    /// What it is.
    Description,
    /// Who made it.
    Author,
    /// Its version.
    Version,
}

impl MetaFieldType {
    /// Every field, in window order.
    const ALL: [Self; 4] = [Self::Name, Self::Description, Self::Author, Self::Version];

    /// The row's word.
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Description => "Description",
            Self::Author => "Author",
            Self::Version => "Version",
        }
    }

    /// The undo step's label.
    fn step(self) -> &'static str {
        match self {
            Self::Name => "Rename Mod",
            Self::Description => "Edit Mod Description",
            Self::Author => "Edit Mod Author",
            Self::Version => "Edit Mod Version",
        }
    }

    /// The field as text.
    fn read(self, meta: &ModMeta) -> String {
        match self {
            Self::Name => meta.name.clone(),
            Self::Description => meta.description.clone(),
            Self::Author => meta.author.clone(),
            Self::Version => meta.version.clone(),
        }
    }

    /// Write `text` to the field.
    fn write(self, meta: &mut ModMeta, text: &str) {
        match self {
            Self::Name => meta.name = text.to_string(),
            Self::Description => meta.description = text.to_string(),
            Self::Author => meta.author = text.to_string(),
            Self::Version => meta.version = text.to_string(),
        }
    }

    /// Why `text` cannot go in the field.
    fn fault(self, text: &str) -> Option<&'static str> {
        match self {
            Self::Name if text.is_empty() => Some("a mod needs a name"),
            Self::Version if !is_safe_id(text) => {
                Some("a version is letters, digits and dots, like 1.0.0")
            }
            _ => None,
        }
    }
}

/// The dependency list: a [`ListEditor`] the history repaints.
#[derive(Component)]
pub(crate) struct DependencyList;

/// The screenshot list: a [`ListEditor`] the history repaints.
#[derive(Component)]
pub(crate) struct ScreenshotList;

/// The icon's select: none, or one of the mod's image files.
#[derive(Component)]
pub(crate) struct IconSelect;

/// What the icon select reads when the mod has no icon.
const NO_ICON: &str = "none";

/// The file kinds a listing image may be.
const IMAGE_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// A mod the editor knows of: loaded this session, or installed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct KnownMod {
    id: String,
    name: String,
    version: String,
    enabled: bool,
}

/// Words in `colour`.
fn line(text: impl Into<String>, colour: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
    )
}

/// A cell of a list row: `text` in `colour`, `width` wide, cut at its end.
fn cell(text: impl Into<String>, colour: UiColor, width: Val) -> impl Bundle {
    (
        line(text, colour),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        Node {
            width,
            flex_shrink: 1.0,
            min_width: px(0),
            overflow: Overflow::clip(),
            ..default()
        },
    )
}

/// A heading over a list.
fn heading(body: &mut RelatedSpawnerCommands<ChildOf>, name: &str, word: &str) {
    body.spawn((
        Name::new(name.to_string()),
        line(word, UiColor::Accent),
        Node {
            margin: UiRect::top(px(6)),
            ..default()
        },
    ));
}

/// File > Mod Settings.
pub(crate) fn on_mod_settings_item(
    _activate: On<bevy::ui_widgets::Activate>,
    mut commands: Commands,
    project: Option<Res<ModProject>>,
    layer: Option<Single<Entity, With<EditorWindowLayer>>>,
    open: Query<(), With<SettingsWindow>>,
    screen: Option<Single<&Window>>,
) {
    let (Some(project), Some(layer)) = (project, layer) else {
        return;
    };
    if !open.is_empty() {
        return;
    }
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let at = Vec2::new(((size.x - WINDOW_W) * 0.5).max(8.0), TOP_MARGIN);
    let meta = project.data.meta.clone();
    let images = image_files(&project.data.resources);
    commands.entity(*layer).with_children(|layer| {
        window_frame(
            layer,
            "Mod Settings Window",
            "Mod Settings",
            at,
            SettingsWindow,
            |body, _window| spawn_body(body, &meta, &images),
        );
    });
}

/// The fields, and the empty lists `sync_settings_window` fills.
fn spawn_body(body: &mut RelatedSpawnerCommands<ChildOf>, meta: &ModMeta, images: &[String]) {
    for field in MetaFieldType::ALL {
        body.spawn((
            Name::new(format!("Settings Label {}", field.label())),
            line(field.label(), UiColor::Label),
        ));
        let limit = match field {
            MetaFieldType::Description => DESCRIPTION_CHARS,
            _ => FIELD_CHARS,
        };
        body.spawn((
            Name::new(format!("Settings Field {}", field.label())),
            field,
            text_field(TextFieldSpec::new(field.read(meta)).max_chars(limit).tool()),
        ));
    }
    body.spawn((
        Name::new("Settings Label Icon"),
        line("Icon", UiColor::Label),
    ));
    let (options, chosen) = icon_options(meta.icon.as_deref(), images);
    body.spawn((
        Name::new("Settings Icon"),
        IconSelect,
        select(options, chosen, layer::WINDOW_MENU_Z),
        observe(on_icon_changed),
    ));
    heading(body, "Screenshots Label", "SCREENSHOTS");
    body.spawn((
        Name::new("Screenshot List"),
        ScreenshotList,
        list_editor(ListEditor {
            z: layer::WINDOW_MENU_Z,
            ..default()
        }),
        observe(on_screenshot_added),
        observe(on_screenshot_removed),
    ));
    heading(body, "Dependencies Label", "DEPENDENCIES");
    body.spawn((
        Name::new("Dependency List"),
        DependencyList,
        list_editor(ListEditor {
            z: layer::WINDOW_MENU_Z,
            ..default()
        }),
        observe(on_dependency_added),
        observe(on_dependency_removed),
    ));
}

/// The image files the mod holds, sorted.
fn image_files(resources: &[String]) -> Vec<String> {
    let mut files: Vec<String> = resources
        .iter()
        .filter(|file| {
            file.rsplit_once('.').is_some_and(|(_, found)| {
                IMAGE_EXTENSIONS
                    .iter()
                    .any(|wanted| found.eq_ignore_ascii_case(wanted))
            })
        })
        .cloned()
        .collect();
    files.sort();
    files
}

/// The icon select's options, and the one `icon` reads as. An icon the mod
/// does not hold stays an option, so the select shows what the file says.
fn icon_options(icon: Option<&str>, images: &[String]) -> (Vec<String>, usize) {
    let mut options = vec![NO_ICON.to_string()];
    options.extend(images.iter().cloned());
    let chosen = match icon {
        None => 0,
        Some(file) => options
            .iter()
            .position(|held| held == file)
            .unwrap_or_else(|| {
                options.push(file.to_string());
                options.len() - 1
            }),
    };
    (options, chosen)
}

/// Commit a submitted field.
pub(crate) fn apply_settings_fields(
    mut commands: Commands,
    mut submitted: MessageReader<TextFieldSubmitted>,
    fields: Query<&MetaFieldType>,
) {
    for message in submitted.read() {
        let Ok(field) = fields.get(message.entity).copied() else {
            continue;
        };
        let text = message.value.trim().to_string();
        commands.queue(move |world: &mut World| {
            if let Some(fault) = field.fault(&text) {
                refuse(world, fault);
                return;
            }
            edit_meta(world, field.step(), move |meta| field.write(meta, &text));
        });
    }
}

/// Every mod the editor knows of but this one and `base`, by id.
fn known_mods(project: &ModProject, index: &AssetIndex) -> Vec<KnownMod> {
    let mut known: Vec<KnownMod> = index
        .loaded_mods()
        .into_iter()
        .chain(store::installed_mods())
        .filter(|(id, _)| id != BASE_MOD_ID && project.id.as_deref() != Some(id))
        .map(|(id, meta)| KnownMod {
            enabled: index.is_enabled(&id),
            name: meta.name,
            version: meta.version,
            id,
        })
        .collect();
    known.sort_by(|a, b| a.id.cmp(&b.id));
    known.dedup_by(|later, first| later.id == first.id);
    known
}

/// What the Add dependency dropdown offers: every known mod not yet declared.
fn dependency_choices(declared: &[String], known: &[KnownMod]) -> Vec<ListChoice> {
    known
        .iter()
        .filter(|held| !declared.contains(&held.id))
        .map(|held| {
            let hint = if held.enabled {
                format!("{} {}", held.id, held.version)
            } else {
                format!("{} {}, not enabled", held.id, held.version)
            };
            ListChoice::new(held.id.clone(), held.name.clone()).hint(hint)
        })
        .collect()
}

/// What the Add screenshot dropdown offers: every image of the mod not yet
/// listed.
fn screenshot_choices(listed: &[String], images: &[String]) -> Vec<ListChoice> {
    images
        .iter()
        .filter(|file| !listed.contains(file))
        .map(|file| ListChoice::new(file.clone(), file.clone()))
        .collect()
}

/// `listed` with `file` added, or why it cannot be.
fn with_screenshot(
    listed: &[String],
    file: &str,
    images: &[String],
) -> Result<Vec<String>, String> {
    if !images.iter().any(|held| held == file) {
        return Err(format!("{file} is not an image of this mod"));
    }
    if listed.iter().any(|held| held == file) {
        return Err(format!("{file} is already a screenshot"));
    }
    let mut next = listed.to_vec();
    next.push(file.to_string());
    Ok(next)
}

/// Repaint the fields and the lists when the history moves.
pub(crate) fn sync_settings_window(
    mut commands: Commands,
    history: Res<EditHistory>,
    project: Option<Res<ModProject>>,
    index: AssetIndex,
    mut fields: Query<(&MetaFieldType, &mut TextFieldValue, Has<TextFieldFocused>)>,
    mut deps: Query<(Entity, &mut ListEditor), (With<DependencyList>, Without<ScreenshotList>)>,
    mut shots: Query<(Entity, &mut ListEditor), (With<ScreenshotList>, Without<DependencyList>)>,
    mut icons: Query<(&mut Select, &Children), With<IconSelect>>,
    mut icon_labels: Query<&mut Text, With<SelectLabel>>,
    fresh: Query<(), Added<DependencyList>>,
    mut shown: Local<Option<u64>>,
) {
    let (Some(project), Ok((dep_list, mut dep_editor))) = (project, deps.single_mut()) else {
        return;
    };
    let revision = history.revision();
    if fresh.is_empty() && *shown == revision {
        return;
    }
    *shown = revision;
    let meta = &project.data.meta;
    for (field, mut value, focused) in &mut fields {
        let held = field.read(meta);
        if !focused && value.0 != held {
            value.0 = held;
        }
    }
    let images = image_files(&project.data.resources);
    if let Ok((mut select, children)) = icons.single_mut() {
        let (options, chosen) = icon_options(meta.icon.as_deref(), &images);
        if select.options != options || select.chosen != chosen {
            for child in children {
                if let Ok(mut text) = icon_labels.get_mut(*child) {
                    text.0.clone_from(&options[chosen]);
                }
            }
            select.options = options;
            select.chosen = chosen;
        }
    }
    if let Ok((shot_list, mut shot_editor)) = shots.single_mut() {
        shot_editor.choices = screenshot_choices(&meta.screenshots, &images);
        shot_editor.empty_reason = if images.is_empty() {
            "the mod holds no image: File > Import File adds one".to_string()
        } else {
            "every image of the mod is listed".to_string()
        };
        commands.entity(shot_list).despawn_related::<Children>();
        let files = meta.screenshots.clone();
        commands.entity(shot_list).with_children(|rows| {
            for (index, file) in files.iter().enumerate() {
                rows.spawn((
                    Name::new(format!("Screenshot Row {file}")),
                    list_entry_row(),
                ))
                .with_children(|row| {
                    let found = images.contains(file);
                    let colour = if found {
                        UiColor::Primary
                    } else {
                        UiColor::Danger
                    };
                    row.spawn(cell(file.clone(), colour, percent(100)));
                    row.spawn((
                        Name::new(format!("Screenshot Remove {file}")),
                        list_remove(shot_list, index),
                    ));
                });
            }
            rows.spawn((
                Name::new("Screenshot Add"),
                list_add(shot_list, "Add screenshot"),
            ));
        });
    }
    let known = known_mods(&project, &index);
    dep_editor.choices = dependency_choices(&meta.dependencies, &known);
    dep_editor.empty_reason = "no other installed mod to add".to_string();
    commands.entity(dep_list).despawn_related::<Children>();
    let declared = meta.dependencies.clone();
    commands.entity(dep_list).with_children(|rows| {
        if declared.is_empty() {
            rows.spawn((
                Name::new("Dependencies Empty"),
                line("Only the base game.", UiColor::Label),
            ));
        }
        for (index, dep) in declared.iter().enumerate() {
            let held = known.iter().find(|held| held.id == *dep);
            rows.spawn((Name::new(format!("Dependency Row {dep}")), list_entry_row()))
                .with_children(|row| {
                    let (name, version, colour) = match held {
                        Some(held) => (held.name.clone(), held.version.clone(), UiColor::Primary),
                        None => (
                            "not installed".to_string(),
                            "-".to_string(),
                            UiColor::Danger,
                        ),
                    };
                    row.spawn(cell(name, colour, percent(45)));
                    row.spawn(cell(dep.clone(), UiColor::Label, percent(35)));
                    row.spawn(cell(version, UiColor::Label, percent(20)));
                    row.spawn((
                        Name::new(format!("Dependency Remove {dep}")),
                        list_remove(dep_list, index),
                    ));
                });
        }
        rows.spawn((
            Name::new("Dependency Add"),
            list_add(dep_list, "Add dependency"),
        ));
    });
}

/// The icon select took a new choice: one step.
fn on_icon_changed(change: On<SelectChanged>, mut commands: Commands, selects: Query<&Select>) {
    let Ok(select) = selects.get(change.entity) else {
        return;
    };
    let picked = select.options.get(change.chosen).cloned();
    commands.queue(move |world: &mut World| {
        edit_meta(world, "Edit Mod Icon", move |meta| {
            meta.icon = picked.filter(|file| file != NO_ICON);
        });
    });
}

/// Add dependency took a mod: one step, or a refusal that says why.
fn on_dependency_added(added: On<ListEntryAdded>, mut commands: Commands) {
    let dep = added.id.clone();
    commands.queue(move |world: &mut World| {
        pressed(world, |world| add_dependency(world, &dep));
    });
}

/// A dependency's `x`: one step, or a refusal naming what still uses it.
fn on_dependency_removed(removed: On<ListEntryRemoved>, mut commands: Commands) {
    let index = removed.index;
    commands.queue(move |world: &mut World| {
        let Some(dep) = world
            .get_resource::<ModProject>()
            .and_then(|project| project.data.meta.dependencies.get(index).cloned())
        else {
            return;
        };
        pressed(world, |world| {
            remove_dependency(world, &dep);
        });
    });
}

/// Add screenshot took a file: one step, or a refusal that says why.
fn on_screenshot_added(added: On<ListEntryAdded>, mut commands: Commands) {
    let file = added.id.clone();
    commands.queue(move |world: &mut World| {
        pressed(world, |world| {
            let Some(project) = world.get_resource::<ModProject>() else {
                return;
            };
            let images = image_files(&project.data.resources);
            match with_screenshot(&project.data.meta.screenshots, &file, &images) {
                Ok(next) => edit_meta(world, "Add Screenshot", move |meta| {
                    meta.screenshots = next;
                }),
                Err(reason) => refuse(world, reason),
            }
        });
    });
}

/// A screenshot's `x`: one step.
fn on_screenshot_removed(removed: On<ListEntryRemoved>, mut commands: Commands) {
    let index = removed.index;
    commands.queue(move |world: &mut World| {
        pressed(world, |world| {
            edit_meta(world, "Remove Screenshot", move |meta| {
                if index < meta.screenshots.len() {
                    meta.screenshots.remove(index);
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{dependency_choices, icon_options, with_screenshot, KnownMod};

    fn known(id: &str, enabled: bool) -> KnownMod {
        KnownMod {
            id: id.to_string(),
            name: format!("{id} name"),
            version: "1.0.0".to_string(),
            enabled,
        }
    }

    /// The dropdown offers what is not declared yet, and says what is off.
    #[test]
    fn the_dependency_dropdown_offers_only_undeclared_mods() {
        let mods = [known("art-pack", true), known("ships", false)];

        let offered = dependency_choices(&["art-pack".to_string()], &mods);

        assert_eq!(offered.len(), 1);
        assert_eq!(offered[0].id, "ships");
        assert_eq!(offered[0].label, "ships name");
        assert_eq!(offered[0].hint, "ships 1.0.0, not enabled");
    }

    /// A screenshot is added once, and only from the mod's own images.
    #[test]
    fn a_screenshot_add_takes_a_new_image_and_refuses_the_rest() {
        let images = ["shots/a.png".to_string(), "shots/b.png".to_string()];
        let listed = ["shots/a.png".to_string()];

        assert_eq!(
            with_screenshot(&listed, "shots/b.png", &images).unwrap(),
            ["shots/a.png", "shots/b.png"]
        );
        assert_eq!(
            with_screenshot(&listed, "shots/a.png", &images).unwrap_err(),
            "shots/a.png is already a screenshot"
        );
        assert_eq!(
            with_screenshot(&listed, "shots/c.png", &images).unwrap_err(),
            "shots/c.png is not an image of this mod"
        );
    }

    /// The icon select reads none, a held image, or a file the mod lacks.
    #[test]
    fn the_icon_select_shows_what_the_manifest_says() {
        let images = ["icon.png".to_string()];

        assert_eq!(
            icon_options(None, &images),
            (vec!["none".into(), "icon.png".into()], 0)
        );
        assert_eq!(icon_options(Some("icon.png"), &images).1, 1);
        let (options, chosen) = icon_options(Some("gone.png"), &images);
        assert_eq!(options[chosen], "gone.png");
    }
}
