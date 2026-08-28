//! system_headless_crt: spike 6 for `nova_channel` - click a map blip through
//! the CRT glass, with no GPU anywhere.
//!
//! The ledger row this proves (task 20260820-174148, nova-channel.html):
//! "Click a map blip through the CRT glass - pointer to + press; the shipped
//! forwarded pointer does the warp math". The open question the row carried
//! was whether the glass EXISTS headless: `setup_nova_os` falls back to
//! terminal-directly-on-screen when the image or material assets are absent.
//! They are not absent in `--norender` - `UiMaterialPlugin` registers
//! `Assets<NovaOsCrtMaterial>` BEFORE its render-app check (bevy
//! `ui_material_pipeline.rs:51` vs `:55`) - so the RTT pipeline assembles and
//! every link in the chain is CPU math: the reconciler sizes the target image
//! from the screen node, the offscreen camera takes its target size from
//! `Assets<Image>`, `bevy_ui` lays the map out against it, and
//! `forward_nova_os_pointer` un-warps window px into image px. Only the
//! SAMPLING of the image is GPU work, and nothing here needs the picture.
//!
//! The drive is the full human shape, all wire: Tab opens the computer, `map`
//! launches over the keyboard path, the run aims at a blip by undoing the CRT
//! warp (`nova_os_window_px_showing` - the shipped inverse), asserts the
//! FORWARDED pointer and not the window mouse is hovering the blip, clicks it,
//! waits for the selection ring, then presses G and reads the verdict off the
//! ship: `Autopilot::engage(Goto { target })` for exactly the contact whose
//! blip was clicked.
//!
//! Map blips carry no `Name` - they are minted per contact. The locator is the
//! label: each blip's child pill holds a `Text` equal to the contact's
//! `MapContactCode` ("AST-2", "HOST-1"), the code component sits on the WORLD
//! entity, and two `ChildOf` hops up from the label is the blip button. That
//! is also this range's slice of the snapshot `ui` block: for the glass, "what
//! can I click" is the code list with each blip's window px, printed once the
//! zoom-out has framed the belt and the aim holds.
//!
//! Run (no display needed):
//! ```text
//! cargo run --example system_headless_crt --features debug
//! # look for: `headless crt: PASS ...` and the `glass census` JSON line.
//! ```

#[cfg(feature = "debug")]
use bevy::{
    input::keyboard::Key,
    picking::{hover::HoverMap, pointer::PointerId},
    prelude::*,
    ui::Pressed,
    window::PrimaryWindow,
};
#[cfg(feature = "debug")]
use nova_input::prelude::{InputBindings, InputSource};
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::{
    map::MapContactCode,
    nova_os::prelude::{NovaOsTerminal, TerminalMode},
    terminal::{nova_os_openness, nova_os_pointer_id, nova_os_window_px_showing},
};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_headless_crt drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let mut app = editor_app(
        false,
        Some(StartupScenario::Id("shakedown_run".to_string())),
    );

    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.init_resource::<Spike>();
    app.add_systems(PreUpdate, drive.after(bevy::input::InputSystems));

    app.run()
}

/// Frames a pointer beat holds: the forwarded pointer is two frames behind by
/// construction (`forward_nova_os_pointer` runs in `Update`, its `PointerInput`
/// is consumed the NEXT frame, observers react a frame after that).
#[cfg(feature = "debug")]
const SETTLE: u32 = 12;

#[cfg(feature = "debug")]
const DEADLINE_SECS: u64 = 180;

#[cfg(feature = "debug")]
#[derive(Resource)]
struct Spike {
    step: usize,
    wait: u32,
    /// Consecutive frames the aim beat could NOT place the target - for a
    /// paced diagnostic instead of a silent stall.
    lost: u32,
    started: std::time::Instant,
}

#[cfg(feature = "debug")]
impl Default for Spike {
    fn default() -> Self {
        Self {
            step: 0,
            wait: 0,
            lost: 0,
            started: std::time::Instant::now(),
        }
    }
}

/// The contact this run clicks: the WORLD entity and its map code. The blip
/// BUTTON is deliberately not cached - the map reconciles its scene, so every
/// beat re-resolves the blip from the code, the same way a channel client
/// would address it.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct GlassTarget {
    contact: Entity,
    code: String,
}

/// The blip the chosen target's code currently labels, freshly resolved.
#[cfg(feature = "debug")]
fn resolve_blip(world: &mut World) -> Option<Entity> {
    let code = world.resource::<GlassTarget>().code.clone();
    blip_labelled(world, &code)
}

/// Whether the target's blip is wearing the selection ring (an amber border on
/// an otherwise transparent blip). Written by `project_map_blips` every frame,
/// hidden or not.
#[cfg(feature = "debug")]
fn target_selected(world: &mut World) -> bool {
    resolve_blip(world)
        .and_then(|blip| world.get::<BorderColor>(blip))
        .is_some_and(|border| border.top.alpha() > 0.0)
}

