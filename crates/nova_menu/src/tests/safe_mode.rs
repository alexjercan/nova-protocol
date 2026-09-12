//! The safe-mode report: what the modal says about the mods the game switched
//! off, and what acknowledging it does (dismiss, and nothing else).

use bevy::{prelude::*, ui_widgets::Activate};
use nova_assets::prelude::{DisabledMod, EnabledMods, ModQuarantine};
use nova_gameplay::prelude::GameStates;

use super::support::{all_texts, app, dummy_scenarios, entity_by_name};
use crate::safe_mode::ModReportOverlay;

const OVERLAY: &str = "Mods Disabled Overlay";
const ACKNOWLEDGE: &str = "Mods Disabled Acknowledge Button";
const OVERFLOW: &str = "Mods Disabled Overflow";

/// A menu app on the front door with `count` mods in quarantine, reported.
fn menu_reporting(count: usize) -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    app.insert_resource(ModQuarantine {
        disabled: (0..count)
            .map(|n| DisabledMod {
                id: format!("broken-{n}"),
                reason: format!("bundle {n} did not parse"),
            })
            .collect(),
        report_pending: true,
    });
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app
}

fn overlays(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&ModReportOverlay>();
    q.iter(app.world()).count()
}

/// The front door owes ONE report, and it names every mod it switched off with
/// the reason the player can act on.
#[test]
fn a_pending_quarantine_raises_one_report_naming_every_disabled_mod() {
    let mut app = menu_reporting(2);

    assert_eq!(overlays(&mut app), 1, "one modal, not one per failed mod");
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "MODS DISABLED"),
        "the report leads with what happened: {texts:?}"
    );
    for n in 0..2 {
        assert!(
            texts
                .iter()
                .any(|t| t.contains(&format!("broken-{n}")) && t.contains("did not parse")),
            "each disabled mod is named with its reason: {texts:?}"
        );
    }

    // A second frame changes nothing: the reconciler rebuilds only on a change.
    app.update();
    assert_eq!(
        overlays(&mut app),
        1,
        "the report is not respawned per frame"
    );
}

/// A mod set can break in bulk. The modal lists what fits and counts the rest,
/// rather than growing past the screen it is drawn on.
#[test]
fn a_long_quarantine_lists_what_fits_and_counts_the_rest() {
    let mut app = menu_reporting(11);

    let rows = {
        let mut q = app.world_mut().query::<(&Name, &Text)>();
        q.iter(app.world())
            .filter(|(name, _)| name.as_str() == "Mods Disabled Row")
            .count()
    };
    assert_eq!(rows, 8, "the list is capped at REPORT_ROWS");
    let overflow = entity_by_name(&mut app, OVERFLOW).expect("the overflow line");
    assert_eq!(
        app.world().get::<Text>(overflow).map(|t| t.0.clone()),
        Some("...and 3 more, listed under Mods.".to_string())
    );
}

/// Nothing to report, no modal: a launch whose mods all loaded shows the menu
/// it always showed.
#[test]
fn an_empty_quarantine_raises_nothing() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(ModQuarantine::default());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    assert_eq!(overlays(&mut app), 0);
    assert!(entity_by_name(&mut app, OVERLAY).is_none());
}

/// `OK, I understand` is an acknowledgement. It takes the modal down and leaves
/// the quarantine exactly as it found it - re-enabling a mod whose content is
/// still broken would put the player back where they started.
#[test]
fn acknowledging_the_report_dismisses_it_and_re_enables_nothing() {
    let mut app = menu_reporting(2);
    let button = entity_by_name(&mut app, ACKNOWLEDGE).expect("the acknowledge button");

    app.world_mut().trigger(Activate { entity: button });
    app.update();

    assert!(
        !app.world().resource::<ModQuarantine>().report_pending,
        "the episode stops owing a report"
    );
    assert_eq!(overlays(&mut app), 0, "the modal is gone");
    assert_eq!(
        app.world().resource::<ModQuarantine>().disabled.len(),
        2,
        "what was disabled stays recorded for the Mods screen"
    );
    let enabled = &app.world().resource::<EnabledMods>().0;
    assert!(
        !enabled.contains("broken-0") && !enabled.contains("broken-1"),
        "acknowledging never switches a broken mod back on: {enabled:?}"
    );
}
