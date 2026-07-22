//! Data-level "the accent is wired" rig for The Ledger's per-chapter look
//! (task 20260722-214115). The minimal-look pass gives each chapter a
//! deliberate STARTING skybox (`cubemap:` on the ScenarioConfig) and adds one
//! motivated mid-scenario `SetSkybox` accent tied to a real story beat, using
//! only base's two existing cubemaps (`cubemap.png`, `cubemap_alt.png`) - no
//! new art, no `self://` refs.
//!
//! The diagnostic brief's caution is "advertised is not wired": a future edit
//! that silently drops one of these swaps should fail a test. So this rig
//! loads each shipped RON and pins, at the data level:
//!
//! 1. the deliberate STARTING cubemap of every chapter (the palette table in
//!    NOTES.md), so a reshuffle is caught;
//! 2. that the intended mid-scenario `SetSkybox` accent is PRESENT in the RIGHT
//!    handler for ch1 (the 4th-ping reveal), ch3 (the debris pinch), and ch4
//!    (the Auditor arrival on the sell path) - the three swaps the task marks
//!    non-optional;
//! 3. that every `SetSkybox` in the mod targets one of base's two cubemaps
//!    (no accidental new-art path sneaking in), and that no chapter carries a
//!    `SetSkybox` in its OnStart (that is what `cubemap:` is for).
//!
//! The VISUAL confirmation (the swap actually rendering at the beat) is the
//! owner's Finish step - this rig is the data proof only, run standalone as
//! `cargo test -p nova_assets --test ledger_skybox`.

use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const CH1_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch1.content.ron");
const CH2_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch2.content.ron");
const CH2B_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch2b.content.ron");
const CH3_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch3.content.ron");
const CH4_RON: &str = include_str!("../../../webmods/the-ledger/ledger_ch4.content.ron");

const CUBEMAP: &str = "dep://base/textures/cubemap.png";
const CUBEMAP_ALT: &str = "dep://base/textures/cubemap_alt.png";

// --- content plumbing (mirrors the sibling ledger rigs) ---------------------

fn scenario_from(ron_str: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron_str).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("content contains a Scenario")
}

/// The cubemap path a `SetSkybox` action targets, if the action is a SetSkybox.
fn skybox_target(action: &EventActionConfig) -> Option<&str> {
    match action {
        EventActionConfig::SetSkybox(config) => Some(
            config
                .cubemap
                .path()
                .expect("SetSkybox cubemap is an authored path"),
        ),
        _ => None,
    }
}

/// Every `SetSkybox` target across all of a scenario's handlers.
fn all_skybox_targets(scenario: &ScenarioConfig) -> Vec<&str> {
    scenario
        .events
        .iter()
        .flat_map(|e| e.actions.iter())
        .filter_map(skybox_target)
        .collect()
}

/// Does this handler carry a StoryMessage whose text contains `needle`? Used
/// to pin a `SetSkybox` to the specific beat that motivates it (rather than
/// "some handler somewhere swaps").
fn handler_has_line(event: &ScenarioEventConfig, needle: &str) -> bool {
    event.actions.iter().any(|a| match a {
        EventActionConfig::StoryMessage(m) => m.text.contains(needle),
        _ => false,
    })
}

/// The single handler whose StoryMessage text contains `needle`.
fn handler_with_line<'a>(scenario: &'a ScenarioConfig, needle: &str) -> &'a ScenarioEventConfig {
    let mut hits = scenario
        .events
        .iter()
        .filter(|e| handler_has_line(e, needle));
    let found = hits
        .next()
        .unwrap_or_else(|| panic!("no handler carries a StoryMessage containing {needle:?}"));
    assert!(
        hits.next().is_none(),
        "expected exactly one handler with a StoryMessage containing {needle:?}"
    );
    found
}

fn handler_swaps_to(event: &ScenarioEventConfig, cubemap: &str) -> bool {
    event
        .actions
        .iter()
        .filter_map(skybox_target)
        .any(|t| t == cubemap)
}

// --- the deliberate STARTING palette (NOTES.md table) -----------------------

