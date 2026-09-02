//! The ship's moving parts: the sounds a hull makes when it is not shooting.
//!
//! Doors, so far - the retractable PDC housing and the torpedo bay's muzzle
//! iris - and the seam is deliberately the same as the weapons': the mechanism
//! reports, content authors the sound, and this module is the only place that
//! turns the two into a voice.
//!
//! Both cues are placed at the mechanism and routed by whose hull it is on, so
//! a raider folding its guns two hundred meters away is heard from over there
//! or not at all.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    routing::route_for, BAY_DOOR_CLOSE_VOLUME, BAY_DOOR_OPEN_VOLUME, STOW_CLOSE_VOLUME,
    STOW_OPEN_VOLUME,
};
use crate::sections::{
    torpedo_section::{TorpedoBayDoorsMoved, TorpedoSectionDoorSound},
    turret_section::{TurretSectionStowSounds, TurretStowDoorsMoved},
};

/// The housing servo, on the frame the lids are told to move.
///
/// AUTHORED-OR-SILENT per DIRECTION: a mount may voice its rise, its fold, both
/// or neither. The two directions carry different gains because they are
/// different recordings - matching their linear numbers would not match their
/// loudness, which is the trap the rest of this table already fell into once.
pub(super) fn on_stow_doors_play_sfx(
    moved: On<TurretStowDoorsMoved>,
    asset_server: Res<AssetServer>,
    q_sounds: Query<&TurretSectionStowSounds>,
    q_where: Query<&GlobalTransform>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    let turret = moved.entity;
    let Ok(sounds) = q_sounds.get(turret) else {
        return;
    };
    let authored = if moved.opening {
        &sounds.open
    } else {
        &sounds.close
    };
    let Some(handle) = authored.as_ref().map(|r| r.resolve(&asset_server)) else {
        return;
    };
    let Ok(at) = q_where.get(turret) else {
        return;
    };
    let volume = if moved.opening {
        STOW_OPEN_VOLUME
    } else {
        STOW_CLOSE_VOLUME
    };
    let route = route_for(turret, &q_child_of, &q_is_root, &q_is_player);
    commands.play_sfx_at(handle, route, volume, at.translation());
}

