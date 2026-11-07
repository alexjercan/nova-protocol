//! The Assets tab: the mod's items as a grid of tiles, like files in a project
//! window. Sections and ships carry a rendered picture and every other kind a
//! glyph. A tile opens on a double click, and its right-click menu holds the
//! verbs.
//!
//! A tile prints a short name and its hover hint holds the whole id. Tiles of
//! one kind that would print alike say on their second line what tells them
//! apart: see [`name_tiles`].

use std::collections::HashMap;

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate, Button},
};
use nova_modding::prelude::Content;
use nova_ship::prelude::GameSections;
use nova_ui::{
    prelude::*,
    theme::{self, density, UiColor},
};

use super::{
    browser::{pane_text, BrowserTabType},
    layer,
    shell::EditorPanels,
    thumbnail::{design_parts, thumbnail_key, AssetThumbnails, ThumbnailParts},
    window::DestructiveVerb,
};
use crate::{
    asset::verbs::{delete_asset, duplicate_asset, rename_asset},
    bundle::{FileWindowKind, FileWindowRequest},
    config::EditorSays,
    glyph::asset_mark,
    history::EditHistory,
    problems::Problems,
    project::{
        verbs::{delete_scenario, duplicate_scenario, open_asset, pressed, rename_scenario},
        ModProject, OpenAsset, KINDS, SCENARIO_KIND,
    },
    ExampleStates,
};

/// A tile's width.
const TILE_W: f32 = 84.0;

/// A tile's height: the picture and two lines of name.
const TILE_H: f32 = 90.0;

/// A tile's picture.
const THUMB_W: f32 = 54.0;

/// The glyph standing in for a picture.
const GLYPH_TEXT: f32 = 26.0;

/// The width of the kind column.
const KIND_W: f32 = 120.0;

/// The characters one line of a tile's name holds. The UI font is monospace.
const LINE_CHARS: usize = 14;

/// The kind a declared file lists under.
const RESOURCE_KIND: &str = "resource";

/// One tile of the Assets tab.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AssetEntry {
    /// The row of [`KINDS`] it lists under, or [`RESOURCE_KIND`].
    pub(crate) kind: &'static str,
    /// The short name: what a rename starts from, and what a filter reads.
    pub(crate) label: String,
    /// The whole id or path, for the tile's hover hint.
    pub(crate) full: String,
    /// The lines the tile prints. See [`name_tiles`].
    printed: String,
    /// Why it does not open, on a tile that does not.
    pub(crate) waits: &'static str,
    /// The item, on a tile that holds verbs.
    pub(crate) id: Option<OpenAsset>,
    /// What its picture shows. Empty for a kind that keeps its glyph.
    parts: ThumbnailParts,
}

/// `id` without the mod's prefix, which every id of the mod repeats.
fn short_id<'a>(id: &'a str, namespace: &str) -> &'a str {
    id.strip_prefix(&format!("{}__", namespace.replace('-', "_")))
        .unwrap_or(id)
}

/// The name a tile prints for `item`. A design id is `<scenario>.<node>`, and
/// the node is the name: the scenario would fill the line and repeat on every
/// tile. A scenario has a name of its own.
fn short_name(item: &Content, namespace: &str) -> String {
    let id = short_id(item.id(), namespace);
    match item {
        Content::Scenario(scenario) => scenario.name.clone(),
        Content::Ship(_) | Content::Station(_) => {
            id.rsplit_once('.').map_or(id, |(_, node)| node).to_string()
        }
        _ => id.to_string(),
    }
}

/// `text` in at most `chars`, its end given up for an ellipsis.
fn cut_end(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_string();
    }
    let kept: String = text.chars().take(chars - 3).collect();
    format!("{kept}...")
}

/// `text` in at most `chars`, its start given up for an ellipsis. A copy differs
/// from its source at the end of its id.
fn cut_start(text: &str, chars: usize) -> String {
    let count = text.chars().count();
    if count <= chars {
        return text.to_string();
    }
    let kept: String = text.chars().skip(count - (chars - 3)).collect();
    format!("...{kept}")
}