#[test]
fn starting_cubemaps_are_the_deliberate_palette() {
    // ch1 calm home belt; ch2/ch2b danger sky; ch3 running dark (quiet, calm);
    // ch4 calm home belt again. Consecutive chapters read distinct: ch2b->ch3
    // is alt->calm, ch3->ch4 both calm but ch4 shifts to danger mid-run.
    assert_eq!(
        scenario_from(CH1_RON).cubemap.path(),
        Some(CUBEMAP),
        "ch1 starts on the calm belt sky"
    );
    assert_eq!(
        scenario_from(CH2_RON).cubemap.path(),
        Some(CUBEMAP_ALT),
        "ch2 opens tense (danger sky)"
    );
    assert_eq!(
        scenario_from(CH2B_RON).cubemap.path(),
        Some(CUBEMAP_ALT),
        "ch2b opens tense (danger sky)"
    );
    assert_eq!(
        scenario_from(CH3_RON).cubemap.path(),
        Some(CUBEMAP),
        "ch3 runs dark/quiet on the calm sky (distinct from the ch2/ch2b danger sky)"
    );
    assert_eq!(
        scenario_from(CH4_RON).cubemap.path(),
        Some(CUBEMAP),
        "ch4 starts calm before the Auditor arrives"
    );
}

// --- the three non-optional accents are wired to their beat ------------------

#[test]
fn ch1_reveal_swaps_the_belt_wrong() {
    // The belt turns wrong the moment the black box shows up: the 4th-ping
    // reveal ANNOUNCE handler swaps to the alt sky.
    let ch1 = scenario_from(CH1_RON);
    let reveal = handler_with_line(&ch1, "fourth return");
    assert!(
        handler_swaps_to(reveal, CUBEMAP_ALT),
        "ch1 4th-ping reveal must SetSkybox to the alt (danger) sky"
    );
}

#[test]
fn ch3_pinch_swaps_the_channel_close() {
    // The channel closes in: the debris-pinch WARNING handler swaps to alt.
    let ch3 = scenario_from(CH3_RON);
    let pinch = handler_with_line(&ch3, "Channel narrows here");
    assert!(
        handler_swaps_to(pinch, CUBEMAP_ALT),
        "ch3 debris-pinch warning must SetSkybox to the alt (close/dark) sky"
    );
}

#[test]
fn ch4_auditor_arrival_swaps_to_danger() {
    // The sale brought the Auditor: the handoff-berth (sell path) handler
    // swaps to the alt (danger) sky as the gunship arrives.
    let ch4 = scenario_from(CH4_RON);
    let arrival = handler_with_line(&ch4, "military burn painting you");
    assert!(
        handler_swaps_to(arrival, CUBEMAP_ALT),
        "ch4 Auditor-arrival (sell path) must SetSkybox to the alt (danger) sky"
    );
}

// --- guardrails: no new art, no OnStart swaps -------------------------------

#[test]
fn every_swap_targets_a_base_cubemap() {
    // The minimal-look pass reuses base's two cubemaps ONLY - no swap may
    // point at a new image file or a self:// mod resource.
    for (name, ron) in [
        ("ch1", CH1_RON),
        ("ch2", CH2_RON),
        ("ch2b", CH2B_RON),
        ("ch3", CH3_RON),
        ("ch4", CH4_RON),
    ] {
        for target in all_skybox_targets(&scenario_from(ron)) {
            assert!(
                target == CUBEMAP || target == CUBEMAP_ALT,
                "{name}: SetSkybox target {target:?} is not one of base's two cubemaps"
            );
        }
    }
}

#[test]
fn no_chapter_swaps_at_on_start() {
    // A starting look belongs in `cubemap:`, never a frame-0 SetSkybox - every
    // accent must sit in the handler for the beat that motivates it.
    for (name, ron) in [
        ("ch1", CH1_RON),
        ("ch2", CH2_RON),
        ("ch2b", CH2B_RON),
        ("ch3", CH3_RON),
        ("ch4", CH4_RON),
    ] {
        let scenario = scenario_from(ron);
        for event in scenario
            .events
            .iter()
            .filter(|e| matches!(e.name, EventConfig::OnStart))
        {
            assert!(
                !event.actions.iter().any(|a| skybox_target(a).is_some()),
                "{name}: OnStart must not carry a SetSkybox (use the cubemap: field)"
            );
        }
    }
}
