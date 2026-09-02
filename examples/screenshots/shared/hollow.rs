//! The Rock hollow: the cast, the rock shell and the beats every hollow
//! screenshot drives.
//!
//! The player starts parked on station at the origin, which is where every
//! combat framing in the set is measured from. Included by each hollow producer
//! with `#[path = "shared/hollow.rs"] mod hollow;`. It pulls in `shared/kit.rs`
//! ITSELF, so a producer that includes this must not also include the kit by
//! `#[path]` - two path copies of one file are two distinct modules with two
//! distinct `NearField` types.
//!
//! What it holds:
//!
//! - [`ambush_hollow`]: the fighting set - the player, the raider it locks, two
//!   friendly corvettes, a hostile pair and a friendly torpedo boat.
//! - [`ordnance_hollow`]: the quiet set - player, raider and boat only, for the
//!   torpedo run.
//! - The `debug`-only beat helpers: stance, radar, trigger, station-keeping,
//!   scripted section death and the torpedo salvo.

// Each producer includes the whole module and uses the part its scene needs;
// the unused half is not dead code, it is another scene's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

#[path = "kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

/// Scenario id of the player's ship.
pub const PLAYER_ID: &str = "hollow_player";

/// Scenario id of the raider the player locks, shoots and finally blows a
/// section off.
pub const RAIDER_ID: &str = "hollow_raider";
/// Where it appears: dead ahead of the parked player, far enough back that the
/// frame has depth between the two hulls, close enough that the target reads.
pub const RAIDER_POSITION: Vec3 = Vec3::new(0.0, 0.6, -34.0);
/// The raider section the scripted blow takes off - the semantic nose part is
/// forward and camera-facing, so the fragments and the hole are both in frame.
pub const RAIDER_BLOWN_SECTION: &str = "nose";
/// The section the torpedo beat takes off on the raider's port side, where a
/// blast arriving from above lands - a BACKSTOP, not the damage itself.
///
/// This named the racer's `wing_port` for a while. The raider is a cargoa,
/// which has no wings, so the blow could never resolve; it has to be a section
/// this hull actually carries.
///
/// The blow was written when a Serpent carried 100 blast damage and left a
/// 70-100 health section standing, so the frame needed help to show a hole. A
/// Serpent carries 750 over a 30-unit radius now, which is enough to take the
/// whole corvette apart in the same tick, root and all. So this usually fires
/// into an already-dead section and warns, harmlessly: the torpedo did the job
/// the blow was there to guarantee. Worth revisiting whether the beat still
/// earns its place - and worth NOT deleting until someone has looked at what
/// the aftermath frame actually captures.
pub const RAIDER_BLAST_SECTION: &str = "pod_port";

/// Scenario id of the friendly torpedo boat - the only hull in the set carrying
/// torpedo pods, and the ship the ordnance beats are shot off.
pub const LANCE_ID: &str = "hollow_lance";
/// Where it sits: high and off the raider's far quarter, so the run comes DOWN
/// onto the target - and, the reason for the height, through open sky. The rock
/// shell is 46 units thick in Y, and a torpedo fired across the hollow at the
/// shell's own height flies into a rock: this bearing clears it. It is also what
/// keeps the blast (30 units across) off the player, parked 34 from the raider.
pub const LANCE_POSITION: Vec3 = Vec3::new(-38.0, 30.0, -56.0);
/// How far short of its target a torpedo detonates: the proximity fuze fires at
/// half the bay's blast radius (`torpedo_section/projectile.rs`), and the
/// cargo-B's bays are authored at 30. The ordnance camera is framed off this,
/// not off the raider - 15 units is a third of the frame at a close camera.
pub const TORPEDO_FUZE_RANGE: f32 = 15.0;
/// How many bays the cargo-B carries, and so how many torpedoes one salvo is.
pub const EXPECTED_TORPEDO_COUNT: usize = 2;

/// Seconds each AI flight holds fire after it spawns, so the shots are taken of
/// a fight that has settled rather than of four ships still sorting out where
/// they are.
pub const ENGAGE_DELAY: f32 = 3.0;
/// How far an AI ship may stray from its post before it breaks off and comes
/// back. Wider than the standoff range the engage maneuver flies to (100), so
/// the fight is not permanently interrupted, tight enough that the hollow keeps
/// its ships instead of watching them leave.
pub const AI_LEASH: f32 = 320.0;

