//! system_open_world_identity: what a generated ship tells the player about
//! itself, read off the screen the player reads it on.
//!
//! Boots the shipped app (via [`editor_app`]), clicks New Game, enters
//! [`IDENTITY_SEED`], clicks Create and waits for the live window. It then takes
//! the nearest intact generated ship and the nearest derelict, each told apart
//! by its `DerelictShipMarker` and named by its `EntityId`, and reads each one
//! twice: travel-locked in the HUD target inset, and picked by a click on its
//! blip in the Map pane.
//!
//! ONE SUBJECT: the inspection identity a generated ship spawns with. The
//! descriptive `Name` (civilization and role, or civilization and former role
//! for a derelict) reaches the inset caption and the selected contact panel.
//! A distant blip shows only the minted contact code. Every frame the Map pane
//! is up, the range asserts that no blip text carries a generated ship's name.
//! The session lifecycle is `system_open_world`'s range.
//!
//! The seed is pinned because it puts one intact ship and one derelict about
//! 10 km from the spawn, inside the sensor range and the first live window, so
//! no beat has to fly the ship.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_open_world_identity --features debug
//! # look for: `identity: intact <id> '<name>'`, `identity: derelict <id> '<name>'`,
//! #           `identity: PASS no blip carried a ship name over <n> map frames, ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1` to write the four `identity_*` shots.

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::{map::MapContactCode, pane::InterfacePaneType};
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use nova_world::prelude::SectorRoot;

#[derive(Parser)]
#[command(name = "system_open_world_identity")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, then one intact generated ship and one derelict read in the HUD target inset and the Map pane. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<IdentityWatch>();
        app.add_systems(Last, watch_blip_labels);
        app.add_plugins(identity_script());
    }

    app.run()
}

/// The seed the range plays: an intact ship 9969 m and a derelict 9092 m from
/// the spawn, both in the origin cell.
#[cfg(feature = "debug")]
const IDENTITY_SEED: u32 = 115;

#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";

/// How many cells the open world keeps live: radius 2 is a 5x5x5 window.
#[cfg(feature = "debug")]
const LIVE_SECTORS: usize = 125;

/// Seconds a load or a stream gets on a software-rendered GPU. Under the
/// harness completion deadline, so a stall names its beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

/// Frames a shown panel gets before its shot, so the text and the inset
/// texture are drawn.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 6;

/// One generated ship the range reads.
#[cfg(feature = "debug")]
#[derive(Debug, Clone)]
struct Subject {
    entity: Entity,
    id: String,
    name: String,
}

/// The two subjects, and what the per-frame blip watch saw.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct IdentityWatch {
    intact: Option<Subject>,
    derelict: Option<Subject>,
    /// Frames that ended with the Map pane up and at least one blip text
    /// checked.
    map_frames: u32,
    /// Of those, the frames that checked the intact and the derelict
    /// subject's own code label, so the watch cannot pass on other blips alone.
    subject_frames: [u32; 2],
}

#[cfg(feature = "debug")]
impl IdentityWatch {
    fn subject(&self, derelict: bool) -> &Subject {
        let subject = if derelict {
            &self.derelict
        } else {
            &self.intact
        };
        subject
            .as_ref()
            .expect("identity: the subjects are picked before they are read")
    }
}

#[cfg(feature = "debug")]
fn state_label(derelict: bool) -> &'static str {
    if derelict {
        "derelict"
    } else {
        "intact"
    }
}

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// Every live generated ship: its entity, `EntityId`, `Name` and derelict
/// marker. A generated ship is the one kind of body that spawns with a
/// `ShipStyle` and an `EntityId`; the player ship is left out.
#[cfg(feature = "debug")]
fn generated_ships(world: &mut World) -> Vec<(Entity, String, String, bool)> {
    world
        .query_filtered::<(Entity, &EntityId, &Name, Has<DerelictShipMarker>), (
            With<ShipStyle>,
            Without<PlayerSpaceshipMarker>,
        )>()
        .iter(world)
        .map(|(entity, id, name, derelict)| (entity, id.0.clone(), name.to_string(), derelict))
        .collect()
}

