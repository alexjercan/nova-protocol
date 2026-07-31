//! The Mods screen's Explore tab: the remote portal catalog, its rows, and the
//! install / uninstall / update action buttons behind them.

use std::{collections::HashMap, time::Duration};

use bevy::{
    picking::hover::Hovered,
    platform::time::Instant,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_assets::prelude::{
    DownloadedMods, EnabledMods, FetchPortalCatalog, InstallJobs, InstallPortalMod, InstallStatus,
    PendingRemovals, PortalEntry, RemoteCatalog, RemoteCatalogState, UninstallPortalMod,
};
use nova_ui::{
    theme,
    widget::{themed_button, Selected, ThemedButton},
};

use crate::mods::{on_mod_row_select, version_author_line, ModRow};

/// A details-pane portal action button: which command to fire for which mod
/// id. One observer (`on_portal_action`) serves every button.
#[derive(Component)]
pub(crate) struct PortalAction {
    pub(crate) id: String,
    pub(crate) kind: PortalActionKind,
}

/// The portal commands the details pane's action buttons carry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PortalActionKind {
    /// Trigger [`InstallPortalMod`] (also the Failed state's Retry).
    Install,
    /// Trigger [`UninstallPortalMod`].
    Uninstall,
    /// Record the id in [`UpdateRequested`] and trigger the uninstall; the
    /// choreography system fires the install once both guards clear.
    Update,
    /// Clear the id's [`InstallJobs`] entry - the recovery affordance for a
    /// Failed job (including one the portal client's fetch-stall timeout
    /// failed, the 163508 R1.3 pair).
    Dismiss,
}

/// One in-flight update request (see [`UpdateRequested`]).
pub(crate) struct UpdateRequest {
    /// When the current stage started (wall clock); the timeout compares
    /// against this, and the stage transition resets it so each stage gets
    /// its own window.
    pub(crate) since: Instant,
    /// The mod was ENABLED when the update started; re-enable it once the
    /// new version's record lands (review 142916 R1.4 - the uninstall strips
    /// EnabledMods per 142906 R1.7, and a fresh install commits disabled, so
    /// an update would otherwise silently disable a mod the player had on).
    pub(crate) re_enable: bool,
    /// The install half was fired; the request now only waits for the new
    /// record to land in [`DownloadedMods`] to restore the enabled bit.
    pub(crate) install_fired: bool,
}

/// UPDATE = uninstall-then-install choreography: the ids whose Update button
/// was clicked. [`drive_update_choreography`] fires [`InstallPortalMod`] only
/// once the id has left BOTH [`DownloadedMods`] and [`PendingRemovals`] (the
/// 163508 race guard: a wasm uninstall's file removal is async, and an
/// install admitted while it runs could have its fresh writes deleted under
/// it), then holds the request until the new record lands to restore the
/// enabled bit; a stage older than [`UPDATE_TIMEOUT`] drops the request with
/// a warn - a wedged uninstall (or a failed install) must not hold a phantom
/// request forever (review 163508 R1.3).
#[derive(Resource, Default)]
pub(crate) struct UpdateRequested(pub(crate) HashMap<String, UpdateRequest>);

/// Wall-clock lifetime of each update-request stage. Generous: a native
/// uninstall settles same-frame and a wasm removal within an IndexedDB
/// transaction; anything still pending after this is wedged (or, stage two,
/// the install failed - its Failed job surface explains), not slow.
pub(crate) const UPDATE_TIMEOUT: Duration = Duration::from_secs(30);

