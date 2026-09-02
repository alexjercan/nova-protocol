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
//! warp (`nova_os_window_px_showing` - the shipped inverse), waits for the
//! FORWARDED pointer to arrive on the blip, clicks it, waits for the selection
//! ring, then presses G and reads the verdict off the ship:
//! `Autopilot::engage(Goto { target })` for exactly the contact whose blip was
//! clicked. That the WINDOW mouse cannot reach the same blip is asserted at the
//! census, because no beat waits on it.
//!
//! Map blips carry no `Name` - they are minted per contact. The locator is the
//! label: each blip's child pill holds a `Text` equal to the contact's
//! `MapContactCode` ("AST-2", "HOST-1"), the code component sits on the WORLD
//! entity, and two `ChildOf` hops up from the label is the blip button. That
//! is also this range's slice of the snapshot `ui` block: for the glass, "what
//! can I click" is the code list with each blip's window px, printed once the
//! zoom-out has framed the belt and the aim has landed.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_headless_crt --features debug
//! # look for: `headless crt: PASS ...` and the `glass census` JSON line.
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

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

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("headless crt: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless crt: Tab opens the monitor")
            .on_enter(press_key(KeyCode::Tab))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::NovaOs
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless crt: release Tab")
            .on_enter(release_key(KeyCode::Tab))
            .add()
            // A click on a collapsing raster lands where the picture no longer
            // is: the warp inverse reads the openness, so it answers with a
            // different window px on every frame of the bloom.
            .step("headless crt: the raster finished opening")
            .until(the_raster_is_open())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless crt: the boot banner drained")
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.is_booted() && !terminal.has_pending_boot_rows()
            }))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless crt: the prompt took the typing")
            .on_enter(type_text("map"))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == "map"
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless crt: Enter launches the map app")
            .on_enter(press_edit_key(Key::Enter))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.active_mode() == (TerminalMode::App { id: "map" })
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // Pick the target: the first (by code) non-SELF contact the map has
            // minted a blip button for. Whether the CRT currently SHOWS it is a
            // later beat's problem - the map opens framed tight on the player,
            // and the belt starts outside the picture.
            .step("headless crt: pick the target contact")
            .each(|world: &mut World, _, _| pick_the_target(world))
            .until(the_target_is_picked())
            .diagnose(plotted_codes)
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The map opens framed tight on the player, and the whole belt sits
            // outside even the max wheel zoom - a run against the real scenario
            // found that, not the trace. So the walk does what the map DESIGN
            // says: cycle the selection ring onto the target with the
            // registry's own `novaos_next` (the packet lane, key resolved from
            // the table, works on a hidden blip), re-frame the camera on it
            // with `novaos_reframe`, then move the ring OFF again - so the
            // click through the glass still has something to prove.
            .step("headless crt: novaos_next cycles the ring onto the target")
            .each(|world: &mut World, _, frame| pulse_action(world, "novaos_next", frame))
            .until(the_target_is_selected())
            .diagnose(ringed_codes)
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The press that landed the ring may still be down (the gate can
            // win the race to this beat), and a key that stays down never
            // counts as just_pressed again.
            .step("headless crt: release novaos_next")
            .on_enter(release_action_key("novaos_next"))
            .add()
            .step("headless crt: press novaos_reframe")
            .on_enter(press_action_key("novaos_reframe"))
            .add()
            .step("headless crt: release novaos_reframe")
            .on_enter(release_action_key("novaos_reframe"))
            .add()
            .step("headless crt: the reframe put the target on the picture")
            .until(the_target_is_on_the_picture())
            .diagnose(aim_diagnosis)
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless crt: press novaos_next off the target")
            .on_enter(press_action_key("novaos_next"))
            .add()
            .step("headless crt: release novaos_next off the target")
            .on_enter(release_action_key("novaos_next"))
            .add()
            .step("headless crt: the ring moved off the target")
            .until(nova_autopilot::predicate::not(the_target_is_selected()))
            .diagnose(ringed_codes)
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // Aim. Re-resolves and re-tracks EVERY frame (the scene reconciles,
            // the camera can still be moving). The blip reports its rect in
            // IMAGE px; the shipped warp inverse says which WINDOW px shows it
            // - aiming at the image rect directly would put the cursor
            // somewhere in the cockpit. A frame that can place nothing takes a
            // wheel notch out as a net.
            //
            // The gate is the FORWARDED pointer arriving, which is what a frame
            // count here was guessing at: `forward_nova_os_pointer` runs in
            // `Update`, its `PointerInput` is consumed the next frame, and the
            // hover lands the frame after that.
            .step("headless crt: the forwarded pointer reaches the blip")
            .each(aim_through_the_glass)
            .until(the_forwarded_pointer_is_on_the_target())
            .diagnose(aim_diagnosis)
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("headless crt: the glass census")
            .on_enter(census_the_glass)
            .add()
            .step("headless crt: press through the glass")
            .on_enter(press_mouse(MouseButton::Left))
            .until(the_target_is_pressed())
            .diagnose(aim_diagnosis)
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless crt: record the press")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: the press lands through the glass",
                    serde_json::json!({}),
                );
            })
            .add()
            // `Activate` fires on release; the selection ring is the ack, so a
            // click that missed stalls HERE, named.
            .step("headless crt: the release selects the blip")
            .on_enter(release_mouse(MouseButton::Left))
            .until(the_target_is_selected())
            .diagnose(ringed_codes)
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // G engages the map's GOTO on the selected contact - the verdict is
            // on the SHIP, far from the pointer path. The gate is only that the
            // ship took SOME autopilot action; that it aims at the contact
            // whose blip was clicked is what the next beat asserts.
            .step("headless crt: G engages the ship autopilot")
            .on_enter(press_action_key("map_goto"))
            .until(the_ship_is_autopiloting())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("headless crt: the GOTO aims at the clicked contact")
            .on_enter(release_action_key("map_goto"))
            .on_enter(assert_the_goto_targets_the_clicked_contact)
            .add(),
    );

    app.run()
}