/// The activatable `ui_widgets::Button` above `entity`, the kind a map blip
/// is. A blip's code label is a text two levels under it.
#[cfg(feature = "debug")]
fn button_above(world: &World, entity: Entity) -> Option<Entity> {
    let mut at = entity;
    while let Some(parent) = world.get::<ChildOf>(at) {
        at = parent.parent();
        if world.get::<bevy::ui_widgets::Button>(at).is_some() {
            return Some(at);
        }
    }
    None
}

#[cfg(feature = "debug")]
fn under_a_button(world: &World, entity: Entity) -> bool {
    button_above(world, entity).is_some()
}

/// The shown code label reading `code` and the blip it sits on.
#[cfg(feature = "debug")]
fn blip_label_of(world: &World, code: &str) -> Option<(Entity, Entity)> {
    let mut labels = world.try_query::<(Entity, &Text, &InheritedVisibility)>()?;
    labels
        .iter(world)
        .filter(|(_, text, shown)| text.0 == code && shown.get())
        .find_map(|(label, _, _)| Some((label, button_above(world, label)?)))
}

/// The shown blip whose label reads `code`.
#[cfg(feature = "debug")]
fn blip_of(world: &World, code: &str) -> Option<Entity> {
    blip_label_of(world, code).map(|(_, blip)| blip)
}

/// The logical-pixel rect of the laid-out node `entity`.
#[cfg(feature = "debug")]
fn node_rect(world: &World, entity: Entity) -> Option<Rect> {
    let transform = world.get::<UiGlobalTransform>(entity)?;
    let computed = world.get::<ComputedNode>(entity)?;
    let scale = computed.inverse_scale_factor();
    let rect = Rect::from_center_size(transform.translation * scale, computed.size() * scale);
    (rect.width() > 0.0 && rect.height() > 0.0).then_some(rect)
}

/// Points on the subject's blip a player could click, with its code: the
/// centre and four inner corners of the tile, then of the code label, which is
/// the same hit target. Another contact's blip can lie over part of either.
#[cfg(feature = "debug")]
fn blip_aim_points(world: &World, derelict: bool) -> Option<(String, Vec<Vec2>)> {
    let entity = world.resource::<IdentityWatch>().subject(derelict).entity;
    let code = code_of(world, entity)?;
    let (text, blip) = blip_label_of(world, &code)?;
    let label = world.get::<ChildOf>(text)?.parent();
    let fractions = [
        (0.5, 0.5),
        (0.25, 0.25),
        (0.75, 0.25),
        (0.25, 0.75),
        (0.75, 0.75),
    ];
    let points = [node_rect(world, blip)?, node_rect(world, label)?]
        .into_iter()
        .flat_map(|rect| {
            fractions
                .into_iter()
                .map(move |(x, y)| rect.min + rect.size() * Vec2::new(x, y))
        })
        .collect();
    Some((code, points))
}

/// The minted map code of `entity`, once the map has minted one.
#[cfg(feature = "debug")]
fn code_of(world: &World, entity: Entity) -> Option<String> {
    world
        .get::<MapContactCode>(entity)
        .map(|code| code.0.clone())
}

