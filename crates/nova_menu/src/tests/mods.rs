//! The mods screen: catalog listing, row selection and details, the enable
//! checkbox and its dependency handling, the installed/explore tab swap, and
//! the base mod staying locked on.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_assets::prelude::{
    EnabledMods, InstallJobs, ModCatalog, ModInfo, ModMeta, PendingRemovals,
};
use nova_ui::widget::Selected;

use super::support::{
    all_texts, app, checkbox_of, downloaded_set, entity_by_name, label_of, mod_row, mods_app,
    observe_portal_events, selected_mod, PortalCaptures,
};
use crate::mods::{dep_status, on_mod_toggle, DepStatus, ModEnableCheckbox, ModToggle};

/// Clicking a non-base mod's toggle flips its id in `EnabledMods` (the set the
/// nova_assets re-merge watches). Driven via `trigger(Activate)` like the other
/// button tests.
#[test]
fn mod_toggle_flips_enabled_state() {
    let mut app = app();
    app.insert_resource(EnabledMods::default());
    let toggle = app
        .world_mut()
        .spawn((
            ModToggle {
                id: "demo".to_string(),
                base: false,
            },
            observe(on_mod_toggle),
        ))
        .id();
    app.update();

    app.world_mut().trigger(Activate { entity: toggle });
    app.update();
    assert!(
        app.world().resource::<EnabledMods>().0.contains("demo"),
        "clicking an off toggle enables the mod"
    );

    app.world_mut().trigger(Activate { entity: toggle });
    app.update();
    assert!(
        !app.world().resource::<EnabledMods>().0.contains("demo"),
        "clicking an on toggle disables the mod"
    );
}

/// Entering the menu with a populated `ModCatalog` builds the two-pane mods
/// screen: one row per mod rendering the bundle META (name, version/author),
/// a quiet enable checkbox on the demo row only (base shows the locked tag),
/// and the details pane default-selected to the FIRST row (base), rendering
/// its description and dependencies from meta.
#[test]
fn mods_panel_lists_catalog_demo_checkbox_base_locked() {
    let mut app = mods_app();

    assert!(mod_row(&mut app, "base").is_some(), "base row exists");
    assert!(mod_row(&mut app, "demo").is_some(), "demo row exists");

    let toggles: Vec<String> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ModToggle, With<ModEnableCheckbox>>();
        q.iter(app.world()).map(|t| t.id.clone()).collect()
    };
    assert!(
        toggles.contains(&"demo".to_string()),
        "the demo mod row carries an enable checkbox"
    );
    assert!(
        !toggles.contains(&"base".to_string()),
        "base is locked - its row has no checkbox"
    );

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "Demo Mod"),
        "rows show the meta name, not the id: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "Base Game"),
        "the base row shows its meta name"
    );
    assert!(
        texts.iter().any(|t| t == "v0.2.0 - by Alice"),
        "rows show the muted version/author line: {texts:?}"
    );

    // Default selection: the first row (base), details rendered from meta.
    assert_eq!(selected_mod(&app).as_deref(), Some("base"));
    assert!(
        texts.iter().any(|t| t == "The core Nova Protocol content."),
        "the details pane renders the default selection's description"
    );
    assert!(
        texts.iter().any(|t| t == "Dependencies:") && texts.iter().any(|t| t == "  none"),
        "no dependencies renders the label plus 'none': {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "Enabled (base)"),
        "base's action area shows the locked tag, not a button"
    );
}

/// Clicking a row (not its checkbox) selects the mod: `SelectedModId` is set,
/// the row highlight moves, and the details pane rebuilds with the clicked
/// mod's meta (description, dependencies, Enable action).
#[test]
fn clicking_a_row_selects_it_and_renders_its_details() {
    let mut app = mods_app();
    let demo_row = mod_row(&mut app, "demo").expect("demo row exists");

    app.world_mut().trigger(Activate { entity: demo_row });
    app.update();

    assert_eq!(selected_mod(&app).as_deref(), Some("demo"));
    let base_row = mod_row(&mut app, "base").unwrap();
    let demo_row = mod_row(&mut app, "demo").unwrap();
    assert!(
        app.world().entity(demo_row).contains::<Selected>(),
        "the clicked row is highlighted"
    );
    assert!(
        !app.world().entity(base_row).contains::<Selected>(),
        "the previous selection is cleared"
    );

    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "A demo mod for testing."),
        "the details pane renders the clicked mod's description: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "  base - enabled"),
        "the details pane renders each dependency with its status (base is enabled): {texts:?}"
    );
    let button = entity_by_name(&mut app, "Mod Details Toggle Button")
        .expect("a non-base selection has an Enable/Disable action");
    assert_eq!(label_of(&app, button), "Enable", "demo starts disabled");
}

