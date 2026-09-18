//! system_wreck_lock: what a combat lock and a component pin do while the hull
//! under them comes apart, and what it takes to shoot the piece that fell off.
//!
//! A lock is a radio link to a BODY and a pin is a choice of one SECTION of it,
//! so severing is the one event that can move either. The three phases below
//! walk the two of them apart:
//!
//! 1. A section somewhere else falls off. Neither the lock nor the pin has any
//!    business noticing, and both must still be exactly where they were.
//! 2. The PINNED section falls off. The pin is the only thing that may move -
//!    the hull it was a pin on is still there, still locked, and a pilot who
//!    lost a component selection has not lost the target.
//! 3. The piece that fell off is a body of its own now. It is unsigned
//!    wreckage, so it returns nothing to a scanner and is lockable at
//!    point-blank only - and NOTHING acquires it for the pilot. The range
//!    clears the lock, watches the slot stay empty, then takes the fragment
//!    deliberately and checks that the reticle, the target inset and the gun
//!    all followed the new choice.
//!
//! The gun is the same gun throughout. A severed fragment is an obstacle for
//! everyone, with no owner immunity and no grace window, so the round that
//! lands on it is the ordinary consequence of aiming at it.
//!
//! # Every body in this scene is parked
//!
//! A sever hands each half a real separation kick - that is
//! `system_section_severing`'s claim and not this range's - and unsigned
//! wreckage is lockable inside 50 m, so a drifting fragment leaves the only
//! range it can be held at within seconds of being made. The range zeroes the
//! velocities of the hulls and the wreckage every frame instead, which makes
//! the geometry a constant and every phase below a claim about the LOCK chain.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_wreck_lock --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `wreck lock: an unrelated sever left the lock and the pin alone`,
//! #           `wreck lock: the fragment took ... hit points from the same gun`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../shared/dev_fixtures/mod.rs"]
mod dev_fixtures;

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_wreck_lock")]
#[command(version = "1.0.0")]
#[command(about = "A combat lock and a component pin through two severs, and the gun that follows the fragment. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario the range loads under.
const SCENARIO_ID: &str = "wreck_lock";

/// The hull the lock is taken on: the shipped salvage skiff, a real
/// multi-section hull rather than a rig built to sever nicely.
const SKIFF_ID: &str = "derelict_skiff";

/// The ship the player flies: a hull, a flight computer and one gun.
const GUNNER_ID: &str = "gunner";

/// The gunner's section slots.
const GUNNER_HULL: &str = "gunner_hull";
const GUNNER_COMPUTER: &str = "gunner_computer";
const GUNNER_TURRET: &str = "gunner_turret";

/// The skiff section the range PINS, and the one it destroys to sever it.
///
/// `plate_17` is a wingtip: it is authored at grid (2, 0, 1) with `plate_14` at
/// (1, 0, 1) its only neighbour, so destroying `plate_14` disconnects exactly
/// this one plate and nothing else. The shipped hull's own geometry decides
/// that - see `assets/base/ships/base.content.ron` - and the range asserts the
/// split it got rather than trusting the reading.
#[cfg(feature = "debug")]
const PINNED_SECTION: &str = "plate_17";
#[cfg(feature = "debug")]
const PINNED_ANCHOR: &str = "plate_14";

/// The UNRELATED cut, at the other end of the hull: `plate_0` at (0, 0, -4)
/// hangs off `plate_1` alone, seven cells from the pin and pointing away from
/// the gunner, so the fragment it makes cannot be mistaken for the pinned one
/// and cannot wander into the shot.
#[cfg(feature = "debug")]
const SPARE_ANCHOR: &str = "plate_1";
#[cfg(feature = "debug")]
const SPARE_SECTION: &str = "plate_0";

/// Where the gunner sits, in ENGINE units: four cells off the pinned wingtip
/// along +X, with clear sky between them.
///
/// Inside [`TargetingSettings::unsigned_lock_range`] (5 units, 50 m), which is
/// the whole reach a body that returns no signature of its own has. Everything
/// in the scene is parked, so this distance is a constant for the run rather
/// than the opening value of a closing geometry.
const GUNNER_AT: Vec3 = Vec3::new(6.0, 0.0, 1.0);