/// Every frame the Map pane is up: no text under a button carries the name of
/// a live generated ship.
#[cfg(feature = "debug")]
fn watch_blip_labels(world: &mut World) {
    let map_up = world
        .get_resource::<State<PauseStates>>()
        .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        && world
            .get_resource::<InterfacePaneType>()
            .is_some_and(|pane| *pane == InterfacePaneType::Map);
    if !map_up {
        return;
    }
    let names: Vec<String> = generated_ships(world)
        .into_iter()
        .map(|(_, _, name, _)| name.to_uppercase())
        .collect();
    let texts: Vec<(Entity, String)> = world
        .query::<(Entity, &Text)>()
        .iter(world)
        .map(|(entity, text)| (entity, text.0.to_uppercase()))
        .collect();
    let watch = world.resource::<IdentityWatch>();
    let subject_codes = [&watch.intact, &watch.derelict].map(|subject| {
        subject
            .as_ref()
            .and_then(|subject| code_of(world, subject.entity))
    });
    let mut checked = 0;
    let mut seen = [false; 2];
    for (entity, text) in texts {
        if !under_a_button(world, entity) {
            continue;
        }
        if let Some(name) = names.iter().find(|name| text.contains(name.as_str())) {
            panic!("identity: a map button reads '{text}', the name of ship '{name}'");
        }
        checked += 1;
        for (seen, code) in seen.iter_mut().zip(&subject_codes) {
            *seen |= code.as_deref() == Some(text.as_str());
        }
    }
    let mut watch = world.resource_mut::<IdentityWatch>();
    if checked > 0 {
        watch.map_frames += 1;
    }
    for (frames, seen) in watch.subject_frames.iter_mut().zip(seen) {
        *frames += u32::from(seen);
    }
}

/// Pick the nearest intact generated ship and the nearest derelict.
#[cfg(feature = "debug")]
fn pick_subjects(world: &mut World) {
    let player = the_player(world).expect("identity: exactly one player ship");
    let origin = world
        .get::<GlobalTransform>(player)
        .expect("identity: the player ship has a transform")
        .translation();
    let ships = generated_ships(world);
    let nearest = |derelict: bool| {
        ships
            .iter()
            .filter(|ship| ship.3 == derelict)
            .map(|ship| {
                let at = world
                    .get::<GlobalTransform>(ship.0)
                    .expect("identity: a live ship has a transform")
                    .translation();
                (at.distance(origin), ship)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1 .1.cmp(&b.1 .1)))
            .map(|(_, ship)| Subject {
                entity: ship.0,
                id: ship.1.clone(),
                name: ship.2.clone(),
            })
    };
    let intact = nearest(false).unwrap_or_else(|| {
        panic!(
            "identity: seed {IDENTITY_SEED} streamed {} generated ships and none is intact",
            ships.len()
        )
    });
    let derelict = nearest(true).unwrap_or_else(|| {
        panic!(
            "identity: seed {IDENTITY_SEED} streamed {} generated ships and none is derelict",
            ships.len()
        )
    });
    assert!(
        !intact.name.contains("derelict"),
        "identity: intact ship {} is named '{}'",
        intact.id,
        intact.name
    );
    assert!(
        derelict.name.contains(" derelict, former "),
        "identity: derelict {} is named '{}'",
        derelict.id,
        derelict.name
    );
    for (label, subject) in [("intact", &intact), ("derelict", &derelict)] {
        info!(
            "identity: {label} {} '{}' entity {:?}",
            subject.id, subject.name, subject.entity
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: the window streams one intact generated ship and one derelict",
        serde_json::json!({
            "intact": { "id": intact.id, "name": intact.name },
            "derelict": { "id": derelict.id, "name": derelict.name },
            "ships": ships.len(),
        }),
    );
    let mut watch = world.resource_mut::<IdentityWatch>();
    watch.intact = Some(intact);
    watch.derelict = Some(derelict);
}

/// Travel-lock the subject, the way a sensor dwell does.
#[cfg(feature = "debug")]
fn lock_on(derelict: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let target = world.resource::<IdentityWatch>().subject(derelict).entity;
        let player = the_player(world).expect("identity: exactly one player ship");
        world
            .get_mut::<TravelLock>(player)
            .expect("identity: the player ship carries a travel lock")
            .0 = Some(target);
    }
}