/// Drive the update requests: fire the deferred install once its uninstall
/// fully settled (id out of [`DownloadedMods`] AND [`PendingRemovals`]), then
/// re-enable the mod once the NEW record lands (when it was enabled at
/// request time); expire stages that outlive [`UPDATE_TIMEOUT`]. The portal
/// resources are optional so minimal rigs (and slim apps) without the portal
/// plugin stay valid - a missing set reads as its guard already cleared.
pub(crate) fn drive_update_choreography(
    mut updates: ResMut<UpdateRequested>,
    downloaded: Option<Res<DownloadedMods>>,
    pending: Option<Res<PendingRemovals>>,
    mut enabled: Option<ResMut<EnabledMods>>,
    mut commands: Commands,
) {
    if updates.0.is_empty() {
        return;
    }
    let now = Instant::now();
    let mut expired: Vec<String> = Vec::new();
    let mut fire: Vec<String> = Vec::new();
    let mut landed: Vec<String> = Vec::new();
    for (id, request) in updates.0.iter() {
        let is_downloaded = downloaded
            .as_ref()
            .is_some_and(|d| d.0.iter().any(|m| m.record.id == *id));
        let removal_pending = pending.as_ref().is_some_and(|p| p.0.contains(id));
        if now.saturating_duration_since(request.since) > UPDATE_TIMEOUT {
            expired.push(id.clone());
        } else if !request.install_fired && !is_downloaded && !removal_pending {
            fire.push(id.clone());
        } else if request.install_fired && is_downloaded {
            landed.push(id.clone());
        }
    }
    for id in expired {
        warn!("mods: the update of '{id}' timed out; dropping the request");
        updates.0.remove(&id);
    }
    for id in fire {
        if let Some(request) = updates.0.get_mut(&id) {
            request.install_fired = true;
            request.since = now;
        }
        commands.trigger(InstallPortalMod { id });
    }
    for id in landed {
        let Some(request) = updates.0.remove(&id) else {
            continue;
        };
        // The new record is in: restore the enabled bit the uninstall
        // stripped, if the player had the mod on (the existing change-gated
        // save system persists the insert). A disabled mod stays disabled.
        if request.re_enable {
            if let Some(enabled) = enabled.as_mut() {
                enabled.0.insert(id);
            }
        }
    }
}

/// Fire the clicked action button's portal command (see [`PortalActionKind`]).
///
/// Install and Update require a READY remote catalog (review 142916 R1.1,
/// defense in depth with the UI gate in [`spawn_portal_actions`]): entries
/// rendered from the stale last-good fallback must not start an install (it
/// can only fail - the portal observer requires Ready) and an offline Update
/// must not uninstall a working mod it cannot replace.
pub(crate) fn on_portal_action(
    activate: On<Activate>,
    buttons: Query<&PortalAction>,
    remote: Option<Res<RemoteCatalog>>,
    enabled: Option<Res<EnabledMods>>,
    mut jobs: Option<ResMut<InstallJobs>>,
    mut updates: ResMut<UpdateRequested>,
    mut commands: Commands,
) {
    let Ok(action) = buttons.get(activate.entity) else {
        return;
    };
    let catalog_ready = remote
        .as_ref()
        .is_some_and(|r| matches!(r.state, RemoteCatalogState::Ready(_)));
    match action.kind {
        PortalActionKind::Install => {
            if !catalog_ready {
                warn!(
                    "mods: refusing to install '{}' - the portal catalog is not ready",
                    action.id
                );
                return;
            }
            commands.trigger(InstallPortalMod {
                id: action.id.clone(),
            });
        }
        PortalActionKind::Uninstall => commands.trigger(UninstallPortalMod {
            id: action.id.clone(),
        }),
        PortalActionKind::Update => {
            if !catalog_ready {
                warn!(
                    "mods: refusing to update '{}' - the portal catalog is not ready",
                    action.id
                );
                return;
            }
            // The enabled bit is read BEFORE the uninstall strips it.
            let re_enable = enabled.as_ref().is_some_and(|e| e.0.contains(&action.id));
            updates.0.insert(
                action.id.clone(),
                UpdateRequest {
                    since: Instant::now(),
                    re_enable,
                    install_fired: false,
                },
            );
            commands.trigger(UninstallPortalMod {
                id: action.id.clone(),
            });
        }
        PortalActionKind::Dismiss => {
            if let Some(jobs) = jobs.as_mut() {
                jobs.0.remove(&action.id);
            }
        }
    }
}

/// The catalog Error state's Retry: force-reset the state to Idle FIRST - the
/// fetch observer refuses re-triggers while `Fetching`, so a wedged fetch
/// (transport callback never fired, review 163508 R1.3) would otherwise
/// refuse recovery forever - then re-trigger the fetch.
pub(crate) fn on_catalog_retry(
    _activate: On<Activate>,
    remote: Option<ResMut<RemoteCatalog>>,
    mut commands: Commands,
) {
    let Some(mut remote) = remote else {
        return;
    };
    remote.state = RemoteCatalogState::Idle;
    commands.trigger(FetchPortalCatalog);
}