/// The fighting set: the player on station, the raider it locks, the live
/// background of four AI corvettes, the torpedo boat, and the rock shell around
/// all of it.
///
/// The whole cast spawns `OnStart`, so a plain run gets the fight by loading the
/// example - there is no trigger to fly into.
pub fn ambush_hollow(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShips,
) -> ScenarioConfig {
    let player_hull = kit::kenney_hull(ships, "cargoa");
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Vec3::ZERO,
        // Square with the world: the radar picks by the CAMERA's look ray
        // (`ActiveLookRay`), which opens down world -Z whatever the hull is
        // doing, and the raider is parked a few degrees off that ray - inside
        // the 18-degree radar cone, and clear of the player's own hull in frame.
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            // Without this the trigger is bound to NOTHING: turret bindings are
            // per-section, snapshotted from this map by section id at spawn
            // (`nova_scenario/src/objects/spaceship.rs`), so an empty map is a
            // ship whose guns no button reaches.
            input_mapping: turret_bindings(sections, &player_hull),
            speed_cap: None,
        }),
        None,
        // The player holds fire through several beats; running dry mid-capture
        // would leave a reload where the tracers should be.
        unlimited_turrets(sections, player_hull.clone()),
    );

    // The lock subject: not AI, because an AI hostile flies to a 100-unit
    // standoff and no close framing survives that. It is not dead still either -
    // [`nudge_raider`] gives it a slow drift, so the lock's DST and CLS readouts
    // are of a moving target.
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        // Nose toward the player, turned off square: a hostile bearing down
        // reads better than a hull presenting its flank, and it puts the
        // section the juice beat blows on the camera's side of the ship.
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        kit::kenney_hull(ships, "cargoa"),
    );

    // The live background: two friendlies working the near flanks, two hostiles
    // across the hollow, all four FLYING a route while the engage grace runs -
    // a fight that opens on four parked hulls reads as a diorama. The grace
    // (`engage_delay`) holds them in `Patrol`, so they are mid-leg and banking
    // when the first shot is taken; leashed so the ring they fly afterwards
    // stays in the set.
    let wingman_a = ship(
        "hollow_wing_a",
        "Wingman",
        Vec3::new(-64.0, 12.0, -44.0),
        Quat::from_rotation_y(0.2),
        fighter(vec![
            Vec3::new(-64.0, 12.0, -44.0),
            Vec3::new(-30.0, 4.0, -96.0),
            Vec3::new(-86.0, -6.0, -70.0),
        ]),
        Some(Allegiance::Player),
        kit::kenney_hull(ships, "cargoa"),
    );
    let wingman_b = ship(
        "hollow_wing_b",
        "Wingman",
        Vec3::new(62.0, -14.0, -58.0),
        Quat::from_rotation_y(-0.2),
        fighter(vec![
            Vec3::new(62.0, -14.0, -58.0),
            Vec3::new(96.0, 6.0, -104.0),
            Vec3::new(40.0, -20.0, -110.0),
        ]),
        Some(Allegiance::Player),
        kit::kenney_hull(ships, "cargoa"),
    );
    let hostile_a = ship(
        "hollow_hostile_a",
        "Raider",
        Vec3::new(-150.0, 34.0, -230.0),
        Quat::from_rotation_y(3.0),
        fighter(vec![
            Vec3::new(-150.0, 34.0, -230.0),
            Vec3::new(-70.0, 18.0, -290.0),
            Vec3::new(-190.0, 6.0, -300.0),
        ]),
        None,
        kit::kenney_hull(ships, "cargoa"),
    );
    let hostile_b = ship(
        "hollow_hostile_b",
        "Raider",
        Vec3::new(176.0, -38.0, -262.0),
        Quat::from_rotation_y(3.3),
        fighter(vec![
            Vec3::new(176.0, -38.0, -262.0),
            Vec3::new(90.0, -14.0, -320.0),
            Vec3::new(210.0, -4.0, -330.0),
        ]),
        None,
        kit::kenney_hull(ships, "cargob"),
    );

    // The torpedo boat: a cargo-B, which is the only Kenney hull in the catalog
    // with launch bays. Posed, not AI - the AI's own launch envelope opens at
    // 3x the blast radius and its cadence is a 10-second playtest knob, so a
    // capture that waited for it would be waiting on a coin flip. The script
    // pulls the trigger instead ([`loose_torpedoes`]) and the bay, the
    // projectile, the guidance and the blast are all the production path.
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION)
            .looking_at(RAIDER_POSITION, Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        kit::kenney_hull(ships, "cargob"),
    );

    ScenarioConfig {
        description: "A rock hollow, and the ambush waiting in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The photo rig is authored content rather than an example-side
            // observer swap: scale 1.0 around the origin reproduces the kit's
            // exact key/rim/fill numbers, so the captured frames are unchanged.
            actions: [
                vec![
                    shell().action(game_assets),
                    player,
                    raider,
                    wingman_a,
                    wingman_b,
                    hostile_a,
                    hostile_b,
                    lance,
                ],
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow".to_string(),
            "Rock Hollow".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The ordnance set: the same hollow with no unrelated combatants and no live
/// guns, so the only thing moving in frame is the salvo.
pub fn ordnance_hollow(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        Vec3::ZERO,
        Quat::IDENTITY,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        None,
        kit::kenney_hull(ships, "cargoa"),
    );
    let raider = ship(
        RAIDER_ID,
        "Raider",
        RAIDER_POSITION,
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        SpaceshipController::None,
        Some(Allegiance::Enemy),
        kit::kenney_hull(ships, "cargoa"),
    );
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION)
            .looking_at(RAIDER_POSITION, Vec3::Y)
            .rotation,
        SpaceshipController::None,
        Some(Allegiance::Player),
        kit::kenney_hull(ships, "cargob"),
    );
    let shell = kit::NearField {
        id_prefix: "ordnance_rock_",
        count: 48,
        seed: 40507,
        distance: (48.0, 130.0),
        radius: (1.2, 3.2),
        y_spread: 46.0,
    };

    ScenarioConfig {
        description: "The rock hollow with only the ordnance cast in it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), player, raider, lance],
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "rock_hollow_ordnance".to_string(),
            "Rock Hollow - Ordnance".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The hollow itself - and it is a HOLLOW: the field starts outside the raider's
/// station (34 units) with room to spare, so the pocket the fight happens in is
/// clear and the rocks read as the wall around it. Tried tighter (28 units):
/// rocks land on the raider, every close framing has one in front of the
/// subject, and a torpedo run into it hits stone.
fn shell() -> kit::NearField {
    kit::NearField {
        id_prefix: "hollow_rock_",
        count: 48,
        seed: 40507,
        distance: (48.0, 130.0),
        radius: (1.2, 3.2),
        y_spread: 46.0,
    }
}

/// One posed ship in the set.
pub fn ship(
    id: &str,
    name: &str,
    position: Vec3,
    rotation: Quat,
    controller: SpaceshipController,
    allegiance: Option<Allegiance>,
    sections: Vec<SpaceshipSectionConfig>,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            allegiance,
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
            ..default()
        }),
    })
}

