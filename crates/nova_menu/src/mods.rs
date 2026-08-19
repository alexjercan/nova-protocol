//! The Mods screen's Installed tab: the panel, its two-pane list/details
//! surface, and the enable/disable toggles.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_assets::prelude::{
    DownloadedMods, EnabledMods, FetchPortalCatalog, InstallJobs, ModCatalog, ModInfo, ModMeta,
    PortalEntry, RemoteCatalog, RemoteCatalogState,
};
use nova_mod_format::BASE_MOD_ID;
use nova_ui::{
    prelude::UiSkin,
    theme,
    widget::{
        badge, checkbox, checkbox_colors, checkbox_glyph, list_row, separator, themed_button,
        BadgeKind, ListRow, Selected, UiText,
    },
};

use crate::{
    portal::{
        explore_entries, on_catalog_retry, portal_display_name, portal_status_tag,
        portal_version_author_line, spawn_explore_note, spawn_explore_row, spawn_portal_actions,
        spawn_portal_button, PortalActionKind, UpdateRequested,
    },
    widgets::MenuSfxButton,
};

/// Marker for the Mods panel root, toggled by the Mods button.
#[derive(Component)]
pub(crate) struct ModsPanel;

/// Which mods-screen tab is active. `Installed` lists the local catalog; `Explore` is
/// the portal browser (`portal.rs`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ModsTabKind {
    #[default]
    Installed,
    Explore,
}

/// A tab-bar button: the tab it activates. `on_mods_tab` reads this on click.
#[derive(Component)]
pub(crate) struct ModsTab(pub(crate) ModsTabKind);

/// The active tab; `refresh_mods_list` rebuilds the list content when it
/// changes. Reset to `Installed` on every menu entry by `setup_menu_ui`.
#[derive(Resource, Default, PartialEq, Eq)]
pub(crate) struct ModsActiveTab(pub(crate) ModsTabKind);

/// The mod id the details pane renders. `None` until the list populates -
/// `refresh_mods_list` default-selects the first row (and repairs a selection
/// that left the catalog); `on_mod_row_select` sets it from a row click.
#[derive(Resource, Default)]
pub(crate) struct SelectedModId(pub(crate) Option<String>);

/// The scrollable container holding the mod rows (a shared
/// `nova_ui::screen::ScrollViewport`); `refresh_mods_list` swaps its children on
/// tab or catalog change.
#[derive(Component)]
pub(crate) struct ModsList;

/// One clickable installed-mod row: clicking it (anywhere but the checkbox,
/// whose click does not propagate) selects the mod for the details pane.
#[derive(Component)]
pub(crate) struct ModRow {
    pub(crate) id: String,
}

/// The details side panel container; `refresh_mod_details` rebuilds its
/// children from the selected mod's bundle meta.
#[derive(Component)]
pub(crate) struct ModDetailsPanel;

/// The details pane's action area. Holds the Enable/Disable button (or the base lock
/// tag) today; the Explore tab spawns its Install/Uninstall/Update buttons into this
/// same container - keep the marker stable.
#[derive(Component)]
pub(crate) struct ModDetailsActions;

/// Marks a row's compact enable checkbox, so `update_mod_checkbox_labels`
/// renders only checkboxes ("x"/"") and never the details pane's
/// Enable/Disable button (whose label is baked by `refresh_mod_details`).
#[derive(Component)]
pub(crate) struct ModEnableCheckbox;

/// An enable/disable control: carries the catalog `id` it toggles and whether
/// it is the locked `base` entry. Shared by the row checkbox and the details
/// pane's Enable/Disable button; `on_mod_toggle` reads it on click.
#[derive(Component)]
pub(crate) struct ModToggle {
    pub(crate) id: String,
    pub(crate) base: bool,
}

/// The muted "v0.2.0 - by Author" line under a mod's name (row and details
/// pane); empty meta fields drop out, both empty yields an empty string (the
/// caller skips spawning it).
pub(crate) fn version_author_line(meta: &ModMeta) -> String {
    let mut line = String::new();
    if !meta.version.is_empty() {
        line.push('v');
        line.push_str(&meta.version);
    }
    if !meta.author.is_empty() {
        if !line.is_empty() {
            line.push_str(" - ");
        }
        line.push_str("by ");
        line.push_str(&meta.author);
    }
    line
}

