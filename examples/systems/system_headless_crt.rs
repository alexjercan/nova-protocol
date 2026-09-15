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
    nova_os::prelude::{NovaOsTerminal, TerminalMode},
    prelude::{nova_os_pointer_id, nova_os_window_px_showing, MapContactCode},
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
    let mut app = editor_app(false, Some(StartupScenario::Id("tutorial".to_string())));

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
            .until(nova_os_raster_open())
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
            // The map opens framed tight on the player, and the whole belt sits
            // outside even the max wheel zoom - a run against the real scenario
            // found that, not the trace. So the walk does what the map DESIGN
            // says: cycle the selection ring onto a contact with the registry's
            // own `novaos_next` (the packet lane, key resolved from the table,
            // works on a hidden blip), then re-frame the camera on it.
            .step("headless crt: novaos_next cycles the ring onto a contact")
            .each(|world: &mut World, _, frame| pulse_action(world, "novaos_next", frame))
            .until(a_blip_is_ringed())
            .diagnose(plotted_codes)
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
            // The click target is a contact the CRT is SHOWING that the ring is
            // not already on - the click needs a ring to move, or the selection
            // beat proves nothing.
            //
            // Off the picture rather than off the contact list, because
            // selecting a contact snaps the map's focus onto it
            // (`map_focus_follow`): cycling the ring away from a framed contact
            // carries the camera off with it, and the belt is wider than the
            // wheel's whole zoom range, so the contact just left behind is
            // gone for good. What the glass shows NOW is what a pointer can
            // reach. A frame showing no second blip takes a wheel notch out as
            // a net.
            .step("headless crt: the reframe put a second blip on the picture")
            .each(pick_the_target)
            .until(the_target_is_picked())
            .diagnose(shown_codes)
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

/// How far a blip may move between two frames and still count as a picture
/// that has STOPPED, in window px.
///
/// Sub-pixel: the reframe eases per frame, so a picture still sliding moves a
/// blip by whole pixels a frame and a settled one by nothing at all.
#[cfg(feature = "debug")]
const STILL_PX: f32 = 0.5;

/// Frames the picture must hold still before a pick reads it.
///
/// More than one, because an input the picture has not answered yet leaves it
/// looking still: a wheel notch reaches the map a frame or two after the frame
/// that took it, and a single quiet frame in between would let the pick fire
/// into the calm before the slide.
#[cfg(feature = "debug")]
const STILL_FRAMES: u32 = 8;

/// Where the picture had every plotted blip on the frame before, sorted by
/// code the way [`plotted_contacts`] sorts, and how many frames running it has
/// held them there. Lets the pick tell a layout that has settled from one the
/// reframe is still sliding.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct GlassLayout {
    at: Vec<(String, Vec2)>,
    still_frames: u32,
}

