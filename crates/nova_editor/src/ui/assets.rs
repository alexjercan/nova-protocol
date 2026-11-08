//! The Assets tab: the mod's items and the files in its folder as one grid of
//! tiles, like a project window, sorted by kind. Sections and ships carry a
//! rendered picture and every other kind a glyph; the file tiles are
//! [`super::asset_files`]'s. A tile opens on a double click, and its
//! right-click menu holds the verbs.
//!
//! A tile prints a short name over a muted kind line, and its hover hint holds
//! the whole id. Tiles of one kind that would print alike say on their muted
//! line what tells them apart instead: see [`name_tiles`]. The kind column's
//! rows are the Hierarchy's [`tree_row`]s.

use std::collections::HashMap;

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui::{InteractionDisabled, UiScale},
    ui_widgets::{observe, Activate, Button},
};
use nova_modding::prelude::Content;
use nova_ship::prelude::GameSections;
use nova_ui::{
    prelude::*,
    theme::{self, density, UiColor},
};

use super::{
    add_menu::below,
    asset_files::{
        file_asset_path, fit, mesh_parts, on_asset_grid_menu, on_import_file, spawn_file_rows,
        view_file, AssetFileTile, AssetImage,
    },
    browser::{pane_text, BrowserTabType},
    layer,
    menu_context::{verb_state, MenuFacts, MenuVerbType, VerbState},
    section_kind::open_section_kind_modal,
    shell::EditorPanels,
    thumbnail::{design_parts, thumbnail_key, AssetThumbnails, ThumbnailParts},
};
use crate::{
    asset::{
        files::{
            file_kind, loads_as_image, mod_files, rename_file, ModFile, ModFolder, FILE_KINDS,
            MESH_KIND,
        },
        verbs::{add_asset, delete_asset, duplicate_asset, rename_asset},
    },
    camera::ViewportPointer,
    config::{EditorCamera, EditorSays, SelectedNode},
    frame::CameraFraming,
    glyph::asset_mark,
    history::EditHistory,
    placement::place_design,
    problems::Problems,
    project::{
        ship::{is_ship_kind, owner_scenario},
        store,
        verbs::{
            add_scenario, delete_scenario, duplicate_scenario, open_asset, pressed, rename_scenario,
        },
        KindRow, ModProject, OpenAsset, KINDS, SCENARIO_KIND,
    },
    ExampleStates,
};

/// A tile's width: [`LINE_CHARS`] of [`density::TEXT_SMALL`] text and the
/// tile's padding.
const TILE_W: f32 = 96.0;

/// A tile's height: the picture, the name line and the kind line.
const TILE_H: f32 = 92.0;

/// A tile's picture.
const THUMB_W: f32 = 54.0;

/// The dot in a tile's corner that marks the item open in the editor.
const OPEN_DOT: f32 = 7.0;

/// The glyph standing in for a picture.
const GLYPH_TEXT: f32 = 26.0;

/// The width of the kind column: a scenario shelf one step in, its mark, a
/// dozen characters of its name and its count.
const KIND_W: f32 = 150.0;

/// The characters one line of a tile holds. The UI font is monospace.
const LINE_CHARS: usize = 16;

/// The mark on a file the package leaves out. DOTTED CIRCLE - a place the
/// package holds nothing.
pub(crate) const UNPACKAGED_MARK: &str = "\u{25cc}";

/// One tile of the Assets tab.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AssetEntry {
    /// The row of [`KINDS`] or of [`FILE_KINDS`] it lists under.
    pub(crate) kind: &'static str,
    /// The short name: what a rename starts from, and what a filter reads.
    pub(crate) label: String,
    /// The whole id or path, for the tile's hover hint.
    pub(crate) full: String,
    /// The name line the tile prints. See [`name_tiles`].
    name: String,
    /// The muted line under the name: the kind, or what tells the tile from
    /// one that prints the same name. See [`name_tiles`].
    note: String,
    /// Why it does not open, on a tile that does not.
    pub(crate) waits: &'static str,
    /// The item, on a tile that holds verbs.
    pub(crate) id: Option<OpenAsset>,
    /// What its picture shows. Empty for a kind that keeps its glyph.
    parts: ThumbnailParts,
    /// The file, on a file tile.
    pub(crate) file: Option<ModFile>,
    /// The scenario that owns it: the design of a ship a scenario made.
    pub(crate) scenario: Option<String>,
}

