//! The prelude is the crate's whole import surface, so it is enforced rather
//! than reviewed: this test names every re-export through
//! `use nova_autopilot::prelude::*` (a compile-time check that nothing was
//! dropped) and scans the module sources for `pub` items (a runtime check that
//! nothing new skipped the prelude).

use bevy::prelude::*;
use nova_autopilot::prelude::*;

/// Every public item of the four driver modules. The compile-time uses below
/// and the source scan share this list, so the two halves cannot disagree.
const EXPORTED: &[&str] = &[
    // autopilot
    "AUTOPILOT_ENV",
    "AutopilotLoop",
    "AutopilotPlugin",
    // completion
    "AUTOPILOT",
    "DEADLINE_ENV",
    "DEFAULT_DEADLINE_SECS",
    "HarnessCompletion",
    "REEL",
    "SCREENSHOT",
    "register",
    // reel
    "REEL_CAPTURE_RESOLUTION",
    "REEL_ENV",
    "ReelBeat",
    "SHOT_DIR_ENV",
    "ScreenshotReelPlugin",
    "capture_window",
    // screenshot
    "MAX_WAIT_FRAMES",
    "SCREENSHOT_ENV",
    "ScreenshotPlugin",
];

/// The module sources the scan reads. `lib.rs` is not in this list because its
/// only `pub` items are the modules themselves and the prelude; it is scanned
/// separately by [`every_module_is_scanned`] so a NEW module cannot slip past.
const MODULES: &[(&str, &str)] = &[
    ("autopilot.rs", include_str!("../src/autopilot.rs")),
    ("completion.rs", include_str!("../src/completion.rs")),
    ("reel.rs", include_str!("../src/reel.rs")),
    ("screenshot.rs", include_str!("../src/screenshot.rs")),
];

/// The crate root, scanned for `pub mod` lines rather than for items.
const LIB_RS: &str = include_str!("../src/lib.rs");

/// A state type of the test's own, not a Nova one: the drivers are generic over
/// `S: States`, and naming them requires some `S`.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Boot,
}

/// Name every prelude export in a typed position. Dropping a re-export from
/// `prelude` fails this file's COMPILE, before any assertion runs.
fn name_every_export() {
    let _: AutopilotPlugin<DemoState> = AutopilotPlugin::new();
    let _: AutopilotLoop = AutopilotLoop;
    let _: &str = AUTOPILOT_ENV;

    let _: HarnessCompletion = HarnessCompletion::default();
    let _: fn(&mut App, &'static str) = register;
    let _: (&str, &str, &str) = (AUTOPILOT, SCREENSHOT, REEL);
    let _: &str = DEADLINE_ENV;
    let _: f32 = DEFAULT_DEADLINE_SECS;

    let _: ScreenshotReelPlugin = ScreenshotReelPlugin::new(vec![ReelBeat::new("shot.png")]);
    let _: fn(&mut World, &str) = capture_window;
    let _: (&str, &str) = (REEL_ENV, SHOT_DIR_ENV);
    let _: (f32, f32) = REEL_CAPTURE_RESOLUTION;

    let _: ScreenshotPlugin<DemoState> = ScreenshotPlugin::new(DemoState::Boot);
    let _: &str = SCREENSHOT_ENV;
    let _: u32 = MAX_WAIT_FRAMES;
}

/// Item kinds a module may declare `pub`. An unlisted kind is a hard failure
/// rather than a skip, so the scan can never quietly stop covering the surface.
const ITEM_KINDS: &[&str] = &["const", "enum", "fn", "static", "struct", "trait", "type"];

/// Names of the `pub` items `source` declares at module level. Methods and
/// fields are indented, so the column-0 `pub ` prefix selects items alone.
fn public_items(module: &str, source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("pub "))
        .map(|rest| {
            let kind = ITEM_KINDS
                .iter()
                .find(|kind| rest.starts_with(&format!("{kind} ")))
                .unwrap_or_else(|| {
                    panic!("{module}: unhandled public item kind in `pub {rest}`; teach the scan")
                });
            rest[kind.len() + 1..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

#[test]
fn prelude_names_every_public_item() {
    name_every_export();

    let declared: Vec<String> = MODULES
        .iter()
        .flat_map(|(module, source)| public_items(module, source))
        .collect();
    assert!(
        !declared.is_empty(),
        "the source scan found nothing: it stopped covering the crate"
    );

    for item in &declared {
        assert!(
            EXPORTED.contains(&item.as_str()),
            "`{item}` is public but not re-exported from `prelude`; add it there \
             and to EXPORTED"
        );
    }
    for item in EXPORTED {
        assert!(
            declared.contains(&item.to_string()),
            "`{item}` is listed in EXPORTED but no module declares it; the list \
             is stale"
        );
    }
}

#[test]
fn every_module_is_scanned() {
    let declared: Vec<&str> = LIB_RS
        .lines()
        .filter_map(|line| line.strip_prefix("pub mod "))
        .map(|rest| rest.trim_end_matches([';', ' ', '{']))
        .filter(|module| *module != "prelude")
        .collect();
    assert!(
        !declared.is_empty(),
        "the lib.rs scan found no modules: it stopped covering the crate"
    );

    for module in &declared {
        assert!(
            MODULES
                .iter()
                .any(|(file, _)| *file == format!("{module}.rs")),
            "`{module}` is a public module but MODULES does not scan it; its \
             public items would skip `prelude_names_every_public_item`"
        );
    }
    assert_eq!(
        declared.len(),
        MODULES.len(),
        "MODULES scans files lib.rs does not declare as public modules"
    );
}

#[test]
fn the_env_contract_names_are_the_documented_ones() {
    // The crate docs, the wiki page and every caller's run script spell these
    // out; renaming one is a contract break, not a refactor.
    assert_eq!(AUTOPILOT_ENV, "NOVA_AUTOPILOT");
    assert_eq!(SCREENSHOT_ENV, "NOVA_SHOT");
    assert_eq!(REEL_ENV, "NOVA_REEL");
    assert_eq!(SHOT_DIR_ENV, "NOVA_SHOT_DIR");
    assert_eq!(DEADLINE_ENV, "NOVA_AUTOPILOT_DEADLINE");
    assert_eq!(
        (AUTOPILOT, SCREENSHOT, REEL),
        ("autopilot", "screenshot", "reel")
    );
}