/// Damage dealt to a section the range wants gone. Far over any shipped
/// section's health: the range is severing on purpose, not measuring what it
/// takes.
#[cfg(feature = "debug")]
const KILLING_BLOW: f32 = 100_000.0;

/// How long the pin is refreshed for each frame the range is holding it.
///
/// The production pin is a CYCLE PRESS that expires unless it is renewed, so a
/// range that wrote one deadline far in the future would be testing a state the
/// game never holds. This is the hold, re-applied while the range is
/// "pressing", and it stops the moment production takes the section away,
/// which is what phase 2 is about.
#[cfg(feature = "debug")]
const PIN_HOLD_SECS: f32 = 1.0;

/// In-step seconds a beat gets to reach its world condition. Generous for the
/// reason `system_collision_damage` gives: the correctness pass runs on a
/// software rasterizer where a frame of a skiff's sections costs real time.
/// A backstop that names a hung beat, not a budget.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 240.0;

/// In-step seconds the scene is given to link its colliders and settle.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 3.0;

/// In-step seconds the emptied lock slot is watched before the range fills it.
///
/// Nothing in the game re-locks on its own (a fresh dwell is what takes a
/// lock), so this window is what turns "the fragment was not acquired" from an
/// instantaneous reading into an observation.
#[cfg(feature = "debug")]
const UNLOCKED_WATCH_SECS: f32 = 2.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<PinHold>();
        app.init_resource::<WreckLog>();
        app.add_systems(Update, hold_the_pin);
        app.add_plugins(nova_screenshot(wreck_script()));
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::NovaProbePlugin::default());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(Update, park_every_body);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}

/// Hold every hull and every piece of wreckage still.
///
/// A sever is momentum-neutral and hands each half a kick sized to its own
/// envelope, so the fragment this range makes leaves at metres a second - and
/// unsigned wreckage is only lockable inside 50 m. Parking the scene is what
/// keeps the phases below about the lock chain: how far a fragment travels is
/// `system_section_severing`'s claim, and re-proving it here would only mean
/// racing it.
fn park_every_body(
    mut q_bodies: Query<
        (&mut LinearVelocity, &mut AngularVelocity),
        Or<(
            With<SpaceshipRootMarker>,
            With<ShipWreckFragmentMarker>,
            With<DetachedPieceMarker>,
        )>,
    >,
) {
    for (mut linear, mut angular) in &mut q_bodies {
        if linear.0 != Vec3::ZERO {
            linear.0 = Vec3::ZERO;
        }
        if angular.0 != Vec3::ZERO {
            angular.0 = Vec3::ZERO;
        }
    }
}

/// The gunner: enough hull to carry a flight computer and the one gun the whole
/// range is fired from. A player ship, because the reticle, the target inset,
/// the component lock and the turret's target feed all read the PLAYER's
/// targeting state and nothing else's.
fn gunner(sections: &GameSections) -> SpaceshipConfig {
    SpaceshipConfig {
        allegiance: Some(Allegiance::Player),
        ..fixtures::ship(
            sections,
            SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: BTreeMap::new(),
            }),
            &[
                SectionSpec::new(GUNNER_HULL, "reinforced_hull_section", Vec3::ZERO),
                SectionSpec::new(
                    GUNNER_COMPUTER,
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                // Unlimited: the range holds the trigger down until the
                // fragment bleeds, and a dry magazine would read as a gun that
                // cannot reach it.
                SectionSpec::new(
                    GUNNER_TURRET,
                    "pdc_kinetic_turret_section",
                    Vec3::new(0.0, 0.75, 0.0),
                )
                .unlimited(),
            ],
        )
    }
}

/// The target: the salvage skiff fixture, neutral, driven by nobody.
///
/// Neutral and pilotless on purpose. A hostile with a pilot would manoeuvre and
/// shoot back, and every phase here is a claim about what the LOCK does while
/// the hull is cut - a moving target would only add ways for the geometry to
/// stop being the geometry the range set up.
fn derelict(sections: &GameSections) -> SpaceshipConfig {
    let _ = sections;
    SpaceshipConfig {
        design: ShipDesignSource::Inline(dev_fixtures::skiff()),
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Neutral),
        ..default()
    }
}