/// `id` without the mod's prefix, which every id of the mod repeats.
fn short_id<'a>(id: &'a str, namespace: &str) -> &'a str {
    id.strip_prefix(&format!("{}__", namespace.replace('-', "_")))
        .unwrap_or(id)
}

/// The name a tile prints for `item`. A design id is `<scenario>.<node>`, and
/// the node is the name: the scenario would fill the line and repeat on every
/// tile. A scenario has a name of its own.
pub(crate) fn short_name(item: &Content, namespace: &str) -> String {
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
/// A tile prints its short name, cut to one line, over its kind. Two scenarios
/// of a mod can hold the same node, so a short name alone repeats: a tile that
/// would print like another takes its [`qualifier`] as the muted line instead
/// of the kind, which its glyph and tint already say. Ids that still read
/// alike once cut are numbered.
fn name_tiles(entries: &mut [AssetEntry], namespace: &str) {
    let mut printing: HashMap<(&'static str, String), usize> = HashMap::new();
    for entry in entries.iter_mut() {
        entry.name = cut_end(&entry.label, LINE_CHARS);
        entry.note = cut_end(&entry.kind.to_uppercase(), LINE_CHARS);
        *printing
            .entry((entry.kind, entry.name.clone()))
            .or_default() += 1;
    }
    let mut seen: HashMap<(&'static str, String, String), usize> = HashMap::new();
    for entry in entries.iter_mut() {
        if printing[&(entry.kind, entry.name.clone())] < 2 {
            continue;
        }
        let scope = qualifier(entry, namespace);
        entry.note = cut_start(&scope, LINE_CHARS);
        let count = seen
            .entry((entry.kind, entry.name.clone(), entry.note.clone()))
            .or_default();
        *count += 1;
        if *count > 1 {
            let mark = format!(" #{count}");
            let room = LINE_CHARS - mark.len();
            entry.note = format!("{}{mark}", cut_start(&scope, room));
        }
    }
}

/// Everything the mod holds, in [`KINDS`] order, then the files of its folder
/// (`on_disk`) and the files it declares, in [`FILE_KINDS`] order. The ships a
/// scenario made follow that scenario, not their kind: they are its.
pub(crate) fn asset_entries(
    project: &ModProject,
    catalog: Option<&GameSections>,
    on_disk: &[String],
) -> Vec<AssetEntry> {
    let data = &project.data;
    let owner = |item: &Content| {
        is_ship_kind(item.kind())
            .then(|| owner_scenario(data, item.id()))
            .flatten()
    };
    let mut items: Vec<(&Content, &KindRow, Option<String>)> = Vec::new();
    for row in &KINDS {
        for item in data
            .items()
            .filter(|item| item.kind() == row.kind && owner(item).is_none())
        {
            items.push((item, row, None));
            if row.kind != SCENARIO_KIND {
                continue;
            }
            for owned in KINDS.iter().filter(|owned| is_ship_kind(owned.kind)) {
                for design in data.items().filter(|design| {
                    design.kind() == owned.kind && owner(design).as_deref() == Some(item.id())
                }) {
                    items.push((design, owned, Some(item.id().to_string())));
                }
            }
        }
    }
    let mut entries = Vec::new();
    for (item, row, scenario) in items {
        let parts = match (item, catalog) {
            // The catalog's copy has its `self://` refs resolved by
            // `sync_project_sections`; the document's would reach the asset
            // server as an unknown source. Until the catalog holds it, the
            // tile has no picture.
            (Content::Section(section), Some(catalog)) => catalog
                .get_section(&section.base.id)
                .map(|resolved| vec![(resolved.clone(), Transform::default())])
                .unwrap_or_default(),
            (Content::Ship(ship), Some(catalog)) => design_parts(&ship.design, catalog),
            _ => Vec::new(),
        };
        entries.push(AssetEntry {
            kind: row.kind,
            label: short_name(item, &project.namespace),
            full: item.id().to_string(),
            name: String::new(),
            note: String::new(),
            waits: row.waits,
            id: row.opens.then(|| OpenAsset {
                kind: row.kind,
                id: item.id().to_string(),
            }),
            parts,
            file: None,
            scenario,
        });
    }
    entries.extend(mod_files(project, on_disk).into_iter().map(|file| {
        let kind = file_kind(&file.path);
        let parts = match &project.id {
            Some(id) if kind == MESH_KIND && file.on_disk => mesh_parts(id, &file.path),
            _ => Vec::new(),
        };
        AssetEntry {
            kind,
            label: file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&file.path)
                .to_string(),
            full: file.path.clone(),
            name: String::new(),
            note: String::new(),
            waits: "",
            id: None,
            parts,
            file: Some(file),
            scenario: None,
        }
    }));
    name_tiles(&mut entries, &project.namespace);
    entries
}

/// The tiles on `shelf` that a filter text keeps. A blank filter keeps them
/// all.
pub(crate) fn filtered_entries(
    entries: &[AssetEntry],
    shelf: &AssetShelfType,
    filter: &str,
) -> Vec<AssetEntry> {
    let filter = filter.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| shelf.holds(entry))
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
        .map(|row| (row.kind, row.heading))
        .chain(FILE_KINDS)
        .find(|(held, _)| *held == kind)
        .map_or("Files", |(_, heading)| heading)
}

/// A kind's colour, the one thing that tells two glyph tiles apart at a glance.
fn kind_tint(kind: &str) -> UiColor {
    match kind {
        SCENARIO_KIND | "campaign" => UiColor::Accent,
        "section" | "ship" | "station" => UiColor::Primary,
        _ if FILE_KINDS.iter().any(|(held, _)| *held == kind) => UiColor::Label,
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

/// What the grid is narrowed to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum AssetShelfType {
    /// Every tile.
    #[default]
    All,
    /// The items of one kind that no scenario owns, or the files of one kind.
    Kind(&'static str),
    /// One scenario and the ships it made.
    Scenario(String),
}

impl AssetShelfType {
    /// Whether `entry` is on this shelf.
    fn holds(&self, entry: &AssetEntry) -> bool {
        match self {
            Self::All => true,
            Self::Kind(kind) => entry.kind == *kind && entry.scenario.is_none(),
            Self::Scenario(id) => {
                entry.scenario.as_ref() == Some(id)
                    || (entry.kind == SCENARIO_KIND && entry.full == *id)
            }
        }
    }
}

/// A row of the kind column, and the shelf it shows.
#[derive(Component, Clone)]
struct AssetKindRow(AssetShelfType);

/// The shelf the grid shows.
#[derive(Resource, Default, PartialEq, Eq)]
pub(crate) struct AssetKindFilter(AssetShelfType);

/// What a rename on a tile renames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RenameTarget {
    /// An item of the mod.
    Item(OpenAsset),
    /// A file in its folder, by path.
    File(String),
}

/// The tile whose name is being typed on it.
#[derive(Resource, Default)]
pub(crate) struct AssetRenaming(pub(crate) Option<RenameTarget>);

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
    /// What the rename renames.
    target: RenameTarget,
    /// The text the box opened on. A rename that leaves it there changes
    /// nothing, even where the stored id or name reads otherwise.
    seed: String,
}

/// What a menu row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetVerbType {
    Open,
    Place,
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
                observe(on_asset_grid_menu),
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

/// The New Asset tree: a row a kind of the mod. Each row makes a blank item
/// and opens it; nothing starts from another item. A section asks its kind
/// first, in a modal. A kind made elsewhere is greyed with the reason.
///
/// The `+`, the Add menu's Asset group and the right-click on empty Assets
/// space all hold it.
pub(crate) fn new_asset_items() -> Vec<CascadeItem> {
    KINDS
        .iter()
        .map(|row| {
            let kind = row.kind;
            let name = format!("Asset Add {}", row.heading);
            if !row.add {
                CascadeItem::leaf(name, kind_label(kind))
                    .lead(asset_mark(kind))
                    .greyed(row.waits)
            } else if kind == "section" {
                CascadeItem::leaf(name, format!("{}...", kind_label(kind)))
                    .lead(asset_mark(kind))
                    .on_row(|row| {
                        row.insert(observe(on_asset_add_section));
                    })
            } else {
                CascadeItem::leaf(name, kind_label(kind))
                    .lead(asset_mark(kind))
                    .on_row(move |row| {
                        row.insert((AssetAddRow(kind), observe(on_asset_add_row)));
                    })
            }
        })
        .collect()
}

/// A kind as a row names it: `ui theme` is `UI Theme`.
fn kind_label(kind: &str) -> String {
    kind.split(' ')
        .map(|word| match word {
            "ui" => "UI".to_string(),
            word => {
                let mut letters = word.chars();
                letters.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(letters).collect()
                })
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Import File, as a row of a menu over the Assets tab.
pub(crate) fn import_file_item() -> CascadeItem {
    CascadeItem::leaf("Asset Add Import File", "Import File...")
        .hint("a file for the folder")
        .on_row(|row| {
            row.insert(observe(on_import_file));
        })
}

/// `+`: the New Asset tree under the button, then Import File where the build
/// writes a folder.
fn on_asset_add(
    activate: On<Activate>,
    mut commands: Commands,
    scale: Option<Res<UiScale>>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform)>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    let Ok((computed, at)) = buttons.get(activate.entity) else {
        return;
    };
    let at = below(computed, at, scale.map_or(1.0, |scale| scale.0));
    let mut items = new_asset_items();
    if store::CAN_SAVE {
        items.push(CascadeItem::separator());
        items.push(import_file_item());
    }
    close_popups(&mut commands, &roots);
    let root = spawn_cascade(&mut commands, at, layer::MENU_Z, items);
    commands
        .entity(root)
        .insert(DespawnOnExit(ExampleStates::Editor));
}

/// An Add row: a blank item of its kind, open.
fn on_asset_add_row(activate: On<Activate>, mut commands: Commands, rows: Query<&AssetAddRow>) {
    let Ok(AssetAddRow(kind)) = rows.get(activate.entity).copied() else {
        return;
    };
    commands.queue(move |world: &mut World| {
        pressed(world, |world| {
            if kind == SCENARIO_KIND {
                add_scenario(world);
            } else {
                add_asset(world, kind, None);
            }
        });
    });
}

/// New Asset > Section: the modal that asks its kind.
fn on_asset_add_section(_activate: On<Activate>, mut commands: Commands) {
    open_section_kind_modal(&mut commands);
}

/// A press on a tile marks it, and a second one opens it.
fn on_asset_tile(
    activate: On<Activate>,
    mut commands: Commands,
    tiles: Query<(Entity, &AssetTile, Has<Selected>)>,
    files: Query<&AssetFileTile>,
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
    let opened = last.press(activate.entity, time.elapsed_secs());
    if let Ok(AssetFileTile(file)) = files.get(activate.entity) {
        let file = file.clone();
        if opened {
            commands.queue(move |world: &mut World| view_file(world, &file));
        } else {
            says.accept();
        }
        return;
    }
    let Ok((_, AssetTile(Some(asset)), _)) = tiles.get(activate.entity) else {
        says.accept();
        return;
    };
    if opened {
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
    tiles: Query<(&AssetTile, Option<&AssetFileTile>), With<Selected>>,
) {
    if !keys.just_pressed(KeyCode::Enter) || *open != BrowserTabType::Assets || !panels.browser {
        return;
    }
    if let Some((_, Some(AssetFileTile(file)))) = tiles.iter().next() {
        let file = file.clone();
        commands.queue(move |world: &mut World| view_file(world, &file));
        return;
    }
    if let Some((AssetTile(Some(asset)), _)) = tiles.iter().next() {
        let asset = asset.clone();
        commands.queue(move |world: &mut World| open_asset(world, asset.kind, &asset.id));
    }
}

/// A right press on a tile: its verbs. A tile with none says why.
fn on_asset_tile_menu(
    click: On<PointerClick>,
    mut commands: Commands,
    tiles: Query<(&AssetTile, Option<&AssetFileTile>)>,
    entries: Query<&AssetWaits>,
    roots: Query<Entity, With<PopupRoot>>,
    facts: MenuFacts,
    selected: Res<SelectedNode>,
    project: Option<Res<ModProject>>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let Ok((AssetTile(asset), file)) = tiles.get(click.event_target()) else {
        return;
    };
    let file = file.map(|tile| tile.0.clone());
    let context = facts.context(&selected);
    close_popups(&mut commands, &roots);
    let asset = asset.clone();
    let waits = entries
        .get(click.event_target())
        .map_or("", |waits| waits.0);
    let root = spawn_popup(
        &mut commands,
        click.pointer.position,
        layer::MENU_Z,
        |menu| {
            if let Some(file) = &file {
                spawn_file_rows(menu, file, &context);
                return;
            }
            let Some(asset) = asset else {
                menu.spawn((
                    Name::new("Asset Menu Waits"),
                    popup_row(waits, ""),
                    InteractionDisabled,
                ));
                return;
            };
            let in_scenario = project.as_ref().is_some_and(|project| {
                is_ship_kind(asset.kind) && owner_scenario(&project.data, &asset.id).is_some()
            });
            for (word, hint, verb) in [
                ("Open", "Enter", AssetVerbType::Open),
                ("Place in Scenario", "", AssetVerbType::Place),
                ("Rename", "", AssetVerbType::Rename),
                ("Duplicate", "", AssetVerbType::Duplicate),
                ("Delete", "", AssetVerbType::Delete),
            ] {
                let tile = MenuVerbType::Tile(verb, asset.kind, in_scenario);
                if verb_state(tile, &context) == VerbState::Hidden {
                    continue;
                }
                menu.spawn((
                    Name::new(format!("Asset Menu {word}")),
                    tile,
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

/// A ship design tile dropped on the stage places the design where it fell, at
/// the framing distance along the pointer's ray. A drop on a panel does
/// nothing.
fn on_asset_tile_drop(
    drop: On<PointerDragEnd>,
    mut commands: Commands,
    tiles: Query<&AssetTile>,
    pointer: Res<ViewportPointer>,
    framing: Res<CameraFraming>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<EditorCamera>>>,
) {
    if drop.button != PointerButton::Primary || pointer.over_ui {
        return;
    }
    let Ok(AssetTile(Some(OpenAsset { kind: "ship", id }))) = tiles.get(drop.event_target()) else {
        return;
    };
    let at = camera.and_then(|camera| {
        let (camera, pose) = *camera;
        let ray = camera.viewport_to_world(pose, drop.pointer.position).ok()?;
        Some(ray.origin + *ray.direction * framing.distance.to_engine())
    });
    let id = id.clone();
    commands.queue(move |world: &mut World| {
        pressed(world, |world| place_design(world, &id, at));
    });
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
        AssetVerbType::Place => {
            commands.queue(move |world: &mut World| {
                pressed(world, |world| place_design(world, &id, None));
            });
        }
        AssetVerbType::Rename => {
            renaming.0 = Some(RenameTarget::Item(asset.clone()));
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
    if let Ok(AssetKindRow(shelf)) = rows.get(activate.entity) {
        filter.0 = shelf.clone();
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
        let Ok(AssetRenameField { target, seed }) = fields.get(*entity) else {
            continue;
        };
        let typed = value.trim().to_string();
        if typed.is_empty() || typed == seed.trim() {
            continue;
        }
        let target = target.clone();
        commands.queue(move |world: &mut World| match target {
            RenameTarget::Item(OpenAsset { kind, id }) if kind == SCENARIO_KIND => {
                rename_scenario(world, &id, &typed);
            }
            RenameTarget::Item(OpenAsset { kind, id }) => rename_asset(world, kind, &id, &typed),
            RenameTarget::File(file) => rename_file(world, &file, &typed),
        });
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
    kind: AssetShelfType,
    open: Option<OpenAsset>,
    renaming: Option<RenameTarget>,
    faulted: Vec<OpenAsset>,
    /// The folder's reading: a file verb that writes only the folder moves it.
    folder: u64,
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
    (folder, server): (Res<ModFolder>, Res<AssetServer>),
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
        || catalog.as_ref().is_some_and(Res::is_changed)
        || folder.is_changed();
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
        // The folder is read here, once a rebuild, never a frame.
        let on_disk = project
            .id
            .as_deref()
            .map(store::files_of)
            .unwrap_or_default();
        *built = Some((
            revision,
            asset_entries(&project, catalog.as_deref(), &on_disk),
        ));
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
        kind: kind.0.clone(),
        faulted: entries
            .iter()
            .filter_map(|entry| entry.id.clone())
            .filter(|id| is_faulted(id, open_item.as_ref(), &problems))
            .collect(),
        open: open_item,
        renaming: renaming.0.clone(),
        folder: folder.0,
    };
    if shown.as_ref() == Some(&wanted) {
        return;
    }
    let (Ok(list), Ok(kind_list)) = (lists.single(), kinds.single()) else {
        return;
    };
    commands.entity(kind_list).despawn_related::<Children>();
    commands.entity(kind_list).with_children(|list| {
        let all = AssetShelfType::All;
        spawn_kind_row(
            list,
            all.clone(),
            "All",
            entries.len(),
            0,
            wanted.kind == all,
        );
        let mut held: Vec<&'static str> = Vec::new();
        for entry in entries.iter().filter(|entry| entry.scenario.is_none()) {
            if !held.contains(&entry.kind) {
                held.push(entry.kind);
            }
        }
        for held_kind in held {
            let shelf = AssetShelfType::Kind(held_kind);
            let count = entries.iter().filter(|entry| shelf.holds(entry)).count();
            let chosen = wanted.kind == shelf;
            spawn_kind_row(list, shelf, heading_of(held_kind), count, 0, chosen);
            if held_kind != SCENARIO_KIND {
                continue;
            }
            // Each scenario that made ships, one step in: the shelf of the
            // scenario and its ships.
            for scenario in entries
                .iter()
                .filter(|entry| entry.kind == SCENARIO_KIND && entry.scenario.is_none())
            {
                let shelf = AssetShelfType::Scenario(scenario.full.clone());
                let ships = entries
                    .iter()
                    .filter(|entry| entry.scenario.as_ref() == Some(&scenario.full))
                    .count();
                if ships == 0 {
                    continue;
                }
                let chosen = wanted.kind == shelf;
                spawn_kind_row(list, shelf, &scenario.label, ships, 1, chosen);
            }
        }
    });
    let tiles = filtered_entries(entries, &wanted.kind, &wanted.filter);
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        if tiles.is_empty() {
            list.spawn((
                Name::new("Asset Grid Empty"),
                pane_text(
                    "nothing here: + adds an item, a right click imports a file",
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
            let image = entry
                .file
                .as_ref()
                .zip(project.id.as_deref())
                .filter(|(file, _)| file.on_disk && loads_as_image(&file.path))
                .map(|(file, id)| server.load(file_asset_path(id, &file.path)));
            spawn_tile(
                list,
                entry,
                (key, image),
                faulted,
                held_open,
                wanted.renaming.as_ref(),
            );
        }
    });
    *shown = Some(wanted);
}

/// One row of the kind column, `depth` steps in: a Hierarchy row, with the
/// kind's mark in front and the count where a node's child count stands.
fn spawn_kind_row(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    shelf: AssetShelfType,
    heading: &str,
    count: usize,
    depth: usize,
    chosen: bool,
) {
    let lead = match &shelf {
        AssetShelfType::All => String::new(),
        AssetShelfType::Kind(kind) => asset_mark(kind).to_string(),
        AssetShelfType::Scenario(_) => asset_mark(SCENARIO_KIND).to_string(),
    };
    let spec = TreeRowSpec {
        depth,
        lead,
        label: heading.to_string(),
        count: count.to_string(),
        ..default()
    };
    let mut row = list.spawn((
        Name::new(format!("Asset Kind {heading}")),
        AssetKindRow(shelf),
        tree_row(&spec),
        observe(on_asset_kind),
    ));
    if chosen {
        row.insert(Selected);
    }
}

/// One line under a tile's picture, already cut to the tile.
fn tile_line(line: &str, colour: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(line),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            justify: Justify::Center,
        },
        TextFont {
            font_size: FontSize::Px(density::TEXT_SMALL),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
        Pickable::IGNORE,
    )
}

/// One tile.
fn spawn_tile(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    entry: &AssetEntry,
    (picture, image): (Option<u64>, Option<Handle<Image>>),
    faulted: bool,
    held_open: bool,
    renaming: Option<&RenameTarget>,
) {
    let target = match (&entry.id, &entry.file) {
        (Some(asset), _) => Some(RenameTarget::Item(asset.clone())),
        (None, Some(file)) => Some(RenameTarget::File(file.path.clone())),
        (None, None) => None,
    };
    let typing = target.is_some() && target.as_ref() == renaming;
    let hint = match &entry.file {
        Some(file) if !file.on_disk => format!("{} (declared, but not in the folder)", entry.full),
        Some(file) if !file.packaged => {
            format!(
                "{} (not in the package: Include in Package adds it)",
                entry.full
            )
        }
        _ if held_open => format!("{} (open)", entry.full),
        _ => entry.full.clone(),
    };
    let mut tile = list.spawn((
        Name::new(format!("Asset Tile {}", entry.label)),
        AssetTile(entry.id.clone()),
        AssetWaits(entry.waits),
        ListRow,
        Tooltip(hint),
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
            observe(on_asset_tile_drop),
        ));
    }
    if let Some(file) = &entry.file {
        tile.insert(AssetFileTile(file.clone()));
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
            let slot = Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                display: Display::None,
                ..default()
            };
            if let Some(key) = picture {
                face.spawn((
                    Name::new("Asset Tile Picture"),
                    AssetThumb(key),
                    slot.clone(),
                    Pickable::IGNORE,
                ));
            }
            if let Some(image) = image {
                face.spawn((
                    Name::new("Asset Tile Picture"),
                    AssetImage(image),
                    slot,
                    Pickable::IGNORE,
                ));
            }
        });
        // The marks stand in the tile's top corners, beside the picture and
        // never on it.
        let missing = entry.file.as_ref().is_some_and(|file| !file.on_disk);
        let unpackaged = entry.file.as_ref().is_some_and(|file| !file.packaged);
        if unpackaged {
            tile.spawn((
                Name::new("Asset Tile Unpackaged"),
                pane_text(UNPACKAGED_MARK, density::TEXT, UiColor::Label),
                Node {
                    position_type: PositionType::Absolute,
                    top: px(1),
                    right: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        if faulted || missing {
            tile.spawn((
                Name::new("Asset Tile Fault"),
                pane_text("!", density::TEXT, UiColor::Danger),
                Node {
                    position_type: PositionType::Absolute,
                    top: px(1),
                    right: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
        if held_open {
            tile.spawn((
                Name::new("Asset Tile Open"),
                Node {
                    position_type: PositionType::Absolute,
                    top: px(5),
                    left: px(5),
                    width: px(OPEN_DOT),
                    height: px(OPEN_DOT),
                    border_radius: BorderRadius::all(px(OPEN_DOT / 2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColor::Accent),
                Pickable::IGNORE,
            ));
        }
        match target.filter(|_| typing) {
            Some(target) => {
                tile.spawn((
                    Name::new("Asset Rename"),
                    AssetRenameField {
                        target,
                        seed: entry.label.clone(),
                    },
                    TextFieldFocused::at_end(&entry.label),
                    text_field(TextFieldSpec::new(&entry.label).max_chars(64).tool()),
                ));
            }
            None => {
                tile.spawn((
                    Name::new("Asset Tile Name"),
                    tile_line(&entry.name, UiColor::Primary),
                ));
                tile.spawn((
                    Name::new("Asset Tile Kind"),
                    tile_line(&entry.note, UiColor::Label),
                ));
            }
        }
    });
}

/// Hang each picture on its tile once it is taken. A tile whose picture never
/// comes keeps its glyph.
///
/// An image file is its own picture, sized to keep its shape.
pub(crate) fn sync_asset_thumbs(
    mut commands: Commands,
    thumbnails: Res<AssetThumbnails>,
    images: Res<Assets<Image>>,
    mut slots: Query<
        (
            Entity,
            Option<&AssetThumb>,
            Option<&AssetImage>,
            &mut Node,
            &ChildOf,
        ),
        (Without<ImageNode>, Or<(With<AssetThumb>, With<AssetImage>)>),
    >,
    faces: Query<&Children>,
    glyphs: Query<(), With<AssetGlyph>>,
) {
    for (entity, thumb, image, mut node, parent) in &mut slots {
        let picture = match (thumb, image) {
            (Some(thumb), _) => thumbnails.picture(thumb.0).cloned(),
            (None, Some(AssetImage(handle))) => images.get(handle).map(|image| {
                let size = fit(image.size_f32(), Vec2::splat(THUMB_W));
                node.width = px(size.x);
                node.height = px(size.y);
                node.position_type = PositionType::Relative;
                handle.clone()
            }),
            (None, None) => None,
        };
        let Some(picture) = picture else {
            continue;
        };
        commands.entity(entity).insert(ImageNode::new(picture));
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
