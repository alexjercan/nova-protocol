//! A cascading menu: rows that open submenus to their right, under a search
//! field that filters every level at once.
//!
//! The tree is data ([`CascadeItem`]). Where the keys and the pointer stand is
//! [`CascadeNav`], which is pure, so the keyboard and the search are tested
//! without an app. The ECS half spawns one column per open level inside a
//! popup catcher ([`PopupRoot`]): a press outside, Escape and [`close_popups`]
//! close it as they close any popup.
//!
//! A LEAF does its work through the caller's bundle ([`CascadeItem::on_row`]):
//! usually an observer of [`Activate`], so a click, and Enter on the
//! highlighted row, run the same code. A live leaf is a [`PopupRow`], so the
//! menu closes after it acts.

use std::sync::Arc;

use bevy::{
    ecs::{relationship::RelatedSpawnerCommands, system::EntityCommands},
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, UiGlobalTransform},
    ui_widgets::{Activate, Button},
};

use super::{
    panel, text_field, ListRow, PopupPanel, PopupRoot, PopupRow, Selected, TextFieldFocused,
    TextFieldSpec, TextFieldValue, ThemedRadius, ThemedSeparator, ThemedText, Tooltip, UiText,
};
use crate::{
    input_mode::{ClaimKeyboard, InputMode},
    theme::{density, UiColorType, BORDER_W},
};

/// What a leaf row carries beside the menu's own components: the observer that
/// does its work, and any component that observer reads.
///
/// A function rather than a bundle, because a column is spawned again each
/// time it opens or the search changes.
pub type CascadeRowBundle = Arc<dyn Fn(&mut EntityCommands) + Send + Sync>;

/// One row of a cascading menu: a leaf, a submenu or a separator.
#[derive(Clone, Default)]
pub struct CascadeItem {
    /// The row entity's `Name`, which walks aim at.
    pub name: String,
    /// What the row says.
    pub label: String,
    /// A muted note at the right, such as the key that runs the row.
    pub hint: String,
    /// A mark in the lead column.
    pub lead: String,
    /// Why the row cannot run now. A greyed row shows and does nothing.
    pub greyed: Option<String>,
    /// The rows of its submenu. A row with children is a submenu.
    pub children: Vec<CascadeItem>,
    /// A rule between rows, which keys and search skip.
    pub separator: bool,
    /// What the row carries for its work.
    pub row: Option<CascadeRowBundle>,
}

impl std::fmt::Debug for CascadeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CascadeItem")
            .field("name", &self.name)
            .field("greyed", &self.greyed)
            .field("children", &self.children)
            .field("separator", &self.separator)
            .finish_non_exhaustive()
    }
}

impl CascadeItem {
    /// A row that does something when chosen.
    pub fn leaf(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            ..default()
        }
    }

    /// A row that opens `children` to its right.
    pub fn submenu(
        name: impl Into<String>,
        label: impl Into<String>,
        children: Vec<CascadeItem>,
    ) -> Self {
        Self {
            children,
            ..Self::leaf(name, label)
        }
    }

    /// A rule between two groups of rows.
    pub fn separator() -> Self {
        Self {
            name: "Cascade Separator".to_string(),
            separator: true,
            ..default()
        }
    }

    /// The muted note at the right.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    /// The mark in the lead column.
    pub fn lead(mut self, lead: impl Into<String>) -> Self {
        self.lead = lead.into();
        self
    }

    /// Grey the row, with `reason` on its hover.
    pub fn greyed(mut self, reason: impl Into<String>) -> Self {
        self.greyed = Some(reason.into());
        self
    }

    /// What the row carries for its work.
    pub fn on_row(mut self, row: impl Fn(&mut EntityCommands) + Send + Sync + 'static) -> Self {
        self.row = Some(Arc::new(row));
        self
    }

    /// Whether the row opens a submenu.
    pub fn is_submenu(&self) -> bool {
        !self.children.is_empty()
    }

    /// Whether the keys and the pointer may rest on the row. A greyed row
    /// cannot run, so it takes no highlight; the pointer on it shows why.
    fn stops_keys(&self) -> bool {
        !self.separator && self.greyed.is_none()
    }
}

/// The item at `path` of `items`: an index per level.
pub fn cascade_item<'a>(items: &'a [CascadeItem], path: &[usize]) -> Option<&'a CascadeItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get(*first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        cascade_item(&item.children, rest)
    }
}