/// `label` on at most two lines. A line breaks only after a word: an id is words
/// joined by `_`, `-`, `.` or `/`, which the text layout does not break at.
fn wrapped_name(label: &str) -> String {
    let chars: Vec<char> = label.chars().collect();
    if chars.len() <= LINE_CHARS {
        return label.to_string();
    }
    let word_end = (1..=LINE_CHARS)
        .rev()
        .find(|at| matches!(chars[at - 1], ' ' | '_' | '-' | '.' | '/'));
    let Some(at) = word_end else {
        return cut_end(label, LINE_CHARS);
    };
    let first: String = chars[..at].iter().collect();
    let rest: String = chars[at..].iter().collect();
    format!(
        "{}\n{}",
        first.trim_end(),
        cut_end(rest.trim_start(), LINE_CHARS)
    )
}

/// What tells `entry` from a tile that would print alike: its id without the
/// name, so the scenario of a design or the folder of a file, or else its id.
fn qualifier(entry: &AssetEntry, namespace: &str) -> String {
    let id = short_id(&entry.full, namespace);
    let rest = id
        .strip_suffix(entry.label.as_str())
        .unwrap_or(id)
        .trim_end_matches(['.', '/']);
    if rest.is_empty() { id } else { rest }.to_string()
}

/// Write what every tile prints, and keep tiles of one kind different.
///
/// Two scenarios of a mod can hold the same node, so a short name alone repeats.
/// A tile that would print like another keeps its name to one line and takes its
/// [`qualifier`] as the second. Ids that still read alike once cut are numbered.
/// The kind's glyph tells apart tiles of two kinds.
fn name_tiles(entries: &mut [AssetEntry], namespace: &str) {
    let mut printing: HashMap<(&'static str, String), usize> = HashMap::new();
    for entry in entries.iter_mut() {
        entry.printed = wrapped_name(&entry.label);
        *printing
            .entry((entry.kind, entry.printed.clone()))
            .or_default() += 1;
    }
    let mut seen: HashMap<(&'static str, String), usize> = HashMap::new();
    for entry in entries.iter_mut() {
        if printing[&(entry.kind, entry.printed.clone())] < 2 {
            continue;
        }
        let name = cut_end(&entry.label, LINE_CHARS);
        let scope = qualifier(entry, namespace);
        entry.printed = format!("{name}\n{}", cut_start(&scope, LINE_CHARS));
        let count = seen.entry((entry.kind, entry.printed.clone())).or_default();
        *count += 1;
        if *count > 1 {
            let mark = format!(" #{count}");
            let room = LINE_CHARS - mark.len();
            entry.printed = format!("{name}\n{}{mark}", cut_start(&scope, room));
        }
    }
}

/// Everything the mod holds, in [`KINDS`] order, then its declared files.
pub(crate) fn asset_entries(
    project: &ModProject,
    catalog: Option<&GameSections>,
) -> Vec<AssetEntry> {
    let data = &project.data;
    let mut entries = Vec::new();
    for row in KINDS {
        for item in data.items().filter(|item| item.kind() == row.kind) {
            let parts = match (item, catalog) {
                (Content::Section(section), _) => {
                    vec![((**section).clone(), Transform::default())]
                }
                (Content::Ship(ship), Some(catalog)) => design_parts(&ship.design, catalog),
                _ => Vec::new(),
            };
            entries.push(AssetEntry {
                kind: row.kind,
                label: short_name(item, &project.namespace),
                full: item.id().to_string(),
                printed: String::new(),
                waits: row.waits,
                id: row.opens.then(|| OpenAsset {
                    kind: row.kind,
                    id: item.id().to_string(),
                }),
                parts,
            });
        }
    }
    entries.extend(data.resources.iter().map(|file| AssetEntry {
        kind: RESOURCE_KIND,
        label: file.rsplit('/').next().unwrap_or(file).to_string(),
        full: file.clone(),
        printed: String::new(),
        waits: "a file the mod declares",
        id: None,
        parts: Vec::new(),
    }));
    name_tiles(&mut entries, &project.namespace);
    entries
}

/// The tiles under `kind` that a filter text keeps. A blank filter keeps them
/// all, and no kind means every kind.
pub(crate) fn filtered_entries(
    entries: &[AssetEntry],
    kind: Option<&str>,
    filter: &str,
) -> Vec<AssetEntry> {
    let filter = filter.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| kind.is_none_or(|kind| entry.kind == kind))
        .filter(|entry| {
            entry.label.to_lowercase().contains(&filter)
                || entry.full.to_lowercase().contains(&filter)
        })
        .cloned()
        .collect()
}

/// Whether Problems names `asset`: the open asset owns the document's findings,
/// and any other item is named by its id in a finding's words.
fn is_faulted(asset: &OpenAsset, open: Option<&OpenAsset>, problems: &Problems) -> bool {
    problems.list.iter().any(|problem| {
        (problem.node.is_some() && open == Some(asset)) || problem.message.contains(&asset.id)
    })
}

/// The kind column's heading for `kind`.
fn heading_of(kind: &str) -> &'static str {
    KINDS
        .iter()
        .find(|row| row.kind == kind)
        .map_or("Resources", |row| row.heading)
}

