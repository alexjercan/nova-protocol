//! bug_failed_assets: a broken mod must not be the end of the game.
//!
//! Stages REAL failures before the app exists - a cataloged optional mod whose
//! bundle is not parseable, and a downloaded one in the same state - then boots
//! the shipped app (via [`editor_app`]) with both switched on and walks what a
//! player walks: the front door comes up, one report names both mods, `OK, I
//! understand` puts it away, and New Game still starts.
//!
//! ONE SUBJECT: who a failed asset is allowed to stop. An OPTIONAL mod is
//! allowed to stop itself - it is disabled, persisted off and reported once. A
//! MANDATORY one stops the game, and then the screen owes the player a report
//! and the one action the platform actually has, which is the last thing the
//! range checks: both fatal presentations, built and inspected on the same
//! host, because a WASM build cannot be walked from here.
//!
//! The staged failures are the subject, so the asset server's own per-handle
//! ERROR is expected noise and this range clamps `bevy_asset` out of the log
//! (nothing here reads a log line - every claim is read off the world).
//!
//! Nothing is taken from the operator's profile or written back to it: the
//! catalog is a mirror in a temp dir, the cache and config roots are pointed
//! into it, and the whole tree goes away when the run ends.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example bug_failed_assets --features debug
//! # look for: `failed_assets: 2 broken mods disabled, 1 report`,
//! #           `failed_assets: the acknowledgement re-enabled nothing`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_failed_assets")]
#[command(version = "1.0.0")]
#[command(
    about = "Broken mods are disabled, persisted and reported once; a mandatory failure reports instead of spinning. Autopilot-only correctness range - play the game to use the menu",
    long_about = None
)]
struct Cli;

/// The cataloged optional mod this range breaks: declared in the mirrored
/// catalog, with a bundle file that is not parseable RON.
#[cfg(feature = "debug")]
const BROKEN_SHIPPED: &str = "broken-shipped";
/// The downloaded mod this range breaks: installed into the cache the same way
/// the portal installs one, with the same unparseable bundle.
#[cfg(feature = "debug")]
const BROKEN_DOWNLOAD: &str = "broken-download";
/// The shipped optional mod that is NOT broken: recovery has to leave it alone.
#[cfg(feature = "debug")]
const GOOD_MOD: &str = "example";
/// A scenario only [`GOOD_MOD`] registers - the proof its content still merged.
#[cfg(feature = "debug")]
const GOOD_MOD_SCENARIO: &str = "example_arena";

/// What a bundle manifest is not.
#[cfg(feature = "debug")]
const NOT_A_BUNDLE: &[u8] = b"( this file is not a bundle manifest";

/// The report's own nodes, by `Name`.
#[cfg(feature = "debug")]
const REPORT_OVERLAY: &str = "Mods Disabled Overlay";
#[cfg(feature = "debug")]
const REPORT_ACKNOWLEDGE: &str = "Mods Disabled Acknowledge Button";
/// The front door's way in.
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";

/// Seconds a boot or a scenario load is given. Sized to outlast a
/// software-rendered CI GPU and kept under the harness completion deadline, so
/// a stall names THIS beat.
#[cfg(feature = "debug")]
const BOOT_SECS: f32 = 90.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // Before the app: the app reads all of this at build time.
    #[cfg(feature = "debug")]
    let staged = stage_broken_mods();

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the run spends its time waiting on loads that
        // are MEANT to fail, and the number that would come out of that window
        // says nothing about the game's speed.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(failed_assets_script());
    }

    let exit = app.run();

    #[cfg(feature = "debug")]
    staged.clean_up();

    exit
}

/// The temp tree the run is staged in, kept so the end of the run can remove
/// it.
#[cfg(feature = "debug")]
struct StagedFailures {
    root: std::path::PathBuf,
}

