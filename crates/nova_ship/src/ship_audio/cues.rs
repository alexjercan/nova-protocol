//! Cockpit cues: the lock/safety chirps the flight computer answers
//! with, and the dry-fire click on an empty trigger pull. All resolve
//! the PLAYER controller's authored sounds, so a ship without them is
//! silent rather than borrowing another hull's voice.
//!
//! All on [`AudioRoute::Hull`]: these are the player's own computer talking to
//! them, so they ride the world track with everything else their ship makes,
//! and they are never placed, attenuated or panned.

use std::collections::HashMap;

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    AMMO_DRY_VOLUME, DRY_FIRE_VOLUME, LOCK_OFF_VOLUME, LOCK_ON_VOLUME, RADAR_DENY_VOLUME,
    RADAR_RETARGET_VOLUME, SAFETY_ON_VOLUME, WARN_HULL_VOLUME, WARN_LOCK_VOLUME,
};
use crate::{
    prelude::*,
    sections::{
        controller_section::{ControllerSectionHullWarning, ControllerSectionSounds},
        turret_section::TurretSectionDryFireSound,
    },
};

/// The lock-gesture UI cues (non-positional one-shots, like the objective
/// cues): LockOn once per radar gesture ([`RadarLockAcquired`] already fires
/// acquire-only), LockOff per cleared lock, the capability deny buzz
/// ([`RadarDenied`]), and the subtle retarget tick ([`RadarRetargeted`]). One
/// cue per kind per frame - a staged double-clear in one frame plays one
/// LockOff, not a chord. The PLAYER ship's controller sounds: the first
/// controller section whose `ChildOf` parent carries [`PlayerSpaceshipMarker`].
/// The radar/lock/safety messages are player-scoped (no entity payload), so
/// this lookup names the computer whose authored voice plays them. `None` when
/// no player controller exists (menu, editor, tests) - the cues stay silent,
/// and readers must still drain.
fn player_controller_sounds<'a>(
    q_controller: &'a Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player: &Query<(), With<PlayerSpaceshipMarker>>,
) -> Option<&'a ControllerSectionSounds> {
    q_controller
        .iter()
        .find(|(_, ChildOf(ship))| q_player.contains(*ship))
        .map(|(sounds, _)| sounds)
}

pub(super) fn play_lock_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_controller: Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut acquired: MessageReader<RadarLockAcquired>,
    mut retargeted: MessageReader<RadarRetargeted>,
    mut cleared: MessageReader<LockClearedToast>,
    mut denied: MessageReader<RadarDenied>,
) {
    // DRAIN each reader unconditionally (count, not next): a leftover unread
    // message would replay the cue on the NEXT frame - and with no player
    // controller (menu, editor, headless tests) the cues are silent but the
    // cursors must still advance (the old no-bank drain, same reason).
    let acquired_now = acquired.read().count() > 0;
    let retargeted_now = retargeted.read().count() > 0;
    let cleared_now = cleared.read().count() > 0;
    let denied_now = denied.read().count() > 0;
    let Some(sounds) = player_controller_sounds(&q_controller, &q_player) else {
        return;
    };
    // AUTHORED-OR-SILENT: each cue plays the player controller's own authored
    // ref, resolved here; an unauthored cue is silent. Base controllers author
    // all of them via gen_content.
    let mut play = |ref_opt: &Option<AssetRef<AudioSource>>, volume: f32| {
        if let Some(handle) = ref_opt.as_ref().map(|r| r.resolve(&asset_server)) {
            commands.play_sfx(handle, AudioRoute::Hull, volume);
        }
    };
    if acquired_now {
        play(&sounds.lock_on, LOCK_ON_VOLUME);
    }
    // The acquire and a retarget can both land in the frames of one gesture,
    // but never the same frame for the same slot (acquire is the first resolve,
    // retarget every change after). Suppress the tick on the acquire frame
    // anyway so a gesture that resolves and immediately settles plays only the
    // richer LockOn, never LockOn + tick.
    if retargeted_now && !acquired_now {
        play(&sounds.radar_retarget, RADAR_RETARGET_VOLUME);
    }
    if cleared_now {
        play(&sounds.lock_off, LOCK_OFF_VOLUME);
    }
    if denied_now {
        play(&sounds.radar_deny, RADAR_DENY_VOLUME);
    }
}