/// A kind's colour, the one thing that tells two glyph tiles apart at a glance.
fn kind_tint(kind: &str) -> UiColor {
    match kind {
        SCENARIO_KIND | "campaign" => UiColor::Accent,
        "section" | "ship" | "station" => UiColor::Primary,
        RESOURCE_KIND => UiColor::Label,
        _ => UiColor::Secondary,
    }
}

/// The filter box in the strip's tab row.
#[derive(Component)]
pub(crate) struct AssetFilter;

/// The strip's tab-row controls that belong to the Assets tab.
#[derive(Component)]
pub(crate) struct AssetTabControl;

/// The grid.
#[derive(Component)]
pub(crate) struct AssetList;

/// The kind column.
#[derive(Component)]
pub(crate) struct AssetKindList;

/// A kind row, and the kind it keeps. `None` is every kind.
#[derive(Component, Clone, Copy)]
struct AssetKindRow(Option<&'static str>);

/// The kind the grid is narrowed to.
#[derive(Resource, Default, PartialEq, Eq)]
pub(crate) struct AssetKindFilter(Option<&'static str>);

/// The item whose name is being typed on its tile.
#[derive(Resource, Default)]
pub(crate) struct AssetRenaming(Option<OpenAsset>);

/// A tile, and the item it holds when it holds one.
#[derive(Component, Clone)]
pub(crate) struct AssetTile(pub(crate) Option<OpenAsset>);

/// A tile's picture slot, and the picture it waits for.
#[derive(Component)]
pub(crate) struct AssetThumb(u64);

/// A tile's stand-in glyph.
#[derive(Component)]
pub(crate) struct AssetGlyph;

/// The field that renames a tile.
#[derive(Component)]
pub(crate) struct AssetRenameField {
    /// The item.
    asset: OpenAsset,
    /// The text the box opened on. A rename that leaves it there changes
    /// nothing, even where the stored id or name reads otherwise.
    seed: String,
}

/// What a menu row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetVerbType {
    Open,
    Rename,
    Duplicate,
    Delete,
}

/// A right-click menu row.
#[derive(Component, Clone)]
pub(crate) struct AssetVerb(pub(crate) AssetVerbType, pub(crate) OpenAsset);

/// An Add row, and the kind of item it starts.
#[derive(Component, Clone, Copy)]
struct AssetAddRow(&'static str);

/// The tab row's controls for this tab: the filter and the `+`.
pub(crate) fn spawn_asset_controls(row: &mut RelatedSpawnerCommands<ChildOf>) {
    row.spawn((
        Name::new("Asset Filter Slot"),
        AssetTabControl,
        Node {
            width: px(170),
            ..default()
        },
    ))
    .with_child((
        Name::new("Asset Filter"),
        AssetFilter,
        text_field(
            TextFieldSpec::new("")
                .placeholder("filter assets")
                .max_chars(40)
                .tool(),
        ),
    ));
    row.spawn((
        Name::new("Asset Add Button"),
        AssetTabControl,
        button(ButtonSpec::new("+").ghost().dense().fit()),
        observe(on_asset_add),
    ));
}

/// The pane: the kind column beside the grid.
pub(crate) fn spawn_asset_body(pane: &mut RelatedSpawnerCommands<ChildOf>) {
    pane.spawn((
        Name::new("Asset Body"),
        Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            column_gap: px(density::GAP),
            ..default()
        },
    ))
    .with_children(|body| {
        body.spawn((
            Name::new("Asset Kinds Scroll"),
            Node {
                width: px(KIND_W),
                flex_basis: px(KIND_W),
                flex_grow: 0.0,
                flex_shrink: 0.0,
                ..scroll_row()
            },
        ))
        .with_children(|scroll| {
            scroll.spawn((
                Name::new("Asset Kinds"),
                AssetKindList,
                Node {
                    align_items: AlignItems::Stretch,
                    ..scroll_column()
                },
                scroll_viewport(),
            ));
        });
        body.spawn((
            Name::new("Asset Grid Scroll"),
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..scroll_row()
            },
        ))
        .with_children(|scroll| {
            scroll.spawn((
                Name::new("Asset Grid"),
                AssetList,
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_content: AlignContent::FlexStart,
                    align_items: AlignItems::FlexStart,
                    column_gap: px(density::GAP),
                    row_gap: px(density::GAP),
                    ..scroll_column()
                },
                scroll_viewport(),
            ));
            scroll.spawn((Name::new("Asset Grid Bar"), scroll_bar()));
        });
    });
}

