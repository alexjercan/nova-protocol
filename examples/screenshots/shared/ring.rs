//! "The ring": the set the flight-computer screenshot examples are shot on,
//! plus the leg-camera rig they pose off it.
//!
//! A gravity planetoid at the origin, a rock ring outside the flight path, a
//! survey beacon over the pole, and the player's racer parked on the ring
//! radius. Everything here is the production flight stack: the helpers insert
//! the same [`Autopilot`] components the ORBIT and GOTO keybinds do and then
//! watch the phase (`Align -> Burn -> Hold`) and the telemetry; no attitude,
//! velocity, ring or trajectory is faked.
//!
//! Included by each flight producer with `#[path = "shared/ring.rs"] mod ring;`.
//! It pulls in `shared/kit.rs` ITSELF, so a producer that includes this must not
//! also include the kit by `#[path]` - two path copies of one file are two
//! distinct modules with two distinct `NearField` types.
//!
//! Sizing, so the numbers are not magic (the well math is
//! `crates/nova_gameplay/src/gravity.rs`):
//! - MEASURED, not assumed: the noise displaces the rock mesh outward, and the
//!   derived `BodyRadius` of this planetoid comes out at 79-108 units for an
//!   authored 20 - a factor of about 4.5, not the 2 a diameter-based reading
//!   suggests. The spread is per RUN, since the displacement is seeded from the
//!   global RNG, so nothing here may sit close to a boundary. Everything - the
//!   well strength, the orbit band, the body in frame - measures from that
//!   radius, and [`engage_orbit`] logs it, because the first cut of this scene
//!   put the ring 19 units off the surface and shot a frame of nothing but rock.
//! - The orbit band runs from `1.5 * (body_radius + 1)` out to `0.9 * 0.85 *
//!   SOI`, and the SOI comes from the authored mass alone
//!   (`sqrt(mu / soi_cutoff_accel)` = 490 units) - roughly 122 to 375 units
//!   here. [`ORBIT_RADIUS`] sits mid-band, so the explicit plan is the ring the
//!   ship actually flies and the body reads as a body rather than as terrain.
//! - The ring's circular speed is `sqrt(mu / r)`, around 14 u/s here: fast
//!   enough that the drive is lit and the hull is visibly banked into its plane,
//!   slow enough that a pinned camera keeps its subject for the length of a
//!   beat.

// Each producer includes the whole set and uses the part its shot needs; the
// unused half is not dead code, it is another shot's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

#[path = "kit.rs"]
mod kit;

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The planetoid's scenario id.
pub const PLANETOID_ID: &str = "ring_planetoid";
/// Authored radius. The mesh draws well past it - about 91 units of real body
/// for this 20 - and the well, the orbit band and every framing measure from
/// that derived radius, not from this number.
pub const PLANETOID_RADIUS: f32 = 20.0;
/// The body's mass parameter (mu, u^3/s^2) - the one authored gravity number,
/// setting both the pull and the SOI (`soi = sqrt(mu / soi_cutoff_accel)`, so
/// 490u here). Sized so [`ORBIT_RADIUS`] sits mid-band on every mesh seed:
/// this set's whole subject is the flight computer flying a REAL well, so a
/// tuned-down one would be a prop.
pub const PLANETOID_MASS: f32 = 60_000.0;

/// The player ship's scenario id.
pub const PLAYER_ID: &str = "ring_player";
/// The ring the ship holds: mid-band (roughly 122 to 375 units for this body),
/// so the explicit plan below is flown as authored rather than clamped. The
/// distance is chosen off the DRAWN body, at about three and a half of its
/// radii - close enough that the planetoid fills the lower third of a framing
/// with a curved limb, far enough that it is a body and not the ground.
pub const ORBIT_RADIUS: f32 = 320.0;
/// The ring's plane: the world horizontal, so the rock ring and the holo ring
/// are the same circle and the shots have one horizon rather than two.
pub const ORBIT_NORMAL: Vec3 = Vec3::Y;
/// Which way out of the ring the ship starts, and so WHERE ON THE RING every
/// orbit shot happens - the framings are built off the ship's own radial and
/// track, but the light rig is fixed in WORLD space, so the phase decides
/// whether the outboard cameras see a lit planetoid or a black one.
///
/// This is the rim light's horizontal direction (`shared/kit.rs` puts the rim
/// at `(3, 4, -8)`, and at 16000 lux it is the brightest lamp on the set), so
/// the cameras that sit outboard of the ship and look back in are looking down
/// the rim at the body's lit face. A run that starts a quarter turn away
/// photographs the night side: the insertion is a real burn whose duration moves
/// with the (per-run) derived body radius, so the shot phase cannot be left to
/// drift out of the light.
pub const START_RADIAL: Vec3 = Vec3::new(0.35, 0.0, -0.94);