/// The keyboard source the registry holds for `action` - resolved the way a
/// channel client would, instead of hard-coding the default key.
#[cfg(feature = "debug")]
fn bound_key(world: &World, action: &str) -> KeyCode {
    let bindings = world.resource::<InputBindings>();
    let spec = bindings
        .get(action)
        .unwrap_or_else(|| panic!("{action} is a registry row"));
    spec.keyboard
        .iter()
        .find_map(|source| match source {
            InputSource::Keyboard(code) => Some(*code),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{action} has a keyboard source"))
}

/// The blip button whose label pill holds `code`, if the map has plotted one:
/// label `Text` -> pill -> blip, the reverse of how `spawn_blip` builds it.
#[cfg(feature = "debug")]
fn blip_labelled(world: &mut World, code: &str) -> Option<Entity> {
    let mut texts = world.query::<(Entity, &Text)>();
    let label = texts
        .iter(world)
        .find(|(_, text)| text.0 == code)
        .map(|(entity, _)| entity)?;
    let pill = world.get::<ChildOf>(label)?.parent();
    let blip = world.get::<ChildOf>(pill)?.parent();
    world.get::<bevy::ui_widgets::Button>(blip)?;
    Some(blip)
}

/// Where the CRT shows `blip`, if it is plotted, visible, and on the picture:
/// the blip's image-space centre pushed through the shipped warp inverse.
#[cfg(feature = "debug")]
fn window_px_of(world: &mut World, blip: Entity) -> Option<Vec2> {
    if !world.get::<InheritedVisibility>(blip)?.get() {
        return None;
    }
    let node = world.get::<ComputedNode>(blip)?;
    let scale = node.inverse_scale_factor();
    if (node.size() * scale).cmple(Vec2::ZERO).any() {
        return None;
    }
    let image_px = world.get::<UiGlobalTransform>(blip)?.translation * scale;
    nova_os_window_px_showing(world, image_px)
}

/// Whether `pointer` is over `target` or anything inside it. Picking reports
/// the DEEPEST node - for a blip that is its label text - so the hit walks up.
#[cfg(feature = "debug")]
fn pointer_reached(world: &World, pointer: PointerId, target: Entity) -> bool {
    let Some(hits) = world.resource::<HoverMap>().get(&pointer) else {
        return false;
    };
    hits.keys().any(|hit| {
        std::iter::successors(Some(*hit), |entity| {
            world.get::<ChildOf>(*entity).map(|child| child.parent())
        })
        .any(|entity| entity == target)
    })
}

#[cfg(feature = "debug")]
fn drive(world: &mut World) {
    let spike = world.resource::<Spike>();
    let (step, wait) = (spike.step, spike.wait);
    if spike.started.elapsed().as_secs() > DEADLINE_SECS {
        panic!("headless crt: STALLED at step {step} after {DEADLINE_SECS}s");
    }

    let advance = |world: &mut World| {
        let mut spike = world.resource_mut::<Spike>();
        spike.step += 1;
        spike.wait = 0;
    };
    let hold = |world: &mut World| world.resource_mut::<Spike>().wait += 1;

    match step {
        0 => {
            if *world.resource::<State<GameStates>>().get() == GameStates::Playing {
                advance(world);
            }
        }
        1 => {
            if wait < 5 {
                hold(world);
            } else {
                press_key(KeyCode::Tab)(world);
                advance(world);
            }
        }
        2 => {
            if wait < 2 {
                hold(world);
            } else {
                release_key(KeyCode::Tab)(world);
                advance(world);
            }
        }
        3 => {
            if *world.resource::<State<PauseStates>>().get() == PauseStates::NovaOs {
                advance(world);
            } else {
                hold(world);
            }
        }
        // A click on a collapsing raster lands where the picture no longer is.
        4 => {
            if nova_os_openness(world).is_some_and(|open| open >= 1.0 - f32::EPSILON) {
                advance(world);
            } else {
                hold(world);
            }
        }
        5 => {
            let terminal = world.resource::<NovaOsTerminal>();
            if terminal.is_booted() && !terminal.has_pending_boot_rows() {
                type_text("map")(world);
                advance(world);
            } else {
                hold(world);
            }
        }
        6 => {
            if world.resource::<NovaOsTerminal>().prompt() == "map" {
                press_edit_key(Key::Enter)(world);
                advance(world);
            } else {
                hold(world);
            }
        }
        7 => {
            if world.resource::<NovaOsTerminal>().active_mode() == (TerminalMode::App { id: "map" })
            {
                advance(world);
            } else {
                hold(world);
            }
        }
        // Pick the target: the first (by code) non-SELF contact the map has
        // minted a blip button for. Whether the CRT currently SHOWS it is the
        // next beat's problem - the map opens framed tight on the player, and
        // the belt starts outside the picture.
        8 => {
            let mut contacts: Vec<(Entity, String)> = {
                let mut query = world.query::<(Entity, &MapContactCode)>();
                query
                    .iter(world)
                    .filter(|(_, code)| code.0 != "SELF")
                    .map(|(entity, code)| (entity, code.0.clone()))
                    .collect()
            };
            contacts.sort_by(|a, b| a.1.cmp(&b.1));
            let target = contacts
                .into_iter()
                .find(|(_, code)| blip_labelled(world, code).is_some())
                .map(|(contact, code)| GlassTarget { contact, code });
            match target {
                Some(target) => {
                    info!("headless crt: the target is {}", target.code);
                    world.insert_resource(target);
                    advance(world);
                }
                None => hold(world),
            }
        }
        // The map opens framed tight on the player, and the whole belt sits
        // outside even the max wheel zoom - a run against the real scenario
        // found that, not the trace. So the walk does what the map DESIGN
        // says: cycle the selection ring onto the target with the registry's
        // own `novaos_next` (the packet lane, key resolved from the table,
        // works on a hidden blip), re-frame the camera on it with
        // `novaos_reframe`, then move the ring OFF again - so the click
        // through the glass still has something to prove.
        9 => {
            if target_selected(world) {
                // The press that landed the ring may still be down (the check
                // can win the race to its release beat), and a key that stays
                // down never counts as just_pressed again.
                release_key(bound_key(world, "novaos_next"))(world);
                info!("headless crt: novaos_next cycled the ring onto the target");
                advance(world);
            } else {
                let next = bound_key(world, "novaos_next");
                match wait % 6 {
                    0 => press_key(next)(world),
                    1 => release_key(next)(world),
                    _ => {}
                }
                hold(world);
            }
        }
        10 => {
            if wait == 0 {
                press_key(bound_key(world, "novaos_reframe"))(world);
                hold(world);
            } else if wait == 1 {
                release_key(bound_key(world, "novaos_reframe"))(world);
                hold(world);
            } else if resolve_blip(world)
                .and_then(|blip| window_px_of(world, blip))
                .is_some()
            {
                info!("headless crt: novaos_reframe put the target on the picture");
                advance(world);
            } else if wait > 400 {
                panic!("headless crt: the reframe never brought the target into view");
            } else {
                hold(world);
            }
        }
        11 => {
            if wait == 0 {
                press_key(bound_key(world, "novaos_next"))(world);
                hold(world);
            } else if wait == 1 {
                release_key(bound_key(world, "novaos_next"))(world);
                hold(world);
            } else if !target_selected(world) {
                advance(world);
            } else if wait > 120 {
                panic!("headless crt: the ring never moved off the target");
            } else {
                if wait % 30 == 29 {
                    let codes: Vec<String> = {
                        let mut query = world.query::<&MapContactCode>();
                        query.iter(world).map(|code| code.0.clone()).collect()
                    };
                    let ringed: Vec<String> = codes
                        .into_iter()
                        .filter(|code| {
                            blip_labelled(world, code)
                                .and_then(|blip| world.get::<BorderColor>(blip))
                                .is_some_and(|border| border.top.alpha() > 0.0)
                        })
                        .collect();
                    info!("headless crt: ring debug - the ring is on {ringed:?}");
                }
                hold(world);
            }
        }
        // Aim. Re-resolves and re-tracks EVERY frame (the scene reconciles,
        // the camera can still be moving) and advances only after SETTLE
        // consecutive frames on target. The blip reports its rect in IMAGE
        // px; the shipped warp inverse says which WINDOW px shows it - aiming
        // at the image rect directly would put the cursor somewhere in the
        // cockpit. The None branch keeps a wheel zoom-out as a net.
        12 => {
            let blip = resolve_blip(world);
            match blip.and_then(|blip| window_px_of(world, blip)) {
                Some(window_px) => {
                    move_cursor(window_px)(world);
                    if wait == 0 {
                        info!("headless crt: aiming via window px {window_px:?}");
                    }
                    if wait < SETTLE {
                        hold(world);
                    } else {
                        advance(world);
                    }
                }
                None => {
                    let mut spike = world.resource_mut::<Spike>();
                    spike.wait = 0;
                    spike.lost += 1;
                    let lost = spike.lost;
                    if lost % 8 == 1 {
                        scroll_lines(-2.0)(world);
                    }
                    if lost % 60 == 1 {
                        let detail = match blip {
                            None => "no blip carries the label".to_string(),
                            Some(blip) => format!(
                                "blip {blip} visible {:?} size {:?}",
                                world
                                    .get::<InheritedVisibility>(blip)
                                    .map(|visibility| visibility.get()),
                                world.get::<ComputedNode>(blip).map(|node| node.size()),
                            ),
                        };
                        let own_ship = blip_labelled(world, "SELF").map(|self_blip| {
                            world
                                .get::<InheritedVisibility>(self_blip)
                                .is_some_and(|visibility| visibility.get())
                        });
                        let cameras: Vec<String> = {
                            let mut query = world.query::<(&Camera, &GlobalTransform)>();
                            query
                                .iter(world)
                                .map(|(camera, transform)| {
                                    format!(
                                        "(order {} viewport {:?} at {:.0})",
                                        camera.order,
                                        camera.logical_viewport_size(),
                                        transform.translation()
                                    )
                                })
                                .collect()
                        };
                        let code = world.resource::<GlassTarget>().code.clone();
                        info!(
                            "headless crt: zooming out ({lost} frames) for {code} - {detail}; \
                             SELF visible {own_ship:?}; cameras {cameras:?}"
                        );
                    }
                }
            }
        }
        // The glass census - the `ui` block's answer to "what can I click on
        // the CRT": every plotted contact code with the window px showing its
        // blip, or null for one the picture does not include even zoomed out.
        13 => {
            let mut codes: Vec<String> = {
                let mut query = world.query::<&MapContactCode>();
                query
                    .iter(world)
                    .filter(|code| code.0 != "SELF")
                    .map(|code| code.0.clone())
                    .collect()
            };
            codes.sort();
            let census: Vec<serde_json::Value> = codes
                .into_iter()
                .map(|code| {
                    let shown =
                        blip_labelled(world, &code).and_then(|blip| window_px_of(world, blip));
                    serde_json::json!({
                        "code": code,
                        "window_px": shown.map(|px| [px.x.round(), px.y.round()]),
                    })
                })
                .collect();
            info!(
                "headless crt: glass census {}",
                serde_json::Value::Array(census)
            );
            let target = resolve_blip(world).expect("the aim held this blip for SETTLE frames");
            assert!(
                pointer_reached(world, nova_os_pointer_id(), target),
                "the forwarded NOVA OS pointer must be hovering the blip"
            );
            assert!(
                !pointer_reached(world, PointerId::Mouse, target),
                "the blip must be reachable only THROUGH the image - a window \
                 mouse hit means it was never behind the image camera"
            );
            info!("headless crt: the forwarded pointer reached the blip");
            advance(world);
        }
        14 => {
            if wait == 0 {
                press_mouse(MouseButton::Left)(world);
            }
            if wait < SETTLE {
                hold(world);
            } else {
                let target = resolve_blip(world).expect("the hovered blip is still on the screen");
                assert!(
                    world.get::<Pressed>(target).is_some(),
                    "the blip must be holding the press that came through the glass"
                );
                advance(world);
            }
        }
        // `Activate` fires on release; the selection ring (an amber border on
        // an otherwise transparent blip) is the advance condition, so a click
        // that missed stalls HERE, named.
        15 => {
            if wait == 0 {
                release_mouse(MouseButton::Left)(world);
                hold(world);
            } else {
                let selected = target_selected(world);
                if selected {
                    info!("headless crt: the click selected the blip");
                    advance(world);
                } else if wait > 400 {
                    panic!("headless crt: the release never selected the blip");
                } else {
                    hold(world);
                }
            }
        }
        // G engages the map's GOTO on the selected contact - the verdict is on
        // the SHIP, far from the pointer path.
        16 => {
            if wait == 0 {
                press_key(bound_key(world, "map_goto"))(world);
                hold(world);
            } else {
                let player = {
                    let mut query = world.query_filtered::<Entity, (
                        With<PlayerSpaceshipMarker>,
                        With<SpaceshipRootMarker>,
                    )>();
                    query.single(world).expect("one player ship root")
                };
                match world.get::<Autopilot>(player).map(|pilot| pilot.action) {
                    Some(action) => {
                        let goto = bound_key(world, "map_goto");
                        release_key(goto)(world);
                        let contact = world.resource::<GlassTarget>().contact;
                        let code = world.resource::<GlassTarget>().code.clone();
                        assert_eq!(
                            action,
                            AutopilotAction::Goto { target: contact },
                            "GOTO must aim at the contact whose blip was clicked ({code})"
                        );
                        info!("headless crt: PASS clicked {code} through the glass, GOTO engaged");
                        world.write_message(AppExit::Success);
                        advance(world);
                    }
                    None if wait > 400 => {
                        panic!("headless crt: G never engaged the autopilot")
                    }
                    None => hold(world),
                }
            }
        }
        _ => {}
    }
}