/// The `+`: a menu of what can start, the ones that wait greyed.
fn on_asset_add(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<&UiGlobalTransform>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    let at = buttons
        .get(activate.entity)
        .map_or(Vec2::ZERO, |transform| transform.translation);
    close_popups(&mut commands, &roots);
    let root = spawn_popup(&mut commands, at, layer::MENU_Z, |menu| {
        for row in KINDS {
            let mut spawned = menu.spawn((
                Name::new(format!("Asset Add {}", row.heading)),
                popup_row(row.heading, row.waits),
            ));
            if row.kind == SCENARIO_KIND {
                spawned.insert(DestructiveVerb::NewScenario);
            } else if row.add {
                spawned.insert((AssetAddRow(row.kind), observe(on_asset_add_row)));
            } else {
                spawned.insert(InteractionDisabled);
            }
        }
    });
    commands
        .entity(root)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// An Add row: a new item starts from what the chooser offers.
fn on_asset_add_row(
    activate: On<Activate>,
    rows: Query<&AssetAddRow>,
    mut window: ResMut<FileWindowRequest>,
) {
    if let Ok(row) = rows.get(activate.entity) {
        window.0 = Some(FileWindowKind::NewAsset(row.0));
    }
}

/// A press on a tile marks it, and a second one opens it.
fn on_asset_tile(
    activate: On<Activate>,
    mut commands: Commands,
    tiles: Query<(Entity, &AssetTile, Has<Selected>)>,
    time: Res<Time<Real>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut last: ResMut<DoubleClick<Entity>>,
    mut says: EditorSays,
) {
    // `Button` activates on any button. The right one is the tile's menu.
    if mouse.just_released(MouseButton::Right) {
        says.accept();
        return;
    }
    for (entity, _, marked) in &tiles {
        if entity == activate.entity {
            commands.entity(entity).insert(Selected);
        } else if marked {
            commands.entity(entity).remove::<Selected>();
        }
    }
    let Ok((_, AssetTile(Some(asset)), _)) = tiles.get(activate.entity) else {
        says.accept();
        return;
    };
    if last.press(activate.entity, time.elapsed_secs()) {
        let asset = asset.clone();
        commands.queue(move |world: &mut World| {
            pressed(world, |world| open_asset(world, asset.kind, &asset.id));
        });
    } else {
        says.accept();
    }
}

/// Enter opens the marked tile: the way in for a hand with no second press.
pub(crate) fn asset_tile_keys(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    open: Res<BrowserTabType>,
    panels: Res<EditorPanels>,
    tiles: Query<&AssetTile, With<Selected>>,
) {
    if !keys.just_pressed(KeyCode::Enter) || *open != BrowserTabType::Assets || !panels.browser {
        return;
    }
    if let Some(AssetTile(Some(asset))) = tiles.iter().next() {
        let asset = asset.clone();
        commands.queue(move |world: &mut World| open_asset(world, asset.kind, &asset.id));
    }
}

/// A right press on a tile: its verbs. A tile with none says why.
fn on_asset_tile_menu(
    click: On<Pointer<Click>>,
    mut commands: Commands,
    tiles: Query<&AssetTile>,
    entries: Query<&AssetWaits>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let Ok(AssetTile(asset)) = tiles.get(click.event_target()) else {
        return;
    };
    close_popups(&mut commands, &roots);
    let asset = asset.clone();
    let waits = entries
        .get(click.event_target())
        .map_or("", |waits| waits.0);
    let root = spawn_popup(
        &mut commands,
        click.pointer_location.position,
        layer::MENU_Z,
        |menu| {
            let Some(asset) = asset else {
                menu.spawn((
                    Name::new("Asset Menu Waits"),
                    popup_row(waits, ""),
                    InteractionDisabled,
                ));
                return;
            };
            // A price table is one per mod: it has no id to change, and no
            // second to copy into.
            let single = asset.kind == "price table";
            for (word, hint, verb) in [
                ("Open", "Enter", AssetVerbType::Open),
                ("Rename", "", AssetVerbType::Rename),
                ("Duplicate", "", AssetVerbType::Duplicate),
                ("Delete", "", AssetVerbType::Delete),
            ] {
                if single && matches!(verb, AssetVerbType::Rename | AssetVerbType::Duplicate) {
                    continue;
                }
                menu.spawn((
                    Name::new(format!("Asset Menu {word}")),
                    AssetVerb(verb, asset.clone()),
                    popup_row(word, hint),
                    ButtonCueType::Owned,
                    observe(on_asset_verb),
                ));
            }
        },
    );
    commands
        .entity(root)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// A menu row.
pub(crate) fn on_asset_verb(
    activate: On<Activate>,
    mut commands: Commands,
    verbs: Query<&AssetVerb>,
    mut renaming: ResMut<AssetRenaming>,
    mut says: EditorSays,
) {
    let Ok(AssetVerb(verb, asset)) = verbs.get(activate.entity) else {
        return;
    };
    let OpenAsset { kind, id } = asset.clone();
    let scenario = kind == SCENARIO_KIND;
    match verb {
        AssetVerbType::Open => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| open_asset(world, kind, &id));
            });
        }
        AssetVerbType::Rename => {
            renaming.0 = Some(asset.clone());
            says.accept();
        }
        AssetVerbType::Duplicate if scenario => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| duplicate_scenario(world, &id));
            });
        }
        AssetVerbType::Duplicate => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| duplicate_asset(world, kind, &id));
            });
        }
        AssetVerbType::Delete if scenario => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| delete_scenario(world, &id));
            });
        }
        AssetVerbType::Delete => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| {
                    delete_asset(world, kind, &id);
                });
            });
        }
    }
}