/// A fighting AI ship's routine: fly `patrol` until the engage grace expires,
/// then fight, and come back when the fight drags it past the leash.
///
/// The route is what makes the set move before the first shot: the grace holds
/// the ship in `Patrol`, which flies the waypoint loop through the real GOTO
/// autopilot instead of station-keeping.
pub fn fighter(patrol: Vec<Vec3>) -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        patrol,
        leash: Some(AI_LEASH),
        engage_delay: Some(ENGAGE_DELAY),
        ..default()
    })
}

/// Bind every turret section of a built hull to the trigger, the way the
/// shipped scenarios do (`shakedown_run` maps its two corvette turret cubes to
/// `Mouse(Left)` + `Gamepad(RightTrigger2)`).
///
/// Read off the BUILT hull rather than typed out, for the same reason
/// [`kit::kenney_hull`] is: the ids ARE the layout, and a hand-listed pair goes
/// stale the moment a hull gains a gun. The map is keyed by INSTANCE id
/// (`nova_scenario` snapshots bindings by section id at spawn), which is the id
/// the assembly gave the mount.
///
/// This used to walk the section CATALOG and strip a `<hull>_` prefix off every
/// turret prototype. Every craft mounts the one shared PDC now, whose id
/// carries no hull prefix, so that filter matched nothing and handed back an
/// empty map - a ship whose guns no button reaches, silently.
/// The same hull with every turret rebuilt without a magazine, so a capture that
/// holds fire never cuts to a reload.
///
/// A prototype section resolves to an inline copy of the catalog entry: the
/// magazine lives in the section config, so a rig that wants unlimited fire has
/// to author the gun rather than reference it.
pub fn unlimited_turrets(
    sections: &GameSections,
    hull: Vec<SpaceshipSectionConfig>,
) -> Vec<SpaceshipSectionConfig> {
    hull.into_iter()
        .map(|mut section| {
            let SectionSource::Prototype(prototype) = &section.source else {
                return section;
            };
            let Some(resolved) = sections.get_section(prototype) else {
                return section;
            };
            if matches!(resolved.kind, SectionKind::Turret(_)) {
                section.source = SectionSource::Inline(resolved.clone().without_magazine());
            }
            section
        })
        .collect()
}