/// The row checkbox flips the mod in `EnabledMods` (absent -> present ->
/// absent) and its mark tracks the state; toggling never moves the selection.
#[test]
fn checkbox_click_flips_enabled_state_and_mark() {
    let mut app = mods_app();
    assert!(!app.world().resource::<EnabledMods>().0.contains("demo"));
    let checkbox = checkbox_of(&mut app, "demo").expect("demo has a checkbox");
    assert_eq!(label_of(&app, checkbox), "", "disabled renders no mark");

    app.world_mut().trigger(Activate { entity: checkbox });
    assert!(
        app.world().resource::<EnabledMods>().0.contains("demo"),
        "clicking an off checkbox enables the mod"
    );
    app.update();
    assert_eq!(label_of(&app, checkbox), "x", "enabled renders the mark");

    app.world_mut().trigger(Activate { entity: checkbox });
    assert!(
        !app.world().resource::<EnabledMods>().0.contains("demo"),
        "clicking an on checkbox disables the mod"
    );
    app.update();
    assert_eq!(label_of(&app, checkbox), "", "disabling clears the mark");

    // Quiet: the checkbox toggles without touching the selection.
    assert_eq!(selected_mod(&app).as_deref(), Some("base"));
}

/// The details pane's Enable/Disable button drives the same `EnabledMods`
/// toggle, and the pane rebuild relabels it.
#[test]
fn details_action_button_toggles_and_relabels() {
    let mut app = mods_app();
    let demo_row = mod_row(&mut app, "demo").expect("demo row exists");
    app.world_mut().trigger(Activate { entity: demo_row });
    app.update();

    let button = entity_by_name(&mut app, "Mod Details Toggle Button").unwrap();
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        app.world().resource::<EnabledMods>().0.contains("demo"),
        "the details Enable button enables the mod"
    );
    app.update();
    // The pane rebuilt on the EnabledMods change: find the fresh button.
    let button = entity_by_name(&mut app, "Mod Details Toggle Button")
        .expect("the rebuilt pane still has the action button");
    assert_eq!(label_of(&app, button), "Disable");
}

/// Switching to the Explore tab swaps the list to the portal catalog's
/// fetch state (in this portal-less rig: the fetching note - the same
/// rendering a real Idle/Fetching shows), moves the tab highlight, and
/// repairs the selection against the (empty) remote entries so no live
/// Enable/Disable survives next to portal content (review 142911 R1.2);
/// switching back restores the installed rows and re-runs the default
/// selection.
#[test]
fn tab_switch_swaps_list_to_the_explore_states() {
    let mut app = mods_app();
    let installed_tab = entity_by_name(&mut app, "Installed Tab").expect("installed tab");
    let explore_tab = entity_by_name(&mut app, "Explore Online Tab").expect("explore tab");
    assert!(
        app.world().entity(installed_tab).contains::<Selected>(),
        "Installed is the default tab"
    );
    // Select demo first, so the Explore switch has a live details action
    // to clear (the 142911 reviewer's exact scenario).
    let demo_row = mod_row(&mut app, "demo").expect("demo row exists");
    app.world_mut().trigger(Activate { entity: demo_row });
    app.update();
    assert!(entity_by_name(&mut app, "Mod Details Toggle Button").is_some());

    app.world_mut().trigger(Activate {
        entity: explore_tab,
    });
    app.update();

    assert!(
        app.world().entity(explore_tab).contains::<Selected>(),
        "the highlight moved to the Explore tab"
    );
    assert!(
        !app.world().entity(installed_tab).contains::<Selected>(),
        "the Installed tab is no longer highlighted"
    );
    assert!(
        mod_row(&mut app, "demo").is_none(),
        "the installed rows are gone on the Explore tab"
    );
    assert_eq!(
        selected_mod(&app),
        None,
        "no remote entries - the Explore tab clears the selection"
    );
    assert!(
        entity_by_name(&mut app, "Mod Details Toggle Button").is_none(),
        "no live Enable/Disable next to the portal content"
    );
    let texts = all_texts(&mut app);
    assert!(
        texts
            .iter()
            .any(|t| t == "Fetching the mod portal catalog..."),
        "the Explore tab shows the fetch state: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "Select a mod to see its details."),
        "the details pane shows its fallback on the Explore tab: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == "A demo mod for testing."),
        "the previously selected mod's details are gone"
    );

    app.world_mut().trigger(Activate {
        entity: installed_tab,
    });
    app.update();
    assert!(
        mod_row(&mut app, "demo").is_some(),
        "switching back restores the installed rows"
    );
    assert_eq!(
        selected_mod(&app).as_deref(),
        Some("base"),
        "switching back re-runs the default selection"
    );
    let texts = all_texts(&mut app);
    assert!(
        !texts
            .iter()
            .any(|t| t == "Fetching the mod portal catalog..."),
        "the fetch note is gone again"
    );
}

