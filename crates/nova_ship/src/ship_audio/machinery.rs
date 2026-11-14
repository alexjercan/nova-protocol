//! The ship's moving parts: the sounds a hull makes when it is not shooting.
//!
//! Doors - the retractable PDC housing, the torpedo bay's muzzle iris, the
//! cargo intake's accordion and the mining emitter's lids - and the intake's
//! drop and take. The seam is
//! deliberately the same as the weapons': the mechanism reports, content
//! authors the sound, and this module is the only place that turns the two
//! into a voice.
//!
//! Every cue plays at the mechanism, on its hull. The category sets the level
//! and the reach, and its repeat interval and voice cap hold per ship, so
//! three bays opening on one hull are one iris.

use bevy::prelude::*;
use nova_sound::prelude::*;

use crate::sections::{
    cargo_intake_section::{
        CargoCanisterEjected, CargoCanisterTaken, CargoIntakeDoorMoved,
        CargoIntakeSectionConfigHelper,
    },
    docking_section::{DockClamped, DockReleased, DockingSectionConfigHelper},
    mining_section::{MiningDoorsMoved, MiningSectionConfigHelper},
    torpedo_section::{TorpedoBayDoorsMoved, TorpedoSectionDoorSound},
    turret_section::{TurretSectionStowSounds, TurretStowDoorsMoved},
};

/// One machinery cue at `section`, which carries the voice's ship.
fn play_at(
    commands: &mut Commands,
    sound: Handle<AudioSource>,
    category: SoundCategoryType,
    section: Entity,
) {
    commands.trigger(PlaySound {
        sound,
        category,
        at: SoundAt::Entity(section),
    });
}

/// The housing servo, on the frame the lids are told to move.
///
/// AUTHORED-OR-SILENT per DIRECTION: a mount may voice its rise, its fold, both
/// or neither.
pub(super) fn on_stow_doors_play_sfx(
    moved: On<TurretStowDoorsMoved>,
    asset_server: Res<AssetServer>,
    q_sounds: Query<&TurretSectionStowSounds>,
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
    play_at(&mut commands, handle, SoundCategoryType::Machinery, turret);
}

/// The bay's muzzle iris, on the frame the petals are told to move.
///
/// ONE authored file for both directions, unlike the housing above: the iris is
/// one servo turning one way or the other, where the housing's rise and fold
/// are two mechanisms.
///
/// A doorless bay never reports, so the authored-or-silent test here is only
/// about a bay that HAS an iris and chose not to voice it.
pub(super) fn on_bay_doors_play_sfx(
    moved: On<TorpedoBayDoorsMoved>,
    asset_server: Res<AssetServer>,
    q_sounds: Query<&TorpedoSectionDoorSound>,
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
    play_at(&mut commands, handle, SoundCategoryType::Machinery, bay);
}

/// The intake's door servo, on the frame the door is told to move. One
/// required file for both directions, like the bay's iris.
pub(super) fn on_intake_door_play_sfx(
    moved: On<CargoIntakeDoorMoved>,
    asset_server: Res<AssetServer>,
    q_intakes: Query<&CargoIntakeSectionConfigHelper>,
    mut commands: Commands,
) {
    let intake = moved.entity;
    let Ok(config) = q_intakes.get(intake) else {
        return;
    };
    play_at(
        &mut commands,
        config.door_sound.resolve(&asset_server),
        SoundCategoryType::Machinery,
        intake,
    );
}

/// The mining emitter's lids, on the frame they are told to move. Two
/// required files, one per direction, like the PDC housing's pair.
pub(super) fn on_mining_doors_play_sfx(
    moved: On<MiningDoorsMoved>,
    asset_server: Res<AssetServer>,
    q_emitters: Query<&MiningSectionConfigHelper>,
    mut commands: Commands,
) {
    let emitter = moved.entity;
    let Ok(config) = q_emitters.get(emitter) else {
        return;
    };
    let sound = if moved.opening {
        &config.door_open_sound
    } else {
        &config.door_close_sound
    };
    play_at(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Machinery,
        emitter,
    );
}

/// A canister leaving the intake, at the intake.
pub(super) fn on_canister_ejected_play_sfx(
    ejected: On<CargoCanisterEjected>,
    asset_server: Res<AssetServer>,
    q_intakes: Query<&CargoIntakeSectionConfigHelper>,
    mut commands: Commands,
) {
    let intake = ejected.entity;
    let Ok(config) = q_intakes.get(intake) else {
        return;
    };
    play_at(
        &mut commands,
        config.eject_sound.resolve(&asset_server),
        SoundCategoryType::Machinery,
        intake,
    );
}