pub fn turret_bindings(
    sections: &GameSections,
    hull: &[SpaceshipSectionConfig],
) -> BTreeMap<String, Vec<InputSource>> {
    hull.iter()
        .filter(|section| {
            let SectionSource::Prototype(prototype) = &section.source else {
                return false;
            };
            sections
                .get_section(prototype)
                .is_some_and(|section| matches!(section.kind, SectionKind::Turret(_)))
        })
        .map(|section| {
            let id = section.id.as_str();
            (
                id.to_string(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )
        })
        .collect()
}

/// Present while the scripted run holds the ship on the station every combat
/// framing is measured from.
#[cfg(feature = "debug")]
#[derive(Resource)]
pub struct HoldStation;

/// Put the HUD on (the contextual rules decide what is actually in shot).
#[cfg(feature = "debug")]
pub fn hud_instrument(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::On;
    }
}

/// Clean the screen, for the beats whose camera has left the player's ship.
#[cfg(feature = "debug")]
pub fn hud_cinematic(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Cinematic;
    }
}

/// Pin the camera for a framing the follow camera does not give.
#[cfg(feature = "debug")]
pub fn pose(world: &mut World, position: Vec3, look_at: Vec3) {
    pose_camera(world, position, look_at);
}

/// Start holding station: from here on the scripted run keeps the ship exactly
/// where the combat set was measured from, and the camera looks the way the
/// pinned hull does.
///
/// Re-seeding the mouse rig is not optional. The rig carries whatever attitude
/// the ship was last flown at (`camera/handback.rs`), so pinning the hull square
/// without re-seeding leaves the camera parked on the wrong side of the ship,
/// filming the combat act over its shoulder from in front.
#[cfg(feature = "debug")]
pub fn hold_station(world: &mut World) {
    world.insert_resource(HoldStation);
    let rigs: Vec<Entity> = world
        .query_filtered::<Entity, With<PointRotationOutput>>()
        .iter(world)
        .collect();
    for rig in rigs {
        world.entity_mut(rig).insert((
            PointRotation {
                initial_rotation: Quat::IDENTITY,
            },
            PointRotationOutput(Quat::IDENTITY),
        ));
    }
}