/// Spawn one installed-mod row: a clickable ThemedButton row (click selects the
/// mod for the details pane) holding the name + muted version/author line and,
/// right-aligned, either the quiet enable checkbox or the muted "base" tag.
pub(crate) fn spawn_mod_row(
    list: &mut ChildSpawnerCommands,
    m: &ModInfo,
    enabled: bool,
    selected: bool,
    skin: UiSkin,
) {
    // The shared interactive `list_row`: `ListRow` + `Button` + `Hovered` so the
    // nova_ui reconciler highlights it on hover/selection (matching the zoo).
    let mut row = list.spawn((
        Name::new(format!("Mod Row: {}", m.id)),
        ModRow { id: m.id.clone() },
        list_row(selected, skin),
        ListRow,
        Button,
        Hovered::default(),
        observe(on_mod_row_select),
    ));
    if selected {
        row.insert(Selected);
    }
    row.with_children(|row| {
        row.spawn((
            Name::new("Mod Row Info"),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                ..default()
            },
        ))
        .with_children(|info| {
            info.spawn((
                Name::new("Mod Name"),
                UiText,
                Text::new(m.meta.name.clone()),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(theme::SCREEN_TEXT),
            ));
            let line = version_author_line(&m.meta);
            if !line.is_empty() {
                info.spawn((
                    Name::new("Mod Version Author"),
                    UiText,
                    Text::new(line),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_DIM),
                ));
            }
        });
        if m.base {
            // The base pack cannot be disabled - a muted badge, not a checkbox.
            row.spawn((
                Name::new("Mod Base Badge"),
                badge(BadgeKind::Mute, BASE_MOD_ID, skin),
            ));
        } else {
            // The shared `checkbox` widget; still a `Button` + `MenuSfxButton` so
            // it clicks + sounds. Its click does not propagate to the row
            // (ui_widgets Button stops it), so toggling never re-selects; the
            // mods-list refresh re-renders it for the new state.
            row.spawn((
                Name::new("Mod Enable Checkbox"),
                ModEnableCheckbox,
                ModToggle {
                    id: m.id.clone(),
                    base: m.base,
                },
                checkbox(enabled, skin),
                MenuSfxButton,
                Button,
                Hovered::default(),
                observe(on_mod_toggle),
            ));
        }
    });
}

pub(crate) fn on_mods(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<ModsPanel>>,
) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

pub(crate) fn on_mods_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<ModsPanel>>,
) {
    **panel = Visibility::Hidden;
}

/// Switch the active mods tab: write [`ModsActiveTab`] (which re-arms
/// `refresh_mods_list`), move the `Selected` highlight to the clicked tab,
/// and - opening Explore - kick the catalog fetch when nothing was ever
/// fetched (`Idle`). Ready/Fetching are left alone; Error renders its own
/// Retry affordance in the list.
pub(crate) fn on_mods_tab(
    activate: On<Activate>,
    tabs: Query<(Entity, &ModsTab)>,
    mut active: ResMut<ModsActiveTab>,
    remote: Option<Res<RemoteCatalog>>,
    mut commands: Commands,
) {
    let Ok((entity, tab)) = tabs.get(activate.entity) else {
        return;
    };
    if active.0 == tab.0 {
        return;
    }
    active.0 = tab.0;
    if tab.0 == ModsTabKind::Explore
        && remote.is_some_and(|r| matches!(r.state, RemoteCatalogState::Idle))
    {
        commands.trigger(FetchPortalCatalog);
    }
    for (other, _) in &tabs {
        commands.entity(other).remove::<Selected>();
    }
    commands.entity(entity).insert(Selected);
}