fn spaceship(
    id: &str,
    name: &str,
    position: Meters3,
    config: SpaceshipConfig,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(config),
    }
}

fn range_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![
        spaceship(
            SKIFF_ID,
            "Derelict Skiff",
            Meters3::ZERO,
            derelict(sections),
        ),
        spaceship(
            GUNNER_ID,
            "Gunner",
            Meters3::from_engine(GUNNER_AT),
            gunner(sections),
        ),
    ];

    ScenarioConfig {
        description: "A locked and pinned hull, two severs, and the gun that follows the piece \
                      that fell off."
            .to_string(),
        events: fixtures::spawn_on_start(objects),
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Wreck Lock".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

// --- The walk ----------------------------------------------------------------

/// Which section the range is "holding the cycle key on", if any.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PinHold(Option<Entity>);

/// What the run has read off the world at each phase, so an assertion reports
/// the figures that produced it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct WreckLog {
    /// The fragment the pinned section landed in.
    fragment: Option<Entity>,
    /// The pinned section's health when the trigger opened.
    health_before_fire: Option<f32>,
    /// The LOWEST reading the fragment held while the gun was firing.
    ///
    /// A low-water mark rather than a reading taken at the verdict, because the
    /// gun keeps firing through the beat that watches it and a wingtip worth a
    /// few hit points can be gone by the next frame - on a slow host it was,
    /// and the verdict found no section to read at all. What the round spent is
    /// the evidence; whether the plate survived spending it is not the claim.
    health_while_firing: Option<f32>,
}

/// Re-apply the pin for this frame, exactly while production still has the
/// section selected.
///
/// The `section` field is PRODUCTION's: `update_component_lock` filters it
/// against the sections still attached to the locked root every frame, and this
/// hold must not fight that. So the range renews only the DEADLINE, and only
/// while the selection it asked for is the selection that is there - which is
/// what a pilot holding the cycle key is, and which stops of its own accord the
/// frame the pinned section leaves the hull.
#[cfg(feature = "debug")]
fn hold_the_pin(
    time: Res<Time>,
    hold: Res<PinHold>,
    mut q_player: Query<&mut ComponentLock, With<PlayerSpaceshipMarker>>,
) {
    let Some(section) = hold.0 else {
        return;
    };
    for mut component in &mut q_player {
        if component.section != Some(section) {
            continue;
        }
        component.mode = ComponentLockMode::Pinned {
            until: time.elapsed_secs() + PIN_HOLD_SECS,
        };
    }
}

/// The scripted run: lock, pin, cut twice, then take the piece deliberately and
/// shoot it.
#[cfg(feature = "debug")]
fn wreck_script() -> Script {
    Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(ship_present(SKIFF_ID), ship_present(GUNNER_ID)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the hulls settle")
        .until(elapsed(SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The lock is written straight into the slot, which is what the radar
        // picker does with it. The dwell that follows is production's: the
        // component layer does not exist until the focus completes.
        .step("lock the derelict and hold it long enough to pick a section")
        .on_enter(lock_the_derelict)
        .until(focus_is_complete())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pin the wingtip")
        .on_enter(pin_the_wingtip)
        .until(the_pin_is_held())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Phase 1: a cut at the far end of the hull.
        .step("sever an unrelated section")
        .on_enter(|world: &mut World| destroy_section(world, SPARE_ANCHOR))
        .until(section_is_severed(SPARE_SECTION))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("assert the lock and the pin ignored it")
        .on_enter(assert_an_unrelated_sever_changes_nothing)
        .add()
        // Phase 2: the cut that takes the pinned section itself.
        .step("sever the pinned section")
        .on_enter(|world: &mut World| destroy_section(world, PINNED_ANCHOR))
        .until(section_is_severed(PINNED_SECTION))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("assert the pin cleared and the lock held")
        .on_enter(assert_the_pin_clears_and_the_lock_holds)
        .add()
        .step("assert the fragment is a close contact of its own")
        .on_enter(assert_the_fragment_is_its_own_contact)
        .add()
        // Phase 3: the slot is emptied and WATCHED. Nothing re-locks on its
        // own, and a fragment that arrived in the slot by itself would make
        // every reading after this one meaningless.
        .step("let the lock go and watch the empty slot")
        .on_enter(release_the_lock)
        .until(elapsed(UNLOCKED_WATCH_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("assert nothing acquired the fragment")
        .on_enter(assert_the_fragment_is_never_acquired_on_its_own)
        .add()
        .step("acquire the fragment deliberately")
        .on_enter(acquire_the_fragment)
        .until(the_reticle_follows_the_fragment())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("assert the reticle and the inset came with it")
        .on_enter(assert_the_hud_follows_the_fragment)
        .add()
        .step("open fire on the fragment")
        .on_enter(open_fire)
        .each(watch_the_fragment_bleed)
        .until(the_fragment_is_bleeding())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("assert the same gun reached it")
        .on_enter(cease_fire)
        .on_enter(assert_the_gun_reaches_the_fragment)
        .add()
}

// --- World access ------------------------------------------------------------

/// The scenario-scoped root carrying `id`.
#[cfg(feature = "debug")]
fn object_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        })
}

/// The live SECTION carrying `id`. Still resolves after a sever: severing
/// re-parents the section, it does not replace it.
#[cfg(feature = "debug")]
fn section_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<SectionMarker>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        })
}

