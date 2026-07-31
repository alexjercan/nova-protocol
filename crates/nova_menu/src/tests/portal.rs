//! The Explore tab: portal resources inserted, no transport - observer captures
//! stand in for the portal client, so every action button is pinned to the RIGHT
//! event with the RIGHT id.

use std::time::Duration;

use bevy::{
    platform::time::Instant,
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_assets::prelude::{
    DownloadedMods, EnabledMods, InstallJobs, InstallStatus, ModMeta, PendingRemovals,
    PortalCatalog, PortalEntry, RemoteCatalog, RemoteCatalogState,
};

use super::support::{
    all_texts, app, downloaded_set, entity_by_name, mod_row, mods_app, observe_portal_events,
    selected_mod, PortalCaptures,
};
use crate::portal::{
    on_portal_action, PortalAction, PortalActionKind, UpdateRequest, UpdateRequested,
    UPDATE_TIMEOUT,
};

fn portal_entry(
    id: &str,
    version: &str,
    name: &str,
    author: &str,
    description: &str,
) -> PortalEntry {
    PortalEntry {
        id: id.to_string(),
        version: version.to_string(),
        bundle: format!("{id}.bundle.ron"),
        meta: ModMeta {
            name: name.to_string(),
            description: description.to_string(),
            author: author.to_string(),
            version: version.to_string(),
            ..Default::default()
        },
        files: vec![],
        total_size: 0,
    }
}

fn ready_catalog(entries: Vec<PortalEntry>) -> RemoteCatalog {
    RemoteCatalog {
        state: RemoteCatalogState::Ready(PortalCatalog {
            schema_version: 1,
            entries,
        }),
        last_good: None,
    }
}

/// A mods_app with the portal resources inserted and the Explore tab
/// opened via its real tab button.
fn explore_app(remote: RemoteCatalog, downloaded: DownloadedMods) -> App {
    let mut app = mods_app();
    app.insert_resource(remote);
    app.insert_resource(downloaded);
    app.insert_resource(InstallJobs::default());
    app.insert_resource(PendingRemovals::default());
    observe_portal_events(&mut app);
    app.update();
    let explore_tab = entity_by_name(&mut app, "Explore Online Tab").expect("explore tab");
    app.world_mut().trigger(Activate {
        entity: explore_tab,
    });
    app.update();
    app
}

/// The `Portal Status Tag` text of `id`'s row, if the row carries one.
fn row_tag(app: &mut App, id: &str) -> Option<String> {
    let row = mod_row(app, id).expect("the row exists");
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(row)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    children.into_iter().find_map(|child| {
        let named = app
            .world()
            .get::<Name>(child)
            .is_some_and(|n| n.as_str() == "Portal Status Tag");
        if named {
            app.world().get::<Text>(child).map(|t| t.0.clone())
        } else {
            None
        }
    })
}

/// A Ready catalog renders one selectable row per entry (wire meta name +
/// version/author line) with the right status tag - none / "installed" /
/// "update" (exact version-string mismatch) - and default-selects the
/// first entry, whose details and Install action render.
#[test]
fn explore_ready_lists_entries_with_status_tags() {
    let mut app = explore_app(
        ready_catalog(vec![
            portal_entry("alpha", "1.0.0", "Alpha Pack", "Ann", "Adds alpha."),
            portal_entry("bravo", "1.0.0", "Bravo Pack", "Bob", "Adds bravo."),
            portal_entry("charlie", "1.0.0", "Charlie Pack", "Cyn", "Adds charlie."),
        ]),
        downloaded_set(&[("bravo", "1.0.0"), ("charlie", "0.9.0")]),
    );

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "Alpha Pack"),
        "rows render the wire meta name: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "v1.0.0 - by Ann"),
        "rows render the catalog version + meta author line: {texts:?}"
    );

    assert_eq!(row_tag(&mut app, "alpha"), None, "not installed - no tag");
    assert_eq!(
        row_tag(&mut app, "bravo").as_deref(),
        Some("installed"),
        "downloaded at the catalog version"
    );
    assert_eq!(
        row_tag(&mut app, "charlie").as_deref(),
        Some("update"),
        "downloaded at a different version string"
    );

    assert_eq!(
        selected_mod(&app).as_deref(),
        Some("alpha"),
        "the first entry is default-selected"
    );
    assert!(
        texts.iter().any(|t| t == "Adds alpha."),
        "the details pane renders the selection's description"
    );
    assert!(
        entity_by_name(&mut app, "Mod Details Install Button").is_some(),
        "a not-installed entry offers Install"
    );
}