/// A kind row.
fn on_asset_kind(
    activate: On<Activate>,
    rows: Query<&AssetKindRow>,
    mut filter: ResMut<AssetKindFilter>,
) {
    if let Ok(AssetKindRow(kind)) = rows.get(activate.entity) {
        filter.0 = *kind;
    }
}

/// Enter or a press elsewhere in the rename field renames the item. Esc leaves
/// it as it was: the field reverts and sends nothing.
pub(crate) fn apply_asset_rename(
    mut commands: Commands,
    mut submitted: MessageReader<TextFieldSubmitted>,
    fields: Query<&AssetRenameField>,
    focused: Query<(), (With<AssetRenameField>, With<TextFieldFocused>)>,
    mut renaming: ResMut<AssetRenaming>,
) {
    for TextFieldSubmitted { entity, value } in submitted.read() {
        let Ok(AssetRenameField { asset, seed }) = fields.get(*entity) else {
            continue;
        };
        let OpenAsset { kind, id } = asset.clone();
        let typed = value.trim().to_string();
        if !typed.is_empty() && typed != seed.trim() {
            commands.queue(move |world: &mut World| {
                if kind == SCENARIO_KIND {
                    rename_scenario(world, &id, &typed);
                } else {
                    rename_asset(world, kind, &id, &typed);
                }
            });
        }
    }
    if renaming.0.is_some() && !fields.is_empty() && focused.is_empty() {
        renaming.0 = None;
    }
}

/// What the Assets pane is showing.
#[derive(Default, PartialEq)]
pub(crate) struct ShownAssets {
    revision: Option<u64>,
    filter: String,
    kind: Option<&'static str>,
    open: Option<OpenAsset>,
    renaming: Option<OpenAsset>,
    faulted: Vec<OpenAsset>,
}