/// The player ship root.
#[cfg(feature = "debug")]
fn player_root(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next())
}

/// Which body `section` currently belongs to.
#[cfg(feature = "debug")]
fn body_of(world: &World, section: Entity) -> Option<Entity> {
    world.get::<ChildOf>(section).map(|child_of| child_of.0)
}

/// Advance once a ship root carries `id`.
#[cfg(feature = "debug")]
fn ship_present(id: &'static str) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| object_by_id(world, id).is_some())
}

/// The focus dwell has completed on the derelict, so the component layer is
/// available at all.
#[cfg(feature = "debug")]
fn focus_is_complete() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(skiff) = object_by_id(world, SKIFF_ID) else {
            return false;
        };
        player_root(world)
            .and_then(|player| world.get::<LockFocus>(player))
            .is_some_and(|focus| focus.focused_on(skiff))
    })
}

/// The range's chosen section is selected and pinned.
#[cfg(feature = "debug")]
fn the_pin_is_held() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(section) = section_by_id(world, PINNED_SECTION) else {
            return false;
        };
        player_root(world)
            .and_then(|player| world.get::<ComponentLock>(player))
            .is_some_and(|component| {
                component.section == Some(section)
                    && matches!(component.mode, ComponentLockMode::Pinned { .. })
            })
    })
}

/// `id`'s section has left the hull it was built on: it now hangs off a
/// `ShipWreckFragmentMarker` body.
#[cfg(feature = "debug")]
fn section_is_severed(
    id: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        section_by_id(world, id)
            .and_then(|section| body_of(world, section))
            .is_some_and(|body| world.get::<ShipWreckFragmentMarker>(body).is_some())
    })
}

/// The combat reticle is anchored to the fragment the range acquired.
#[cfg(feature = "debug")]
fn the_reticle_follows_the_fragment(
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(fragment) = world
            .get_resource::<WreckLog>()
            .and_then(|log| log.fragment)
        else {
            return false;
        };
        reticle_anchor(world) == Some(ScreenIndicatorAnchorKind::Entity(fragment))
    })
}

/// The pinned section has lost hit points since the trigger opened.
#[cfg(feature = "debug")]
fn the_fragment_is_bleeding() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(log) = world.get_resource::<WreckLog>() else {
            return false;
        };
        match (log.health_before_fire, log.health_while_firing) {
            (Some(before), Some(lowest)) => lowest < before,
            _ => false,
        }
    })
}

/// Keep the fragment's lowest reading while the gun is firing.
///
/// Runs every frame of the trigger beat: the verdict is made against this
/// rather than against a fresh read, so a plate that is destroyed the frame
/// after it bleeds still reports what the round spent on it.
#[cfg(feature = "debug")]
fn watch_the_fragment_bleed(world: &mut World, _elapsed: f32, _frame: u32) {
    let Some(now) = section_health(world, PINNED_SECTION) else {
        return;
    };
    let mut log = world.resource_mut::<WreckLog>();
    if log.health_while_firing.is_none_or(|lowest| now < lowest) {
        log.health_while_firing = Some(now);
    }
}