/// Hold the player at the hollow's origin, for the combat beats of a scripted
/// run.
///
/// Not cosmetic, and not the STOP autopilot: the set's geometry is measured from
/// a player at the origin, and the radar picks the body nearest the AIM RAY
/// (`crates/nova_gameplay/src/input/targeting/radar.rs`), so a player a few tens
/// of units off station swings the parked raider off the ray and latches a
/// hostile two kilometers out instead.
#[cfg(feature = "debug")]
pub fn pin_player(
    mut player: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (mut transform, mut linear, mut angular) in &mut player {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// Set the raider drifting: slow, and across the line of sight rather than
/// along it, so it stays on the aim ray the radar picks by while the lock's
/// distance and closing-speed readouts have something to say.
#[cfg(feature = "debug")]
pub fn nudge_raider(world: &mut World) {
    let Some(raider) = raider_root(world) else {
        warn!("hollow: no raider to nudge");
        return;
    };
    if let Some(mut velocity) = world
        .entity_mut(raider)
        .get_mut::<avian3d::prelude::LinearVelocity>()
    {
        velocity.0 = Vec3::new(0.35, 0.12, -0.25);
    }
}

/// Hold the radar gesture. Which slot it latches depends on the stance.
#[cfg(feature = "debug")]
pub fn hold_radar(world: &mut World) {
    press_action("radar_hold")(world);
}

/// Release the radar gesture.
#[cfg(feature = "debug")]
pub fn release_radar(world: &mut World) {
    release_action("radar_hold")(world);
}

/// Raise the weapons, switching the radar from the nav slot to combat.
#[cfg(feature = "debug")]
pub fn raise_stance(world: &mut World) {
    press_action("combat_stance")(world);
}

/// Hold the trigger (LMB) so the player's turret is firing in the combat shots.
#[cfg(feature = "debug")]
pub fn open_fire(world: &mut World) {
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
}

/// Blow one hull section off the raider through the production damage path -
/// the same `HealthApplyDamage` a bullet delivers, so the shot is of the real
/// destruction, not of a prop.
#[cfg(feature = "debug")]
pub fn blow_raider_section(world: &mut World, section: &str) {
    let Some(node) = raider_section_health(world, section) else {
        warn!("hollow: no health node under section '{section}' to blow");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("hollow: blew '{section}' off the raider");
}

/// The player's ship root.
#[cfg(feature = "debug")]
pub fn player_root(world: &mut World) -> Option<Entity> {
    let mut query =
        world.query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>();
    query.iter(world).next()
}

/// The player's ship root, from a read-only world (what a predicate gets).
#[cfg(feature = "debug")]
pub fn player_root_ref(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?
        .iter(world)
        .next()
}

/// The raider's ship root.
#[cfg(feature = "debug")]
pub fn raider_root(world: &mut World) -> Option<Entity> {
    ship_by_id(world, RAIDER_ID)
}

/// Where the raider actually is right now; its spawn point if it has gone.
#[cfg(feature = "debug")]
pub fn raider_position(world: &mut World) -> Vec3 {
    raider_root(world)
        .and_then(|raider| world.get::<GlobalTransform>(raider))
        .map(|transform| transform.translation())
        .unwrap_or(RAIDER_POSITION)
}

/// Advance once the raider is in the world - it is the subject of every close
/// beat, so a set that came up without it aborts here by name.
#[cfg(feature = "debug")]
pub fn raider_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == RAIDER_ID))
    })
}

/// The ship root carrying scenario id `id`.
#[cfg(feature = "debug")]
pub fn ship_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// The first entity carrying scenario id `id`.
#[cfg(feature = "debug")]
pub fn entity_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EntityId)>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// One of the raider's section entities, by prototype id. Picked BY SHIP: the
/// corvettes in the set share section ids, as every shipped multi-ship scenario
/// does.
#[cfg(feature = "debug")]
pub fn raider_section(world: &mut World, section: &str) -> Option<Entity> {
    let raider = raider_root(world)?;
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SectionMarker>>();
    let candidates: Vec<Entity> = query
        .iter(world)
        .filter(|(_, id)| id.0 == section)
        .map(|(entity, _)| entity)
        .collect();
    candidates
        .into_iter()
        .find(|&entity| under(world, entity, raider))
}

/// The `Health` node of one of the raider's sections: the health lives on the
/// section entity or on one of its children.
#[cfg(feature = "debug")]
pub fn raider_section_health(world: &mut World, section: &str) -> Option<Entity> {
    let section = raider_section(world, section)?;
    if world.get::<Health>(section).is_some() {
        return Some(section);
    }
    let children: Vec<Entity> = world
        .get::<Children>(section)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find(|&child| world.get::<Health>(child).is_some())
}