/// Advance once the inset caption's first line is the subject's name.
#[cfg(feature = "debug")]
fn the_inset_names(
    derelict: bool,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let name = &world.resource::<IdentityWatch>().subject(derelict).name;
        world
            .try_query_filtered::<&Text, With<TargetInsetCaptionMarker>>()
            .is_some_and(|mut captions| {
                captions
                    .iter(world)
                    .any(|caption| caption.0.lines().next() == Some(name.as_str()))
            })
    })
}

/// Advance once the subject's minted code is on a laid-out blip.
#[cfg(feature = "debug")]
fn the_blip_is_up(derelict: bool) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let entity = world.resource::<IdentityWatch>().subject(derelict).entity;
        code_of(world, entity).is_some_and(|code| {
            world
                .try_query::<(Entity, &Text, &InheritedVisibility, &ComputedNode)>()
                .is_some_and(|mut texts| {
                    texts.iter(world).any(|(label, text, shown, computed)| {
                        text.0 == code
                            && shown.get()
                            && computed.size().x > 0.0
                            && under_a_button(world, label)
                    })
                })
        })
    })
}

/// Why [`the_blip_is_up`] still waits: no minted code, or each text that reads
/// the code with its visibility, size and button ancestry.
#[cfg(feature = "debug")]
fn blip_diagnosis(derelict: bool) -> impl Fn(&World) -> String + Send + Sync + 'static {
    move |world: &World| {
        let subject = world.resource::<IdentityWatch>().subject(derelict);
        let Some(code) = code_of(world, subject.entity) else {
            return format!("{} has no MapContactCode", subject.id);
        };
        let Some(mut texts) =
            world.try_query::<(Entity, &Text, &InheritedVisibility, &ComputedNode)>()
        else {
            return "no laid-out text is registered".to_string();
        };
        let reads: Vec<String> = texts
            .iter(world)
            .filter(|(_, text, _, _)| text.0 == code)
            .map(|(label, _, shown, computed)| {
                format!(
                    "{label:?} shown {} size {} under a button {}",
                    shown.get(),
                    computed.size(),
                    under_a_button(world, label)
                )
            })
            .collect();
        format!("{} code {code}: texts [{}]", subject.id, reads.join("; "))
    }
}

/// Wheel notches a missed pass turns.
#[cfg(feature = "debug")]
const ZOOM_NOTCHES: f32 = 3.0;

/// Move the pointer over the subject's blip, one aim point every other frame
/// so picking can answer for the last one. A pass that finds another blip on
/// top at every point ends the way a player works through crowded contacts:
/// click the tile, which selects the contact on top, and turn the wheel in
/// before the next pass.
#[cfg(feature = "debug")]
fn sweep_the_blip(derelict: bool) -> impl Fn(&mut World, f32, u32) + Send + Sync + 'static {
    move |world: &mut World, _, frame: u32| {
        let Some((code, points)) = blip_aim_points(world, derelict) else {
            return;
        };
        let sweep = 2 * points.len();
        let at = (frame as usize - 1) % (sweep + 6);
        match at.checked_sub(sweep) {
            None if at.is_multiple_of(2) => {
                debug!(
                    "identity: aim {code} point {} at {:?}",
                    at / 2,
                    points[at / 2]
                );
                move_cursor(points[at / 2])(world);
            }
            None => {}
            Some(0) => {
                info!("identity: another blip covers {code}; click it and zoom in");
                move_cursor(points[0])(world);
            }
            Some(1) => press_mouse(MouseButton::Left)(world),
            Some(2) => release_mouse(MouseButton::Left)(world),
            Some(3) => scroll_lines(ZOOM_NOTCHES)(world),
            Some(_) => {}
        }
    }
}

/// Advance once the pick map puts the mouse pointer over the subject's blip.
#[cfg(feature = "debug")]
fn the_pointer_is_on_the_blip(
    derelict: bool,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use bevy::picking::{hover::HoverMap, pointer::PointerId};
    std::sync::Arc::new(move |world: &World| {
        let entity = world.resource::<IdentityWatch>().subject(derelict).entity;
        let Some(blip) = code_of(world, entity).and_then(|code| blip_of(world, &code)) else {
            return false;
        };
        world
            .get_resource::<HoverMap>()
            .and_then(|hovers| hovers.get(&PointerId::Mouse))
            .is_some_and(|hits| {
                hits.keys()
                    .any(|hit| *hit == blip || button_above(world, *hit) == Some(blip))
            })
    })
}