/// The Explore tab's VISIBLE entries: Ready's own, or - when the fetch failed
/// but a last-good catalog survives - the stale fallback's (rendered under
/// the offline note). Idle/Fetching (and an Error with no fallback) show
/// none.
pub(crate) fn explore_entries(remote: &RemoteCatalog) -> Option<&[PortalEntry]> {
    match &remote.state {
        RemoteCatalogState::Ready(catalog) => Some(&catalog.entries),
        RemoteCatalogState::Error(_) => remote.last_good.as_ref().map(|c| c.entries.as_slice()),
        RemoteCatalogState::Idle | RemoteCatalogState::Fetching => None,
    }
}

/// An Explore row's right-aligned install-state tag: "installed" when the id
/// is downloaded at the catalog's exact version string, "update" when it is
/// downloaded at a DIFFERENT one (v1: exact string compare; semver ordering
/// is deferred per the spike), none otherwise.
pub(crate) fn portal_status_tag(
    entry: &PortalEntry,
    downloaded: Option<&DownloadedMods>,
) -> Option<&'static str> {
    let record = downloaded?.0.iter().find(|m| m.record.id == entry.id)?;
    if record.record.version == entry.version {
        Some("installed")
    } else {
        Some("update")
    }
}

/// The muted "v1.0.0 - by Author" line for a portal entry: the CATALOG's
/// top-level version (the authoritative one the update compare uses), the
/// meta's author.
pub(crate) fn portal_version_author_line(entry: &PortalEntry) -> String {
    let mut meta = entry.meta.clone();
    meta.version = entry.version.clone();
    version_author_line(&meta)
}

/// A portal entry's display name; wire meta may be empty, fall back to the id
/// (the `ModInfo::new` normalization).
pub(crate) fn portal_display_name(entry: &PortalEntry) -> String {
    if entry.meta.name.is_empty() {
        entry.id.clone()
    } else {
        entry.meta.name.clone()
    }
}