/// Opening the Explore tab fetches the catalog ONLY from Idle: Ready is
/// left alone (no gratuitous refetch), and the Idle/Fetching list renders
/// the muted fetching note.
#[test]
fn opening_explore_fetches_only_from_idle() {
    let mut app = explore_app(RemoteCatalog::default(), DownloadedMods::default());
    assert_eq!(
        app.world().resource::<PortalCaptures>().fetches,
        1,
        "Idle fetches on tab open"
    );
    let texts = all_texts(&mut app);
    assert!(
        texts
            .iter()
            .any(|t| t == "Fetching the mod portal catalog..."),
        "the fetching note renders: {texts:?}"
    );

    let app = explore_app(
        ready_catalog(vec![portal_entry(
            "alpha",
            "1.0.0",
            "Alpha Pack",
            "Ann",
            "Adds alpha.",
        )]),
        DownloadedMods::default(),
    );
    assert_eq!(
        app.world().resource::<PortalCaptures>().fetches,
        0,
        "Ready is left alone on tab open"
    );
}

/// A failed fetch renders the error + Retry; a surviving last-good
/// catalog renders below an offline note, browsable and selectable.
/// Retry force-resets the state to Idle BEFORE re-triggering (the 163508
/// R1.3 wedge recovery: the fetch observer refuses re-triggers while
/// Fetching, so a reset-less retry could be refused forever).
#[test]
fn catalog_error_renders_retry_and_the_stale_fallback() {
    let mut app = explore_app(
        RemoteCatalog {
            state: RemoteCatalogState::Error("portal catalog fetch failed: boom".to_string()),
            last_good: Some(PortalCatalog {
                schema_version: 1,
                entries: vec![portal_entry(
                    "alpha",
                    "1.0.0",
                    "Alpha Pack",
                    "Ann",
                    "Adds alpha.",
                )],
            }),
        },
        DownloadedMods::default(),
    );

    let texts = all_texts(&mut app);
    assert!(
        texts
            .iter()
            .any(|t| t == "portal catalog fetch failed: boom"),
        "the error text renders: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "offline - showing the last fetched catalog"),
        "the stale note renders over a surviving last_good: {texts:?}"
    );
    assert!(
        mod_row(&mut app, "alpha").is_some(),
        "the last-good entries render below the note"
    );
    assert_eq!(
        selected_mod(&app).as_deref(),
        Some("alpha"),
        "stale entries are selectable"
    );
    assert!(
        texts.iter().any(|t| t == "Adds alpha."),
        "the stale selection's details render"
    );

    let retry = entity_by_name(&mut app, "Portal Retry Button").expect("retry button");
    app.world_mut().trigger(Activate { entity: retry });
    app.update();
    let cap = app.world().resource::<PortalCaptures>();
    assert_eq!(cap.fetches, 1, "Retry re-triggers the fetch");
    assert!(
        cap.fetch_seen_idle,
        "the state was force-reset to Idle before the re-trigger"
    );
}

/// An Error with NO last-good catalog renders the error + Retry alone -
/// no offline note, no phantom rows.
#[test]
fn catalog_error_without_last_good_renders_no_stale_note() {
    let mut app = explore_app(
        RemoteCatalog {
            state: RemoteCatalogState::Error("boom".to_string()),
            last_good: None,
        },
        DownloadedMods::default(),
    );
    let texts = all_texts(&mut app);
    assert!(texts.iter().any(|t| t == "boom"));
    assert!(
        !texts
            .iter()
            .any(|t| t == "offline - showing the last fetched catalog"),
        "no stale note without a last_good: {texts:?}"
    );
    assert!(entity_by_name(&mut app, "Portal Retry Button").is_some());
    assert_eq!(selected_mod(&app), None, "nothing to select");
}