/// Where the racer is parked before the verb takes it: on the ring, out along
/// [`START_RADIAL`].
pub fn start_position() -> Vec3 {
    START_RADIAL.normalize() * ORBIT_RADIUS
}

/// Pointing along the ring's travel direction (`normal x radial`), so the racer
/// is already squared with the track it is about to be put on rather than
/// swinging through 90 degrees on the first frame of the insertion.
pub fn start_rotation() -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Z, ORBIT_NORMAL.cross(START_RADIAL.normalize()))
}

/// The survey beacon the departure leg flies to.
pub const BEACON_ID: &str = "ring_beacon";
/// Where it sits: over the well's pole, and far enough out for a full
/// align-burn-coast-FLIP-brake profile. Polar on purpose - the ring is
/// horizontal, so a leg straight up out of it clears the body from EVERY point
/// on the ring (the closest a path from the ring to here passes the planetoid is
/// about 300 units, against a 92-unit body), and the departure does not depend
/// on where in its orbit the ship happened to be when the verb changed.
pub const BEACON_POSITION: Vec3 = Vec3::new(0.0, 760.0, 0.0);
/// Radar signature; lock range is 30 world units per unit of it. Not needed to
/// fly the leg - GOTO takes an entity, not a lock - but it is what puts the
/// beacon's own bracket on screen for the whole approach.
pub const BEACON_SIGNATURE: f32 = 40.0;

/// Seconds the ship holds the ring before the shots, so the last of the
/// circularization wobble is out of the hull's attitude.
#[cfg(feature = "debug")]
pub const STEADY_SECS: f32 = 2.0;

/// The set: a gravity planetoid at the origin, a rock ring outside the flight
/// path, and the player's racer parked on the ring radius.
pub fn the_ring(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    the_ring_with_hull(game_assets, ships, "racer")
}

/// The ring with a selected recipe hull for visual capture variants.
pub fn the_ring_with_hull(
    game_assets: &GameAssets,
    ships: &GameShips,
    hull: &str,
) -> ScenarioConfig {
    let player = ship(
        PLAYER_ID,
        "Player Ship",
        start_position(),
        start_rotation(),
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
            infinite_ammo: true,
        }),
        None,
        kit::kenney_hull(ships, hull),
    );

    // The ring debris: OUTSIDE the flight path, and small. Two rules, both
    // learned from a frame:
    // - The scatter region is an annulus centred on the origin - the same centre
    //   the orbit is - so a band overlapping the ring puts rocks in the ship's
    //   track, and a band INSIDE it walls off the one thing this set is about.
    //   Everything the camera looks inward through has to be empty.
    // - The 4.5x draw factor applies to these too, so an authored 2-7 is a field
    //   of 9-32 unit boulders. Authored small, they read as the debris the ring
    //   sweeps up rather than as a second asteroid field.
    let debris = kit::NearField {
        id_prefix: "ring_rock_",
        count: 34,
        seed: 71104,
        distance: (420.0, 760.0),
        radius: (1.0, 3.0),
        y_spread: 110.0,
    };

    ScenarioConfig {
        description: "A planetoid, its debris ring, and a ship flying the ORBIT verb around it."
            .to_string(),
        events: vec![
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                once: false,
                filters: vec![],
                // The photo rig, authored content rather than an example-side
                // observer swap: scale 1.0 around the origin reproduces the kit's
                // exact key/rim/fill numbers, so the captured frames are unchanged.
                actions: [
                    vec![
                        planetoid(game_assets),
                        debris.action(game_assets),
                        player,
                        beacon(),
                        EventActionConfig::VariableSet(VariableSetActionConfig {
                            key: "orbit_stable".to_string(),
                            expression: VariableExpressionNode::new_term(
                                VariableTermNode::new_factor(VariableFactorNode::new_literal(
                                    VariableLiteral::Number(0.0),
                                )),
                            ),
                        }),
                    ],
                    ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
                ]
                .concat(),
            },
            ScenarioEventConfig {
                name: EventConfig::OnOrbitStable,
                once: false,
                filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(PLANETOID_ID.to_string()),
                    other_id: Some(PLAYER_ID.to_string()),
                    ..default()
                })],
                actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "orbit_stable".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                })],
            },
        ],
        ..ScenarioConfig::new(
            "the_ring".to_string(),
            "The Ring".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The planetoid: the set's subject and the well the whole scene is about.
/// Invulnerable - nothing should be able to shoot the scenery out of a shot.
pub fn planetoid(game_assets: &GameAssets) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLANETOID_ID.to_string(),
            name: "Planetoid".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: PLANETOID_RADIUS,
            texture: game_assets.asteroid_texture.clone().into(),
            impact_sound: None,
            destroy_sound: None,
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    })
}