/// The blip the chosen target's code currently labels, freshly resolved.
#[cfg(feature = "debug")]
fn resolve_blip(world: &World) -> Option<Entity> {
    blip_labelled(world, &world.get_resource::<GlassTarget>()?.code)
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

/// Whether the picture has held the same layout for [`STILL_FRAMES`] frames,
/// remembering this frame's for the next call.
///
/// The reframe eases the map into place over several FRAMES, and a pick made
/// part way through that slide names whichever blip was loneliest in a layout
/// that no longer exists a frame later. A run with a full-rate frame clock
/// finishes the slide inside one beat and never sees it; CI runs the same beat
/// on far fewer frames, and there the pick read `AST-64` at window px the
/// settled picture puts 600 px away - among neighbours, one of whose pills is
/// what the pointer then found instead (`AST-71`, run 34959859343).
///
/// Frame to frame, not against the layout the run opened on: the map follows
/// what it is showing, so a picture that has stopped easing can still drift by
/// a fraction of a pixel a frame, and a drift that never ends is not what this
/// waits out.
#[cfg(feature = "debug")]
fn picture_has_stopped(world: &mut World, shown: &[(Entity, String, Vec2)]) -> bool {
    let at: Vec<(String, Vec2)> = shown
        .iter()
        .map(|(_, code, at)| (code.clone(), *at))
        .collect();
    let before = world.remove_resource::<GlassLayout>().unwrap_or_default();
    let held = !at.is_empty()
        && at.len() == before.at.len()
        && at
            .iter()
            .zip(&before.at)
            .all(|((code, now), (was_code, was))| {
                code == was_code && was.distance(*now) <= STILL_PX
            });
    let still_frames = if held { before.still_frames + 1 } else { 0 };
    world.insert_resource(GlassLayout { at, still_frames });
    still_frames >= STILL_FRAMES
}

/// Record the LONELIEST contact a STOPPED picture is showing that the ring is
/// not on as this run's target, and take a wheel notch out on a frame that
/// shows no such blip.
///
/// Loneliest, not first, because a click resolves to the TOPMOST node under the
/// pointer and every blip wears a label pill several times its own width. In a
/// belt of seventy contacts the first plotted one is routinely under a
/// neighbour's pill, and the aim beat then stalls with the pointer sitting on a
/// contact that is not the target. Taking the blip with the most window px
/// between it and its nearest neighbour makes the pick a property of the
/// picture rather than of the cycling order.
///
/// Stopped, because that property is only worth having if the picture the pick
/// reads is the picture the aim will use - see [`picture_has_stopped`]. The
/// wheel notch waits on the same condition: a net thrown at a sliding picture
/// would keep it sliding.
///
/// The beat holds until the pick lands, so this runs every frame of it: the
/// choice is made ONCE and the later frames cost nothing, which also keeps the
/// target from moving under a run whose map plots a new contact mid-beat.
#[cfg(feature = "debug")]
fn pick_the_target(world: &mut World, _elapsed: f32, frame: u32) {
    if world.get_resource::<GlassTarget>().is_some() {
        return;
    }
    let shown: Vec<(Entity, String, Vec2)> = plotted_contacts(world)
        .into_iter()
        .filter_map(|(contact, code)| {
            let at = blip_labelled(world, &code).and_then(|blip| window_px_of(world, blip))?;
            Some((contact, code, at))
        })
        .collect();
    let stopped = picture_has_stopped(world, &shown);
    let ringed = ringed_code(world);
    let loneliest = shown
        .iter()
        .filter(|(_, code, _)| ringed.as_deref() != Some(code.as_str()))
        .max_by(|(_, _, a), (_, _, b)| {
            let room = |at: &Vec2| {
                shown
                    .iter()
                    .filter(|(_, _, other)| other != at)
                    .map(|(_, _, other)| other.distance(*at))
                    .fold(f32::INFINITY, f32::min)
            };
            room(a).total_cmp(&room(b))
        });
    match loneliest.filter(|_| stopped) {
        Some((contact, code, at)) => {
            info!("headless crt: the target is {code} at {at:?}, with the ring on {ringed:?}");
            world.insert_resource(GlassTarget {
                contact: *contact,
                code: code.clone(),
            });
        }
        None if stopped && frame % 8 == 1 => scroll_lines(-2.0)(world),
        None => {}
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

/// Advance once a contact has been chosen and its blip located.
#[cfg(feature = "debug")]
fn the_target_is_picked() -> Gate {
    Arc::new(|world: &World| world.get_resource::<GlassTarget>().is_some())
}

/// Whether the target's blip is wearing the selection ring (an opaque amber
/// outline on an otherwise quiet blip). Written by `project_map_blips` every
/// frame, hidden or not - see [`ringed_code_list`] for why the read is an
/// `Outline` and not a border.
#[cfg(feature = "debug")]
fn the_target_is_selected() -> Gate {
    Arc::new(|world: &World| {
        resolve_blip(world)
            .and_then(|blip| world.get::<Outline>(blip))
            .is_some_and(|ring| ring.color.alpha() >= 1.0)
    })
}

/// Advance once SOME blip is wearing the ring - `novaos_next`'s ack, which the
/// walk needs before it can pick a target the ring is NOT on.
#[cfg(feature = "debug")]
fn a_blip_is_ringed() -> Gate {
    Arc::new(|world: &World| ringed_code(world).is_some())
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

/// Every plotted code whose blip wears the selection ring. One of them, unless
/// the map has lost track of its own selection.
///
/// The ring is an `Outline`, not a border - the map draws it outside the box so
/// the label beside it still starts at the target's own edge - and the read is
/// its ALPHA: the map lights the selection opaque and leaves every other
/// contact's ring quiet. Reading a `BorderColor` here found nothing at all and
/// reported an empty map for thirty seconds.
#[cfg(feature = "debug")]
fn ringed_code_list(world: &World) -> Vec<String> {
    plotted_contacts(world)
        .into_iter()
        .filter(|(_, code)| {
            blip_labelled(world, code)
                .and_then(|blip| world.get::<Outline>(blip))
                .is_some_and(|ring| ring.color.alpha() >= 1.0)
        })
        .map(|(_, code)| code)
        .collect()
}

/// The code the selection ring is on, if the map has a selection it can plot.
#[cfg(feature = "debug")]
fn ringed_code(world: &World) -> Option<String> {
    ringed_code_list(world).into_iter().next()
}

/// Which codes are wearing the selection ring, so a ring beat that stalls says
/// where the ring actually is.
#[cfg(feature = "debug")]
fn ringed_codes(world: &World) -> String {
    format!("the ring is on {:?}", ringed_code_list(world))
}

/// Which plotted codes the CRT is actually showing, so a beat with nothing to
/// click says whether the picture is empty or the ring is simply on all of it.
#[cfg(feature = "debug")]
fn shown_codes(world: &World) -> String {
    let shown: Vec<String> = plotted_contacts(world)
        .into_iter()
        .filter(|(_, code)| {
            blip_labelled(world, code)
                .and_then(|blip| window_px_of(world, blip))
                .is_some()
        })
        .map(|(_, code)| code)
        .collect();
    format!("the CRT is showing {shown:?} and {}", ringed_codes(world))
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
    format!(
        "{detail}; aimed at {:?}; the CRT pointer is on {}; SELF visible {own_ship:?}; cameras \
         {cameras:?}",
        target_window_px(world),
        hovered_through_the_glass(world),
    )
}

/// What the forwarded CRT pointer is actually over, named the way the picture
/// names it: the label a hit sits under, or the bare entity when it sits under
/// none.
///
/// The aim beat can fail two ways that look identical from the target's side -
/// the pointer landed on nothing, or it landed on the wrong contact - and the
/// warp round-trip is exactly the kind of thing that puts it one blip over.
#[cfg(feature = "debug")]
fn hovered_through_the_glass(world: &World) -> String {
    let Some(hits) = world.resource::<HoverMap>().get(&nova_os_pointer_id()) else {
        return "nothing (the pointer is not on the glass)".to_string();
    };
    let named: Vec<String> = hits
        .keys()
        .map(|hit| {
            std::iter::successors(Some(*hit), |entity| {
                world.get::<ChildOf>(*entity).map(|child| child.parent())
            })
            .find_map(|entity| {
                let mut labels = world.try_query::<&Text>()?;
                world
                    .get::<Children>(entity)
                    .into_iter()
                    .flat_map(Children::iter)
                    .find_map(|child| labels.get(world, child).ok().map(|text| text.0.clone()))
            })
            .unwrap_or_else(|| format!("{hit}"))
        })
        .collect();
    format!("{named:?}")
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

    assert!(
        !the_target_is_selected()(world),
        "the click has to have a ring to MOVE onto the target, or the selection \
         beat after it proves nothing"
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