/// The action buttons fire the RIGHT portal event with the RIGHT id:
/// Install for a fresh entry, Uninstall for an installed one (same
/// version: no Update offered), and Update records the request +
/// triggers the uninstall, deferring the install until the id leaves
/// DownloadedMods (rendering "Updating..." meanwhile).
#[test]
fn explore_actions_trigger_the_right_events_with_the_right_ids() {
    let mut app = explore_app(
        ready_catalog(vec![
            portal_entry("alpha", "1.0.0", "Alpha Pack", "Ann", "Adds alpha."),
            portal_entry("bravo", "1.0.0", "Bravo Pack", "Bob", "Adds bravo."),
            portal_entry("charlie", "1.0.0", "Charlie Pack", "Cyn", "Adds charlie."),
        ]),
        downloaded_set(&[("bravo", "1.0.0"), ("charlie", "0.9.0")]),
    );

    // alpha (default selection, not installed): Install.
    let install = entity_by_name(&mut app, "Mod Details Install Button").expect("install");
    app.world_mut().trigger(Activate { entity: install });
    app.update();
    assert_eq!(
        app.world().resource::<PortalCaptures>().installs,
        vec!["alpha".to_string()],
        "Install fires InstallPortalMod with the selected id"
    );

    // bravo (installed, same version): Uninstall only.
    let bravo = mod_row(&mut app, "bravo").expect("bravo row");
    app.world_mut().trigger(Activate { entity: bravo });
    app.update();
    assert!(
        entity_by_name(&mut app, "Mod Details Update Button").is_none(),
        "matching version strings offer no Update"
    );
    let uninstall = entity_by_name(&mut app, "Mod Details Uninstall Button").expect("uninstall");
    app.world_mut().trigger(Activate { entity: uninstall });
    app.update();
    assert_eq!(
        app.world().resource::<PortalCaptures>().uninstalls,
        vec!["bravo".to_string()],
        "Uninstall fires UninstallPortalMod with the selected id"
    );

    // charlie (installed at 0.9.0, catalog 1.0.0): Update triggers the
    // uninstall and records the request; the install half must NOT fire
    // while the id is still in DownloadedMods.
    let charlie = mod_row(&mut app, "charlie").expect("charlie row");
    app.world_mut().trigger(Activate { entity: charlie });
    app.update();
    let update = entity_by_name(&mut app, "Mod Details Update Button").expect("update");
    app.world_mut().trigger(Activate { entity: update });
    app.update();
    {
        let cap = app.world().resource::<PortalCaptures>();
        assert_eq!(
            cap.uninstalls,
            vec!["bravo".to_string(), "charlie".to_string()],
            "Update fires the uninstall half immediately"
        );
        assert_eq!(
            cap.installs,
            vec!["alpha".to_string()],
            "no install while charlie is still in DownloadedMods"
        );
    }
    assert!(
        all_texts(&mut app).iter().any(|t| t == "Updating..."),
        "the pending update renders as progress, not buttons"
    );

    // The uninstall lands: the deferred install fires, once, right id;
    // the request then waits for the new record (the R1.4 enablement
    // stage) and clears when it lands.
    app.world_mut()
        .resource_mut::<DownloadedMods>()
        .0
        .retain(|m| m.record.id != "charlie");
    app.update();
    let cap = app.world().resource::<PortalCaptures>();
    assert_eq!(
        cap.installs,
        vec!["alpha".to_string(), "charlie".to_string()],
        "the install half fires once the id left DownloadedMods"
    );
    assert!(
        app.world()
            .resource::<UpdateRequested>()
            .0
            .contains_key("charlie"),
        "the request waits for the new record to land"
    );
    app.world_mut()
        .resource_mut::<DownloadedMods>()
        .0
        .extend(downloaded_set(&[("charlie", "1.0.0")]).0);
    app.update();
    assert!(
        app.world().resource::<UpdateRequested>().0.is_empty(),
        "the request clears once the new record landed"
    );
    assert!(
        !app.world().resource::<EnabledMods>().0.contains("charlie"),
        "charlie was disabled before the update; it stays disabled"
    );
}