/// Installed-tab parity: a DOWNLOADED mod's details gain an Uninstall
/// button (next to Enable/Disable) that fires UninstallPortalMod with the
/// right id; non-downloaded (shipped) mods never show one.
#[test]
fn installed_tab_details_offer_uninstall_for_downloaded_mods() {
    let mut app = mods_app();
    app.insert_resource(downloaded_set(&[("demo", "0.2.0")]));
    app.insert_resource(InstallJobs::default());
    app.insert_resource(PendingRemovals::default());
    observe_portal_events(&mut app);
    app.update();

    // base (default selection, shipped): no Uninstall.
    assert!(
        entity_by_name(&mut app, "Mod Details Uninstall Button").is_none(),
        "a shipped mod's details carry no Uninstall"
    );

    let demo_row = mod_row(&mut app, "demo").expect("demo row");
    app.world_mut().trigger(Activate { entity: demo_row });
    app.update();
    assert!(
        entity_by_name(&mut app, "Mod Details Toggle Button").is_some(),
        "Enable/Disable stays alongside Uninstall"
    );
    let uninstall = entity_by_name(&mut app, "Mod Details Uninstall Button")
        .expect("a downloaded mod's details gain Uninstall on the Installed tab");
    app.world_mut().trigger(Activate { entity: uninstall });
    app.update();
    assert_eq!(
        app.world().resource::<PortalCaptures>().uninstalls,
        vec!["demo".to_string()],
        "the Installed-tab Uninstall fires with the right id"
    );
}

/// The base mod is locked on: even if a `ModToggle { base: true }` were clicked,
/// `on_mod_toggle` is a no-op, so base stays enabled.
#[test]
fn base_mod_toggle_is_locked_on() {
    let mut app = app();
    app.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    let toggle = app
        .world_mut()
        .spawn((
            ModToggle {
                id: "base".to_string(),
                base: true,
            },
            observe(on_mod_toggle),
        ))
        .id();
    app.update();

    app.world_mut().trigger(Activate { entity: toggle });
    app.update();
    assert!(
        app.world().resource::<EnabledMods>().0.contains("base"),
        "base is locked - toggling it must not disable it"
    );
}

// --- Mod dependencies ------------------------------------------------------

fn dep_mod(id: &str, base: bool, deps: &[&str]) -> ModInfo {
    ModInfo {
        id: id.to_string(),
        base,
        meta: ModMeta {
            name: id.to_string(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        },
    }
}

/// A catalog where `cool` depends on `lib`; `base` is implicit.
fn dep_catalog() -> ModCatalog {
    ModCatalog(vec![
        dep_mod("base", true, &[]),
        dep_mod("lib", false, &[]),
        dep_mod("cool", false, &["lib"]),
    ])
}

fn toggle_entity(app: &mut App, id: &str) -> Entity {
    let e = app
        .world_mut()
        .spawn((
            ModToggle {
                id: id.to_string(),
                base: false,
            },
            observe(on_mod_toggle),
        ))
        .id();
    app.update();
    e
}

fn is_enabled(app: &App, id: &str) -> bool {
    app.world().resource::<EnabledMods>().0.contains(id)
}

/// Enabling a mod auto-enables its (transitive) dependencies - Factorio.
#[test]
fn enabling_a_mod_auto_enables_its_dependencies() {
    let mut app = app();
    app.insert_resource(dep_catalog());
    app.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    let cool = toggle_entity(&mut app, "cool");

    app.world_mut().trigger(Activate { entity: cool });
    app.update();

    assert!(is_enabled(&app, "cool"), "the toggled mod is enabled");
    assert!(
        is_enabled(&app, "lib"),
        "its dependency was auto-enabled with it"
    );
}

/// Disabling a mod that an enabled mod still depends on is BLOCKED (block +
/// warn); once the dependent is disabled, the dependency can be disabled.
#[test]
fn disabling_a_depended_on_mod_is_blocked_until_its_dependents_go() {
    let mut app = app();
    app.insert_resource(dep_catalog());
    app.insert_resource(EnabledMods(
        ["base", "lib", "cool"]
            .into_iter()
            .map(String::from)
            .collect(),
    ));
    let lib = toggle_entity(&mut app, "lib");
    let cool = toggle_entity(&mut app, "cool");

    // Disabling lib is refused while cool (which needs it) is enabled.
    app.world_mut().trigger(Activate { entity: lib });
    app.update();
    assert!(
        is_enabled(&app, "lib"),
        "lib stays enabled - cool still depends on it"
    );

    // Disable the dependent first...
    app.world_mut().trigger(Activate { entity: cool });
    app.update();
    assert!(!is_enabled(&app, "cool"), "the leaf dependent disables");

    // ...now lib can be disabled.
    app.world_mut().trigger(Activate { entity: lib });
    app.update();
    assert!(!is_enabled(&app, "lib"), "with no dependents, lib disables");
}

/// The details-pane dependency status: enabled / installed-disabled / missing.
#[test]
fn dep_status_classifies_enabled_installed_and_missing() {
    let catalog = ModCatalog(vec![dep_mod("base", true, &[]), dep_mod("lib", false, &[])]);
    let enabled = EnabledMods(["base"].into_iter().map(String::from).collect());
    assert_eq!(
        dep_status("base", Some(&catalog), Some(&enabled)),
        DepStatus::Enabled
    );
    assert_eq!(
        dep_status("lib", Some(&catalog), Some(&enabled)),
        DepStatus::InstalledDisabled,
        "installed but not enabled"
    );
    assert_eq!(
        dep_status("ghost", Some(&catalog), Some(&enabled)),
        DepStatus::Missing,
        "not in the catalog"
    );
}