/// The bay's muzzle iris, on the frame the petals are told to move.
///
/// ONE authored file for both directions, unlike the housing above: the iris is
/// one servo turning one way or the other, where the housing's rise and fold
/// are two mechanisms. The levels still differ - petals seating is a heavier
/// event than petals unseating - which is a gain decision, not a second sound.
///
/// A doorless bay never reports, so the authored-or-silent test here is only
/// about a bay that HAS an iris and chose not to voice it.
pub(super) fn on_bay_doors_play_sfx(
    moved: On<TorpedoBayDoorsMoved>,
    asset_server: Res<AssetServer>,
    q_sounds: Query<&TorpedoSectionDoorSound>,
    q_where: Query<&GlobalTransform>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    let bay = moved.entity;
    let Some(handle) = q_sounds
        .get(bay)
        .ok()
        .and_then(|sound| sound.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let Ok(at) = q_where.get(bay) else {
        return;
    };
    let volume = if moved.opening {
        BAY_DOOR_OPEN_VOLUME
    } else {
        BAY_DOOR_CLOSE_VOLUME
    };
    let route = route_for(bay, &q_child_of, &q_is_root, &q_is_player);
    commands.play_sfx_at(handle, route, volume, at.translation());
}

#[cfg(test)]
mod tests {
    use super::{super::test_support::LastPlayed, *};

    /// A live turret with the given authored pair, on a ship of its own.
    fn app_with(open: Option<&str>, close: Option<&str>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_stow_doors_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionStowSounds {
                    open: open.map(AssetRef::from),
                    close: close.map(AssetRef::from),
                },
                GlobalTransform::default(),
                ChildOf(ship),
            ))
            .id();
        (app, turret)
    }

    fn move_doors(app: &mut App, turret: Entity, opening: bool) {
        app.world_mut().trigger(TurretStowDoorsMoved {
            entity: turret,
            opening,
        });
        app.world_mut().flush();
    }

    #[test]
    fn each_direction_plays_the_side_its_housing_authored() {
        // The pair is not one sound played twice: a housing rising and a
        // housing folding are two files, and the event's direction picks.
        let (mut app, turret) = app_with(
            Some("base/sounds/pdc_stow_open.wav"),
            Some("base/sounds/pdc_stow_close.wav"),
        );
        let server = app.world().resource::<AssetServer>().clone();

        move_doors(&mut app, turret, true);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(server.load("base/sounds/pdc_stow_open.wav")),
            "parting lids must play the open side"
        );

        move_doors(&mut app, turret, false);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(server.load("base/sounds/pdc_stow_close.wav")),
            "shutting lids must play the close side"
        );
    }

    #[test]
    fn a_housing_that_authors_only_one_side_is_silent_in_the_other() {
        // Authored-or-silent per direction. The authored half is the delivery
        // guard for the silent half.
        let (mut app, turret) = app_with(Some("base/sounds/pdc_stow_open.wav"), None);

        move_doors(&mut app, turret, true);
        assert!(
            app.world().resource::<LastPlayed>().0.is_some(),
            "the authored side still plays"
        );

        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        move_doors(&mut app, turret, false);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "the unauthored side is silent, not a fallback to the other"
        );
    }

    /// A live torpedo bay with the given authored iris sound, on its own ship.
    fn bay_app(door: Option<&str>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_bay_doors_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let bay = app
            .world_mut()
            .spawn((
                TorpedoSectionDoorSound(door.map(AssetRef::from)),
                GlobalTransform::default(),
                ChildOf(ship),
            ))
            .id();
        (app, bay)
    }

    /// Every `PlaySfx` the iris rig sees, as `(handle, volume)`.
    fn move_iris(app: &mut App, bay: Entity, opening: bool) {
        app.world_mut().trigger(TorpedoBayDoorsMoved {
            entity: bay,
            opening,
        });
        app.world_mut().flush();
    }

    #[test]
    fn one_iris_file_plays_both_ways_at_two_different_levels() {
        // The counterpart to the housing above, and deliberately the other
        // answer: one servo, one recording, and the DIRECTION is a gain
        // decision rather than a second file.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_observer(on_bay_doors_play_sfx);
        #[derive(Resource, Default)]
        struct Heard(Vec<(Handle<AudioSource>, f32)>);
        app.init_resource::<Heard>();
        app.add_observer(|ev: On<PlaySfx>, mut heard: ResMut<Heard>| {
            heard.0.push((ev.handle.clone(), ev.volume));
        });
        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let bay = app
            .world_mut()
            .spawn((
                TorpedoSectionDoorSound(Some(AssetRef::from("base/sounds/bay_door.wav"))),
                GlobalTransform::default(),
                ChildOf(ship),
            ))
            .id();

        move_iris(&mut app, bay, true);
        move_iris(&mut app, bay, false);

        let heard = &app.world().resource::<Heard>().0;
        assert_eq!(heard.len(), 2, "each command to the petals is one cue");
        assert_eq!(heard[0].0, heard[1].0, "one iris is one recording");
        assert!(
            (heard[0].1 - BAY_DOOR_OPEN_VOLUME).abs() < 1e-6
                && (heard[1].1 - BAY_DOOR_CLOSE_VOLUME).abs() < 1e-6,
            "petals seating is the heavier event, got {:?}",
            (heard[0].1, heard[1].1)
        );
    }

    #[test]
    fn a_bay_that_names_no_iris_sound_opens_in_silence() {
        // Authored-or-silent, and the guard that matters for the cut-cube pods:
        // they report nothing, and a bay that DOES report without a sound must
        // not fall through to another section's voice.
        let (mut app, bay) = bay_app(None);
        move_iris(&mut app, bay, true);
        assert_eq!(app.world().resource::<LastPlayed>().0, None);
    }
}