/// Insert a raw update request (the shape `on_portal_action` records).
fn request_update(app: &mut App, id: &str, since: Instant, re_enable: bool) {
    app.world_mut().resource_mut::<UpdateRequested>().0.insert(
        id.to_string(),
        UpdateRequest {
            since,
            re_enable,
            install_fired: false,
        },
    );
}

/// The choreography guards, focused: the install half fires only after
/// the id has left BOTH DownloadedMods AND PendingRemovals (the 163508
/// race guard - a wasm uninstall's async file removal must not race the
/// reinstall's writes), and it fires exactly once.
#[test]
fn update_choreography_fires_only_after_both_guards_clear() {
    let mut app = app();
    observe_portal_events(&mut app);
    app.insert_resource(downloaded_set(&[("pack", "0.9.0")]));
    let mut pending = PendingRemovals::default();
    pending.0.insert("pack".to_string());
    app.insert_resource(pending);
    request_update(&mut app, "pack", Instant::now(), false);

    app.update();
    assert!(
        app.world().resource::<PortalCaptures>().installs.is_empty(),
        "still downloaded: the install must not fire"
    );

    app.world_mut().resource_mut::<DownloadedMods>().0.clear();
    app.update();
    assert!(
        app.world().resource::<PortalCaptures>().installs.is_empty(),
        "removal still pending: the install must not fire (the 163508 race guard)"
    );

    app.world_mut().resource_mut::<PendingRemovals>().0.clear();
    app.update();
    assert_eq!(
        app.world().resource::<PortalCaptures>().installs,
        vec!["pack".to_string()],
        "both guards cleared: the install fires with the right id"
    );
    app.update();
    assert_eq!(
        app.world().resource::<PortalCaptures>().installs.len(),
        1,
        "the install fires exactly once"
    );
}

/// A request stage older than the 30s wall-clock timeout is dropped
/// (with a warn) instead of holding a phantom install forever - and
/// stays dead even if the wedged uninstall settles later.
#[test]
fn update_request_times_out_and_stays_dead() {
    let mut app = app();
    observe_portal_events(&mut app);
    // The uninstall never lands: the id stays in DownloadedMods.
    app.insert_resource(downloaded_set(&[("pack", "0.9.0")]));
    app.insert_resource(PendingRemovals::default());
    let stale = Instant::now()
        .checked_sub(UPDATE_TIMEOUT + Duration::from_secs(1))
        .expect("the clock has more than 31s of history");
    request_update(&mut app, "pack", stale, false);

    app.update();
    assert!(
        app.world().resource::<UpdateRequested>().0.is_empty(),
        "the stale request is dropped"
    );

    app.world_mut().resource_mut::<DownloadedMods>().0.clear();
    app.update();
    assert!(
        app.world().resource::<PortalCaptures>().installs.is_empty(),
        "a dropped request never fires, even after the uninstall settles"
    );
}