/// One row of one column: where its item sits in the tree, and for a search
/// result, the labels of the submenus over it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CascadeEntry {
    /// The item's path in the tree.
    pub path: Vec<usize>,
    /// The submenus over a search result, as `Ship > Designs`. Empty outside a
    /// search.
    pub crumb: String,
}

/// Where the keys and the pointer stand in a cascading menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CascadeNav {
    /// The highlighted row of each open column, the root first. A column the
    /// pointer opened has no highlight until a key or the pointer gives it one.
    pub levels: Vec<Option<usize>>,
    /// The search text. While it holds something, the root column lists every
    /// leaf that matches it, and the menu opens no submenu.
    pub query: String,
}

impl Default for CascadeNav {
    fn default() -> Self {
        Self {
            levels: vec![None],
            query: String::new(),
        }
    }
}

impl CascadeNav {
    /// The rows of column `depth`, or none past the open columns.
    pub fn column(&self, items: &[CascadeItem], depth: usize) -> Vec<CascadeEntry> {
        if depth == 0 {
            if self.query.trim().is_empty() {
                return (0..items.len())
                    .map(|index| CascadeEntry {
                        path: vec![index],
                        crumb: String::new(),
                    })
                    .collect();
            }
            return search_cascade(items, &self.query);
        }
        let Some(Some(highlight)) = self.levels.get(depth - 1) else {
            return Vec::new();
        };
        let above = self.column(items, depth - 1);
        let Some(parent) = above.get(*highlight) else {
            return Vec::new();
        };
        let Some(item) = cascade_item(items, &parent.path) else {
            return Vec::new();
        };
        (0..item.children.len())
            .map(|index| {
                let mut path = parent.path.clone();
                path.push(index);
                CascadeEntry {
                    path,
                    crumb: String::new(),
                }
            })
            .collect()
    }

    /// The deepest open column.
    pub fn depth(&self) -> usize {
        self.levels.len().saturating_sub(1)
    }

    /// The highlighted entry of the deepest column and its item.
    fn highlighted<'a>(&self, items: &'a [CascadeItem]) -> Option<(CascadeEntry, &'a CascadeItem)> {
        let index = (*self.levels.last()?)?;
        let entry = self.column(items, self.depth()).into_iter().nth(index)?;
        let item = cascade_item(items, &entry.path)?;
        Some((entry, item))
    }

    /// Move the highlight of the deepest column one row down, round to the
    /// top, over the separators.
    pub fn down(&mut self, items: &[CascadeItem]) {
        self.step(items, true);
    }

    /// Move the highlight one row up, round to the bottom.
    pub fn up(&mut self, items: &[CascadeItem]) {
        self.step(items, false);
    }

    fn step(&mut self, items: &[CascadeItem], down: bool) {
        let rows = self.column(items, self.depth());
        let stops: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                cascade_item(items, &entry.path).is_some_and(|item| item.stops_keys())
            })
            .map(|(index, _)| index)
            .collect();
        if stops.is_empty() {
            return;
        }
        let Some(last) = self.levels.last_mut() else {
            return;
        };
        *last = Some(match *last {
            None if down => stops[0],
            None => stops[stops.len() - 1],
            Some(at) if down => stops
                .iter()
                .copied()
                .find(|stop| *stop > at)
                .unwrap_or(stops[0]),
            Some(at) => stops
                .iter()
                .rev()
                .copied()
                .find(|stop| *stop < at)
                .unwrap_or(stops[stops.len() - 1]),
        });
    }

    /// Open the highlighted submenu, its first row highlighted.
    pub fn right(&mut self, items: &[CascadeItem]) {
        if !self.query.trim().is_empty() {
            return;
        }
        let Some((_, item)) = self.highlighted(items) else {
            return;
        };
        if item.is_submenu() {
            self.levels.push(None);
            self.down(items);
        }
    }

    /// Close the deepest submenu. The root column stays.
    pub fn left(&mut self) {
        if self.levels.len() > 1 {
            self.levels.pop();
        }
    }

    /// Enter: open the highlighted submenu, or answer the path of the
    /// highlighted leaf when it can run.
    pub fn enter(&mut self, items: &[CascadeItem]) -> Option<Vec<usize>> {
        let (entry, item) = self.highlighted(items)?;
        if item.is_submenu() {
            if self.query.trim().is_empty() {
                self.right(items);
            }
            return None;
        }
        (item.greyed.is_none() && !item.separator).then_some(entry.path)
    }

    /// The pointer came onto row `index` of column `depth`: highlight it, close
    /// what was open deeper, and open it when it is a submenu. A greyed row
    /// only closes what was open deeper.
    pub fn hover(&mut self, items: &[CascadeItem], depth: usize, index: usize) {
        if depth >= self.levels.len() {
            return;
        }
        let entry = self.column(items, depth).into_iter().nth(index);
        let Some(item) = entry.and_then(|entry| cascade_item(items, &entry.path)) else {
            return;
        };
        if item.separator {
            return;
        }
        self.levels.truncate(depth + 1);
        if !item.stops_keys() {
            self.levels[depth] = None;
            return;
        }
        self.levels[depth] = Some(index);
        if item.is_submenu() && self.query.trim().is_empty() {
            self.levels.push(None);
        }
    }

    /// The search text became `query`: the root column lists what matches,
    /// the first match highlighted.
    pub fn set_query(&mut self, items: &[CascadeItem], query: &str) {
        self.query = query.to_string();
        self.levels = vec![None];
        if !self.query.trim().is_empty() {
            self.down(items);
        }
    }
}