/// Take the gunner's finger off the trigger.
#[cfg(feature = "debug")]
fn cease_fire(world: &mut World) {
    let turrets: Vec<Entity> = world
        .try_query_filtered::<Entity, With<TurretSectionMarker>>()
        .map(|mut query| query.iter(world).collect())
        .unwrap_or_default();
    for turret in turrets {
        if let Some(mut input) = world.entity_mut(turret).get_mut::<TurretSectionInput>() {
            **input = false;
        }
    }
}

/// What the combat reticle is anchored to right now.
#[cfg(feature = "debug")]
fn reticle_anchor(world: &World) -> Option<ScreenIndicatorAnchorKind> {
    let mut query =
        world.try_query_filtered::<&ScreenIndicatorAnchor, With<TorpedoTargetReticleMarker>>()?;
    **query.iter(world).next()?
}

/// The target inset's caption line, the field that follows the lock whatever
/// the inset's camera can or cannot frame.
#[cfg(feature = "debug")]
fn inset_caption(world: &World) -> Option<String> {
    let mut query = world.try_query_filtered::<&Text, With<TargetInsetCaptionMarker>>()?;
    query.iter(world).next().map(|text| text.0.clone())
}

/// The live hit points of the section carrying `id`.
#[cfg(feature = "debug")]
fn section_health(world: &World, id: &str) -> Option<f32> {
    let section = section_by_id(world, id)?;
    world.get::<Health>(section).map(|health| health.current)
}

// --- Steps -------------------------------------------------------------------

/// Write the combat lock, the way the radar picker commits one.
#[cfg(feature = "debug")]
fn lock_the_derelict(world: &mut World) {
    let skiff = object_by_id(world, SKIFF_ID).expect("wreck lock: the derelict must be up");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    world
        .get_mut::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0 = Some(skiff);
}

/// Select the wingtip and start holding the pin on it.
#[cfg(feature = "debug")]
fn pin_the_wingtip(world: &mut World) {
    let section =
        section_by_id(world, PINNED_SECTION).expect("wreck lock: the derelict carries the wingtip");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    let until = world.resource::<Time>().elapsed_secs() + PIN_HOLD_SECS;
    {
        let mut component = world
            .get_mut::<ComponentLock>(player)
            .expect("wreck lock: a player ship carries the component lock");
        component.section = Some(section);
        component.mode = ComponentLockMode::Pinned { until };
    }
    world.resource_mut::<PinHold>().0 = Some(section);
}

/// Take a section down, the only way anything takes a section down.
#[cfg(feature = "debug")]
fn destroy_section(world: &mut World, id: &str) {
    let section = section_by_id(world, id)
        .unwrap_or_else(|| panic!("wreck lock: the derelict must still carry '{id}'"));
    world.trigger(HealthApplyDamage {
        entity: section,
        source: None,
        amount: KILLING_BLOW,
    });
}

/// Empty the lock slot and stop holding the pin.
#[cfg(feature = "debug")]
fn release_the_lock(world: &mut World) {
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    world
        .get_mut::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0 = None;
    world.resource_mut::<PinHold>().0 = None;
}

/// Take the fragment deliberately - the acquisition the range exists to make
/// explicit.
#[cfg(feature = "debug")]
fn acquire_the_fragment(world: &mut World) {
    let fragment = world
        .resource::<WreckLog>()
        .fragment
        .expect("wreck lock: the fragment is recorded when the pinned section severs");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    world
        .get_mut::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0 = Some(fragment);
}