/// What the mouse pointer picks instead of the subject's blip: each hit with
/// its name, its text and the button above it.
#[cfg(feature = "debug")]
fn hover_diagnosis(derelict: bool) -> impl Fn(&World) -> String + Send + Sync + 'static {
    use bevy::picking::{hover::HoverMap, pointer::PointerId};
    move |world: &World| {
        let entity = world.resource::<IdentityWatch>().subject(derelict).entity;
        let blip = code_of(world, entity).and_then(|code| blip_of(world, &code));
        let cursor = world
            .try_query_filtered::<&Window, With<bevy::window::PrimaryWindow>>()
            .and_then(|mut windows| windows.iter(world).next().and_then(Window::cursor_position));
        let hits: Vec<String> = world
            .get_resource::<HoverMap>()
            .and_then(|hovers| hovers.get(&PointerId::Mouse))
            .map(|hits| {
                hits.keys()
                    .map(|hit| {
                        let mut ancestry = Vec::new();
                        let mut at = *hit;
                        while let Some(parent) = world.get::<ChildOf>(at) {
                            at = parent.parent();
                            ancestry.push(format!(
                                "{at:?} {:?}",
                                world.get::<Name>(at).map(Name::as_str)
                            ));
                        }
                        format!(
                            "{hit:?} name {:?} text {:?} button {:?} under [{}]",
                            world.get::<Name>(*hit).map(Name::as_str),
                            world.get::<Text>(*hit).map(|text| text.0.as_str()),
                            world
                                .get::<bevy::ui_widgets::Button>(*hit)
                                .map(|_| *hit)
                                .or_else(|| button_above(world, *hit)),
                            ancestry.join(" < ")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        format!("blip {blip:?} cursor {cursor:?} hits [{}]", hits.join("; "))
    }
}

/// Advance once the contact panel names the subject, off a text that is not
/// under a button.
#[cfg(feature = "debug")]
fn the_panel_names(
    derelict: bool,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let name = &world.resource::<IdentityWatch>().subject(derelict).name;
        world
            .try_query::<(Entity, &Text)>()
            .is_some_and(|mut texts| {
                texts
                    .iter(world)
                    .any(|(entity, text)| &text.0 == name && !under_a_button(world, entity))
            })
    })
}

/// Report one subject read on the map: its code on the blip and its name on
/// the panel.
#[cfg(feature = "debug")]
fn report_map_read(derelict: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let subject = world.resource::<IdentityWatch>().subject(derelict).clone();
        let code = code_of(world, subject.entity).expect("identity: the blip beat saw the code");
        assert!(
            world.get::<DerelictShipMarker>(subject.entity).is_some() == derelict,
            "identity: {} lost its derelict state",
            subject.id
        );
        info!(
            "identity: map {} {} blip {code} panel '{}'",
            state_label(derelict),
            subject.id,
            subject.name
        );
        nova_probe::probe_marker(
            world,
            "outcome: a picked ship shows its name in the panel and only its code on the blip",
            serde_json::json!({ "id": subject.id, "name": subject.name, "code": code }),
        );
    }
}

/// The caption shot of the subject in the HUD target inset.
#[cfg(feature = "debug")]
fn inset_shot(derelict: bool) -> String {
    format!("identity_{}_inset.png", state_label(derelict))
}

/// The Map pane shot with the subject picked.
#[cfg(feature = "debug")]
fn map_shot(derelict: bool) -> String {
    format!("identity_{}_map.png", state_label(derelict))
}

/// Advance once the interface is open on the Map pane.
#[cfg(feature = "debug")]
fn the_map_shows() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|pane| *pane == InterfacePaneType::Map)
    })
}