/// A canister taken into the hold, at the intake that took it.
pub(super) fn on_canister_taken_play_sfx(
    taken: On<CargoCanisterTaken>,
    asset_server: Res<AssetServer>,
    q_intakes: Query<&CargoIntakeSectionConfigHelper>,
    mut commands: Commands,
) {
    let intake = taken.entity;
    let Ok(config) = q_intakes.get(intake) else {
        return;
    };
    play_at(
        &mut commands,
        config.take_sound.resolve(&asset_server),
        SoundCategoryType::Pickup,
        intake,
    );
}

/// The clamp closing, at the port of the hull that pressed `DOCK`, the
/// frame the joint holds. AUTHORED-OR-SILENT: the port's own `clamp_sound`.
/// A request echoed on one frame plays once through the category's repeat
/// interval, which holds per hull.
pub(super) fn on_dock_clamped_play_sfx(
    clamped: On<DockClamped>,
    asset_server: Res<AssetServer>,
    q_ports: Query<&DockingSectionConfigHelper>,
    mut commands: Commands,
) {
    let port = clamped.entity;
    let Some(sound) = q_ports.get(port).ok().and_then(|c| c.clamp_sound.as_ref()) else {
        return;
    };
    play_at(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Machinery,
        port,
    );
}

/// The clamp letting go, at the port of the hull that asked to be free.
/// AUTHORED-OR-SILENT: the port's own `release_sound`. The damage releases
/// never report, so they never reach here.
pub(super) fn on_dock_released_play_sfx(
    released: On<DockReleased>,
    asset_server: Res<AssetServer>,
    q_ports: Query<&DockingSectionConfigHelper>,
    mut commands: Commands,
) {
    let port = released.entity;
    let Some(sound) = q_ports
        .get(port)
        .ok()
        .and_then(|c| c.release_sound.as_ref())
    else {
        return;
    };
    play_at(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Machinery,
        port,
    );
}

#[cfg(test)]
mod tests {
    use nova_events::units::prelude::*;
    use nova_gameplay::prelude::*;

    use super::{super::test_support::LastPlayed, *};
    use crate::sections::{
        cargo_intake_section::{preview_cargo_intake_section, CargoIntakeSectionConfig},
        docking_section::{preview_docking_section, DockingSectionConfig},
        mining_section::{preview_mining_section, MiningSectionConfig},
    };

    /// A live turret with the given authored pair, on a ship of its own.
    fn app_with(open: Option<&str>, close: Option<&str>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_stow_doors_play_sfx);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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

    fn move_iris(app: &mut App, bay: Entity, opening: bool) {
        app.world_mut().trigger(TorpedoBayDoorsMoved {
            entity: bay,
            opening,
        });
        app.world_mut().flush();
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

    #[test]
    fn the_cargo_intake_voices_its_door_drop_and_take_where_authored() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_observer(on_intake_door_play_sfx);
        app.add_observer(on_canister_ejected_play_sfx);
        app.add_observer(on_canister_taken_play_sfx);
        #[derive(Resource, Default)]
        struct Heard(Vec<(Handle<AudioSource>, SoundCategoryType, SoundAt)>);
        app.init_resource::<Heard>();
        app.add_observer(|ev: On<PlaySound>, mut heard: ResMut<Heard>| {
            heard.0.push((ev.sound.clone(), ev.category, ev.at));
        });
        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let point = Vec3::new(3.0, -1.0, 2.0);
        let intake = app
            .world_mut()
            .spawn((
                preview_cargo_intake_section(CargoIntakeSectionConfig {
                    render_mesh: AssetRef::from("intake.glb#Scene0"),
                    render_mesh_transform: None,
                    canister_mesh: AssetRef::from("canister.glb#Scene0"),
                    door_sound: AssetRef::from("base/sounds/bay_door.wav"),
                    eject_sound: AssetRef::from("base/sounds/cargo_eject.wav"),
                    take_sound: AssetRef::from("base/sounds/salvage_pickup.wav"),
                    detection_range: Meters(40.0),
                    capture_gap: Meters(1.0),
                    aperture_width: Meters(14.1),
                    aperture_height: Meters(16.0),
                    eject_speed: MetersPerSecond(3.0),
                }),
                GlobalTransform::from_translation(point),
                ChildOf(ship),
            ))
            .id();

        for opening in [true, false] {
            app.world_mut().trigger(CargoIntakeDoorMoved {
                entity: intake,
                opening,
            });
        }
        app.world_mut()
            .trigger(CargoCanisterEjected { entity: intake });
        app.world_mut().trigger(CargoCanisterTaken {
            entity: intake,
            stacks: Vec::new(),
        });
        app.world_mut().flush();

        let server = app.world().resource::<AssetServer>().clone();
        let door = server.load("base/sounds/bay_door.wav");
        let at = SoundAt::Entity(intake);
        assert_eq!(
            app.world().resource::<Heard>().0,
            vec![
                (door.clone(), SoundCategoryType::Machinery, at),
                (door, SoundCategoryType::Machinery, at),
                (
                    server.load("base/sounds/cargo_eject.wav"),
                    SoundCategoryType::Machinery,
                    at
                ),
                (
                    server.load("base/sounds/salvage_pickup.wav"),
                    SoundCategoryType::Pickup,
                    at
                ),
            ]
        );
    }