/// Hold the trigger down on the one gun this range has, and record what the
/// fragment was worth before the first round leaves.
#[cfg(feature = "debug")]
fn open_fire(world: &mut World) {
    let before = section_health(world, PINNED_SECTION)
        .expect("wreck lock: the severed wingtip keeps its hit points");
    world.resource_mut::<WreckLog>().health_before_fire = Some(before);

    let turrets: Vec<Entity> = {
        let mut query = world
            .try_query_filtered::<Entity, With<TurretSectionMarker>>()
            .expect("wreck lock: the gunner carries a turret");
        query.iter(world).collect()
    };
    assert!(
        !turrets.is_empty(),
        "wreck lock: the gunner must carry the one gun the whole range is fired from"
    );
    for turret in turrets {
        if let Some(mut input) = world.entity_mut(turret).get_mut::<TurretSectionInput>() {
            **input = true;
        }
    }
}

// --- Assertions --------------------------------------------------------------

/// Phase 1: a sever somewhere else is not an event the lock or the pin has any
/// business reading.
///
/// Both halves matter. A lock that let go would mean severing is a lock drop,
/// which it is not - the hull is still there and still the thing the pilot
/// chose. A pin that moved would mean the component layer re-derives its
/// selection whenever the hull's section set changes, which would take the
/// player's aim point off the part they picked every time the ship is hit.
#[cfg(feature = "debug")]
fn assert_an_unrelated_sever_changes_nothing(world: &mut World) {
    let skiff = object_by_id(world, SKIFF_ID).expect("wreck lock: the derelict must still be up");
    let pinned =
        section_by_id(world, PINNED_SECTION).expect("wreck lock: the wingtip is still a section");
    let spare = section_by_id(world, SPARE_SECTION).expect("wreck lock: the spare plate survives");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");

    let severed_body = body_of(world, spare).expect("wreck lock: a severed section has a body");
    assert!(
        severed_body != skiff,
        "wreck lock: '{SPARE_SECTION}' is still bolted to the derelict, so nothing was severed \
         and the phase proves nothing"
    );
    assert!(
        body_of(world, pinned) == Some(skiff),
        "wreck lock: the unrelated cut took the PINNED section with it; the two plates sit seven \
         cells apart on opposite ends of the hull, so this is the hull's connectivity changing \
         under the range, not the lock layer"
    );

    let lock = world
        .get::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0;
    assert!(
        lock == Some(skiff),
        "wreck lock: the combat lock read {lock:?} after an unrelated section fell off the hull \
         it is held on; a sever is not a lock drop - the target is still there"
    );
    let component = world
        .get::<ComponentLock>(player)
        .expect("wreck lock: a player ship carries the component lock")
        .clone();
    assert!(
        component.section == Some(pinned)
            && matches!(component.mode, ComponentLockMode::Pinned { .. }),
        "wreck lock: the component pin moved to {:?} ({:?}) when a section at the OTHER end of \
         the hull fell off; a pin is the pilot's choice of part and only its own section's fate \
         may move it",
        component.section,
        component.mode
    );
    info!("wreck lock: an unrelated sever left the lock and the pin alone");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: an unrelated sever leaves the lock and the pin standing",
        serde_json::json!({
            "t": elapsed,
            "severed": SPARE_SECTION,
            "pinned": PINNED_SECTION,
        }),
    );
}