/// Select the clicked row's mod: write [`SelectedModId`] (which re-arms
/// `refresh_mod_details`) and move the row `Selected` highlight. The row
/// checkbox never reaches this - the ui_widgets Button stops the click's
/// propagation at the checkbox.
pub(crate) fn on_mod_row_select(
    activate: On<Activate>,
    rows: Query<(Entity, &ModRow)>,
    selected_rows: Query<Entity, (With<ModRow>, With<Selected>)>,
    mut selected: ResMut<SelectedModId>,
    mut commands: Commands,
) {
    let Ok((entity, row)) = rows.get(activate.entity) else {
        return;
    };
    if selected.0.as_deref() == Some(row.id.as_str()) {
        return;
    }
    for previous in &selected_rows {
        commands.entity(previous).remove::<Selected>();
    }
    commands.entity(entity).insert(Selected);
    selected.0 = Some(row.id.clone());
}

/// `refresh_mods_list` runs when the active tab or the catalog changed (the
/// catalog changes live: a downloaded bundle's async load upgrades its row),
/// when the remote catalog transitioned (Explore's fetch states), or when the
/// downloaded set changed (the Explore rows' installed/update status tags).
pub(crate) fn mods_list_dirty(
    active: Res<ModsActiveTab>,
    skin: Res<UiSkin>,
    catalog: Option<Res<ModCatalog>>,
    remote: Option<Res<RemoteCatalog>>,
    downloaded: Option<Res<DownloadedMods>>,
) -> bool {
    // `skin` too: a UiSkin flip rebuilds the rows so their shared widgets
    // (list_row, checkbox, badge) re-spawn for the new skin.
    active.is_changed()
        || skin.is_changed()
        || catalog.is_some_and(|c| c.is_changed())
        || remote.is_some_and(|r| r.is_changed())
        || downloaded.is_some_and(|d| d.is_changed())
}

/// `refresh_mod_details` runs when the tab, the selection, the catalogs
/// (installed meta upgrade / remote transition), the enabled set
/// (Enable/Disable label), the job table (progress/Failed/Dismiss), the
/// downloaded set (Install vs Uninstall/Update actions) or the update
/// requests ("Updating..." rendering) changed.
pub(crate) fn mod_details_dirty(
    active: Res<ModsActiveTab>,
    selected: Res<SelectedModId>,
    catalog: Option<Res<ModCatalog>>,
    enabled: Option<Res<EnabledMods>>,
    remote: Option<Res<RemoteCatalog>>,
    jobs: Option<Res<InstallJobs>>,
    downloaded: Option<Res<DownloadedMods>>,
    updates: Res<UpdateRequested>,
) -> bool {
    active.is_changed()
        || selected.is_changed()
        || catalog.is_some_and(|c| c.is_changed())
        || enabled.is_some_and(|e| e.is_changed())
        || remote.is_some_and(|r| r.is_changed())
        || jobs.is_some_and(|j| j.is_changed())
        || downloaded.is_some_and(|d| d.is_changed())
        || updates.is_changed()
}