/// Review 142916 R1.4: updating an ENABLED mod restores its enabled bit
/// once the new record lands (the uninstall strips EnabledMods and a
/// fresh install commits disabled - without this, Update silently
/// disables the mod); a DISABLED mod stays disabled through the same
/// choreography.
#[test]
fn update_preserves_the_enabled_bit() {
    let mut app = explore_app(
        ready_catalog(vec![portal_entry(
            "charlie",
            "1.0.0",
            "Charlie Pack",
            "Cyn",
            "Adds charlie.",
        )]),
        downloaded_set(&[("charlie", "0.9.0")]),
    );
    // The player has the mod ON when the update starts.
    app.world_mut()
        .resource_mut::<EnabledMods>()
        .0
        .insert("charlie".to_string());
    app.update();

    let update = entity_by_name(&mut app, "Mod Details Update Button").expect("update");
    app.world_mut().trigger(Activate { entity: update });
    // The portal side of the uninstall (captured, not executed here):
    // strip the record AND the enabled bit, as production does.
    app.world_mut()
        .resource_mut::<EnabledMods>()
        .0
        .remove("charlie");
    app.world_mut()
        .resource_mut::<DownloadedMods>()
        .0
        .retain(|m| m.record.id != "charlie");
    app.update(); // the install half fires
    assert_eq!(
        app.world().resource::<PortalCaptures>().installs,
        vec!["charlie".to_string()]
    );
    // The install commits: the new record lands (disabled, as always).
    app.world_mut()
        .resource_mut::<DownloadedMods>()
        .0
        .extend(downloaded_set(&[("charlie", "1.0.0")]).0);
    app.update();
    assert!(
        app.world().resource::<EnabledMods>().0.contains("charlie"),
        "the update restores the enabled bit the uninstall stripped"
    );
    assert!(
        app.world().resource::<UpdateRequested>().0.is_empty(),
        "the finished request is cleared"
    );

    // The disabled path is covered by the tail of
    // explore_actions_trigger_the_right_events_with_the_right_ids:
    // charlie was disabled there and stays disabled after the update.
}

/// Review 142916 R1.1: entries rendered from the STALE last-good fallback
/// must not offer Install or Update - an offline install can only fail,
/// and an offline Update would uninstall a working mod it cannot replace.
/// Uninstall stays (purely local), under the muted offline note; and the
/// action handler itself refuses Install/Update without a Ready catalog
/// (defense in depth), so even a stale button cannot destroy an install.
#[test]
fn stale_entries_offer_no_install_or_update() {
    // An update-available installed entry, rendered from last_good.
    let mut app = explore_app(
        RemoteCatalog {
            state: RemoteCatalogState::Error("boom".to_string()),
            last_good: Some(PortalCatalog {
                schema_version: 1,
                entries: vec![
                    portal_entry("alpha", "1.0.0", "Alpha Pack", "Ann", "Adds alpha."),
                    portal_entry("charlie", "1.0.0", "Charlie Pack", "Cyn", "Adds charlie."),
                ],
            }),
        },
        downloaded_set(&[("charlie", "0.9.0")]),
    );
    let charlie = mod_row(&mut app, "charlie").expect("charlie row");
    app.world_mut().trigger(Activate { entity: charlie });
    app.update();

    assert!(
        entity_by_name(&mut app, "Mod Details Update Button").is_none(),
        "no Update on a stale entry"
    );
    assert!(
        entity_by_name(&mut app, "Mod Details Uninstall Button").is_some(),
        "Uninstall (purely local) stays available"
    );
    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t == "offline - reconnect to install or update"),
        "the withheld action is explained by the offline note"
    );

    // A not-installed stale entry: no Install either, just the note.
    let alpha = mod_row(&mut app, "alpha").expect("alpha row");
    app.world_mut().trigger(Activate { entity: alpha });
    app.update();
    assert!(
        entity_by_name(&mut app, "Mod Details Install Button").is_none(),
        "no Install on a stale entry"
    );
    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t == "offline - reconnect to install or update"),
        "the offline note renders for the not-installed entry too"
    );

    // Defense in depth: even a synthetic Update/Install action (a stale
    // button surviving a race) is refused without a Ready catalog -
    // nothing is uninstalled, nothing recorded, nothing installed.
    let stale_update = app
        .world_mut()
        .spawn((
            PortalAction {
                id: "charlie".to_string(),
                kind: PortalActionKind::Update,
            },
            observe(on_portal_action),
        ))
        .id();
    let stale_install = app
        .world_mut()
        .spawn((
            PortalAction {
                id: "alpha".to_string(),
                kind: PortalActionKind::Install,
            },
            observe(on_portal_action),
        ))
        .id();
    app.update();
    app.world_mut().trigger(Activate {
        entity: stale_update,
    });
    app.world_mut().trigger(Activate {
        entity: stale_install,
    });
    app.update();
    let cap = app.world().resource::<PortalCaptures>();
    assert!(
        cap.uninstalls.is_empty(),
        "a refused Update must not fire the uninstall half"
    );
    assert!(cap.installs.is_empty(), "a refused Install fires nothing");
    assert!(
        app.world().resource::<UpdateRequested>().0.is_empty(),
        "a refused Update records no request"
    );
}