/// Phase 2: the pinned section leaves, and the pin - only the pin - goes with
/// it.
///
/// "Clears cleanly" is not "the selection becomes `None` forever": with the
/// pin gone the component layer falls back to its SNAP mode and picks whatever
/// section of the still-locked hull is nearest the crosshair, which is exactly
/// what a pilot whose pinned part was blown off should get. What must hold is
/// that the pin is no longer pinned, that it no longer names the section that
/// left, and that the selection never wanders onto a body the player did not
/// lock.
#[cfg(feature = "debug")]
fn assert_the_pin_clears_and_the_lock_holds(world: &mut World) {
    let skiff = object_by_id(world, SKIFF_ID).expect("wreck lock: the derelict must still be up");
    let pinned = section_by_id(world, PINNED_SECTION)
        .expect("wreck lock: a severed section is re-parented, not deleted");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    let fragment = body_of(world, pinned).expect("wreck lock: the severed wingtip has a body");
    assert!(
        world.get::<ShipWreckFragmentMarker>(fragment).is_some(),
        "wreck lock: '{PINNED_SECTION}' left the hull without becoming wreckage; the phase needs \
         a fragment to go on and has none"
    );
    world.resource_mut::<WreckLog>().fragment = Some(fragment);

    let lock = world
        .get::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0;
    assert!(
        lock == Some(skiff),
        "wreck lock: the combat lock read {lock:?} after the PINNED section was severed; losing a \
         component selection is not losing the target, and a lock that followed the piece would \
         hand the pilot a body they never chose"
    );
    let component = world
        .get::<ComponentLock>(player)
        .expect("wreck lock: a player ship carries the component lock")
        .clone();
    assert!(
        !matches!(component.mode, ComponentLockMode::Pinned { .. }),
        "wreck lock: the pin is still pinned ({:?}) with its section gone; a pin outlives neither \
         its deadline nor its section",
        component.mode
    );
    assert!(
        component.section != Some(pinned),
        "wreck lock: the component selection still names '{PINNED_SECTION}', which is no longer \
         part of the locked hull - the gun would be aiming at a body the lock does not hold"
    );
    if let Some(selected) = component.section {
        assert!(
            body_of(world, selected) == Some(skiff),
            "wreck lock: the component selection fell through to a section of {:?} rather than \
             the locked hull; the component layer only ever selects within the combat lock",
            body_of(world, selected)
        );
    }
    info!(
        "wreck lock: the pin cleared to {:?} and the lock stayed on the derelict",
        component.mode
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: severing the pinned section clears the pin and keeps the lock",
        serde_json::json!({
            "t": elapsed,
            "pinned": PINNED_SECTION,
            "mode": format!("{:?}", component.mode),
            "selection_is_on_the_locked_hull": component
                .section
                .and_then(|selected| body_of(world, selected))
                == Some(skiff),
        }),
    );
}

/// The fragment is a contact in its own right: a body the gunner can see, with
/// a clear line to it, ranked as itself rather than as part of what it left.
///
/// This is the claim the acquisition below stands on. Unsigned wreckage returns
/// nothing to a scanner, so it is lockable at point-blank only - the gunner is
/// parked inside that reach on purpose, and a range that drifted out of it
/// would fail the acquisition for a reason that has nothing to do with the
/// lock layer.
#[cfg(feature = "debug")]
fn assert_the_fragment_is_its_own_contact(world: &mut World) {
    let fragment = world
        .resource::<WreckLog>()
        .fragment
        .expect("wreck lock: the fragment is recorded when the pinned section severs");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    let contacts = world
        .get::<SensorContacts>(player)
        .expect("wreck lock: an observing ship publishes its contacts")
        .clone();
    let contact = contacts.get(fragment).copied();
    let range = Meters::from_engine(contact.map_or(f32::INFINITY, |contact| {
        contact.anchor.distance(contacts.origin)
    }));
    assert!(
        contact.is_some(),
        "wreck lock: the severed fragment is not in the gunner's contact set at all; a piece of \
         hull that came off is a body of its own and a scanner standing next to it must see it"
    );
    assert!(
        contact.is_some_and(|contact| contact.in_sight),
        "wreck lock: the fragment is a contact but not in sight at {:.0} m; a lock is a radio \
         link and one cannot be taken through cover",
        range.get()
    );
    assert!(
        contact.is_some_and(|contact| contact.entity == fragment),
        "wreck lock: the contact set names the fragment as something else - it must be ranked as \
         itself, not folded into the hull it left"
    );
    info!(
        "wreck lock: the fragment is its own contact at {:.0} m",
        range.get()
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: the severed fragment is its own close contact",
        serde_json::json!({
            "t": elapsed,
            "range_m": range.get(),
            "in_sight": contact.is_some_and(|contact| contact.in_sight),
            "is_ship": contact.is_some_and(|contact| contact.is_ship),
        }),
    );
}