/// The survey beacon: the departure leg's destination, and the only thing in
/// the set the player is given a reason to fly to.
pub fn beacon() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BEACON_ID.to_string(),
            name: "Survey Beacon".to_string(),
            position: BEACON_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: "SURVEY".to_string(),
            // Bigger than a nav point needs to be: the arrival frame is a
            // picture of the thing at the end of the leg, and a 3-unit orb at
            // the far end of a standoff is a pixel.
            radius: 6.0,
            color: Color::srgb(0.4, 0.75, 1.0),
            // No trigger area: nothing in this set springs on arrival.
            area_radius: None,
            lock_signature: Some(BEACON_SIGNATURE),
        }),
    })
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

/// Pin a STILL framing: the pose, plus stopping any leg camera that would
/// otherwise re-solve it out from under this one on the next frame.
#[cfg(feature = "debug")]
pub fn pin(world: &mut World, position: Vec3, look_at: Vec3) {
    world.remove_resource::<LegCamera>();
    pose(world, position, look_at);
}

/// Where the photo rig's key light comes FROM (`shared/kit.rs`). The rig is
/// direction-only, so which half of a hull is lit depends on the hull's
/// ATTITUDE, and a maneuvering ship changes attitude for a living.
#[cfg(feature = "debug")]
pub const KEY_FROM: Vec3 = Vec3::new(-6.0, 5.0, 6.0);

/// A camera offset perpendicular to `track` that keeps the lens on the key
/// light's side of the subject: [`KEY_FROM`] with its along-track component
/// removed. Every leg framing offsets along this rather than along a world axis,
/// which is the difference between a hull with shape on it and the flat dark
/// silhouette an arbitrary side gives once the ship has swung around.
#[cfg(feature = "debug")]
pub fn lit_side(track: Vec3) -> Vec3 {
    let key = KEY_FROM.normalize();
    (key - track * key.dot(track))
        .try_normalize()
        .unwrap_or(Vec3::Y)
}

/// A camera that FLIES with the ship: the offsets are in the ship's own frame
/// (`side` along [`lit_side`], `along` down its track, `up` in world Y) and
/// [`drive_leg_camera`] re-solves them every frame while this is present.
///
/// A leg camera CANNOT be a single pinned pose. The ship crosses the transfer at
/// 65 u/s, so the ~0.3 s a framing beat holds is twenty units of travel - the
/// first cut of the flip beat pinned its camera 22 units ahead of the ship and
/// the ship flew straight through it, leaving an empty frame.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
pub struct LegCamera {
    /// Offset onto the key light's side of the hull.
    pub side: f32,
    /// Offset down the track: negative is behind the ship, positive ahead.
    pub along: f32,
    /// Offset in world Y.
    pub up: f32,
    /// How far up the track the lens aims past the ship. Positive drops the hull
    /// low in the frame and gives the space it is flying into to the shot.
    pub look_ahead: f32,
}