/// Every leaf whose label, or the label of a submenu over it, holds each word
/// of `query`, in tree order. Case does not count.
pub fn search_cascade(items: &[CascadeItem], query: &str) -> Vec<CascadeEntry> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut found = Vec::new();
    walk(items, &mut Vec::new(), &mut Vec::new(), &words, &mut found);
    found
}

fn walk(
    items: &[CascadeItem],
    path: &mut Vec<usize>,
    crumbs: &mut Vec<String>,
    words: &[String],
    found: &mut Vec<CascadeEntry>,
) {
    for (index, item) in items.iter().enumerate() {
        if item.separator {
            continue;
        }
        path.push(index);
        if item.is_submenu() {
            crumbs.push(item.label.clone());
            walk(&item.children, path, crumbs, words, found);
            crumbs.pop();
        } else {
            let said = format!("{} {}", crumbs.join(" "), item.label).to_lowercase();
            if words.iter().all(|word| said.contains(word.as_str())) {
                found.push(CascadeEntry {
                    path: path.clone(),
                    crumb: crumbs.join(" > "),
                });
            }
        }
        path.pop();
    }
}

/// An open cascading menu, on its [`PopupRoot`].
#[derive(Component)]
pub struct CascadeMenu {
    items: Vec<CascadeItem>,
    /// Where the keys and the pointer stand.
    pub nav: CascadeNav,
}

impl CascadeMenu {
    /// The tree the menu shows.
    pub fn items(&self) -> &[CascadeItem] {
        &self.items
    }
}

/// The node that holds the rows of one column, and what it was last filled
/// from.
#[derive(Component)]
pub struct CascadeColumn {
    menu: Entity,
    depth: usize,
    filled: Option<ColumnKey>,
}

/// What a column shows: the root column a search, a submenu the item it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ColumnKey {
    Root(String),
    Under(Vec<usize>),
}

/// One row of an open cascading menu.
#[derive(Component, Clone, Debug)]
pub struct CascadeRow {
    menu: Entity,
    depth: usize,
    index: usize,
    path: Vec<usize>,
}

/// The search field of a cascading menu.
#[derive(Component)]
pub struct CascadeSearch {
    menu: Entity,
    /// [`sync_cascade_search`] has seen the field once. It takes the caret
    /// from the next frame on: see there.
    seen: bool,
}

/// The narrowest a column draws.
const COLUMN_MIN_W: f32 = 170.0;

/// A column's inside margin.
const COLUMN_PAD: f32 = 4.0;

/// The mark at the right of a row that opens a submenu. BLACK RIGHT-POINTING
/// SMALL TRIANGLE.
const SUBMENU_MARK: &str = "\u{25b8}";

/// The characters the search takes.
const SEARCH_CHARS: usize = 40;