/// One muted informational row in the Explore list (fetching/offline notes).
pub(crate) fn spawn_explore_note(list: &mut ChildSpawnerCommands, name: &'static str, text: &str) {
    list.spawn((
        Name::new(name),
        Node {
            align_self: AlignSelf::Stretch,
            min_height: px(40),
            margin: UiRect::bottom(px(4)),
            padding: UiRect::all(px(8)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR_MUTED),
        BackgroundColor(theme::SPACE),
        children![(
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
        )],
    ));
}

/// Spawn one portal-entry row: a clickable ThemedButton row (selection reuse -
/// same `ModRow`/observer as the Installed rows) holding the name + muted
/// version/author line and, right-aligned where the Installed rows put their
/// checkbox, the install-state tag ("installed" muted / "update" amber).
pub(crate) fn spawn_explore_row(
    list: &mut ChildSpawnerCommands,
    entry: &PortalEntry,
    tag: Option<&'static str>,
    selected: bool,
) {
    let mut row = list.spawn((
        Name::new(format!("Portal Row: {}", entry.id)),
        ModRow {
            id: entry.id.clone(),
        },
        Node {
            flex_direction: FlexDirection::Row,
            align_self: AlignSelf::Stretch,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(8),
            padding: UiRect::all(px(8)),
            margin: UiRect::bottom(px(4)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        ThemedButton,
        Button,
        Hovered::default(),
        BorderColor::all(theme::PHOSPHOR_MUTED),
        BackgroundColor(theme::SCREEN_0),
        observe(on_mod_row_select),
    ));
    if selected {
        row.insert(Selected);
    }
    row.with_children(|row| {
        row.spawn((
            Name::new("Portal Row Info"),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                ..default()
            },
        ))
        .with_children(|info| {
            info.spawn((
                Name::new("Mod Name"),
                Text::new(portal_display_name(entry)),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(theme::SCREEN_TEXT),
            ));
            let line = portal_version_author_line(entry);
            if !line.is_empty() {
                info.spawn((
                    Name::new("Mod Version Author"),
                    Text::new(line),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ));
            }
        });
        if let Some(tag) = tag {
            row.spawn((
                Name::new("Portal Status Tag"),
                Text::new(tag),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(if tag == "update" {
                    theme::AMBER_NOVA
                } else {
                    theme::PHOSPHOR_MUTED
                }),
            ));
        }
    });
}

/// One fixed-width portal action button (the percent-width themed button must
/// not span the details pane), wired to [`on_portal_action`].
pub(crate) fn spawn_portal_button(
    row: &mut ChildSpawnerCommands,
    name: &'static str,
    label: &str,
    id: &str,
    kind: PortalActionKind,
) {
    row.spawn((
        Name::new(format!("{name} Slot")),
        Node {
            width: px(140),
            ..default()
        },
    ))
    .with_children(|slot| {
        slot.spawn((
            Name::new(name),
            themed_button(label),
            PortalAction {
                id: id.to_string(),
                kind,
            },
            observe(on_portal_action),
        ));
    });
}

/// A progress/status text line in the details action area.
pub(crate) fn spawn_action_text(
    actions: &mut ChildSpawnerCommands,
    name: &'static str,
    text: String,
    color: Color,
) {
    actions.spawn((
        Name::new(name),
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(color),
    ));
}

/// The Explore details' action area, by install state (the R1.3 recovery
/// surface lives here): a live job renders progress text and NO buttons; a
/// Failed job renders the error + Retry (re-trigger) + Dismiss (clear the
/// job entry); a pending update renders "Updating..."; otherwise Install, or
/// Uninstall (+ Update when the installed version string differs).
///
/// `catalog_ready` is false when the entry renders from the stale last-good
/// fallback (review 142916 R1.1): Install/Update need the READY catalog to
/// succeed, and an offline Update would uninstall a working mod it cannot
/// replace - so only Uninstall renders, under a muted offline note.
pub(crate) fn spawn_portal_actions(
    actions: &mut ChildSpawnerCommands,
    id: &str,
    remote_version: &str,
    job: Option<InstallStatus>,
    installed_version: Option<String>,
    updating: bool,
    catalog_ready: bool,
) {
    match job {
        Some(InstallStatus::Failed(reason)) => {
            spawn_action_text(actions, "Mod Details Job Error", reason, theme::AMBER_NOVA);
            actions
                .spawn((
                    Name::new("Mod Details Action Buttons"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(8),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    spawn_portal_button(
                        row,
                        "Mod Details Retry Button",
                        "Retry",
                        id,
                        PortalActionKind::Install,
                    );
                    spawn_portal_button(
                        row,
                        "Mod Details Dismiss Button",
                        "Dismiss",
                        id,
                        PortalActionKind::Dismiss,
                    );
                });
        }
        Some(status) => {
            let text = match status {
                InstallStatus::Fetching { done, total } => {
                    format!("Downloading {}/{}...", done + 1, total)
                }
                InstallStatus::Verifying => "Verifying...".to_string(),
                InstallStatus::Committing => "Committing...".to_string(),
                // Handled by the arm above; keep the match total.
                InstallStatus::Failed(reason) => reason,
            };
            spawn_action_text(actions, "Mod Details Progress", text, theme::PHOSPHOR);
        }
        None if updating => {
            spawn_action_text(
                actions,
                "Mod Details Progress",
                "Updating...".to_string(),
                theme::PHOSPHOR,
            );
        }
        None => {
            // An Install/Update that the stale fallback cannot honor is
            // replaced by the offline note (set when a button was withheld).
            let mut offline_note = false;
            if installed_version.is_some() || catalog_ready {
                actions
                    .spawn((
                        Name::new("Mod Details Action Buttons"),
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(8),
                            ..default()
                        },
                    ))
                    .with_children(|row| match installed_version {
                        Some(installed) => {
                            spawn_portal_button(
                                row,
                                "Mod Details Uninstall Button",
                                "Uninstall",
                                id,
                                PortalActionKind::Uninstall,
                            );
                            if installed != remote_version {
                                if catalog_ready {
                                    spawn_portal_button(
                                        row,
                                        "Mod Details Update Button",
                                        "Update",
                                        id,
                                        PortalActionKind::Update,
                                    );
                                } else {
                                    offline_note = true;
                                }
                            }
                        }
                        None => {
                            // catalog_ready holds here (the guard above).
                            spawn_portal_button(
                                row,
                                "Mod Details Install Button",
                                "Install",
                                id,
                                PortalActionKind::Install,
                            );
                        }
                    });
            } else {
                offline_note = true;
            }
            if offline_note {
                spawn_action_text(
                    actions,
                    "Mod Details Offline Note",
                    "offline - reconnect to install or update".to_string(),
                    theme::PHOSPHOR_MUTED,
                );
            }
        }
    }
}