#[cfg(feature = "debug")]
impl StagedFailures {
    fn clean_up(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Build the broken installation this range boots on, and point the game at it.
///
/// The asset root is a MIRROR: every shipped directory is symlinked into a temp
/// tree, and only `mods.catalog.ron` is rewritten - with one more optional entry
/// whose bundle file is garbage. Nothing in `assets/` is touched, so the range
/// cannot leave a broken mod behind in the repository.
#[cfg(feature = "debug")]
fn stage_broken_mods() -> StagedFailures {
    let root = std::env::temp_dir().join(format!("nova-failed-assets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    // The shipped tree this run mirrors: what bevy would have read.
    let source = std::path::PathBuf::from(
        std::env::var("BEVY_ASSET_ROOT").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_string()),
    )
    .join("assets");
    let mirror = root.join("assets");
    std::fs::create_dir_all(mirror.join("mods")).expect("the mirrored asset root");

    for entry in std::fs::read_dir(&source).expect("the shipped asset root") {
        let entry = entry.expect("an asset root entry");
        let name = entry.file_name();
        match name.to_string_lossy().as_ref() {
            // Rewritten below, with the broken entry added.
            "mods.catalog.ron" => {}
            // One level deeper, so the broken mod can live beside the shipped
            // ones without a directory of its own in the repository.
            "mods" => {
                for installed in std::fs::read_dir(entry.path()).expect("the shipped mods dir") {
                    let installed = installed.expect("an installed mod");
                    link(
                        &installed.path(),
                        &mirror.join("mods").join(installed.file_name()),
                    );
                }
            }
            _ => link(&entry.path(), &mirror.join(name)),
        }
    }

    let broken_dir = mirror.join("mods").join(BROKEN_SHIPPED);
    std::fs::create_dir_all(&broken_dir).expect("the broken mod's dir");
    std::fs::write(broken_dir.join("broken.bundle.ron"), NOT_A_BUNDLE)
        .expect("the broken bundle manifest");

    let catalog = std::fs::read_to_string(source.join("mods.catalog.ron")).expect("the catalog");
    let close = catalog.rfind("])").expect("the catalog's mod list closes");
    let mut patched = catalog.clone();
    patched.insert_str(
        close,
        &format!(
            "    (\n        id: \"{BROKEN_SHIPPED}\",\n        \
             bundle: \"mods/{BROKEN_SHIPPED}/broken.bundle.ron\",\n    ),\n"
        ),
    );
    std::fs::write(mirror.join("mods.catalog.ron"), patched).expect("the mirrored catalog");
    std::env::set_var("BEVY_ASSET_ROOT", &root);

    // The profile: this run's own cache and config, never the operator's.
    std::env::set_var(
        nova_assets::mod_cache::MOD_CACHE_ROOT_ENV,
        root.join("cache"),
    );
    std::env::set_var(nova_assets::storage::CONFIG_ROOT_ENV, root.join("config"));
    nova_assets::mod_cache::install_local(
        BROKEN_DOWNLOAD,
        "1.0.0",
        "broken.bundle.ron",
        &[("broken.bundle.ron".to_string(), NOT_A_BUNDLE.to_vec())],
    )
    .expect("the broken download installs into the cache");

    // Switched ON, all four: a mod the player never enabled is nobody's
    // problem, and the whole point is what happens to the ones they did.
    nova_assets::mod_prefs::save_enabled_ids(&[
        "base".to_string(),
        GOOD_MOD.to_string(),
        BROKEN_SHIPPED.to_string(),
        BROKEN_DOWNLOAD.to_string(),
    ]);

    // The loader says ERROR about every handle this range broke on purpose.
    // Probe's log gate cannot tell those from a real one, and the range reads
    // the world rather than the log, so the staged noise is clamped out.
    let rust_log = match std::env::var("RUST_LOG") {
        Ok(existing) if !existing.is_empty() => format!("{existing},bevy_asset=off"),
        _ => "bevy_asset=off".to_string(),
    };
    std::env::set_var("RUST_LOG", rust_log);

    StagedFailures { root }
}

/// Symlink `source` into the mirror at `target`.
#[cfg(feature = "debug")]
fn link(source: &std::path::Path, target: &std::path::Path) {
    std::os::unix::fs::symlink(source, target).expect("the mirrored asset entry");
}

/// The disabled ids and their reasons, as safe mode recorded them.
#[cfg(feature = "debug")]
fn quarantined(world: &World) -> Vec<(String, String)> {
    world
        .resource::<ModQuarantine>()
        .disabled
        .iter()
        .map(|disabled| (disabled.id.clone(), disabled.reason.clone()))
        .collect()
}

/// Whether `id` is in the live enabled set.
#[cfg(feature = "debug")]
fn enabled(world: &World, id: &str) -> bool {
    world.resource::<EnabledMods>().0.contains(id)
}

/// Whether `id` is in the SAVED enabled set - what the next launch reads.
#[cfg(feature = "debug")]
fn saved(id: &str) -> bool {
    nova_assets::mod_prefs::load_enabled_ids()
        .unwrap_or_default()
        .iter()
        .any(|saved| saved == id)
}

/// Advance once the recovered front door is up with its report on it.
#[cfg(feature = "debug")]
fn the_report_is_up() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let settled = state_is(GameStates::MainMenu);
    let laid_out = ui_node_present(REPORT_ACKNOWLEDGE);
    std::sync::Arc::new(move |world: &World| settled(world) && laid_out(world))
}

/// Advance once the report is off the screen - the acknowledgement landed.
#[cfg(feature = "debug")]
fn the_report_is_gone() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let present = ui_node_present(REPORT_OVERLAY);
    std::sync::Arc::new(move |world: &World| !present(world))
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn failed_assets_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // THE claim the whole range rests on: two broken enabled mods, and the
        // boot still ends at the front door rather than on the loading screen.
        .step("failed_assets: the front door comes up anyway")
        .until(the_report_is_up())
        .deadline(BOOT_SECS)
        .add()
        .step("failed_assets: both broken mods are disabled and reported")
        .on_enter(|world: &mut World| {
            let disabled = quarantined(world);
            let ids: Vec<&str> = disabled.iter().map(|(id, _)| id.as_str()).collect();
            assert!(
                ids.contains(&BROKEN_SHIPPED) && ids.contains(&BROKEN_DOWNLOAD),
                "both a cataloged and a downloaded failure belong in the quarantine, saw {ids:?}"
            );
            assert_eq!(
                ids.len(),
                2,
                "each failure is recorded ONCE - a bundle sits in Failed on every \
                 frame after it fails; saw {disabled:?}"
            );
            assert!(
                disabled.iter().all(|(_, reason)| !reason.is_empty()),
                "a disabled mod carries the reason it was disabled: {disabled:?}"
            );
            assert!(
                !enabled(world, BROKEN_SHIPPED) && !enabled(world, BROKEN_DOWNLOAD),
                "a quarantined mod is switched OFF, not merely complained about"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a broken optional mod is disabled instead of blocking the boot",
                serde_json::json!({ "disabled": disabled }),
            );
            info!("failed_assets: 2 broken mods disabled, 1 report");
        })
        .add()
        .step("failed_assets: the choice is on disk for the next launch")
        .on_enter(|world: &mut World| {
            assert!(
                !saved(BROKEN_SHIPPED) && !saved(BROKEN_DOWNLOAD),
                "safe mode persists what it switched off, or the next launch breaks again"
            );
            assert!(
                saved(GOOD_MOD),
                "persisting the disablement must not drop the mods that work"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the disabled set is persisted",
                serde_json::json!({ "saved": nova_assets::mod_prefs::load_enabled_ids() }),
            );
        })
        .add()
        .step("failed_assets: the mods that work are still merged")
        .on_enter(|world: &mut World| {
            let scenarios = world.resource::<GameScenarios>();
            assert!(
                scenarios.contains_key(GOOD_MOD_SCENARIO),
                "recovery drops the BROKEN mod, not the content of the ones that loaded"
            );
            assert!(
                enabled(world, GOOD_MOD),
                "the good mod stays enabled through its neighbour's failure"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the mods that still work are untouched",
                serde_json::json!({ "scenario": GOOD_MOD_SCENARIO }),
            );
        })
        .add()
        // The acknowledgement, through a real click on the real button.
        .step("failed_assets: click OK, I understand")
        .on_enter(click_named(REPORT_ACKNOWLEDGE))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("failed_assets: release OK, I understand")
        .on_enter(release_mouse(MouseButton::Left))
        .until(the_report_is_gone())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("failed_assets: the acknowledgement only acknowledges")
        .on_enter(|world: &mut World| {
            assert!(
                !world.resource::<ModQuarantine>().report_pending,
                "the report is answered and must not come back on the next frame"
            );
            assert_eq!(
                quarantined(world).len(),
                2,
                "dismissing the report does not un-record what failed"
            );
            assert!(
                !enabled(world, BROKEN_SHIPPED) && !enabled(world, BROKEN_DOWNLOAD),
                "Continue is an acknowledgement: it must never switch a broken mod back on"
            );
            assert!(
                !saved(BROKEN_SHIPPED) && !saved(BROKEN_DOWNLOAD),
                "...and it must not write them back to disk either"
            );
            let disabled = quarantined(world);
            nova_probe::probe_marker(
                world,
                "outcome: the acknowledgement re-enables nothing",
                serde_json::json!({ "disabled": disabled }),
            );
            info!("failed_assets: the acknowledgement re-enabled nothing");
        })
        .add()
        // ...and the recovered game is a game: the menu behind the report works.
        .step("failed_assets: click New Game")
        .on_enter(click_named(NEW_GAME_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("failed_assets: release New Game")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .deadline(BOOT_SECS)
        .add()
        .step("failed_assets: the recovered game plays")
        .on_enter(|world: &mut World| {
            let disabled = quarantined(world).len();
            nova_probe::probe_marker(
                world,
                "outcome: the recovered game still starts",
                serde_json::json!({ "disabled": disabled }),
            );
            info!("failed_assets: the recovered game reached gameplay");
        })
        .add()
        // The other half of the policy: the failure that is NOT survivable.
        // Both presentations are built here and read off the live tree - a web
        // build cannot be walked from a native host, and the difference between
        // them is exactly what must not rot.
        .step("failed_assets: the fatal report says what the platform can do")
        .on_enter(|world: &mut World| {
            let native = spawn_report(world, FailurePlatform::Native);
            let web = spawn_report(world, FailurePlatform::Web);

            let quits = descendant_named(world, native, FAILURE_QUIT_BUTTON);
            assert!(
                quits.is_some(),
                "a native fatal report offers the one action a desktop build has: Quit"
            );
            let native_text = report_text(world, native);
            assert!(
                native_text.contains("start the game again"),
                "...and says what to do about it: {native_text:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the native fatal report offers Quit",
                serde_json::json!({ "text": native_text }),
            );

            assert!(
                descendant_named(world, web, FAILURE_QUIT_BUTTON).is_none(),
                "a web build has no window to close: a Quit button there would be a lie"
            );
            let web_text = report_text(world, web);
            assert!(
                web_text.contains("Reload the page"),
                "the web report explains what the browser can do instead: {web_text:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the web fatal report explains instead of offering Quit",
                serde_json::json!({ "text": web_text }),
            );
            info!("failed_assets: both fatal presentations read back");

            // Inspected, and gone: the run ends on the game, not on a report
            // nothing in this process actually triggered.
            world.entity_mut(native).despawn();
            world.entity_mut(web).despawn();
        })
        .add()
}

/// Spawn one fatal report for `platform` and return its root.
#[cfg(feature = "debug")]
fn spawn_report(world: &mut World, platform: FailurePlatform) -> Entity {
    let mut commands = world.commands();
    let root = spawn_failure_report(
        &mut commands,
        platform,
        "the base game's assets did not load",
        Handle::default(),
    );
    world.flush();
    root
}

/// The entity named `name` under `root`, if the report spawned one.
#[cfg(feature = "debug")]
fn descendant_named(world: &mut World, root: Entity, name: &str) -> Option<Entity> {
    let children: Vec<Entity> = world
        .get::<Children>(root)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children.into_iter().find(|child| {
        world
            .get::<Name>(*child)
            .is_some_and(|child| child.as_str() == name)
    })
}

/// Everything the report says, joined - its own lines and its children's.
#[cfg(feature = "debug")]
fn report_text(world: &mut World, root: Entity) -> String {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            out.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    out.join(" | ")
}