/// The safety re-engage click on the PLAYER's hot -> cold edge (a held burst
/// must not just silently stop). Changed-gated; the Local remembers the last
/// seen state so an unrelated change (spawn) cannot click.
pub(super) fn play_safety_engaged_cue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_controller: Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player_sounds: Query<(), With<PlayerSpaceshipMarker>>,
    q_player: Query<Ref<WeaponsHot>, (With<PlayerSpaceshipMarker>, Changed<WeaponsHot>)>,
    mut was_hot: Local<bool>,
) {
    for hot in &q_player {
        let is_hot = hot.0;
        // The `Local` is process-global and outlives the ship it tracked: dying
        // while hot left it `true`, and the replacement ship's default `false`
        // read as a hot -> cold edge on its very first frame. A fresh
        // `WeaponsHot` starts from its own state instead.
        if hot.is_added() {
            *was_hot = is_hot;
            continue;
        }
        if *was_hot && !is_hot {
            // AUTHORED-OR-SILENT: the click is the player controller's own
            // authored safety_on ref (the weapons computer's voice).
            if let Some(handle) = player_controller_sounds(&q_controller, &q_player_sounds)
                .and_then(|sounds| sounds.safety_on.as_ref())
                .map(|r| r.resolve(&asset_server))
            {
                commands.play_sfx(handle, AudioRoute::Hull, SAFETY_ON_VOLUME);
            }
        }
        *was_hot = is_hot;
    }
}

