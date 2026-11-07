//! Mod Settings: the manifest's fields and the dependency list.
//!
//! Every change is a step on the one undo stack, so the window repaints from
//! the project whenever the history moves. A field the builder is typing in is
//! left alone.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_assets::mod_cache::prelude::is_safe_id;
use nova_modding::prelude::{ModMeta, BASE_MOD_ID};
use nova_ui::{
    prelude::{
        button, text_field, ButtonSpec, TextFieldFocused, TextFieldSpec, TextFieldSubmitted,
        TextFieldValue, UiText,
    },
    theme::UiColor,
    widget::{ButtonCueType, ThemedText},
};

use crate::{
    asset_index::AssetIndex,
    history::EditHistory,
    project::{
        store,
        verbs::{add_dependency, edit_meta, pressed, refuse, remove_dependency},
        ModProject,
    },
    ui::window::{window_frame, EditorWindowLayer, TOP_MARGIN, WINDOW_W},
};

/// How long a one-line field may be.
const FIELD_CHARS: usize = 96;
/// How long the description may be.
const DESCRIPTION_CHARS: usize = 400;

/// The window.
#[derive(Component)]
pub(crate) struct SettingsWindow;

/// One manifest field.
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
    /// The file its listing shows. Blank means none.
    Icon,
    /// Listing images, comma separated.
    Screenshots,
}

impl MetaFieldType {
    /// Every field, in window order.
    const ALL: [Self; 6] = [
        Self::Name,
        Self::Description,
        Self::Author,
        Self::Version,
        Self::Icon,
        Self::Screenshots,
    ];

    /// The row's word.
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Description => "Description",
            Self::Author => "Author",
            Self::Version => "Version",
            Self::Icon => "Icon",
            Self::Screenshots => "Screenshots",
        }
    }

    /// The undo step's label.
    fn step(self) -> &'static str {
        match self {
            Self::Name => "Rename Mod",
            Self::Description => "Edit Mod Description",
            Self::Author => "Edit Mod Author",
            Self::Version => "Edit Mod Version",
            Self::Icon => "Edit Mod Icon",
            Self::Screenshots => "Edit Mod Screenshots",
        }
    }

    /// The field as text.
    fn read(self, meta: &ModMeta) -> String {
        match self {
            Self::Name => meta.name.clone(),
            Self::Description => meta.description.clone(),
            Self::Author => meta.author.clone(),
            Self::Version => meta.version.clone(),
            Self::Icon => meta.icon.clone().unwrap_or_default(),
            Self::Screenshots => meta.screenshots.join(", "),
        }
    }

    /// Write `text` to the field.
    fn write(self, meta: &mut ModMeta, text: &str) {
        match self {
            Self::Name => meta.name = text.to_string(),
            Self::Description => meta.description = text.to_string(),
            Self::Author => meta.author = text.to_string(),
            Self::Version => meta.version = text.to_string(),
            Self::Icon => meta.icon = (!text.is_empty()).then(|| text.to_string()),
            Self::Screenshots => {
                meta.screenshots = text
                    .split(',')
                    .map(str::trim)
                    .filter(|file| !file.is_empty())
                    .map(str::to_string)
                    .collect();
            }
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

/// The dependency rows.
#[derive(Component)]
pub(crate) struct DependencyList;

/// The line that lists what holds a dependency in place.
#[derive(Component)]
pub(crate) struct DependencyRefusal;

/// A dependency row's Remove.
#[derive(Component, Clone)]
pub(crate) struct RemoveDependency(String);

/// A candidate row's Add.
#[derive(Component, Clone)]
pub(crate) struct AddDependency(String);

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

/// File > Mod Settings.
pub(crate) fn on_mod_settings_item(
    _activate: On<Activate>,
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
    commands.entity(*layer).with_children(|layer| {
        window_frame(
            layer,
            "Mod Settings Window",
            "Mod Settings",
            at,
            SettingsWindow,
            |body, _window| spawn_body(body, &meta),
        );
    });
}

/// The fields and the empty list `sync_settings_window` fills.
fn spawn_body(body: &mut RelatedSpawnerCommands<ChildOf>, meta: &ModMeta) {
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
        Name::new("Dependencies Label"),
        line("DEPENDENCIES", UiColor::Accent),
        Node {
            margin: UiRect::top(px(6)),
            ..default()
        },
    ));
    body.spawn((
        Name::new("Dependency Refusal"),
        DependencyRefusal,
        line("", UiColor::Danger),
    ));
    body.spawn((
        Name::new("Dependency List"),
        DependencyList,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(2),
            ..default()
        },
    ));
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