/// The filtered world-space pose of a moving leg camera.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct LegCameraTrack {
    position: Vec3,
    look_at: Vec3,
}

/// Re-solve the leg camera against the ship's live position and track.
#[cfg(feature = "debug")]
pub fn drive_leg_camera(world: &mut World) {
    let Some(rig) = world.get_resource::<LegCamera>().copied() else {
        world.remove_resource::<LegCameraTrack>();
        return;
    };
    let ship = ship_position(world);
    let heading = ship_heading(world);
    let offset = lit_side(heading) * rig.side + heading * rig.along + Vec3::Y * rig.up;
    let desired_position = ship + offset;
    let desired_look_at = ship + heading * rig.look_ahead;
    let delta = world.resource::<Time>().delta_secs().min(0.1);
    let alpha = 1.0 - (-10.0 * delta).exp();
    let mut track = world
        .remove_resource::<LegCameraTrack>()
        .unwrap_or(LegCameraTrack {
            position: desired_position,
            look_at: desired_look_at,
        });
    track.position = track.position.lerp(desired_position, alpha);
    track.look_at = track.look_at.lerp(desired_look_at, alpha);
    pose(world, track.position, track.look_at);
    world.insert_resource(track);
}

/// Fly the camera behind the ship on the key side. The framing for a ship under
/// power, since the drive is at the back and so is the lens.
#[cfg(feature = "debug")]
pub fn chase(world: &mut World, side: f32, back: f32, up: f32, look_ahead: f32) {
    world.remove_resource::<LegCameraTrack>();
    world.insert_resource(LegCamera {
        side,
        along: -back,
        up,
        look_ahead,
    });
}

/// Fly the camera ahead of the ship on the key side. The framing for a ship
/// under RETRO power: braking, the drive fires down the track, so the lens has
/// to be down the track with it.
#[cfg(feature = "debug")]
pub fn lead(world: &mut World, side: f32, ahead: f32, up: f32) {
    world.remove_resource::<LegCameraTrack>();
    world.insert_resource(LegCamera {
        side,
        along: ahead,
        up,
        look_ahead: 0.0,
    });
}

/// Engage the ORBIT verb on the planetoid's well with an explicit plan - the
/// same [`Autopilot`] component the keybind inserts, so the ring the ship flies
/// is the flight computer's and not an animation.
///
/// The plan is explicit rather than left to the verb because the ring is the
/// SET's geometry: an unplanned engage circularizes at whatever radius the ship
/// happens to be at, and every camera here is measured off [`ORBIT_RADIUS`].
#[cfg(feature = "debug")]
pub fn engage_orbit(world: &mut World) {
    let well = world
        .query_filtered::<Entity, With<GravityWell>>()
        .iter(world)
        .next();
    let (Some(well), Some(player)) = (well, player_root(world)) else {
        warn!("flight: no well or player to engage ORBIT on");
        return;
    };
    // The derived body radius, logged because it is the number this whole set
    // is sized off and it is NOT the authored one: the noise displaces the mesh
    // outward by a seeded factor, so a run that drifts far from the sizing note
    // above shows it here rather than in a frame full of rock.
    let body_radius = world.get::<GravityWell>(well).map(|well| well.body_radius);
    world
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: Some(OrbitPlan {
                radius: ORBIT_RADIUS,
                normal: ORBIT_NORMAL,
            }),
        }));
    info!("flight: ORBIT engaged at r = {ORBIT_RADIUS} on a body of radius {body_radius:?}");
}

/// Hand the camera back to the game: stop flying any leg rig, then drop the
/// pinned pose.
#[cfg(feature = "debug")]
pub fn unpose(world: &mut World) {
    world.remove_resource::<LegCamera>();
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    if let Some(camera) = camera {
        world.entity_mut(camera).remove::<ScriptedCameraPose>();
    }
}

