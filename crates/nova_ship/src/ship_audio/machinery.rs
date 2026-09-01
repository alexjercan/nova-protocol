//! The ship's moving parts: the sounds a hull makes when it is not shooting.
//!
//! One cue today - the retractable PDC housing - and the seam is deliberately
//! the same as the weapons': the mechanism reports, content authors the sound,
//! and this module is the only place that turns the two into a voice.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{routing::route_for, STOW_CLOSE_VOLUME, STOW_OPEN_VOLUME};
use crate::sections::turret_section::{TurretSectionStowSounds, TurretStowDoorsMoved};

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
}