/// Why a tile holds no verbs, read by its right-click menu.
#[derive(Component)]
struct AssetWaits(&'static str);

/// Rebuild the tiles when their tab is open and the mod or the filter moved.
pub(crate) fn sync_assets(
    mut commands: Commands,
    open: Res<BrowserTabType>,
    history: Res<EditHistory>,
    project: Option<Res<ModProject>>,
    catalog: Option<Res<GameSections>>,
    problems: Res<Problems>,
    kind: Res<AssetKindFilter>,
    renaming: Res<AssetRenaming>,
    mut thumbnails: ResMut<AssetThumbnails>,
    filters: Query<&TextFieldValue, With<AssetFilter>>,
    lists: Query<Entity, With<AssetList>>,
    kinds: Query<Entity, With<AssetKindList>>,
    fresh: Query<(), Added<AssetList>>,
    mut shown: Local<Option<ShownAssets>>,
    mut built: Local<Option<(Option<u64>, Vec<AssetEntry>)>>,
) {
    if !fresh.is_empty() {
        *shown = None;
    }
    // Read before the tab check: a change seen only on a skipped frame is lost.
    let moved = project.as_ref().is_some_and(Res::is_changed)
        || catalog.as_ref().is_some_and(Res::is_changed);
    if moved {
        *built = None;
    }
    if *open != BrowserTabType::Assets {
        return;
    }
    let Some(project) = project else {
        return;
    };
    let revision = history.revision();
    if built.as_ref().is_none_or(|(held, _)| *held != revision) {
        *built = Some((revision, asset_entries(&project, catalog.as_deref())));
    }
    let Some((_, entries)) = built.as_ref() else {
        return;
    };
    let open_item = project.data.open.clone();
    let wanted = ShownAssets {
        revision: history.revision(),
        filter: filters
            .iter()
            .next()
            .map(|value| value.0.clone())
            .unwrap_or_default(),
        kind: kind.0,
        faulted: entries
            .iter()
            .filter_map(|entry| entry.id.clone())
            .filter(|id| is_faulted(id, open_item.as_ref(), &problems))
            .collect(),
        open: open_item,
        renaming: renaming.0.clone(),
    };
    if shown.as_ref() == Some(&wanted) {
        return;
    }
    let (Ok(list), Ok(kind_list)) = (lists.single(), kinds.single()) else {
        return;
    };
    commands.entity(kind_list).despawn_related::<Children>();
    commands.entity(kind_list).with_children(|list| {
        spawn_kind_row(list, None, "All", entries.len(), wanted.kind.is_none());
        let mut held: Vec<&'static str> = Vec::new();
        for entry in entries {
            if !held.contains(&entry.kind) {
                held.push(entry.kind);
            }
        }
        for held_kind in held {
            let count = entries.iter().filter(|row| row.kind == held_kind).count();
            spawn_kind_row(
                list,
                Some(held_kind),
                heading_of(held_kind),
                count,
                wanted.kind == Some(held_kind),
            );
        }
    });
    let tiles = filtered_entries(entries, wanted.kind, &wanted.filter);
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        if tiles.is_empty() {
            list.spawn((
                Name::new("Asset Grid Empty"),
                pane_text(
                    "nothing here: + adds an item",
                    density::TEXT,
                    UiColor::Label,
                ),
            ));
        }
        for entry in &tiles {
            let key = (!entry.parts.is_empty()).then(|| {
                let key = thumbnail_key(&entry.full, &entry.parts);
                thumbnails.request(key, entry.parts.clone());
                key
            });
            let faulted = entry
                .id
                .as_ref()
                .is_some_and(|id| wanted.faulted.contains(id));
            let held_open = entry.id.is_some() && entry.id == wanted.open;
            spawn_tile(
                list,
                entry,
                key,
                faulted,
                held_open,
                wanted.renaming.as_ref(),
            );
        }
    });
    *shown = Some(wanted);
}

/// One row of the kind column.
fn spawn_kind_row(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    kind: Option<&'static str>,
    heading: &str,
    count: usize,
    chosen: bool,
) {
    let mut row = list.spawn((
        Name::new(format!("Asset Kind {heading}")),
        AssetKindRow(kind),
        ListRow,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            height: px(density::ROW_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(density::PANEL_PAD)),
            border: UiRect::all(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        observe(on_asset_kind),
        children![pane_text(
            &format!("{heading} {count}"),
            density::TEXT,
            UiColor::Primary
        )],
    ));
    if chosen {
        row.insert(Selected);
    }
}

/// The name under a tile, already cut to its lines.
fn tile_name(printed: &str) -> impl Bundle {
    (
        UiText,
        Text::new(printed),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            justify: Justify::Center,
        },
        TextFont {
            font_size: FontSize::Px(density::TEXT_SMALL),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColor::Primary),
        Pickable::IGNORE,
        Node {
            max_height: px(density::TEXT_SMALL * 2.6),
            overflow: Overflow::clip(),
            ..default()
        },
    )
}