/// One candidate for the Add list.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    id: String,
    loaded: bool,
}

/// What may be added: every loaded or installed mod but this one, `base`, and
/// those already declared.
fn candidates(project: &ModProject, index: &AssetIndex) -> Vec<Candidate> {
    let declared = &project.data.meta.dependencies;
    let mut ids: Vec<String> = index
        .loaded_deps()
        .into_iter()
        .map(|(id, _)| id)
        .chain(store::installed_deps().into_iter().map(|(id, _)| id))
        .filter(|id| {
            id != BASE_MOD_ID && !declared.contains(id) && project.id.as_deref() != Some(id)
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .map(|id| Candidate {
            loaded: index.is_enabled(&id),
            id,
        })
        .collect()
}

/// Repaint the fields and the lists when the history moves.
pub(crate) fn sync_settings_window(
    mut commands: Commands,
    history: Res<EditHistory>,
    project: Option<Res<ModProject>>,
    index: AssetIndex,
    mut fields: Query<(&MetaFieldType, &mut TextFieldValue, Has<TextFieldFocused>)>,
    lists: Query<Entity, With<DependencyList>>,
    fresh: Query<(), Added<DependencyList>>,
    mut shown: Local<Option<u64>>,
) {
    let (Some(project), Ok(list)) = (project, lists.single()) else {
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
    let offered = candidates(&project, &index);
    let installed = store::installed_deps();
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        if meta.dependencies.is_empty() {
            list.spawn((
                Name::new("Dependencies Empty"),
                line("Only the base game.", UiColor::Label),
            ));
        }
        for dep in &meta.dependencies {
            let found = index.has_bundle(dep) || installed.iter().any(|(id, _)| id == dep);
            if !found {
                list.spawn((
                    Name::new(format!("Dependency Fault {dep}")),
                    line(format!("{dep} is not installed"), UiColor::Danger),
                ));
            }
            list.spawn((
                Name::new(format!("Dependency Row {dep}")),
                RemoveDependency(dep.clone()),
                button(ButtonSpec::new(format!("Remove {dep}")).dense().block()),
                ButtonCueType::Owned,
                observe(on_remove_dependency),
            ));
        }
        let offers_line = if offered.is_empty() {
            "No other installed mod to add"
        } else {
            "Add from what is installed"
        };
        list.spawn((
            Name::new("Dependency Offers Label"),
            line(offers_line, UiColor::Label),
        ));
        for candidate in offered {
            let word = if candidate.loaded {
                format!("Add {}", candidate.id)
            } else {
                format!("Add {} (not enabled)", candidate.id)
            };
            list.spawn((
                Name::new(format!("Dependency Offer {}", candidate.id)),
                AddDependency(candidate.id),
                button(ButtonSpec::new(word).dense().block()),
                ButtonCueType::Owned,
                observe(on_add_dependency),
            ));
        }
    });
}

/// Add.
fn on_add_dependency(activate: On<Activate>, mut commands: Commands, rows: Query<&AddDependency>) {
    if let Ok(row) = rows.get(activate.entity).cloned() {
        commands.queue(move |world: &mut World| {
            pressed(world, |world| add_dependency(world, &row.0));
        });
    }
}

/// Remove, or say what is in the way.
fn on_remove_dependency(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&RemoveDependency>,
) {
    let Ok(row) = rows.get(activate.entity).cloned() else {
        return;
    };
    commands.queue(move |world: &mut World| {
        pressed(world, |world| {
            let references = remove_dependency(world, &row.0);
            let refusal = if references.is_empty() {
                String::new()
            } else {
                format!("{} is in use by: {}", row.0, references.join(", "))
            };
            let mut lines = world.query_filtered::<&mut Text, With<DependencyRefusal>>();
            for mut text in lines.iter_mut(world) {
                text.0.clone_from(&refusal);
            }
        });
    });
}