/// Whether `entity` sits under `root` in the hierarchy.
#[cfg(feature = "debug")]
pub fn under(world: &World, entity: Entity, root: Entity) -> bool {
    let mut current = entity;
    for _ in 0..8 {
        match world.get::<ChildOf>(current) {
            Some(parent) if parent.parent() == root => return true,
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
    false
}

/// Pull the torpedo boat's triggers: every bay on the lance, fired at once, so
/// the beat is a salvo rather than a single round.
///
/// The bays are the ship root's own children (which is how the AI's launch
/// system finds them), and writing [`TorpedoSectionInput`] is exactly what the
/// player's trigger observer and the AI's envelope do - from here on the launch
/// is the production path.
#[cfg(feature = "debug")]
pub fn loose_torpedoes(world: &mut World) {
    let Some(lance) = ship_by_id(world, LANCE_ID) else {
        warn!("hollow: no torpedo boat to fire");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == lance)
        .map(|(bay, _)| bay)
        .collect();
    if bays.is_empty() {
        warn!("hollow: the torpedo boat has no bays");
        return;
    }
    for bay in &bays {
        if let Some(mut input) = world.entity_mut(*bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
    info!("hollow: {} torpedo bay(s) firing", bays.len());
}

/// Commit the salvo to the raider and drop the trigger.
///
/// A torpedo's target is decided exactly once, right after launch: the player
/// commits from the crosshair lock and the AI from its own `AITarget`
/// (`input/player/intent.rs`, `input/ai/torpedo.rs`), both by inserting
/// [`TorpedoTargetChosen`] and a [`TorpedoTargetEntity`] on the fresh
/// projectile. The boat is neither, so the script does that one write and the
/// guidance, arming, fuze and blast run themselves. Releasing the bays here is
/// what keeps it to one salvo - the bays would otherwise relaunch on their own
/// fire-rate clock.
#[cfg(feature = "debug")]
pub fn commit_torpedoes(world: &mut World) {
    let Some(raider) = raider_root(world) else {
        warn!("hollow: no raider to commit the salvo to");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = false;
        }
    }
    let torpedoes: Vec<Entity> = world
        .query_filtered::<Entity, (With<TorpedoProjectileMarker>, Without<TorpedoTargetChosen>)>()
        .iter(world)
        .collect();
    assert_eq!(
        torpedoes.len(),
        EXPECTED_TORPEDO_COUNT,
        "hollow: the ordnance set must commit the complete salvo"
    );
    for torpedo in &torpedoes {
        world
            .entity_mut(*torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(raider)));
    }
    info!(
        "hollow: {} torpedo(es) committed to the raider",
        torpedoes.len()
    );
}

/// What the ordnance frames are about: the midpoint of the raider and the point
/// the fuze will go off at, which is [`TORPEDO_FUZE_RANGE`] short of it along
/// the boat's bearing. Framing on the raider alone puts the blast at the edge of
/// the frame; framing on this holds both.
#[cfg(feature = "debug")]
pub fn ordnance_subject(world: &mut World) -> Vec3 {
    let raider = raider_position(world);
    let bearing = (LANCE_POSITION - raider).normalize_or_zero();
    raider + bearing * (TORPEDO_FUZE_RANGE * 0.5)
}

/// Advance once the whole salvo is actually in the world.
#[cfg(feature = "debug")]
pub fn torpedo_salvo_in_flight(
    expected: usize,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .is_some_and(|mut torpedoes| torpedoes.iter(world).count() == expected)
    })
}

/// Advance once the last torpedo is gone - the fuze despawns it and spawns the
/// blast in the same frame, so this IS the detonation.
#[cfg(feature = "debug")]
pub fn no_torpedo_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| torpedo_range(world).is_none())
}

/// Advance once the leading torpedo is within `distance` of the raider.
#[cfg(feature = "debug")]
pub fn torpedo_within(
    distance: f32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        torpedo_range(world).is_some_and(|range| range < distance)
    })
}

/// Fail the run if the salvo dies before the capture range, rather than shooting
/// an empty approach.
#[cfg(feature = "debug")]
pub fn assert_salvo_still_live(world: &mut World, _: f32, _: u32) {
    let live = world
        .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
        .iter(world)
        .count();
    assert!(
        live > 0,
        "hollow: the complete torpedo salvo was lost before reaching the capture range"
    );
}

/// How far the closest live torpedo is from the raider, if there is one of each.
#[cfg(feature = "debug")]
pub fn torpedo_range(world: &World) -> Option<f32> {
    let raider = world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?
        .iter(world)
        .find(|(_, id)| id.0 == RAIDER_ID)
        .map(|(entity, _)| entity)?;
    let target = world.get::<GlobalTransform>(raider)?.translation();
    world
        .try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?
        .iter(world)
        .map(|transform| transform.translation().distance(target))
        .min_by(f32::total_cmp)
}