/// Rebuild the left list's rows for the active tab. Installed: one row per
/// catalog entry, default-selecting the first row when nothing (still) valid
/// is selected - written BEFORE the chained details refresh, so the pane
/// renders it the same frame. Explore: the portal catalog's fetch states -
/// Fetching note, Error row + Retry (over the stale last-good entries when
/// one survives), Ready rows with install-state tags; selection is repaired
/// against the VISIBLE remote entries exactly like the Installed branch.
pub(crate) fn refresh_mods_list(
    mut commands: Commands,
    skin: Res<UiSkin>,
    active: Res<ModsActiveTab>,
    catalog: Option<Res<ModCatalog>>,
    enabled: Option<Res<EnabledMods>>,
    remote: Option<Res<RemoteCatalog>>,
    downloaded: Option<Res<DownloadedMods>>,
    mut selected: ResMut<SelectedModId>,
    lists: Query<Entity, With<ModsList>>,
) {
    let Ok(list) = lists.single() else {
        return;
    };
    commands.entity(list).despawn_related::<Children>();
    match active.0 {
        ModsTabKind::Installed => {
            let mods: Vec<ModInfo> = catalog.map(|c| c.0.clone()).unwrap_or_default();
            if !mods
                .iter()
                .any(|m| selected.0.as_deref() == Some(m.id.as_str()))
            {
                let first = mods.first().map(|m| m.id.clone());
                if selected.0 != first {
                    selected.0 = first;
                }
            }
            let is_enabled = |id: &str| enabled.as_ref().is_some_and(|e| e.0.contains(id));
            commands.entity(list).with_children(|list| {
                for m in &mods {
                    let is_selected = selected.0.as_deref() == Some(m.id.as_str());
                    spawn_mod_row(list, m, is_enabled(&m.id), is_selected, *skin);
                }
            });
        }
        ModsTabKind::Explore => {
            let entries: &[PortalEntry] = remote
                .as_ref()
                .and_then(|r| explore_entries(r))
                .unwrap_or_default();
            // Selection repair against the visible REMOTE entries (the
            // Installed-branch discipline): no live installed-mod action can
            // survive next to Explore content (review 142911 R1.2), and the
            // details pane keys the id into the remote catalog.
            if !entries
                .iter()
                .any(|e| selected.0.as_deref() == Some(e.id.as_str()))
            {
                let first = entries.first().map(|e| e.id.clone());
                if selected.0 != first {
                    selected.0 = first;
                }
            }
            commands.entity(list).with_children(|list| {
                match remote.as_ref().map(|r| &r.state) {
                    // A rig/slim app without the portal plugin never leaves
                    // Idle; in production Idle only renders for the frame the
                    // tab-open fetch trigger is still in flight.
                    None | Some(RemoteCatalogState::Idle | RemoteCatalogState::Fetching) => {
                        spawn_explore_note(
                            list,
                            "Portal Fetching Note",
                            "Fetching the mod portal catalog...",
                        );
                    }
                    Some(RemoteCatalogState::Error(error)) => {
                        list.spawn((
                            Name::new("Portal Error Row"),
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::Stretch,
                                row_gap: px(8),
                                padding: UiRect::all(px(8)),
                                margin: UiRect::bottom(px(4)),
                                border: UiRect::all(px(theme::BORDER_W)),
                                border_radius: BorderRadius::all(px(theme::RADIUS)),
                                ..default()
                            },
                            BorderColor::all(theme::PHOSPHOR_MUTED),
                            BackgroundColor(theme::SPACE),
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Name::new("Portal Error Text"),
                                Text::new(error.clone()),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(theme::AMBER_NOVA),
                            ));
                            row.spawn((
                                Name::new("Portal Retry Slot"),
                                Node {
                                    width: px(140),
                                    ..default()
                                },
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    Name::new("Portal Retry Button"),
                                    themed_button("Retry"),
                                    observe(on_catalog_retry),
                                ));
                            });
                        });
                        if !entries.is_empty() {
                            spawn_explore_note(
                                list,
                                "Portal Offline Note",
                                "offline - showing the last fetched catalog",
                            );
                        }
                    }
                    Some(RemoteCatalogState::Ready(_)) => {}
                }
                for entry in entries {
                    let is_selected = selected.0.as_deref() == Some(entry.id.as_str());
                    let tag = portal_status_tag(entry, downloaded.as_deref());
                    spawn_explore_row(list, entry, tag, is_selected);
                }
            });
        }
    }
}

/// The details pane's empty fallback: the hint text plus the (empty) action
/// container, so [`ModDetailsActions`] exists in every state.
pub(crate) fn spawn_details_empty(details: &mut ChildSpawnerCommands) {
    details.spawn((
        Name::new("Mod Details Empty"),
        Text::new("Select a mod to see its details."),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR_MUTED),
    ));
    details.spawn((
        Name::new("Mod Details Actions"),
        ModDetailsActions,
        Node::default(),
    ));
}

/// A declared dependency's status for the details panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DepStatus {
    /// Installed and enabled (will merge).
    Enabled,
    /// Installed but not enabled (enabling this mod will auto-enable it).
    InstalledDisabled,
    /// Not installed - Install from Explore pulls it, or it must be added.
    Missing,
}