/// Open a cascading menu of `items` at `at`, in logical window pixels, on rung
/// `z`. The search field has the caret, so typing searches at once. The caller
/// adds any cleanup marker to the returned catcher, and closes any popup open
/// first.
pub fn spawn_cascade(commands: &mut Commands, at: Vec2, z: i32, items: Vec<CascadeItem>) -> Entity {
    let root = commands
        .spawn((
            Name::new("Cascade Menu"),
            PopupRoot,
            GlobalZIndex(z),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            CascadeMenu {
                items,
                nav: CascadeNav::default(),
            },
        ))
        .id();
    commands.entity(root).with_children(|catcher| {
        catcher
            .spawn((
                Name::new("Cascade Panel"),
                PopupPanel(at),
                column_node(),
                ThemedRadius::control(),
                panel(),
            ))
            .with_children(|column| {
                column.spawn((
                    Name::new("Cascade Search"),
                    CascadeSearch {
                        menu: root,
                        seen: false,
                    },
                    text_field(
                        TextFieldSpec::new("")
                            .placeholder("search")
                            .max_chars(SEARCH_CHARS)
                            .tool(),
                    ),
                ));
                column.spawn((
                    Name::new("Cascade Rows"),
                    CascadeColumn {
                        menu: root,
                        depth: 0,
                        filled: None,
                    },
                    Node {
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::top(px(COLUMN_PAD)),
                        ..default()
                    },
                ));
            });
    });
    root
}

/// The node of a column panel. The root one is placed by `place_popups`, a
/// submenu by [`place_cascade_columns`].
fn column_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        min_width: px(COLUMN_MIN_W),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(px(COLUMN_PAD)),
        border: UiRect::all(px(BORDER_W)),
        ..default()
    }
}

