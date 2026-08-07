//! Cockpit cues: the lock/safety chirps the flight computer answers
//! with, and the dry-fire click on an empty trigger pull. All resolve
//! the PLAYER controller's authored sounds, so a ship without them is
//! silent rather than borrowing another hull's voice.

use std::collections::HashMap;

use bevy::prelude::*;

use super::{
    DRY_FIRE_VOLUME, LOCK_OFF_VOLUME, LOCK_ON_VOLUME, RADAR_DENY_VOLUME, RADAR_RETARGET_VOLUME,
    SAFETY_ON_VOLUME,
};
use crate::{
    prelude::*,
    sections::{
        controller_section::ControllerSectionSounds, turret_section::TurretSectionDryFireSound,
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
            commands.play_sfx_volume(handle, volume);
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
                commands.play_sfx_volume(handle, SAFETY_ON_VOLUME);
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
    q_ship: Query<&WeaponsHot, With<PlayerSpaceshipMarker>>,
    mut latched: Local<HashMap<Entity, bool>>,
) {
    // Rebuilt rather than updated in place: a despawned turret's latch would
    // otherwise stay in the map for the rest of the session. Every turret the cue
    // can fire for is visited below, so the new map is exactly the live set.
    let mut live: HashMap<Entity, bool> = HashMap::with_capacity(latched.len());
    for (turret, input, ammo, dry_sound, ChildOf(ship)) in &q_turret {
        // Dry-firing = trigger held, weapons hot, magazine present and empty,
        // on the player's ship. `q_ship` matches only the player, so a
        // non-player parent reads `hot == false` and never dry-fires.
        let hot = q_ship.get(*ship).is_ok_and(|weapons| weapons.0);
        let empty = ammo.is_some_and(SectionAmmo::is_empty);
        let dry = **input && hot && empty;
        let was = latched.get(&turret).copied().unwrap_or(false);
        if dry && !was {
            if let Some(handle) = dry_sound
                .and_then(|s| s.0.as_ref())
                .map(|r| r.resolve(&asset_server))
            {
                commands.play_sfx_volume(handle, DRY_FIRE_VOLUME);
            }
        }
        live.insert(turret, dry);
    }
    *latched = live;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::test_support::{LastPlayed, PlayedSfx};

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
}