    #[test]
    fn each_mining_door_direction_plays_its_own_file_at_its_own_level_at_the_section() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_observer(on_mining_doors_play_sfx);
        #[derive(Resource, Default)]
        struct Heard(Vec<(Handle<AudioSource>, SoundCategoryType, SoundAt)>);
        app.init_resource::<Heard>();
        app.add_observer(|ev: On<PlaySound>, mut heard: ResMut<Heard>| {
            heard.0.push((ev.sound.clone(), ev.category, ev.at));
        });
        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let point = Vec3::new(-2.0, 4.0, 1.0);
        let emitter = app
            .world_mut()
            .spawn((
                preview_mining_section(MiningSectionConfig {
                    render_mesh: AssetRef::from("mining.glb#Scene0"),
                    render_mesh_transform: None,
                    pulse_sound: AssetRef::from("base/sounds/mining_pulse.wav"),
                    refusal_sound: AssetRef::from("base/sounds/radar_deny.wav"),
                    door_open_sound: AssetRef::from("base/sounds/mining_door_open.wav"),
                    door_close_sound: AssetRef::from("base/sounds/mining_door_close.wav"),
                    reach: Meters(100.0),
                    pulse_interval_seconds: 1.0,
                    carve_radius_cells: 1.5,
                }),
                GlobalTransform::from_translation(point),
                ChildOf(ship),
            ))
            .id();

        for opening in [true, false] {
            app.world_mut().trigger(MiningDoorsMoved {
                entity: emitter,
                opening,
            });
        }
        app.world_mut().flush();

        let server = app.world().resource::<AssetServer>().clone();
        let at = SoundAt::Entity(emitter);
        assert_eq!(
            app.world().resource::<Heard>().0,
            vec![
                (
                    server.load("base/sounds/mining_door_open.wav"),
                    SoundCategoryType::Machinery,
                    at
                ),
                (
                    server.load("base/sounds/mining_door_close.wav"),
                    SoundCategoryType::Machinery,
                    at
                ),
            ]
        );
    }

    /// The clamp and the release each play the port's own authored file at
    /// the port, and a port that authors neither stays silent.
    #[test]
    fn the_clamp_and_the_release_play_the_ports_authored_files_at_the_port() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_observer(on_dock_clamped_play_sfx);
        app.add_observer(on_dock_released_play_sfx);
        #[derive(Resource, Default)]
        struct Heard(Vec<(Handle<AudioSource>, SoundCategoryType, SoundAt)>);
        app.init_resource::<Heard>();
        app.add_observer(|ev: On<PlaySound>, mut heard: ResMut<Heard>| {
            heard.0.push((ev.sound.clone(), ev.category, ev.at));
        });
        let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
        let partner = app.world_mut().spawn(SpaceshipRootMarker).id();
        let point = Vec3::new(1.0, 2.0, -3.0);
        let voiced = app
            .world_mut()
            .spawn((
                preview_docking_section(DockingSectionConfig {
                    clamp_sound: Some(AssetRef::from("base/sounds/dock_clamp.wav")),
                    release_sound: Some(AssetRef::from("base/sounds/dock_release.wav")),
                    ..default()
                }),
                GlobalTransform::from_translation(point),
                ChildOf(ship),
            ))
            .id();
        let silent = app
            .world_mut()
            .spawn((
                preview_docking_section(DockingSectionConfig::default()),
                GlobalTransform::from_translation(point),
                ChildOf(partner),
            ))
            .id();

        for port in [voiced, silent] {
            app.world_mut().trigger(DockClamped {
                entity: port,
                partner,
            });
            app.world_mut().trigger(DockReleased {
                entity: port,
                partner,
            });
        }
        app.world_mut().flush();

        let server = app.world().resource::<AssetServer>().clone();
        let at = SoundAt::Entity(voiced);
        assert_eq!(
            app.world().resource::<Heard>().0,
            vec![
                (
                    server.load("base/sounds/dock_clamp.wav"),
                    SoundCategoryType::Machinery,
                    at
                ),
                (
                    server.load("base/sounds/dock_release.wav"),
                    SoundCategoryType::Machinery,
                    at
                ),
            ]
        );
    }
}