/// A Failed job renders its error with Retry + Dismiss: Retry re-triggers
/// the install with the right id; Dismiss clears the InstallJobs entry
/// (the 163508 R1.3 recovery affordance) and the pane recovers to the
/// plain Install action.
#[test]
fn failed_job_renders_error_retry_and_dismiss() {
    let mut app = explore_app(
        ready_catalog(vec![portal_entry(
            "alpha",
            "1.0.0",
            "Alpha Pack",
            "Ann",
            "Adds alpha.",
        )]),
        DownloadedMods::default(),
    );
    app.world_mut().resource_mut::<InstallJobs>().0.insert(
        "alpha".to_string(),
        InstallStatus::Failed("disk full".to_string()),
    );
    app.update();

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "disk full"),
        "the pane renders the failure reason: {texts:?}"
    );
    assert!(
        entity_by_name(&mut app, "Mod Details Install Button").is_none(),
        "no plain Install next to a failed job"
    );

    let retry = entity_by_name(&mut app, "Mod Details Retry Button").expect("retry");
    app.world_mut().trigger(Activate { entity: retry });
    app.update();
    assert_eq!(
        app.world().resource::<PortalCaptures>().installs,
        vec!["alpha".to_string()],
        "Retry re-triggers the install"
    );

    let dismiss = entity_by_name(&mut app, "Mod Details Dismiss Button").expect("dismiss");
    app.world_mut().trigger(Activate { entity: dismiss });
    app.update();
    assert!(
        app.world().resource::<InstallJobs>().0.is_empty(),
        "Dismiss clears the job entry"
    );
    assert!(
        entity_by_name(&mut app, "Mod Details Dismiss Button").is_none(),
        "the failed-state buttons are gone"
    );
    assert!(
        entity_by_name(&mut app, "Mod Details Install Button").is_some(),
        "the pane recovers to the Install action"
    );
}

/// A live job renders its progress stage as text and NO action buttons
/// (nothing to click mid-download; recovery affordances only exist for
/// Failed).
#[test]
fn in_flight_job_renders_progress_and_no_buttons() {
    let mut app = explore_app(
        ready_catalog(vec![portal_entry(
            "alpha",
            "1.0.0",
            "Alpha Pack",
            "Ann",
            "Adds alpha.",
        )]),
        DownloadedMods::default(),
    );
    app.world_mut().resource_mut::<InstallJobs>().0.insert(
        "alpha".to_string(),
        InstallStatus::Fetching { done: 1, total: 3 },
    );
    app.update();

    assert!(
        all_texts(&mut app)
            .iter()
            .any(|t| t == "Downloading 2/3..."),
        "the per-file progress renders"
    );
    for name in [
        "Mod Details Install Button",
        "Mod Details Uninstall Button",
        "Mod Details Update Button",
        "Mod Details Retry Button",
        "Mod Details Dismiss Button",
    ] {
        assert!(
            entity_by_name(&mut app, name).is_none(),
            "{name} must not render during a live job"
        );
    }

    app.world_mut()
        .resource_mut::<InstallJobs>()
        .0
        .insert("alpha".to_string(), InstallStatus::Committing);
    app.update();
    assert!(
        all_texts(&mut app).iter().any(|t| t == "Committing..."),
        "the commit stage renders"
    );
}