/// Resolve a dependency id's status against the installed catalog + enabled set.
/// An enabled id counts as enabled even if hidden (not in `ModCatalog`).
pub(crate) fn dep_status(
    id: &str,
    catalog: Option<&ModCatalog>,
    enabled: Option<&EnabledMods>,
) -> DepStatus {
    if enabled.is_some_and(|e| e.0.contains(id)) {
        DepStatus::Enabled
    } else if catalog.is_some_and(|c| c.0.iter().any(|m| m.id == id)) {
        DepStatus::InstalledDisabled
    } else {
        DepStatus::Missing
    }
}

pub(crate) fn spawn_details_meta(
    details: &mut ChildSpawnerCommands,
    name: &str,
    line: &str,
    meta: &ModMeta,
    catalog: Option<&ModCatalog>,
    enabled: Option<&EnabledMods>,
) {
    details.spawn((
        Name::new("Mod Details Name"),
        Text::new(name.to_string()),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(theme::SCREEN_TEXT),
    ));
    if !line.is_empty() {
        details.spawn((
            Name::new("Mod Details Version Author"),
            Text::new(line.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
        ));
    }
    details.spawn((Name::new("Mod Details Separator"), separator()));
    if !meta.description.is_empty() {
        details.spawn((
            Name::new("Mod Details Description"),
            Text::new(meta.description.clone()),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
            Node {
                margin: UiRect::bottom(px(8)),
                ..default()
            },
        ));
    }
    details.spawn((
        Name::new("Mod Details Dependencies"),
        Text::new("Dependencies:"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR_MUTED),
    ));
    if meta.dependencies.is_empty() {
        details.spawn((
            Name::new("Mod Details Dependency: none"),
            Text::new("  none"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
        ));
    } else {
        // One line per dep, coloured by whether it is enabled / installed / missing
        // so the player sees what enabling this mod will pull in.
        for dep in &meta.dependencies {
            let (suffix, color) = match dep_status(dep, catalog, enabled) {
                DepStatus::Enabled => ("enabled", theme::PHOSPHOR),
                DepStatus::InstalledDisabled => ("installed, disabled", theme::PHOSPHOR_MUTED),
                DepStatus::Missing => ("missing", theme::AMBER_NOVA),
            };
            details.spawn((
                Name::new(format!("Mod Details Dependency: {dep}")),
                Text::new(format!("  {dep} - {suffix}")),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(color),
            ));
        }
    }
}

/// Rebuild the details pane for the selected mod: name header, version/author
/// line, description, dependencies, then the action area
/// ([`ModDetailsActions`]). Installed tab: the Enable/Disable button (base: a
/// locked tag), plus Uninstall for DOWNLOADED mods (managing installs must
/// not require the Explore tab). Explore tab: the selection keys into the
/// visible remote entries and the action area follows the install state
/// ([`spawn_portal_actions`]). The action container is spawned even with
/// nothing selected, so the marker contract holds in every state.
pub(crate) fn refresh_mod_details(
    mut commands: Commands,
    active: Res<ModsActiveTab>,
    selected: Res<SelectedModId>,
    catalog: Option<Res<ModCatalog>>,
    enabled: Option<Res<EnabledMods>>,
    remote: Option<Res<RemoteCatalog>>,
    jobs: Option<Res<InstallJobs>>,
    downloaded: Option<Res<DownloadedMods>>,
    updates: Res<UpdateRequested>,
    panels: Query<Entity, With<ModDetailsPanel>>,
) {
    let Ok(panel) = panels.single() else {
        return;
    };
    commands.entity(panel).despawn_related::<Children>();
    let installed_version_of = |id: &str| -> Option<String> {
        downloaded
            .as_ref()
            .and_then(|d| d.0.iter().find(|m| m.record.id == id))
            .map(|m| m.record.version.clone())
    };
    match active.0 {
        ModsTabKind::Installed => {
            let info: Option<ModInfo> = selected.0.as_ref().and_then(|id| {
                catalog
                    .as_ref()
                    .and_then(|c| c.0.iter().find(|m| &m.id == id))
                    .cloned()
            });
            let is_enabled = info
                .as_ref()
                .is_some_and(|m| enabled.as_ref().is_some_and(|e| e.0.contains(&m.id)));
            commands.entity(panel).with_children(|details| {
                let Some(m) = info else {
                    spawn_details_empty(details);
                    return;
                };
                let is_downloaded = installed_version_of(&m.id).is_some();
                spawn_details_meta(
                    details,
                    &m.meta.name,
                    &version_author_line(&m.meta),
                    &m.meta,
                    catalog.as_deref(),
                    enabled.as_deref(),
                );
                details
                    .spawn((
                        Name::new("Mod Details Actions"),
                        ModDetailsActions,
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(8),
                            margin: UiRect::top(px(12)),
                            ..default()
                        },
                    ))
                    .with_children(|actions| {
                        if m.base {
                            actions.spawn((
                                Name::new("Mod Details Locked"),
                                Text::new("Enabled (base)"),
                                TextFont {
                                    font_size: FontSize::Px(14.0),
                                    ..default()
                                },
                                TextColor(theme::PHOSPHOR),
                            ));
                        } else {
                            // Fixed-width slot: the percent-width themed
                            // button must not span the whole details pane.
                            actions
                                .spawn((
                                    Name::new("Mod Details Toggle Slot"),
                                    Node {
                                        width: px(180),
                                        ..default()
                                    },
                                ))
                                .with_children(|slot| {
                                    slot.spawn((
                                        Name::new("Mod Details Toggle Button"),
                                        themed_button(if is_enabled {
                                            "Disable"
                                        } else {
                                            "Enable"
                                        }),
                                        ModToggle {
                                            id: m.id.clone(),
                                            base: m.base,
                                        },
                                        observe(on_mod_toggle),
                                    ));
                                });
                            // Installed-tab parity: a DOWNLOADED mod is
                            // uninstallable from here too.
                            if is_downloaded {
                                spawn_portal_button(
                                    actions,
                                    "Mod Details Uninstall Button",
                                    "Uninstall",
                                    &m.id,
                                    PortalActionKind::Uninstall,
                                );
                            }
                        }
                    });
            });
        }
        ModsTabKind::Explore => {
            let entry: Option<PortalEntry> = selected.0.as_ref().and_then(|id| {
                remote
                    .as_ref()
                    .and_then(|r| explore_entries(r))
                    .and_then(|entries| entries.iter().find(|e| &e.id == id))
                    .cloned()
            });
            commands.entity(panel).with_children(|details| {
                let Some(entry) = entry else {
                    spawn_details_empty(details);
                    return;
                };
                let job = jobs.as_ref().and_then(|j| j.0.get(&entry.id)).cloned();
                let installed_version = installed_version_of(&entry.id);
                let updating = updates.0.contains_key(&entry.id);
                // False when the entry renders from the stale last-good
                // fallback: Install/Update are withheld there (R1.1).
                let catalog_ready = remote
                    .as_ref()
                    .is_some_and(|r| matches!(r.state, RemoteCatalogState::Ready(_)));
                spawn_details_meta(
                    details,
                    &portal_display_name(&entry),
                    &portal_version_author_line(&entry),
                    &entry.meta,
                    catalog.as_deref(),
                    enabled.as_deref(),
                );
                details
                    .spawn((
                        Name::new("Mod Details Actions"),
                        ModDetailsActions,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(8),
                            margin: UiRect::top(px(12)),
                            ..default()
                        },
                    ))
                    .with_children(|actions| {
                        spawn_portal_actions(
                            actions,
                            &entry.id,
                            &entry.version,
                            job,
                            installed_version,
                            updating,
                            catalog_ready,
                        );
                    });
            });
        }
    }
}