/// Nothing picked the fragment up while the slot was empty.
///
/// The design decision this range is built on: fragment acquisition is
/// EXPLICIT. A slot that filled itself here would mean a pilot who let a
/// target go gets handed the nearest piece of debris instead, and every
/// reading after this one would be about a lock the range did not take.
#[cfg(feature = "debug")]
fn assert_the_fragment_is_never_acquired_on_its_own(world: &mut World) {
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    let lock = world
        .get::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0;
    assert!(
        lock.is_none(),
        "wreck lock: the combat lock filled itself with {lock:?} over {UNLOCKED_WATCH_SECS} s of \
         an empty slot with a fresh fragment parked alongside; nothing re-locks on its own - a \
         fresh dwell is what takes a lock"
    );
    let anchor = reticle_anchor(world);
    assert!(
        anchor.is_none(),
        "wreck lock: the combat reticle is still anchored to {anchor:?} with no lock held; the \
         reticle is the lock made visible and an empty slot draws nothing"
    );
    info!("wreck lock: the empty slot stayed empty beside a fresh fragment");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: a fragment is never acquired on its own",
        serde_json::json!({ "t": elapsed, "watched_secs": UNLOCKED_WATCH_SECS }),
    );
}

/// The HUD followed the deliberate acquisition: the reticle is drawn on the
/// fragment, and the target inset names it rather than the hull it left.
///
/// The inset is read through its CAPTION and not through its render camera.
/// The zoom camera is only spawned for bodies tagged zoomable - ship roots and
/// committed torpedoes - and a bare wreck fragment is not one, so the inset
/// legitimately sits in its no-signal state. What must follow the lock in
/// every case is the line that names what is held.
#[cfg(feature = "debug")]
fn assert_the_hud_follows_the_fragment(world: &mut World) {
    let fragment = world
        .resource::<WreckLog>()
        .fragment
        .expect("wreck lock: the fragment is recorded when the pinned section severs");
    let player = player_root(world).expect("wreck lock: the gunner must be the player ship");
    let lock = world
        .get::<CombatLock>(player)
        .expect("wreck lock: a player ship carries the targeting state")
        .0;
    assert!(
        lock == Some(fragment),
        "wreck lock: the deliberate acquisition did not stick - the slot reads {lock:?}; a \
         fragment inside point-blank range with a clear line is a target a pilot may hold"
    );
    let anchor = reticle_anchor(world);
    assert!(
        anchor == Some(ScreenIndicatorAnchorKind::Entity(fragment)),
        "wreck lock: the combat reticle is anchored to {anchor:?} rather than the fragment the \
         pilot took; the reticle draws whatever the combat slot holds"
    );
    let caption = inset_caption(world).expect("wreck lock: the target inset carries a caption");
    let fragment_name = world
        .get::<Name>(fragment)
        .map(|name| name.to_string())
        .expect("wreck lock: a severed fragment is a named body");
    assert!(
        caption.starts_with(&fragment_name),
        "wreck lock: the target inset reads {caption:?} while the pilot holds {fragment_name:?}; \
         the inset names what is locked, and a caption left on the old hull is a pilot looking at \
         the wrong body"
    );
    info!("wreck lock: the reticle and the inset ({caption}) followed the fragment");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: the reticle and the inset follow the acquired fragment",
        serde_json::json!({ "t": elapsed, "caption": caption }),
    );
}

/// The same gun, aimed by the same targeting state, reaches what it was pointed
/// at.
///
/// Projectile OBSTRUCTION, not owner immunity: a severed fragment is a solid
/// body for everyone, so a round aimed at it lands on it. The gun here has been
/// on this ship since the range loaded and nothing about it changed when the
/// target did.
#[cfg(feature = "debug")]
fn assert_the_gun_reaches_the_fragment(world: &mut World) {
    let before = world
        .resource::<WreckLog>()
        .health_before_fire
        .expect("wreck lock: the trigger beat records what the fragment was worth");
    let after = world
        .resource::<WreckLog>()
        .health_while_firing
        .expect("wreck lock: the trigger beat reads the fragment every frame it fires");
    let spent = before - after;
    assert!(
        spent > 0.0,
        "wreck lock: the fragment is worth {after:.1} hit points, the same {before:.1} it was \
         worth when the trigger opened; a wreck is a solid body and the round that hits it spends \
         its health"
    );
    info!("wreck lock: the fragment took {spent:.1} hit points from the same gun");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: the same gun hits the fragment it acquired",
        serde_json::json!({
            "t": elapsed,
            "health_before": before,
            "health_after": after,
            "spent": spent,
        }),
    );
}