/// One text of a row.
fn row_text(line: &str, colour: UiColorType) -> impl Bundle {
    (
        UiText,
        Text::new(line.to_string()),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextFont {
            font_size: FontSize::Px(density::TEXT),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
        Pickable::IGNORE,
    )
}

/// Spawn the rows of one column into `list`.
fn spawn_rows(
    list: &mut RelatedSpawnerCommands<ChildOf>,
    menu: Entity,
    depth: usize,
    items: &[CascadeItem],
    entries: &[CascadeEntry],
    highlight: Option<usize>,
) {
    for (index, entry) in entries.iter().enumerate() {
        let Some(item) = cascade_item(items, &entry.path) else {
            continue;
        };
        if item.separator {
            list.spawn((
                Name::new("Cascade Separator"),
                ThemedSeparator::Rule,
                Node {
                    width: percent(100),
                    height: px(BORDER_W),
                    min_height: px(BORDER_W),
                    margin: UiRect::vertical(px(3)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
            continue;
        }
        let label_colour = if item.greyed.is_some() {
            UiColorType::TextMuted
        } else {
            UiColorType::Text
        };
        let hint = if entry.crumb.is_empty() {
            item.hint.clone()
        } else {
            entry.crumb.clone()
        };
        let mark = if item.is_submenu() { SUBMENU_MARK } else { "" };
        let mut row = list.spawn((
            Name::new(item.name.clone()),
            CascadeRow {
                menu,
                depth,
                index,
                path: entry.path.clone(),
            },
            Button,
            Hovered::default(),
            Node {
                height: px(density::ROW_H),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::axes(px(6), px(0)),
                border: UiRect::all(px(BORDER_W)),
                ..default()
            },
            ThemedRadius::control(),
            BorderColor::all(Color::NONE),
            BackgroundColor(Color::NONE),
            children![
                // Body, not Label: the highlight's fill drowns a muted line.
                (
                    row_text(&item.lead, UiColorType::TextBody),
                    Node {
                        width: px(12),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ),
                (
                    row_text(&item.label, label_colour),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                ),
                row_text(&hint, UiColorType::TextBody),
                (
                    row_text(mark, UiColorType::TextBody),
                    Node {
                        width: px(10),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ),
            ],
        ));
        // A greyed row is no ListRow: it takes no hover fill, which would
        // drown its muted label. Its tooltip says why it cannot run.
        if let Some(reason) = &item.greyed {
            row.insert((InteractionDisabled, Tooltip(reason.clone())));
        } else {
            row.insert(ListRow);
            if !item.is_submenu() {
                row.insert(PopupRow);
            }
        }
        if highlight == Some(index) {
            row.insert(Selected);
        }
        if let Some(bundle) = &item.row {
            bundle(&mut row);
        }
    }
}

/// Keep each open column filled from the menu's tree and nav, and the
/// highlighted rows marked [`Selected`].
pub(super) fn sync_cascades(
    mut commands: Commands,
    menus: Query<(Entity, &CascadeMenu)>,
    mut columns: Query<(Entity, &mut CascadeColumn)>,
    rows: Query<(Entity, &CascadeRow, Has<Selected>)>,
) {
    for (menu_entity, menu) in &menus {
        let nav = &menu.nav;
        let mut refilled = Vec::new();
        for depth in 0..nav.levels.len() {
            let wanted = if depth == 0 {
                ColumnKey::Root(nav.query.trim().to_string())
            } else {
                let above = nav.column(&menu.items, depth - 1);
                let Some(parent) = nav.levels[depth - 1].and_then(|at| above.get(at)) else {
                    continue;
                };
                ColumnKey::Under(parent.path.clone())
            };
            let existing = columns
                .iter_mut()
                .find(|(_, column)| column.menu == menu_entity && column.depth == depth);
            if let Some((_, column)) = &existing {
                if column.filled.as_ref() == Some(&wanted) {
                    continue;
                }
            }
            refilled.push(depth);
            let entries = nav.column(&menu.items, depth);
            let highlight = nav.levels[depth];
            match existing {
                Some((list, mut column)) if depth == 0 => {
                    column.filled = Some(wanted);
                    commands.entity(list).despawn_related::<Children>();
                    commands.entity(list).with_children(|list| {
                        spawn_rows(list, menu_entity, 0, &menu.items, &entries, highlight);
                    });
                }
                existing => {
                    if let Some((stale, _)) = existing {
                        commands.entity(stale).try_despawn();
                    }
                    commands.entity(menu_entity).with_children(|catcher| {
                        catcher
                            .spawn((
                                Name::new(format!("Cascade Column {depth}")),
                                CascadeColumn {
                                    menu: menu_entity,
                                    depth,
                                    filled: Some(wanted),
                                },
                                column_node(),
                                ThemedRadius::control(),
                                panel(),
                            ))
                            .with_children(|list| {
                                spawn_rows(
                                    list,
                                    menu_entity,
                                    depth,
                                    &menu.items,
                                    &entries,
                                    highlight,
                                );
                            });
                    });
                }
            }
        }
        for (entity, column) in &columns {
            if column.menu == menu_entity && column.depth >= nav.levels.len() {
                commands.entity(entity).try_despawn();
                refilled.push(column.depth);
            }
        }
        for (entity, row, marked) in &rows {
            if row.menu != menu_entity || refilled.contains(&row.depth) {
                continue;
            }
            let wanted = nav.levels.get(row.depth).copied().flatten() == Some(row.index);
            match (wanted, marked) {
                (true, false) => {
                    commands.entity(entity).try_insert(Selected);
                }
                (false, true) => {
                    commands.entity(entity).try_remove::<Selected>();
                }
                _ => {}
            }
        }
    }
}

/// The pointer coming onto a row highlights it, and opens it when it is a
/// submenu.
pub(super) fn cascade_hover(
    rows: Query<(&CascadeRow, &Hovered), Changed<Hovered>>,
    mut menus: Query<&mut CascadeMenu>,
) {
    for (row, hovered) in &rows {
        if !hovered.get() {
            continue;
        }
        let Ok(mut menu) = menus.get_mut(row.menu) else {
            continue;
        };
        let mut nav = menu.nav.clone();
        nav.hover(&menu.items, row.depth, row.index);
        if nav != menu.nav {
            menu.nav = nav;
        }
    }
}

/// A press on a submenu row opens it, as the pointer coming onto it does. A
/// leaf's press is the caller's observer.
pub(super) fn open_cascade_row(
    activate: On<Activate>,
    rows: Query<&CascadeRow>,
    mut menus: Query<&mut CascadeMenu>,
) {
    let Ok(row) = rows.get(activate.entity) else {
        return;
    };
    let Ok(mut menu) = menus.get_mut(row.menu) else {
        return;
    };
    let is_submenu = cascade_item(&menu.items, &row.path).is_some_and(CascadeItem::is_submenu);
    let open = menu.nav.levels.get(row.depth).copied().flatten() == Some(row.index)
        && menu.nav.levels.len() > row.depth + 1;
    if is_submenu && !open {
        let mut nav = menu.nav.clone();
        nav.hover(&menu.items, row.depth, row.index);
        menu.nav = nav;
    }
}

/// The search field's text is the menu's search, and the field keeps the
/// caret while the menu is open: a press on a row takes it, and typing must
/// still search.
///
/// Not on the first frame the field is seen: a key that opened the menu
/// (Shift+A) left its text in the keyboard messages, and a field focused on
/// that frame reads them on the next one, so the menu would open searching
/// for "A". This system runs after the field's typing, so a caret given a
/// frame late meets only messages written after the key.
pub(super) fn sync_cascade_search(
    mut commands: Commands,
    mut fields: Query<(
        Entity,
        &mut CascadeSearch,
        &TextFieldValue,
        Has<TextFieldFocused>,
    )>,
    mut menus: Query<&mut CascadeMenu>,
) {
    for (field, mut search, value, focused) in &mut fields {
        if !search.seen {
            search.seen = true;
            continue;
        }
        let Ok(mut menu) = menus.get_mut(search.menu) else {
            continue;
        };
        if menu.nav.query != value.0 {
            let mut nav = menu.nav.clone();
            nav.set_query(&menu.items, &value.0);
            menu.nav = nav;
        }
        if !focused {
            commands
                .entity(field)
                .try_insert(TextFieldFocused::at_end(&value.0));
        }
    }
}

/// The menu's keys: Up and Down move the highlight, Right and Enter open a
/// submenu, Left closes one, Enter runs a leaf and Escape closes the menu.
/// While a search is typed, Left and Right move the caret instead.
///
/// After the text field's own keys, so a close here lands after the field lets
/// go of the caret.
pub(super) fn cascade_keys(
    mut commands: Commands,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut menus: Query<(Entity, &mut CascadeMenu)>,
    rows: Query<(Entity, &CascadeRow)>,
) {
    let Some(keys) = keys else {
        return;
    };
    let Some((root, mut menu)) = menus.iter_mut().next() else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        commands.entity(root).try_despawn();
        return;
    }
    let mut nav = menu.nav.clone();
    let items = &menu.items;
    if keys.just_pressed(KeyCode::ArrowDown) {
        nav.down(items);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        nav.up(items);
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        nav.right(items);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) && nav.query.trim().is_empty() {
        nav.left();
    }
    let chosen = if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        nav.enter(items)
    } else {
        None
    };
    let depth = nav.depth();
    if nav != menu.nav {
        menu.nav = nav;
    }
    let Some(path) = chosen else {
        return;
    };
    if let Some((row, _)) = rows
        .iter()
        .find(|(_, row)| row.menu == root && row.depth == depth && row.path == path)
    {
        commands.trigger(Activate { entity: row });
    }
}

/// An open cascading menu holds the keyboard: its arrows and Enter are not the
/// verbs behind it. Its search field claims Insert over this while it has the
/// caret.
pub(super) fn a_cascade_claims_the_keyboard(
    menus: Query<(), With<CascadeMenu>>,
    mut claims: MessageWriter<ClaimKeyboard>,
) {
    if !menus.is_empty() {
        claims.write(ClaimKeyboard(InputMode::Browse));
    }
}

/// Hang each submenu column off the row that opened it: at its right, or at
/// its left where the window has no room, and inside the window.
pub(super) fn place_cascade_columns(
    menus: Query<(&CascadeMenu, &ComputedNode)>,
    rows: Query<(&CascadeRow, &ComputedNode, &UiGlobalTransform)>,
    mut columns: Query<(&CascadeColumn, &ComputedNode, &mut Node)>,
) {
    for (column, computed, mut node) in &mut columns {
        if column.depth == 0 {
            continue;
        }
        let Ok((menu, window)) = menus.get(column.menu) else {
            continue;
        };
        let Some(Some(opener)) = menu.nav.levels.get(column.depth - 1) else {
            continue;
        };
        let Some((_, row, at)) = rows.iter().find(|(row, _, _)| {
            row.menu == column.menu && row.depth == column.depth - 1 && row.index == *opener
        }) else {
            continue;
        };
        let to_px = row.inverse_scale_factor();
        let half = row.size() * 0.5 * to_px;
        let centre = at.translation * to_px;
        let edge = COLUMN_PAD + BORDER_W;
        let size = computed.size() * computed.inverse_scale_factor();
        let room = window.size() * window.inverse_scale_factor();
        let mut left = centre.x + half.x + edge;
        if left + size.x > room.x {
            left = centre.x - half.x - edge - size.x;
        }
        let left = left.max(0.0);
        let top = (centre.y - half.y - edge).min(room.y - size.y).max(0.0);
        if node.left != px(left) {
            node.left = px(left);
        }
        if node.top != px(top) {
            node.top = px(top);
        }
    }
}

#[cfg(test)]
mod tests;