/// A step gate, spelled once for the local predicates.
#[cfg(feature = "debug")]
type Gate = Arc<nova_protocol::nova_debug::harness::Predicate>;

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
fn resolve_blip(world: &World) -> Option<Entity> {
    let code = world.get_resource::<GlassTarget>()?.code.clone();
    blip_labelled(world, &code)
}

/// The blip button whose label pill holds `code`, if the map has plotted one:
/// label `Text` -> pill -> blip, the reverse of how `spawn_blip` builds it.
#[cfg(feature = "debug")]
fn blip_labelled(world: &World, code: &str) -> Option<Entity> {
    let mut texts = world.try_query::<(Entity, &Text)>()?;
    let label = texts
        .iter(world)
        .find(|(_, text)| text.0 == code)
        .map(|(entity, _)| entity)?;
    let pill = world.get::<ChildOf>(label)?.parent();
    let blip = world.get::<ChildOf>(pill)?.parent();
    world.get::<bevy::ui_widgets::Button>(blip)?;
    Some(blip)
}

/// Every non-SELF contact code the map has plotted, sorted.
#[cfg(feature = "debug")]
fn plotted_contacts(world: &World) -> Vec<(Entity, String)> {
    let Some(mut codes) = world.try_query::<(Entity, &MapContactCode)>() else {
        return Vec::new();
    };
    let mut contacts: Vec<(Entity, String)> = codes
        .iter(world)
        .filter(|(_, code)| code.0 != "SELF")
        .map(|(entity, code)| (entity, code.0.clone()))
        .collect();
    contacts.sort_by(|a, b| a.1.cmp(&b.1));
    contacts
}