/// The dry-fire click on the PLAYER's turrets: when a turret's trigger is held
/// with weapons hot but its magazine is empty, the shoot system silently blocks
/// the shot (`shoot_spawn_projectile`, the empty magazine `continue`). This
/// gives that dead trigger a voice - a dull click on the RISING EDGE of the
/// empty-and-pulling state, so a held burst that runs dry is not just silence.
///
/// Edge-latched per turret so holding an empty trigger clicks once, not every
/// frame; a release-and-re-pull clicks again. Player-only: `q_ship` is filtered
/// to `PlayerSpaceshipMarker`, so an AI turret running dry never reaches the
/// cue (it would otherwise click in the player's ear). A turret with no
/// `SectionAmmo` (unlimited ammo, e.g. the shakedown player) never dry-fires.
/// AUTHORED-OR-SILENT: the click is the turret's own
/// [`TurretSectionConfig::dry_fire_sound`] (snapshotted as
/// [`TurretSectionDryFireSound`], resolved here); a turret that authors none
/// runs dry silently. The edge latch still advances for every turret so an
/// authored sound added later (live edit) does not replay a stale edge.
pub(super) fn play_dry_fire_cue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_turret: Query<
        (
            Entity,
            &TurretSectionInput,
            Option<&SectionAmmo>,
            Option<&TurretSectionDryFireSound>,
            &ChildOf,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_controller: Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_ship: Query<&WeaponsHot, With<PlayerSpaceshipMarker>>,
    mut latched: Local<HashMap<Entity, bool>>,
) {
    // Rebuilt rather than updated in place: a despawned turret's latch would
    // otherwise stay in the map for the rest of the session. Every turret the cue
    // can fire for is visited below, so the new map is exactly the live set.
    let mut live: HashMap<Entity, bool> = HashMap::with_capacity(latched.len());
    let mut any_ran_dry = false;
    for (turret, input, ammo, dry_sound, ChildOf(ship)) in &q_turret {
        // Dry-firing = trigger held, weapons hot, magazine present and empty,
        // on the player's ship. `q_ship` matches only the player, so a
        // non-player parent reads `hot == false` and never dry-fires.
        let hot = q_ship.get(*ship).is_ok_and(|weapons| weapons.0);
        let empty = ammo.is_some_and(SectionAmmo::is_empty);
        let dry = **input && hot && empty;
        let was = latched.get(&turret).copied().unwrap_or(false);
        if dry && !was {
            any_ran_dry = true;
            if let Some(handle) = dry_sound
                .and_then(|s| s.0.as_ref())
                .map(|r| r.resolve(&asset_server))
            {
                commands.play_sfx(handle, AudioRoute::Hull, DRY_FIRE_VOLUME);
            }
        }
        live.insert(turret, dry);
    }
    *latched = live;

    // THE GAUGE, once for the ship however many mounts ran dry on this frame.
    // A broadside is one magazine state, not eight, and eight gauge pips on
    // one frame would be a chord where the panel meant to report a fact. The
    // gun's own click stays per-mount: that one is hardware, out on the mount,
    // and eight of them IS what eight dead triggers sound like.
    //
    // Latched by the same pass, so it follows the guns exactly - including the
    // rule that a held empty trigger clicks once and a re-pull clicks again.
    if !any_ran_dry {
        return;
    }
    let Some(handle) = player_controller_sounds(&q_controller, &q_player)
        .and_then(|sounds| sounds.ammo_dry.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    commands.play_sfx(handle, AudioRoute::Hull, AMMO_DRY_VOLUME);
}

/// The threat alarm: a HOSTILE has this ship in its combat lock.
///
/// Derived from the world rather than reported by the shooter, because a lock
/// is a state somebody holds and not an event they send: an AI that acquires,
/// loses and re-acquires the player over a long fight would otherwise need to
/// remember what it had already announced. Reading the live set every frame
/// and latching the EDGE puts that memory in one place.
///
/// Hostility is the [`Allegiance`] test the rest of combat uses, not "is not
/// the player": a neutral freighter that happens to carry a lock slot is not a
/// threat, and a scripted defection changes the answer for free.
///
/// Player-only. An AI being locked is not news to anybody in the seat, and the
/// alarm is a cockpit instrument - it plays on [`AudioRoute::Hull`] with the
/// rest of the computer's voice, unplaced.
pub(super) fn play_threat_lock_cue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_controller: Query<(&ControllerSectionSounds, &ChildOf)>,
    q_player: Query<(Entity, Option<&Allegiance>), With<PlayerSpaceshipMarker>>,
    q_lockers: Query<(&CombatLock, Option<&Allegiance>)>,
    mut latched: Local<bool>,
) {
    let Some((player, mine)) = q_player.iter().next() else {
        // No player: forget the edge, or re-entering a scenario already under
        // fire would open on a stale latch and never sound the alarm.
        *latched = false;
        return;
    };
    let locked = q_lockers.iter().any(|(lock, theirs)| {
        lock.0 == Some(player) && relation(theirs, mine) == Relation::Hostile
    });
    let was = std::mem::replace(&mut *latched, locked);
    if !locked || was {
        return;
    }
    let Some(handle) = q_controller
        .iter()
        .find(|(_, ChildOf(ship))| *ship == player)
        .and_then(|(sounds, _)| sounds.warn_lock.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    commands.play_sfx(handle, AudioRoute::Hull, WARN_LOCK_VOLUME);
}

/// How far back above its threshold a hull must come before the alarm can
/// sound again.
///
/// Nothing repairs a hull today, so in a fight this only ever prevents a
/// re-trigger from float noise on the aggregate. It exists because the day
/// something does repair one, an alarm sitting exactly on its threshold would
/// chatter, and a hysteresis band is cheaper than finding that out in the seat.
const WARN_HULL_REARM_MARGIN: f32 = 0.05;

/// The hull alarm: this ship is down to the fraction of itself its computer
/// warns at.
///
/// ONE tier, on the falling edge, latched with a rearm band. Several tiers
/// would be a gauge, and a gauge wants a readout to sit next to - there is no
/// hull readout in the HUD at all today, so this alarm IS the integrity
/// instrument and it says one thing.
///
/// The fraction is the aggregate `Health` on the ship ROOT, which
/// `aggregate_ship_health` recomputes every frame as the sum over standing
/// sections against the pinned built maximum. That is the same quantity
/// structural collapse is priced in, so the alarm's threshold and the collapse
/// threshold are directly comparable numbers.
///
/// Silent once the hull has actually COLLAPSED: the peel drives the fraction
/// straight to zero, and an alarm about damage under the sound of the ship
/// coming apart is noise. A one-shot kill from full therefore plays the
/// collapse and not this.
///
/// Player-only and unplaced, like the rest of the computer's voice - an AI's
/// hull integrity is not news to anybody in the seat.
pub(super) fn play_hull_warning_cue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_player: Query<
        (Entity, &Health),
        (
            With<PlayerSpaceshipMarker>,
            With<SpaceshipRootMarker>,
            Without<StructuralCollapseMarker>,
        ),
    >,
    q_controller: Query<(
        &ControllerSectionSounds,
        &ControllerSectionHullWarning,
        &ChildOf,
    )>,
    mut latched: Local<Option<(Entity, bool)>>,
) {
    // Keyed by the ship, not a bare flag: a new scenario is a new hull, and a
    // latch left over from the last one would swallow its first warning.
    let Some((player, health)) = q_player.iter().next() else {
        return;
    };
    // A root mid-spawn has no sections counted yet. Reading it as "zero of
    // zero" would sound the alarm on every ship at birth.
    if health.max <= 0.0 {
        return;
    }
    let Some((sounds, warn_at, _)) = q_controller
        .iter()
        .find(|(_, _, ChildOf(ship))| *ship == player)
    else {
        return;
    };
    let fraction = health.current / health.max;
    let was = latched.and_then(|(ship, warned)| (ship == player).then_some(warned));
    let warned = match was {
        Some(true) => fraction < warn_at.0 + WARN_HULL_REARM_MARGIN,
        _ => fraction < warn_at.0,
    };
    *latched = Some((player, warned));
    // The FALLING edge only. Coming back up through the rearm band is a state
    // change too, and it is not something to announce.
    if !warned || was.unwrap_or(false) {
        return;
    }
    let Some(handle) = sounds.warn_hull.as_ref().map(|r| r.resolve(&asset_server)) else {
        return;
    };
    commands.play_sfx(handle, AudioRoute::Hull, WARN_HULL_VOLUME);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship_audio::test_support::{LastPlayed, PlayedSfx};

    /// App rig for the lock/safety controller cues: the real systems with a
    /// `PlaySfx` capture. No bank - the cues resolve the player controller's
    /// authored refs (authored-or-silent).
    fn controller_cue_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarRetargeted>();
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarDenied>();
        app.add_systems(Update, (play_lock_cues, play_safety_engaged_cue));
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        app
    }

    /// A player ship carrying a controller with the given sounds; returns the
    /// ship entity.
    fn spawn_player_controller(app: &mut App, sounds: ControllerSectionSounds) -> Entity {
        let ship = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        app.world_mut().spawn((sounds, ChildOf(ship)));
        ship
    }

    #[test]
    fn lock_cue_plays_the_player_controllers_authored_sound() {
        // The controller-owned cue path: a lock acquire plays the PLAYER
        // controller's authored lock_on ref. Delivery guard for the silent
        // cases below.
        let mut app = controller_cue_app();
        spawn_player_controller(
            &mut app,
            ControllerSectionSounds {
                lock_on: Some(AssetRef::from("mods/x/sounds/chirp.wav")),
                ..default()
            },
        );
        let expected: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/sounds/chirp.wav");
        app.world_mut()
            .write_message(RadarLockAcquired { combat: true });
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "the player controller's authored lock_on must play"
        );
    }

    #[test]
    fn lock_cues_are_silent_without_a_player_controller_and_still_drain() {
        // No player controller (menu/editor/headless): silent, but the reader
        // cursors MUST advance - a message sent while controller-less must not
        // replay once a controller appears.
        let mut app = controller_cue_app();
        app.world_mut()
            .write_message(RadarLockAcquired { combat: true });
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "no player controller -> silent"
        );

        // Controller arrives AFTER the message was drained: no stale replay.
        spawn_player_controller(
            &mut app,
            ControllerSectionSounds {
                lock_on: Some(AssetRef::from("mods/x/sounds/chirp.wav")),
                ..default()
            },
        );
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "a drained message must not replay when the controller appears"
        );

        // And an unauthored cue on an existing controller stays silent while a
        // different authored cue plays (per-cue authorship, not
        // all-or-nothing).
        app.world_mut().write_message(RadarDenied);
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "unauthored radar_deny -> silent"
        );
        app.world_mut()
            .write_message(RadarLockAcquired { combat: true });
        app.update();
        assert!(
            app.world().resource::<LastPlayed>().0.is_some(),
            "the authored lock_on still plays (delivery guard)"
        );
    }

    #[test]
    fn safety_cue_plays_the_controllers_authored_click_on_hot_to_cold() {
        let mut app = controller_cue_app();
        let ship = spawn_player_controller(
            &mut app,
            ControllerSectionSounds {
                safety_on: Some(AssetRef::from("base/sounds/safety_on.wav")),
                ..default()
            },
        );
        let expected: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/safety_on.wav");
        app.world_mut().entity_mut(ship).insert(WeaponsHot(true));
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "arming is silent"
        );
        app.world_mut().entity_mut(ship).insert(WeaponsHot(false));
        app.update();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "the hot -> cold edge plays the controller's authored click"
        );
    }

    /// An App rig for the dry-fire cue: the real `play_dry_fire_cue` system
    /// with a `PlaySfx` counter, no audio device needed. No bank: the cue is
    /// authored-or-silent, so each test turret authors its own click via
    /// [`dry_click`].
    fn dry_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<PlayedSfx>();
        app.add_systems(Update, play_dry_fire_cue);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app
    }

    /// An authored dry-fire click for test turrets (the base default's path).
    fn dry_click() -> TurretSectionDryFireSound {
        TurretSectionDryFireSound(Some(AssetRef::from("base/sounds/dry_fire.wav")))
    }

    fn dings(app: &App) -> usize {
        app.world().resource::<PlayedSfx>().0
    }

    #[test]
    fn dry_fire_clicks_on_the_empty_pull_edge_then_stays_quiet_while_held() {
        let mut app = dry_fire_app();
        let player = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(true)))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                TurretSectionInput(true),
                SectionAmmo::new(0),
                dry_click(),
                ChildOf(player),
            ))
            .id();

        // Trigger held on an empty magazine: one click on the rising edge.
        app.update();
        assert_eq!(dings(&app), 1, "the empty pull edge clicks once");

        // Still held: no repeat (the latch suppresses a per-frame buzz).
        app.update();
        assert_eq!(
            dings(&app),
            1,
            "holding an empty trigger does not machine-gun"
        );

        // Release then re-pull: a fresh edge clicks again.
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionInput(false));
        app.update();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionInput(true));
        app.update();
        assert_eq!(
            dings(&app),
            2,
            "a re-pull on an empty magazine clicks again"
        );
    }

    #[test]
    fn dry_fire_is_gated_to_the_player_hot_and_empty() {
        // Four turrets in one frame; only the player + hot + empty + held one
        // may click. The `== 1` is self-guarding: it is also the delivery guard
        // that the rig fires at all, so the three silent cases are real gates,
        // not a dead system.
        let mut app = dry_fire_app();
        let player_hot = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(true)))
            .id();
        let player_cold = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(false)))
            .id();
        // An AI ship: hot weapons, but no player marker.
        let ai = app.world_mut().spawn(WeaponsHot(true)).id();

        let held_empty = |app: &mut App, ship: Entity| {
            app.world_mut().spawn((
                TurretSectionMarker,
                TurretSectionInput(true),
                SectionAmmo::new(0),
                dry_click(),
                ChildOf(ship),
            ));
        };
        held_empty(&mut app, player_hot); // valid: clicks (delivery guard)
        held_empty(&mut app, player_cold); // gated: weapons cold
        held_empty(&mut app, ai); // gated: not the player
                                  // Player + hot but a LOADED magazine: gated
                                  // on ammo.
        app.world_mut().spawn((
            TurretSectionMarker,
            TurretSectionInput(true),
            SectionAmmo::new(3),
            dry_click(),
            ChildOf(player_hot),
        ));
        // Player + hot + empty + held, but NO authored dry_fire_sound: gated on
        // authorship (authored-or-silent).
        app.world_mut().spawn((
            TurretSectionMarker,
            TurretSectionInput(true),
            SectionAmmo::new(0),
            TurretSectionDryFireSound(None),
            ChildOf(player_hot),
        ));

        app.update();
        assert_eq!(
            dings(&app),
            1,
            "only the player's hot, empty, held, AUTHORED turret dry-fires"
        );
    }

    #[test]
    fn a_broadside_running_dry_is_eight_clicks_and_one_gauge() {
        // The gun's click is hardware, out on the mount, and eight dead
        // triggers IS eight of them. The gauge is the panel reporting one
        // magazine state, so it must sound ONCE however many mounts starved
        // on the frame.
        let mut app = dry_fire_app();
        let player = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(true)))
            .id();
        app.world_mut().spawn((
            ControllerSectionSounds {
                ammo_dry: Some(AssetRef::from("base/sounds/ammo_dry.wav")),
                ..default()
            },
            ChildOf(player),
        ));
        for _ in 0..8 {
            app.world_mut().spawn((
                TurretSectionMarker,
                TurretSectionInput(true),
                SectionAmmo::new(0),
                dry_click(),
                ChildOf(player),
            ));
        }

        app.update();
        assert_eq!(
            dings(&app),
            9,
            "eight mount clicks plus one gauge pip, not eight pips"
        );

        // Still held: the gauge is latched by the same pass as the guns.
        app.update();
        assert_eq!(dings(&app), 9, "a held dead trigger says nothing more");
    }

    /// App rig for the threat alarm.
    fn threat_lock_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<PlayedSfx>();
        app.add_systems(Update, play_threat_lock_cue);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app
    }

    /// The player's ship with an authored `warn_lock` on its flight computer.
    fn spawn_warned_player(app: &mut App) -> Entity {
        let ship = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, Allegiance::Player))
            .id();
        app.world_mut().spawn((
            ControllerSectionSounds {
                warn_lock: Some(AssetRef::from("base/sounds/warn_lock.wav")),
                ..default()
            },
            ChildOf(ship),
        ));
        ship
    }

    #[test]
    fn the_alarm_sounds_on_the_edge_a_hostile_lock_arrives_and_again_after_it_breaks() {
        // A lock is a state somebody holds, not an event they send, so the
        // system reads the live set and latches. Holding must be silent;
        // losing and re-acquiring must sound again.
        let mut app = threat_lock_app();
        let player = spawn_warned_player(&mut app);
        let raider = app
            .world_mut()
            .spawn((Allegiance::Enemy, CombatLock(None)))
            .id();

        app.update();
        assert_eq!(dings(&app), 0, "nobody is looking at us yet");

        app.world_mut()
            .entity_mut(raider)
            .insert(CombatLock(Some(player)));
        app.update();
        assert_eq!(dings(&app), 1);

        app.update();
        assert_eq!(dings(&app), 1, "a held lock is not news twice");

        app.world_mut().entity_mut(raider).insert(CombatLock(None));
        app.update();
        app.world_mut()
            .entity_mut(raider)
            .insert(CombatLock(Some(player)));
        app.update();
        assert_eq!(dings(&app), 2, "a re-acquire is a new threat");
    }

    #[test]
    fn only_a_hostile_lock_on_the_player_raises_the_alarm() {
        // Hostility is the allegiance test the rest of combat uses: a neutral
        // freighter tracking us is not a threat, and a lock on somebody else
        // is not ours to hear.
        let mut app = threat_lock_app();
        let player = spawn_warned_player(&mut app);
        let bystander = app.world_mut().spawn(Allegiance::Neutral).id();

        app.world_mut()
            .spawn((Allegiance::Neutral, CombatLock(Some(player))));
        app.world_mut()
            .spawn((Allegiance::Enemy, CombatLock(Some(bystander))));
        app.update();
        assert_eq!(dings(&app), 0);

        // Delivery guard: the same rig with a hostile lock does sound.
        app.world_mut()
            .spawn((Allegiance::Enemy, CombatLock(Some(player))));
        app.update();
        assert_eq!(dings(&app), 1);
    }

    /// App rig for the hull alarm.
    fn hull_warning_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<PlayedSfx>();
        app.add_systems(Update, play_hull_warning_cue);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app
    }

    /// The player's ship at full health, with a computer that warns at
    /// `warn_at` and (optionally) authors the alarm.
    fn spawn_hull(app: &mut App, warn_at: f32, authored: bool) -> Entity {
        let ship = app
            .world_mut()
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                Health::new(100.0),
            ))
            .id();
        app.world_mut().spawn((
            ControllerSectionSounds {
                warn_hull: authored.then(|| AssetRef::from("base/sounds/warn_hull.wav")),
                ..default()
            },
            ControllerSectionHullWarning(warn_at),
            ChildOf(ship),
        ));
        ship
    }

    /// Take the hull down to `fraction` of what it was built with and run a
    /// frame - the shape `aggregate_ship_health` writes every tick.
    fn hull_at(app: &mut App, ship: Entity, fraction: f32) {
        app.world_mut().entity_mut(ship).insert(Health {
            current: 100.0 * fraction,
            max: 100.0,
        });
        app.update();
    }

    #[test]
    fn the_hull_alarm_sounds_once_on_the_way_down_through_its_threshold() {
        // ONE tier, on the falling edge. Chip damage below the line must not
        // re-sound it: this is the alarm, not a gauge.
        let mut app = hull_warning_app();
        let ship = spawn_hull(&mut app, 0.30, true);

        hull_at(&mut app, ship, 1.0);
        hull_at(&mut app, ship, 0.31);
        assert_eq!(dings(&app), 0, "still above the line");

        hull_at(&mut app, ship, 0.29);
        assert_eq!(dings(&app), 1);

        hull_at(&mut app, ship, 0.20);
        hull_at(&mut app, ship, 0.06);
        assert_eq!(dings(&app), 1, "a hull that keeps falling says it once");
    }

    #[test]
    fn the_alarm_rearms_only_well_clear_of_the_line_it_tripped_on() {
        // Nothing repairs a hull today, so this band exists for the day
        // something does: coming back to exactly the threshold must not arm a
        // second alarm the next hit would sound.
        let mut app = hull_warning_app();
        let ship = spawn_hull(&mut app, 0.30, true);
        hull_at(&mut app, ship, 0.25);
        assert_eq!(dings(&app), 1);

        hull_at(&mut app, ship, 0.32);
        hull_at(&mut app, ship, 0.25);
        assert_eq!(dings(&app), 1, "inside the band, still latched");

        hull_at(&mut app, ship, 0.50);
        hull_at(&mut app, ship, 0.25);
        assert_eq!(dings(&app), 2, "clear of the band, the alarm is live again");
    }

    #[test]
    fn a_collapsing_hull_leaves_the_alarm_to_the_wreck() {
        // The peel drives the fraction straight to zero, and a damage warning
        // under the sound of the ship coming apart is noise. A one-shot kill
        // from full plays the collapse and not this.
        let mut app = hull_warning_app();
        let ship = spawn_hull(&mut app, 0.30, true);
        hull_at(&mut app, ship, 1.0);

        app.world_mut()
            .entity_mut(ship)
            .insert(StructuralCollapseMarker::default());
        hull_at(&mut app, ship, 0.02);
        assert_eq!(dings(&app), 0);
    }

    #[test]
    fn a_ship_being_born_is_not_a_ship_in_trouble() {
        // A root mid-spawn has no sections counted yet. Read as a fraction that
        // is zero of zero, so every ship would scream at birth.
        let mut app = hull_warning_app();
        let ship = spawn_hull(&mut app, 0.30, true);
        app.world_mut().entity_mut(ship).insert(Health {
            current: 0.0,
            max: 0.0,
        });
        app.update();
        assert_eq!(dings(&app), 0);

        // Delivery guard: the same rig warns once the hull has a maximum.
        hull_at(&mut app, ship, 0.10);
        assert_eq!(dings(&app), 1);
    }

    #[test]
    fn a_computer_that_authors_no_hull_alarm_says_nothing_and_one_that_never_warns_is_allowed() {
        // Authored-or-silent, and the other half of the knob: `0.0` is a
        // computer that warns only when there is nothing left, which is what
        // makes the fraction worth authoring at all.
        let mut app = hull_warning_app();
        let silent = spawn_hull(&mut app, 0.30, false);
        hull_at(&mut app, silent, 0.10);
        assert_eq!(dings(&app), 0, "no authored alarm -> silent");

        let mut app = hull_warning_app();
        let never = spawn_hull(&mut app, 0.0, true);
        hull_at(&mut app, never, 0.01);
        assert_eq!(dings(&app), 0, "a computer authored not to warn does not");
    }

    #[test]
    fn a_ship_that_authors_no_warn_lock_is_locked_in_silence() {
        let mut app = threat_lock_app();
        let ship = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, Allegiance::Player))
            .id();
        app.world_mut()
            .spawn((ControllerSectionSounds::default(), ChildOf(ship)));
        app.world_mut()
            .spawn((Allegiance::Enemy, CombatLock(Some(ship))));
        app.update();
        assert_eq!(dings(&app), 0);
    }
}