/// The walk: into the world, then each subject in the inset, then each on the
/// map.
#[cfg(feature = "debug")]
fn identity_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("identity: reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "identity: click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("identity: enter the seed")
        .on_enter(|world: &mut World| {
            let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
            let (_, mut value) = fields
                .iter_mut(world)
                .find(|(name, _)| name.as_str() == SEED_FIELD)
                .expect("identity: the modal has a seed field");
            value.0 = IDENTITY_SEED.to_string();
        })
        .until(frames(2))
        .add()
        .click_named(
            "identity: click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("identity: the window streams around the player")
        .until(std::sync::Arc::new(|world: &World| {
            the_player(world).is_some()
                && world
                    .try_query_filtered::<(), With<SectorRoot>>()
                    .is_some_and(|mut roots| roots.iter(world).count() == LIVE_SECTORS)
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("identity: pick the subjects")
        .on_enter(|world: &mut World| {
            let seed = world.resource::<OpenWorldSession>().seed;
            assert_eq!(
                seed, IDENTITY_SEED,
                "identity: Create must start the typed seed"
            );
            pick_subjects(world);
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .add();

    for derelict in [false, true] {
        let label = state_label(derelict);
        script = script
            .step(format!("identity: travel-lock the {label} ship"))
            .on_enter(lock_on(derelict))
            .until(the_inset_names(derelict))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("identity: settle the {label} inset"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("identity: shoot the {label} inset"))
            .on_enter(move |world: &mut World| {
                assert!(
                    the_inset_names(derelict)(world),
                    "identity: the {label} inset lost its caption"
                );
                shoot(world, &inset_shot(derelict));
            })
            .until(shot_written(inset_shot(derelict)))
            .deadline(BEAT_DEADLINE_SECS)
            .add();
    }

    script = script
        .step("identity: open the interface")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("identity: let the key up and wait for the map")
        .on_enter(release_action("interface_toggle"))
        .until(the_map_shows())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("identity: settle the map")
        .until(frames(SETTLE_FRAMES))
        .add();

    for derelict in [false, true] {
        let label = state_label(derelict);
        script = script
            .step(format!("identity: the {label} blip shows its code"))
            .until(the_blip_is_up(derelict))
            .diagnose(blip_diagnosis(derelict))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("identity: aim at the {label} blip"))
            .each(sweep_the_blip(derelict))
            .until(the_pointer_is_on_the_blip(derelict))
            .diagnose(hover_diagnosis(derelict))
            .deadline(SESSION_SECS)
            .add()
            .step(format!("identity: press on the {label} blip"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("identity: release on the {label} blip"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(the_panel_names(derelict))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("identity: settle the {label} panel"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("identity: shoot the {label} map"))
            .on_enter(move |world: &mut World| {
                report_map_read(derelict)(world);
                shoot(world, &map_shot(derelict));
            })
            .until(shot_written(map_shot(derelict)))
            .deadline(BEAT_DEADLINE_SECS)
            .add();
    }

    script
        .step("identity: report the blip watch")
        .on_enter(|world: &mut World| {
            let watch = world.resource::<IdentityWatch>();
            let (frames, [intact, derelict]) = (watch.map_frames, watch.subject_frames);
            assert!(frames > 0, "identity: the blip watch never saw the map");
            assert!(
                intact > 0 && derelict > 0,
                "identity: the blip watch checked the intact code on {intact} frames and the \
                 derelict code on {derelict}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: no map blip carries a generated ship's name",
                serde_json::json!({
                    "map_frames": frames,
                    "intact_blip_frames": intact,
                    "derelict_blip_frames": derelict,
                }),
            );
            info!(
                "identity: PASS no blip carried a ship name over {frames} map frames, \
                 {intact} with the intact code and {derelict} with the derelict code"
            );
        })
        .add()
}