/// Engage the travel computer on the survey beacon - the same [`Autopilot`] the
/// G keybind inserts. Engaging it straight over a held orbit replaces the
/// component, which is what a pilot changing verbs does.
#[cfg(feature = "debug")]
pub fn engage_goto(world: &mut World) {
    let (Some(player), Some(beacon)) = (player_root(world), entity_by_id(world, BEACON_ID)) else {
        warn!("flight: no player or beacon to engage the travel computer on");
        return;
    };
    world
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
    info!("flight: GOTO engaged on the survey beacon");
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

/// Advance once the leg is actually under way: the computer has published its
/// numbers and the ship is closing on the destination.
#[cfg(feature = "debug")]
pub fn player_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        maneuver(world).is_some_and(|telemetry| telemetry.closing_speed > 20.0)
    })
}

/// Advance the frame the flip starts: the telemetry drops its flip point once
/// the brake is planned, which is the same instant the computer turns the ship
/// around.
#[cfg(feature = "debug")]
pub fn player_braking() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        maneuver(world).is_some_and(|telemetry| {
            telemetry.flip_point.is_none() && telemetry.closing_speed > 5.0
        })
    })
}

/// Advance once the flip has finished and the retro burn is lit: braking, and
/// past the align phase the swing spends its time in.
#[cfg(feature = "debug")]
pub fn player_retro_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let braking = maneuver(world).is_some_and(|telemetry| telemetry.flip_point.is_none());
        braking && autopilot_phase(world) == Some(AutopilotPhase::Burn)
    })
}

/// Advance once the leg is essentially over: inside the arrival standoff
/// (`FlightSettings::arrival_standoff` is 50 units, and the telemetry's distance
/// is to the target SURFACE), with the closing speed off the top of the brake.
#[cfg(feature = "debug")]
pub fn player_arrived() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        maneuver(world)
            .is_some_and(|telemetry| telemetry.distance < 80.0 && telemetry.closing_speed < 60.0)
    })
}

/// The live numbers of the player's engaged leg, if there is one.
#[cfg(feature = "debug")]
pub fn maneuver(world: &World) -> Option<&ManeuverTelemetry> {
    world.get::<ManeuverTelemetry>(player_root_ref(world)?)
}

/// Advance once the insertion burn is actually lit.
#[cfg(feature = "debug")]
pub fn orbit_burning() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| autopilot_phase(world) == Some(AutopilotPhase::Burn))
}

/// Advance once the ship is circularized: ORBIT reports `Hold` when the velocity
/// error is inside the hold tolerance (`flight/autopilot.rs`), which is the
/// engine's own definition of "on the ring" and better than any settling time
/// this file could guess.
#[cfg(feature = "debug")]
pub fn orbit_holding() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| autopilot_phase(world) == Some(AutopilotPhase::Hold))
}

/// The phase of the player's engaged maneuver, if there is one.
#[cfg(feature = "debug")]
pub fn autopilot_phase(world: &World) -> Option<AutopilotPhase> {
    Some(world.get::<Autopilot>(player_root_ref(world)?)?.phase)
}

/// Where the ship actually is right now; its spawn point if it has gone.
#[cfg(feature = "debug")]
pub fn ship_position(world: &mut World) -> Vec3 {
    player_root(world)
        .and_then(|player| world.get::<GlobalTransform>(player))
        .map(|transform| transform.translation())
        .unwrap_or_else(start_position)
}

/// The direction the ship is travelling: its velocity, because on a ring the
/// track and the nose are not the same thing (the hull leads its own turn), and
/// the chase camera is a camera on the TRACK. Falls back to the hull's forward
/// while the ship is still at rest.
#[cfg(feature = "debug")]
pub fn ship_heading(world: &mut World) -> Vec3 {
    let Some(player) = player_root(world) else {
        return Vec3::NEG_Z;
    };
    let velocity = world
        .get::<avian3d::prelude::LinearVelocity>(player)
        .map(|velocity| velocity.0)
        .unwrap_or(Vec3::ZERO);
    velocity.try_normalize().unwrap_or_else(|| {
        world
            .get::<GlobalTransform>(player)
            .map(|transform| transform.forward().as_vec3())
            .unwrap_or(Vec3::NEG_Z)
    })
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