/// One tile.
fn spawn_tile(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    entry: &AssetEntry,
    picture: Option<u64>,
    faulted: bool,
    held_open: bool,
    renaming: Option<&OpenAsset>,
) {
    let typing = entry.id.is_some() && entry.id.as_ref() == renaming;
    let mut tile = list.spawn((
        Name::new(format!("Asset Tile {}", entry.label)),
        AssetTile(entry.id.clone()),
        AssetWaits(entry.waits),
        ListRow,
        Tooltip(entry.full.clone()),
        Node {
            width: px(TILE_W),
            height: px(TILE_H),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
            padding: UiRect::all(px(2)),
            border: UiRect::all(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    ));
    // A press on the field must place its caret, not mark or open the tile.
    if !typing {
        tile.insert((
            Button,
            ButtonCueType::Owned,
            observe(on_asset_tile),
            observe(on_asset_tile_menu),
        ));
    }
    tile.with_children(|tile| {
        tile.spawn((
            Name::new("Asset Tile Face"),
            Node {
                width: px(THUMB_W),
                height: px(THUMB_W),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::bottom(px(2)),
                ..default()
            },
            BorderColor::all(Color::NONE),
            ThemedBorder::new(kind_tint(entry.kind)),
            Pickable::IGNORE,
        ))
        .with_children(|face| {
            face.spawn((
                AssetGlyph,
                pane_text(asset_mark(entry.kind), GLYPH_TEXT, kind_tint(entry.kind)),
                Pickable::IGNORE,
            ));
            if let Some(key) = picture {
                face.spawn((
                    Name::new("Asset Tile Picture"),
                    AssetThumb(key),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        display: Display::None,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            if faulted {
                face.spawn((
                    Name::new("Asset Tile Fault"),
                    pane_text("!", density::TEXT, UiColor::Danger),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        right: px(0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            if held_open {
                face.spawn((
                    Name::new("Asset Tile Open"),
                    pane_text("open", density::TEXT_SMALL, UiColor::Accent),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        left: px(0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
        });
        match entry.id.as_ref().filter(|_| typing) {
            Some(asset) => {
                tile.spawn((
                    Name::new("Asset Rename"),
                    AssetRenameField {
                        asset: asset.clone(),
                        seed: entry.label.clone(),
                    },
                    TextFieldFocused::at_end(&entry.label),
                    text_field(TextFieldSpec::new(&entry.label).max_chars(64).tool()),
                ));
            }
            None => {
                tile.spawn(tile_name(&entry.printed));
            }
        }
    });
}

/// Hang each picture on its tile once it is taken. A tile whose picture never
/// comes keeps its glyph.
pub(crate) fn sync_asset_thumbs(
    mut commands: Commands,
    thumbnails: Res<AssetThumbnails>,
    mut slots: Query<(Entity, &AssetThumb, &mut Node, &ChildOf), Without<ImageNode>>,
    faces: Query<&Children>,
    glyphs: Query<(), With<AssetGlyph>>,
) {
    for (entity, thumb, mut node, parent) in &mut slots {
        let Some(picture) = thumbnails.picture(thumb.0) else {
            continue;
        };
        commands
            .entity(entity)
            .insert(ImageNode::new(picture.clone()));
        node.display = Display::Flex;
        let Ok(held) = faces.get(parent.parent()) else {
            continue;
        };
        for child in held.iter().filter(|child| glyphs.contains(*child)) {
            commands.entity(child).despawn();
        }
    }
}

/// Show the tab row's controls on the Assets tab only.
pub(crate) fn sync_asset_controls(
    open: Res<BrowserTabType>,
    panels: Res<EditorPanels>,
    mut controls: Query<&mut Node, With<AssetTabControl>>,
) {
    let display = if *open == BrowserTabType::Assets && panels.browser {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut controls {
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
mod tests;