/// The mod dependency graph (id -> declared dependency ids) from the catalog. Every
/// installed mod is a key (deps possibly empty), so `contains_key(id)` doubles as "is
/// this id installed". `base` is implicit and never a declared dependency.
pub(crate) fn mod_dep_graph(catalog: &ModCatalog) -> nova_mod_format::deps::DepGraph {
    catalog
        .0
        .iter()
        .map(|m| (m.id.clone(), m.meta.dependencies.clone()))
        .collect()
}

/// Toggle a mod's enabled state on click, resolving dependencies:
/// - ENABLING a mod also enables its transitive dependencies (Factorio).
/// - DISABLING a mod that enabled mods still depend on is REFUSED with a warning
///   naming them (block + warn); the player disables those dependents first.
///
/// Reads the clicked button's [`ModToggle`] and flips its id in [`EnabledMods`], which
/// nova_assets' `resource_changed` re-merge then applies live. The `base` mod is locked
/// on (its row has no toggle button, but guard here too).
///
/// `base` is implicit (locked on, seeded) so it is never toggled here and never auto-
/// enabled. A declared dependency that is not installed is warned about but does not
/// block enabling the mod (it simply will not merge).
pub(crate) fn on_mod_toggle(
    activate: On<Activate>,
    toggles: Query<&ModToggle>,
    catalog: Option<Res<ModCatalog>>,
    mut enabled: ResMut<EnabledMods>,
) {
    let Ok(toggle) = toggles.get(activate.entity) else {
        return;
    };
    if toggle.base {
        return;
    }
    let graph = catalog
        .as_ref()
        .map(|c| mod_dep_graph(c))
        .unwrap_or_default();

    if enabled.0.contains(&toggle.id) {
        // Disable: block if any ENABLED mod still declares this one as a
        // dependency (Factorio - never strand an enabled mod without its dep).
        let blockers = nova_mod_format::deps::dependents(
            &toggle.id,
            enabled.0.iter().map(String::as_str),
            &graph,
        );
        if !blockers.is_empty() {
            warn!(
                "cannot disable mod '{}': still required by enabled mod(s) {}; disable those first",
                toggle.id,
                blockers.join(", ")
            );
            return;
        }
        enabled.0.remove(&toggle.id);
    } else {
        // Enable: this mod plus all of its (transitive) dependencies.
        enabled.0.insert(toggle.id.clone());
        let deps = match nova_mod_format::deps::transitive_deps(&graph, &toggle.id) {
            Ok(deps) => deps,
            Err(e) => {
                // Refusing the enable is the safe answer: the dependency set is
                // unknown, so enabling would load an unresolvable mod.
                warn!("cannot enable mod '{}': {e}", toggle.id);
                enabled.0.remove(&toggle.id);
                return;
            }
        };
        for dep in deps {
            if dep == BASE_MOD_ID {
                continue; // base is implicit, always on
            }
            if !graph.contains_key(&dep) {
                warn!(
                    "mod '{}' depends on '{dep}', which is not installed; enabling anyway - \
                     the mod may not work until '{dep}' is installed",
                    toggle.id
                );
                continue;
            }
            enabled.0.insert(dep);
        }
    }
}