/// Where the CRT shows `blip`, if it is visible and on the picture: the blip's
/// image-space centre pushed through the shipped warp inverse.
#[cfg(feature = "debug")]
fn window_px_of(world: &World, blip: Entity) -> Option<Vec2> {
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

/// Where the CRT shows the target's blip, if it is plotted at all.
#[cfg(feature = "debug")]
fn target_window_px(world: &World) -> Option<Vec2> {
    window_px_of(world, resolve_blip(world)?)
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

/// Press the key the registry binds to `action`.
#[cfg(feature = "debug")]
fn press_action_key(action: &'static str) -> impl Fn(&mut World) {
    move |world: &mut World| press_key(bound_key(world, action))(world)
}

/// Release the key the registry binds to `action`.
#[cfg(feature = "debug")]
fn release_action_key(action: &'static str) -> impl Fn(&mut World) {
    move |world: &mut World| release_key(bound_key(world, action))(world)
}

/// Tap `action` once every six frames: a repeatable verb needs a fresh edge,
/// and a key held down never counts as pressed again.
#[cfg(feature = "debug")]
fn pulse_action(world: &mut World, action: &'static str, frame: u32) {
    match frame % 6 {
        1 => press_action_key(action)(world),
        2 => release_action_key(action)(world),
        _ => {}
    }
}

/// Record the first plotted contact that has a blip as this run's target.
#[cfg(feature = "debug")]
fn pick_the_target(world: &mut World) {
    let picked = plotted_contacts(world)
        .into_iter()
        .find(|(_, code)| blip_labelled(world, code).is_some());
    if let Some((contact, code)) = picked {
        info!("headless crt: the target is {code}");
        world.insert_resource(GlassTarget { contact, code });
    }
}

/// Re-aim at the target every frame, and take a wheel notch out on a frame that
/// can place nothing - the belt can still be off the picture after the reframe.
#[cfg(feature = "debug")]
fn aim_through_the_glass(world: &mut World, _elapsed: f32, frame: u32) {
    match target_window_px(world) {
        Some(window_px) => {
            if frame == 1 {
                info!("headless crt: aiming via window px {window_px:?}");
            }
            move_cursor(window_px)(world);
        }
        None if frame % 8 == 1 => scroll_lines(-2.0)(world),
        None => {}
    }
}

/// Advance once the CRT's raster has finished blooming open.
#[cfg(feature = "debug")]
fn the_raster_is_open() -> Gate {
    Arc::new(|world: &World| nova_os_openness(world).is_some_and(|open| open >= 1.0 - f32::EPSILON))
}

/// Advance once a contact has been chosen and its blip located.
#[cfg(feature = "debug")]
fn the_target_is_picked() -> Gate {
    Arc::new(|world: &World| world.get_resource::<GlassTarget>().is_some())
}

/// Whether the target's blip is wearing the selection ring (an amber border on
/// an otherwise transparent blip). Written by `project_map_blips` every frame,
/// hidden or not.
#[cfg(feature = "debug")]
fn the_target_is_selected() -> Gate {
    Arc::new(|world: &World| {
        resolve_blip(world)
            .and_then(|blip| world.get::<BorderColor>(blip))
            .is_some_and(|border| border.top.alpha() > 0.0)
    })
}

/// Advance once the CRT actually shows the target - the reframe's real ack,
/// where a frame count only said the camera had been asked to move.
#[cfg(feature = "debug")]
fn the_target_is_on_the_picture() -> Gate {
    Arc::new(|world: &World| target_window_px(world).is_some())
}

/// Advance once the pointer the CRT forwards through the warp is hovering the
/// target's blip.
#[cfg(feature = "debug")]
fn the_forwarded_pointer_is_on_the_target() -> Gate {
    Arc::new(|world: &World| {
        resolve_blip(world).is_some_and(|blip| pointer_reached(world, nova_os_pointer_id(), blip))
    })
}

/// Advance once the blip is holding the press that came through the glass.
#[cfg(feature = "debug")]
fn the_target_is_pressed() -> Gate {
    Arc::new(|world: &World| {
        resolve_blip(world).is_some_and(|blip| world.get::<Pressed>(blip).is_some())
    })
}

/// Advance once the player ship has taken SOME autopilot action.
#[cfg(feature = "debug")]
fn the_ship_is_autopiloting() -> Gate {
    Arc::new(|world: &World| {
        player_ship(world).is_some_and(|ship| world.get::<Autopilot>(ship).is_some())
    })
}

/// The one player ship root, if the scenario has spawned it.
#[cfg(feature = "debug")]
fn player_ship(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<PlayerSpaceshipMarker>, With<SpaceshipRootMarker>)>()?
        .single(world)
        .ok()
}

/// Which codes the map has plotted - the fix for a target that never appeared
/// is a different scenario, not a longer wait.
#[cfg(feature = "debug")]
fn plotted_codes(world: &World) -> String {
    let codes: Vec<String> = plotted_contacts(world)
        .into_iter()
        .map(|(_, code)| code)
        .collect();
    format!("the map has plotted {codes:?}")
}

/// Which codes are wearing the selection ring, so a ring beat that stalls says
/// where the ring actually is.
#[cfg(feature = "debug")]
fn ringed_codes(world: &World) -> String {
    let ringed: Vec<String> = plotted_contacts(world)
        .into_iter()
        .filter(|(_, code)| {
            blip_labelled(world, code)
                .and_then(|blip| world.get::<BorderColor>(blip))
                .is_some_and(|border| border.top.alpha() > 0.0)
        })
        .map(|(_, code)| code)
        .collect();
    format!("the ring is on {ringed:?}")
}

/// Why the aim cannot place the target: whether the blip exists at all, whether
/// it is visible and laid out, whether the map is drawing anything (SELF), and
/// where the cameras are looking.
#[cfg(feature = "debug")]
fn aim_diagnosis(world: &World) -> String {
    let blip = resolve_blip(world);
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
    let cameras: Vec<String> = world
        .try_query::<(&Camera, &GlobalTransform)>()
        .map(|mut query| {
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
        })
        .unwrap_or_default();
    format!("{detail}; SELF visible {own_ship:?}; cameras {cameras:?}")
}

/// The `ui` block's answer to "what can I click on the CRT": every plotted
/// contact code with the window px showing its blip, or null for one the
/// picture does not include even zoomed out.
///
/// The window mouse is asserted HERE because no beat waits on it: the aim gate
/// proved the FORWARDED pointer arrived, and the claim this spike exists for is
/// that the same blip is unreachable from window space.
#[cfg(feature = "debug")]
fn census_the_glass(world: &mut World) {
    let census: Vec<serde_json::Value> = plotted_contacts(world)
        .into_iter()
        .map(|(_, code)| {
            let shown = blip_labelled(world, &code).and_then(|blip| window_px_of(world, blip));
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

    let target = resolve_blip(world).expect("the aim beat held on this blip");
    assert!(
        !pointer_reached(world, PointerId::Mouse, target),
        "the blip must be reachable only THROUGH the image - a window mouse hit \
         means it was never behind the image camera"
    );
    info!("headless crt: the forwarded pointer reached the blip");
    nova_probe::probe_marker(
        world,
        "outcome: the forwarded pointer reaches the blip",
        serde_json::json!({}),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the window mouse cannot reach behind the glass",
        serde_json::json!({}),
    );
}

/// The verdict, off the SHIP: the autopilot the click engaged must aim at the
/// contact whose blip was clicked, not merely at something.
#[cfg(feature = "debug")]
fn assert_the_goto_targets_the_clicked_contact(world: &mut World) {
    let ship = player_ship(world).expect("one player ship root");
    let action = world
        .get::<Autopilot>(ship)
        .map(|pilot| pilot.action)
        .expect("the previous beat held until the ship took an autopilot action");
    let contact = world.resource::<GlassTarget>().contact;
    let code = world.resource::<GlassTarget>().code.clone();
    assert_eq!(
        action,
        AutopilotAction::Goto { target: contact },
        "GOTO must aim at the contact whose blip was clicked ({code})"
    );
    info!("headless crt: PASS clicked {code} through the glass, GOTO engaged");
    nova_probe::probe_marker(
        world,
        "outcome: the clicked blip engages GOTO on its contact",
        serde_json::json!({ "code": code }),
    );
}