/// Keep each row's enable checkbox in sync with [`EnabledMods`] (after a click,
/// or a future persisted set) IN PLACE - repaint the shared `checkbox` widget's
/// fill/border/glyph for the new state without rebuilding the row (rows only
/// rebuild on tab/catalog change). Uses nova_ui's `checkbox_colors`/
/// `checkbox_glyph` so it stays identical to the `checkbox()` factory.
pub(crate) fn sync_mod_checkboxes(
    enabled: Option<Res<EnabledMods>>,
    skin: Res<UiSkin>,
    mut checkboxes: Query<
        (
            &ModToggle,
            &Children,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ModEnableCheckbox>,
    >,
    mut glyphs: Query<(&mut Text, &mut TextColor)>,
) {
    let Some(enabled) = enabled else {
        return;
    };
    for (toggle, children, mut bg, mut border) in &mut checkboxes {
        let on = enabled.0.contains(&toggle.id);
        let (fill, edge, glyph_color) = checkbox_colors(on, *skin);
        *bg = fill.into();
        border.set_all(edge);
        for &child in children {
            if let Ok((mut text, mut color)) = glyphs.get_mut(child) {
                let mark = checkbox_glyph(on);
                if text.0 != mark {
                    text.0 = mark.to_string();
                }
                if color.0 != glyph_color {
                    color.0 = glyph_color;
                }
            }
        }
    }
}
