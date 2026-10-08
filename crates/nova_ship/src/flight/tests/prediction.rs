//! The flight prediction against the flight it predicts.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use nova_gameplay::{
    prelude::*,
    test_support::{settle, unfinished_integrity_physics_app},
};

use super::support::*;
use crate::{
    flight::{
        prediction::{predict_flight_path, FlightPredictionRun},
        NovaFlightPlugin,
    },
    prelude::*,
    sections::thruster_section::thruster_impulse_system,
};

#[test]
fn flight_prediction_follows_the_flown_center_of_mass_within_one_unit() {
    // Every leg flies one fixed physics tick per update with the production
    // predictor after avian's writeback. Each published prediction is laid
    // over the live centre of mass at every fixed tick it spans, the drawn
    // polyline between its points included. The GOTO legs outlast the 30 s
    // horizon, so their early predictions end at it. A GOTO order consumes
    // its FLIP on its first braking tick and keeps it consumed through every
    // replan; the same GOTO re-engaged on that tick is a new order that
    // consumes its own FLIP on its own braking tick. STOP never consumes one.
    // A shifted ballast makes the single off-axis drive torque the hull; a
    // separate STOP starts inbound under the live gravity of a nearby well.
    let goal = Vec3::new(0.0, 0.0, -5000.0);
    let well_center = Vec3::new(95.0, 0.0, -300.0);
    let gravity = GravitySettings::default();
    let well_data = nova_gameplay::gravity::GravityWell::from_mass(8000.0, 40.0, &gravity);
    // (GOTO, well, slow turn, shifted COM, STOP inside the well)
    for (goto, with_well, low_turn_authority, shifted_com, stop_in_well) in [
        (true, false, false, false, false),
        (true, true, false, false, false),
        (true, false, true, false, false),
        (true, true, true, false, false),
        (false, false, true, false, false),
        (true, false, false, true, false),
        (false, true, false, false, true),
    ] {
        let case = format!(
            "goto={goto} well={with_well} low_turn={low_turn_authority} \
             shifted_com={shifted_com} stop_in_well={stop_in_well}"
        );
        let mut app = orbit_app();
        let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
        let dt = fixed_step.as_secs_f32();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
        app.add_systems(
            FixedPostUpdate,
            predict_flight_path.after(PhysicsSystems::Writeback),
        );
        if with_well {
            app.world_mut().spawn((
                RigidBody::Static,
                Transform::from_translation(if stop_in_well {
                    Vec3::ZERO
                } else {
                    well_center
                }),
                well_data.clone(),
            ));
        }
        let (ship, controller) = if shifted_com {
            let (ship, _) = spawn_damage_shifted_single_drive(&mut app, false);
            (ship, None)
        } else {
            let (ship, _, controller) = spawn_ship(&mut app);
            (ship, Some(controller))
        };
        if low_turn_authority {
            let controller = controller.expect("the slow-turn case has a controller");
            app.world_mut()
                .get_mut::<PDController>(controller)
                .unwrap()
                .max_angular_acceleration = 0.5;
        }
        app.world_mut()
            .entity_mut(ship)
            .insert(PlayerSpaceshipMarker);
        if stop_in_well {
            app.world_mut()
                .entity_mut(ship)
                .insert(Transform::from_xyz(0.0, 0.0, 150.0));
        }
        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(goal),
                GlobalTransform::from(Transform::from_translation(goal)),
            ))
            .id();
        settle(&mut app);
        if shifted_com {
            let com = app.world().get::<ComputedCenterOfMass>(ship).unwrap().0;
            assert!(
                com.x > 0.5,
                "{case}: the ballast must shift the local COM off the drive"
            );
        }
        if stop_in_well {
            // Fall from rest before ordering the brake. The gravity plugin
            // opts every ship root into real gravity.
            for _ in 0..300 {
                app.update();
            }
            let entry_speed = velocity_of(&app, ship).length();
            assert!(
                entry_speed > 1.0,
                "{case}: gravity must accelerate the ship before STOP"
            );
            assert!(
                app.world().get::<DominantWell>(ship).is_some(),
                "{case}: the well must own the live gravity pull"
            );
        }
        let action = if goto {
            AutopilotAction::Goto { target }
        } else {
            // The free-flight STOP coasts sideways before its slow flip.
            // The well STOP instead brakes an inbound gravity-driven fall.
            if !stop_in_well {
                app.world_mut()
                    .entity_mut(ship)
                    .insert(LinearVelocity(Vec3::new(30.0, 0.0, 0.0)));
            }
            AutopilotAction::Stop
        };
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(action));

        let com_of = |app: &App| {
            let position = app.world().get::<Position>(ship).unwrap().0;
            let rotation = app.world().get::<Rotation>(ship).unwrap();
            let com = app.world().get::<ComputedCenterOfMass>(ship).unwrap().0;
            rotation.mul_vec3(com) + position
        };
        // Index u is the state after update u; index 0 is the engaged state.
        let mut coms = vec![com_of(&app)];
        let mut elapsed = vec![app.world().resource::<Time<Fixed>>().elapsed()];
        let mut predictions: Vec<FlightPrediction> = Vec::new();
        let mut released_at = None;
        let mut entered_well = false;
        let mut max_spin = 0.0f32;
        let mut order_braked = false;
        let mut was_braking = false;
        let mut braking_entries = 0;
        let mut reengaged_at = None;
        for update in 1..=8000 {
            app.update();
            entered_well |= app.world().get::<DominantWell>(ship).is_some();
            max_spin = max_spin.max(angular_speed_of(&app, ship));
            coms.push(com_of(&app));
            elapsed.push(app.world().resource::<Time<Fixed>>().elapsed());
            if let Some(autopilot) = app.world().get::<Autopilot>(ship) {
                let braking = app
                    .world()
                    .get::<ManeuverTelemetry>(ship)
                    .is_some_and(|numbers| numbers.braking);
                braking_entries += usize::from(braking && !was_braking);
                was_braking = braking;
                order_braked |= goto && braking;
                assert_eq!(
                    autopilot.flip_marker_consumed, order_braked,
                    "{case}: after update {update} the order's FLIP is consumed exactly when \
                     its leg has braked"
                );
                // GOTO reads the phase only as output, so re-engaging the same
                // leg changes nothing the ship flies or the predictor seeds.
                if order_braked && reengaged_at.is_none() {
                    reengaged_at = Some(update);
                    order_braked = false;
                    app.world_mut()
                        .entity_mut(ship)
                        .insert(Autopilot::engage(action));
                }
            }
            if let Some(prediction) = app.world().get::<FlightPrediction>(ship) {
                if predictions
                    .last()
                    .is_none_or(|last| last.seed_time != prediction.seed_time)
                {
                    predictions.push(prediction.clone());
                }
            }
            if app.world().get::<Autopilot>(ship).is_none() {
                released_at = Some(update);
                break;
            }
        }
        let released_at = released_at.unwrap_or_else(|| panic!("{case}: the leg must complete"));
        assert_eq!(
            reengaged_at.is_some() && order_braked,
            goto,
            "{case}: a GOTO consumes its FLIP, and so does its re-engagement"
        );
        assert_eq!(
            entered_well, with_well,
            "{case}: the ship must fly through only the present well's SOI"
        );
        if shifted_com {
            assert!(
                max_spin > 0.01,
                "{case}: the off-axis drive must torque the live hull"
            );
        }
        assert!(
            app.world().get::<FlightPrediction>(ship).is_none(),
            "{case}: a released leg keeps no prediction"
        );
        // The completing tick flies from the state after the update before it.
        let live_end = released_at - 1;

        let (max_point_error, max_tick_error) = assert_predictions_follow_the_flown_center_of_mass(
            &case,
            &predictions,
            &coms,
            &elapsed,
            live_end,
            dt,
        );
        println!(
            "{case}: {} predictions over {live_end} ticks; max error at points {max_point_error} \
             u, at every tick {max_tick_error} u; {braking_entries} braking entries, \
             re-engaged at {reengaged_at:?}",
            predictions.len()
        );
        if goto {
            assert!(
                predictions
                    .iter()
                    .any(|prediction| prediction.end == FlightPredictionEndType::Horizon),
                "{case}: the leg must outlast the horizon"
            );
        }
    }
}

/// Lays each prediction over the live centre of mass at every fixed tick it
/// spans, the drawn polyline between its points included: `coms[u]` is the
/// state after update `u`, at `Time<Fixed>` elapsed `elapsed[u]`, and
/// `live_end` is the last tick the leg flew. Asserts how each prediction ends
/// and when its last point is timed, that the predictions cover every flown
/// tick after the first, and that the drawn path stays within one unit.
/// Returns the largest error at the points and at every tick.
fn assert_predictions_follow_the_flown_center_of_mass(
    case: &str,
    predictions: &[FlightPrediction],
    coms: &[Vec3],
    elapsed: &[Duration],
    live_end: usize,
    dt: f32,
) -> (f32, f32) {
    let mut max_point_error = 0.0f32;
    let mut max_tick_error = 0.0f32;
    let mut covered = vec![false; live_end + 1];
    for prediction in predictions {
        let seed = elapsed
            .iter()
            .position(|&time| time == prediction.seed_time)
            .unwrap_or_else(|| panic!("{case}: a prediction seeds on a flown tick"));
        let last = prediction.points.len() - 1;
        let regular_end = seed + 8 * last;
        let end_tick = match prediction.end {
            FlightPredictionEndType::Horizon => {
                assert!(
                    live_end >= regular_end,
                    "{case}: the leg seeded at {seed} is predicted to fly past tick \
                     {regular_end} but completed at {live_end}"
                );
                regular_end
            }
            FlightPredictionEndType::Completed => {
                assert!(
                    live_end <= regular_end && live_end + 8 > regular_end,
                    "{case}: the leg seeded at {seed} is predicted to complete by tick \
                     {regular_end} but completed at {live_end}"
                );
                live_end
            }
            end => panic!("{case}: the leg seeded at {seed} is predicted to end {end:?}"),
        };
        assert_eq!(
            prediction.final_point_time,
            (end_tick - seed) as f32 * dt,
            "{case}: the last point of the leg seeded at {seed} is timed at the tick it \
             was flown at"
        );
        if let Some(flip) = prediction.flip_index {
            assert!(flip <= last, "{case}: the flip index names a point");
        }
        let tick_of = |index: usize| {
            if index == last {
                end_tick
            } else {
                seed + 8 * index
            }
        };
        for index in 0..=last {
            max_point_error =
                max_point_error.max(prediction.points[index].distance(coms[tick_of(index)]));
        }
        for tick in seed..=end_tick {
            let index = (0..last)
                .find(|&index| tick <= tick_of(index + 1))
                .unwrap_or(last);
            let drawn = if index == last {
                prediction.points[last]
            } else {
                let (from, to) = (tick_of(index), tick_of(index + 1));
                let t = (tick - from) as f32 / (to - from) as f32;
                prediction.points[index].lerp(prediction.points[index + 1], t)
            };
            max_tick_error = max_tick_error.max(drawn.distance(coms[tick]));
            covered[tick] = true;
        }
    }
    assert!(
        covered[1..].iter().all(|&covered| covered),
        "{case}: predictions must cover every flown tick after the first"
    );
    assert!(
        max_tick_error <= 1.0,
        "{case}: the drawn prediction strays {max_tick_error} u from the flown path"
    );
    (max_point_error, max_tick_error)
}

#[test]
fn a_goto_at_a_well_body_is_predicted_to_park_where_the_live_leg_hands_off_to_orbit() {
    // The playtest GOTO at a well body from outside its SOI, inbound at speed,
    // flown one fixed tick per update. A leg predicted to park ends on the
    // state the live autopilot parks from. ORBIT is not a predicted leg, so
    // the handoff drops the prediction and nothing draws the ring. The
    // braking GOTO consumes its FLIP; the ORBIT it parks into is a new order
    // and never consumes one.
    let mut app = orbit_app();
    let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
    let dt = fixed_step.as_secs_f32();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
    app.add_systems(
        FixedPostUpdate,
        predict_flight_path.after(PhysicsSystems::Writeback),
    );
    let gravity = GravitySettings::default();
    let well = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            nova_gameplay::gravity::GravityWell::from_mass(8000.0, 40.0, &gravity),
            BodyRadius(40.0),
        ))
        .id();
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert((PlayerSpaceshipMarker, Transform::from_xyz(0.0, 0.0, 500.0)));
    settle(&mut app);
    app.world_mut().entity_mut(ship).insert((
        LinearVelocity(Vec3::new(0.0, 0.0, -25.0)),
        Autopilot::engage(AutopilotAction::Goto { target: well }),
    ));

    let com_of = |app: &App| {
        let position = app.world().get::<Position>(ship).unwrap().0;
        let rotation = app.world().get::<Rotation>(ship).unwrap();
        let com = app.world().get::<ComputedCenterOfMass>(ship).unwrap().0;
        rotation.mul_vec3(com) + position
    };
    // Index u is the state after update u; index 0 is the engaged state.
    let mut coms = vec![com_of(&app)];
    let mut elapsed = vec![app.world().resource::<Time<Fixed>>().elapsed()];
    let mut predictions: Vec<FlightPrediction> = Vec::new();
    let mut parked_at = None;
    let mut goto_consumed = false;
    for update in 1..=6000 {
        app.update();
        coms.push(com_of(&app));
        elapsed.push(app.world().resource::<Time<Fixed>>().elapsed());
        match app.world().get::<Autopilot>(ship).copied() {
            Some(Autopilot {
                action: AutopilotAction::Orbit { .. },
                ..
            }) => {
                parked_at = Some(update);
                break;
            }
            Some(autopilot) => goto_consumed = autopilot.flip_marker_consumed,
            None => panic!("a GOTO at a well body must park, not release"),
        }
        if let Some(prediction) = app.world().get::<FlightPrediction>(ship) {
            if predictions
                .last()
                .is_none_or(|last| last.seed_time != prediction.seed_time)
            {
                predictions.push(prediction.clone());
            }
        }
    }
    let parked_at = parked_at.expect("the GOTO must hand off to ORBIT in budget");
    assert!(
        goto_consumed,
        "the inbound GOTO consumes its FLIP before it parks"
    );
    assert!(
        app.world().get::<FlightPrediction>(ship).is_none()
            && app.world().get::<FlightPredictionRun>(ship).is_none(),
        "the handoff to ORBIT drops the GOTO prediction"
    );
    // The parking tick flies from the state after the update before it.
    let live_end = parked_at - 1;

    let mut parks = 0;
    let mut off_grid_parks = 0;
    for prediction in &predictions {
        let seed = elapsed
            .iter()
            .position(|&time| time == prediction.seed_time)
            .expect("a prediction seeds on a flown tick");
        match prediction.end {
            FlightPredictionEndType::Horizon => continue,
            FlightPredictionEndType::ParkedIntoOrbit => parks += 1,
            end => panic!("the leg seeded at {seed} is predicted to end {end:?}"),
        }
        let last = prediction.points.len() - 1;
        let regular_end = seed + 8 * last;
        assert!(
            live_end <= regular_end && live_end + 8 > regular_end,
            "the leg seeded at {seed} is predicted to park by tick {regular_end} but parked \
             from {live_end}"
        );
        assert_eq!(
            prediction.final_point_time,
            (live_end - seed) as f32 * dt,
            "the last point of the leg seeded at {seed} is timed at the tick it parks from"
        );
        if (live_end - seed) % 8 != 0 {
            off_grid_parks += 1;
        }
        let error = prediction.points[last].distance(coms[live_end]);
        assert!(
            error <= 1.0,
            "the leg seeded at {seed} is predicted to park {error} u from where it parked"
        );
    }
    println!(
        "{} predictions, {parks} parking ({off_grid_parks} between two samples), over \
         {live_end} ticks",
        predictions.len()
    );
    assert!(
        parks > 0,
        "the leg must be predicted to park before it does"
    );
    assert!(
        off_grid_parks > 0,
        "a predicted park must end between two samples to time an off-grid last point"
    );

    for _ in 0..240 {
        app.update();
        assert!(
            matches!(
                app.world().get::<Autopilot>(ship).copied(),
                Some(Autopilot {
                    action: AutopilotAction::Orbit { .. },
                    flip_marker_consumed: false,
                    ..
                })
            ),
            "the parked ship keeps flying ORBIT and inherits no consumed FLIP"
        );
        assert!(
            app.world().get::<FlightPrediction>(ship).is_none(),
            "ORBIT is never predicted"
        );
    }
}

#[test]
fn a_goto_ordered_on_the_paused_map_is_predicted_before_the_clock_resumes() {
    // The shipped plugin, so the registrations under test are the game's. The
    // map orders a GOTO in Update while TAB holds both clocks paused: no fixed
    // tick runs, yet the prediction publishes from the frozen state. Unpaused,
    // the ship flies the published path and each later run advances on the
    // fixed tick alone, so it publishes as many ticks after its seed as the
    // paused run took frames. A non-finite paused seed then stays failed
    // until a different order or a fixed tick gives it a new seed.
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        PDControllerPlugin,
        NovaFlightPlugin,
        ControllerSectionPlugin { render: false },
    ));
    // The thruster plugin carries render-material deps, so the impulse system
    // is registered directly, as the flight harness does.
    app.add_systems(
        FixedUpdate,
        thruster_impulse_system.in_set(SpaceshipSectionSystems),
    );
    app.finish();
    let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(PlayerSpaceshipMarker);
    let goal = Vec3::new(0.0, 0.0, -5000.0);
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(goal),
            GlobalTransform::from(Transform::from_translation(goal)),
        ))
        .id();
    settle(&mut app);

    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.world_mut().resource_mut::<Time<Physics>>().pause();
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    let com_of = |app: &App| {
        let position = app.world().get::<Position>(ship).unwrap().0;
        let rotation = app.world().get::<Rotation>(ship).unwrap();
        let com = app.world().get::<ComputedCenterOfMass>(ship).unwrap().0;
        rotation.mul_vec3(com) + position
    };
    let state_of = |app: &App| {
        (
            app.world().get::<Position>(ship).unwrap().0,
            app.world().get::<LinearVelocity>(ship).unwrap().0,
            app.world().resource::<Time<Fixed>>().elapsed(),
        )
    };
    let frozen = state_of(&app);
    let frozen_com = com_of(&app);
    let mut paused_frames = 0;
    while app.world().get::<FlightPrediction>(ship).is_none() {
        assert!(paused_frames < 64, "the paused GOTO must be predicted");
        app.update();
        paused_frames += 1;
        assert_eq!(
            state_of(&app),
            frozen,
            "the paused frame must not fly the ship"
        );
    }
    let prediction = app.world().get::<FlightPrediction>(ship).unwrap().clone();
    assert_eq!(
        prediction.seed_time, frozen.2,
        "the paused prediction seeds on the frozen tick"
    );
    assert_eq!(prediction.points[0], frozen_com);
    assert_eq!(
        prediction.end,
        FlightPredictionEndType::Horizon,
        "the leg outlasts the horizon"
    );
    println!("the paused GOTO is predicted after {paused_frames} frames");

    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    app.world_mut().resource_mut::<Time<Physics>>().unpause();
    for _ in 0..8 {
        app.update();
        assert!(
            app.world().get::<FlightPrediction>(ship).is_some(),
            "the GOTO keeps a prediction while it flies"
        );
    }
    assert_eq!(
        app.world().resource::<Time<Fixed>>().elapsed(),
        frozen.2 + fixed_step * 8,
        "each unpaused update runs one fixed tick"
    );
    let error = prediction.points[1].distance(com_of(&app));
    assert!(
        error <= 1.0,
        "the ship flies the paused prediction; {error} u off its second point"
    );
    for _ in 8..paused_frames {
        app.update();
        assert!(
            app.world().get::<FlightPrediction>(ship).is_some(),
            "the GOTO keeps a prediction while it flies"
        );
    }
    assert_eq!(
        app.world().resource::<Time<Fixed>>().elapsed(),
        frozen.2 + fixed_step * paused_frames,
        "each unpaused update runs one fixed tick"
    );
    let reseeded = app.world().get::<FlightPrediction>(ship).unwrap();
    assert_ne!(reseeded.seed_time, frozen.2, "the flying leg republishes");
    assert_eq!(
        app.world().resource::<Time<Fixed>>().elapsed() - reseeded.seed_time,
        fixed_step * (paused_frames - 1),
        "an unpaused run advances on the fixed tick only"
    );

    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::new(300.0, 0.0, -600.0),
        }));
    app.update();
    assert!(
        app.world().get::<FlightPrediction>(ship).is_none(),
        "a new leg drops the old leg's prediction at once"
    );
    let changed_at = app.world().resource::<Time<Fixed>>().elapsed() - fixed_step;
    for _ in 0..paused_frames {
        app.update();
    }
    let prediction = app
        .world()
        .get::<FlightPrediction>(ship)
        .expect("the new leg is predicted");
    assert!(
        prediction.seed_time >= changed_at,
        "the new leg's prediction seeds after the change"
    );

    // A bad predicted acceleration must keep a failure sentinel. The clock
    // stays frozen: retrying this seed on each paused frame would repeat the
    // same non-finite calculation without flying the ship.
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.world_mut().resource_mut::<Time<Physics>>().pause();
    let frozen = state_of(&app);
    let original_gravity = app.world().resource::<Gravity>().0;
    app.world_mut().resource_mut::<Gravity>().0 = Vec3::splat(f32::NAN);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::new(-5000.0, 0.0, -600.0),
        }));
    app.update();
    assert_eq!(state_of(&app), frozen, "the bad seed must not fly");
    assert!(
        app.world().get::<FlightPredictionRun>(ship).is_some(),
        "a non-finite prediction keeps its failure sentinel"
    );
    assert!(
        app.world().get::<FlightPrediction>(ship).is_none(),
        "a non-finite new leg hides the old prediction"
    );
    let failed_tick = app
        .world()
        .entity(ship)
        .get_change_ticks::<FlightPredictionRun>()
        .unwrap()
        .changed;
    for _ in 0..24 {
        app.update();
        assert_eq!(state_of(&app), frozen, "paused retries must not fly");
        assert!(
            app.world().get::<FlightPredictionRun>(ship).is_some(),
            "the failed seed stays recorded"
        );
        assert_eq!(
            app.world()
                .entity(ship)
                .get_change_ticks::<FlightPredictionRun>()
                .unwrap()
                .changed,
            failed_tick,
            "the same frozen seed must not be recalculated"
        );
        assert!(app.world().get::<FlightPrediction>(ship).is_none());
    }
    app.world_mut().resource_mut::<Gravity>().0 = original_gravity;
    app.update();
    assert_eq!(state_of(&app), frozen);
    assert!(
        app.world().get::<FlightPrediction>(ship).is_none(),
        "a fixed seed does not retry even when the acceleration becomes valid"
    );

    // A different order is a new seed even without a live tick.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::new(5000.0, 0.0, -600.0),
        }));
    for _ in 0..64 {
        if app.world().get::<FlightPrediction>(ship).is_some() {
            break;
        }
        app.update();
    }
    assert_eq!(state_of(&app), frozen);
    assert_eq!(
        app.world()
            .get::<FlightPrediction>(ship)
            .expect("a different order retries")
            .seed_time,
        frozen.2
    );

    // A failed seed also retries when the fixed clock advances while its
    // action stays unchanged. Keep gravity invalid only for the paused seed.
    app.world_mut().resource_mut::<Gravity>().0 = Vec3::splat(f32::NAN);
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: Vec3::new(0.0, 0.0, -5000.0),
        }));
    app.update();
    assert!(app.world().get::<FlightPredictionRun>(ship).is_some());
    assert!(app.world().get::<FlightPrediction>(ship).is_none());
    app.world_mut().resource_mut::<Gravity>().0 = original_gravity;
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    app.world_mut().resource_mut::<Time<Physics>>().unpause();
    for _ in 0..64 {
        if app.world().get::<FlightPrediction>(ship).is_some() {
            break;
        }
        app.update();
    }
    let retried = app
        .world()
        .get::<FlightPrediction>(ship)
        .expect("a later fixed tick retries");
    assert!(
        retried.seed_time > frozen.2,
        "the retry must seed on a later tick"
    );
}

#[test]
fn a_goto_to_a_coasting_target_is_predicted_along_the_target_flight() {
    // The predictor flies the target ahead as avian moves it with no input:
    // it coasts, a well pulls it at its origin, and it tumbles about its
    // centre of mass. Each case flies one fixed physics tick per update to
    // completion, and every published prediction is laid over the live centre
    // of mass at every fixed tick it spans.
    let gravity = GravitySettings::default();
    let well_data = nova_gameplay::gravity::GravityWell::from_mass(8000.0, 40.0, &gravity);
    // In the well's fade band, so the pull bends a slow coast without
    // dragging the target out of reach.
    let side_well = Vec3::new(160.0, 0.0, -120.0);
    // Deep in the well, where a goal error grows into units of flown path.
    let deep_well = Vec3::new(100.0, 0.0, -120.0);
    let near_well = Vec3::new(0.0, 0.0, -120.0);
    let toward_well = Vec3::new(0.0, 0.0, 0.5);
    let spin = Vec3::new(0.0, 0.6, 0.0);
    // (case, target, well, start, velocity, spin)
    for (case, kind, well, start, velocity, spin) in [
        (
            "spinning offset-COM hull in flat space",
            "hull",
            None,
            Vec3::new(0.0, 0.0, -200.0),
            Vec3::new(0.8, 0.0, 0.0),
            spin,
        ),
        (
            "spinning offset-COM hull deep in a well",
            "hull",
            Some(deep_well),
            near_well,
            toward_well,
            spin,
        ),
        (
            "ship coasting through a well",
            "ship",
            Some(side_well),
            near_well,
            toward_well,
            Vec3::ZERO,
        ),
        (
            "rock coasting through a well",
            "rock",
            Some(side_well),
            near_well,
            toward_well,
            Vec3::ZERO,
        ),
        (
            "tumbling offset-COM rock in a well",
            "offset rock",
            Some(side_well),
            near_well,
            toward_well,
            Vec3::new(0.1, 0.2, 0.0),
        ),
    ] {
        let mut app = orbit_app();
        let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
        let dt = fixed_step.as_secs_f32();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
        app.add_systems(
            FixedPostUpdate,
            predict_flight_path.after(PhysicsSystems::Writeback),
        );
        if let Some(well_center) = well {
            app.world_mut().spawn((
                RigidBody::Static,
                Transform::from_translation(well_center),
                well_data.clone(),
            ));
        }
        let (ship, _, _) = spawn_ship(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(PlayerSpaceshipMarker);
        let target = match kind {
            "rock" => app
                .world_mut()
                .spawn((
                    RigidBody::Dynamic,
                    Collider::sphere(5.0),
                    BodyRadius(5.0),
                    GravityAffected,
                ))
                .id(),
            // The goal is the origin, which the tumble carries around the
            // centre of mass.
            "offset rock" => app
                .world_mut()
                .spawn((
                    RigidBody::Dynamic,
                    Collider::compound(vec![(
                        Vec3::new(2.0, 0.0, 0.0),
                        Quat::IDENTITY,
                        Collider::cuboid(4.0, 2.0, 3.0),
                    )]),
                    BodyRadius(8.0),
                    GravityAffected,
                ))
                .id(),
            "hull" => {
                // A derelict: the shifted ballast hull without its controller,
                // so nothing holds its attitude against the spin.
                let (target, _) = spawn_damage_shifted_single_drive(&mut app, false);
                let controllers: Vec<Entity> = app
                    .world_mut()
                    .query_filtered::<(Entity, &ChildOf), With<ControllerSectionMarker>>()
                    .iter(app.world())
                    .filter(|(_, parent)| parent.parent() == target)
                    .map(|(controller, _)| controller)
                    .collect();
                for controller in controllers {
                    app.world_mut().despawn(controller);
                }
                target
            }
            _ => spawn_ship(&mut app).0,
        };
        app.world_mut()
            .entity_mut(target)
            .insert(Transform::from_translation(start));
        settle(&mut app);
        app.world_mut()
            .entity_mut(target)
            .insert((LinearVelocity(velocity), AngularVelocity(spin)));
        let start_rotation = app.world().get::<Rotation>(target).unwrap().0;
        if spin != Vec3::ZERO {
            let com = app.world().get::<ComputedCenterOfMass>(target).unwrap().0;
            assert!(
                com.length() > 0.5,
                "{case}: the target's local COM must sit off its origin"
            );
        }
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));

        let com_of = |app: &App| {
            let position = app.world().get::<Position>(ship).unwrap().0;
            let rotation = app.world().get::<Rotation>(ship).unwrap();
            let com = app.world().get::<ComputedCenterOfMass>(ship).unwrap().0;
            rotation.mul_vec3(com) + position
        };
        // Index u is the state after update u; index 0 is the engaged state.
        let mut coms = vec![com_of(&app)];
        let mut elapsed = vec![app.world().resource::<Time<Fixed>>().elapsed()];
        let mut predictions: Vec<FlightPrediction> = Vec::new();
        let mut released_at = None;
        let mut target_in_well = false;
        for update in 1..=8000 {
            app.update();
            target_in_well |= app.world().get::<DominantWell>(target).is_some();
            coms.push(com_of(&app));
            elapsed.push(app.world().resource::<Time<Fixed>>().elapsed());
            if let Some(prediction) = app.world().get::<FlightPrediction>(ship) {
                if predictions
                    .last()
                    .is_none_or(|last| last.seed_time != prediction.seed_time)
                {
                    predictions.push(prediction.clone());
                }
            }
            if app.world().get::<Autopilot>(ship).is_none() {
                released_at = Some(update);
                break;
            }
        }
        let released_at = released_at.unwrap_or_else(|| panic!("{case}: the leg must complete"));
        let end_velocity = velocity_of(&app, target);
        let turned = app
            .world()
            .get::<Rotation>(target)
            .unwrap()
            .0
            .angle_between(start_rotation);
        let bend = end_velocity.angle_between(velocity);
        println!(
            "{case}: completed at {released_at}; target moved {} u, bent {bend} rad, turned \
             {turned} rad, in a well: {target_in_well}; {} predictions",
            position_of(&app, target).distance(start),
            predictions.len()
        );
        assert!(
            position_of(&app, target).distance(start) > 1.0,
            "{case}: the target must move"
        );
        assert_eq!(
            target_in_well,
            well.is_some(),
            "{case}: only the present well must pull the target"
        );
        if well.is_some() {
            assert!(bend > 0.05, "{case}: the well must bend the target's coast");
        }
        if spin != Vec3::ZERO {
            assert!(turned > 0.5, "{case}: the target must turn");
        }
        // The completing tick flies from the state after the update before it.
        let (max_point_error, max_tick_error) = assert_predictions_follow_the_flown_center_of_mass(
            case,
            &predictions,
            &coms,
            &elapsed,
            released_at - 1,
            dt,
        );
        println!(
            "{case}: max error at points {max_point_error} u, at every tick {max_tick_error} u"
        );
    }
}

#[test]
fn a_target_course_change_past_the_delta_v_tolerance_reseeds_the_prediction() {
    // A coasting ship target in flat space. Each path is checked against the
    // target flown from its own seed: a change that a later run already
    // includes still removes an earlier path once its own residual passes
    // 0.01 u/s, and a change past it in one step reseeds the run.
    let mut app = orbit_app();
    let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
    app.add_systems(
        FixedPostUpdate,
        predict_flight_path.after(PhysicsSystems::Writeback),
    );
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(PlayerSpaceshipMarker);
    let (target, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(target)
        .insert(Transform::from_xyz(0.0, 0.0, -800.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(target)
        .insert(LinearVelocity(Vec3::new(0.5, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    let elapsed = |app: &App| app.world().resource::<Time<Fixed>>().elapsed();
    let seed = |app: &App| {
        app.world()
            .get::<FlightPrediction>(ship)
            .map(|prediction| prediction.seed_time)
    };
    let kick = |app: &mut App, delta_v: f32| {
        app.world_mut().get_mut::<LinearVelocity>(target).unwrap().0 += Vec3::Y * delta_v;
    };
    let until_published = |app: &mut App| {
        (0..200)
            .find_map(|_| {
                app.update();
                seed(app)
            })
            .expect("a coasting target must be predicted")
    };

    let shown = until_published(&mut app);
    kick(&mut app, 0.006);
    app.update();
    assert_eq!(
        seed(&app),
        Some(shown),
        "a 0.006 u/s change keeps the path inside the tolerance"
    );
    // The run seeded on that tick includes the first change.
    let run_seed = elapsed(&app);
    kick(&mut app, 0.006);
    app.update();
    assert_eq!(
        seed(&app),
        None,
        "0.012 u/s since the shown path's seed removes it"
    );
    assert_eq!(
        until_published(&mut app),
        run_seed,
        "the run that includes the first change is 0.006 u/s off and is kept"
    );
    kick(&mut app, 0.02);
    app.update();
    let kicked_at = elapsed(&app);
    assert_eq!(seed(&app), None, "a 0.02 u/s change removes the path");
    assert!(
        until_published(&mut app) >= kicked_at,
        "the next path is seeded after the change"
    );
}

#[test]
fn a_target_turning_under_its_flight_computer_reseeds_the_prediction() {
    // A coasting ship target whose live flight computer holds its attitude.
    // A turn command is torque the coast does not model: the shown path is
    // removed, every later path is seeded after the command, and one is shown
    // again once the target holds its new attitude.
    let mut app = orbit_app();
    let fixed_step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fixed_step));
    app.add_systems(
        FixedPostUpdate,
        predict_flight_path.after(PhysicsSystems::Writeback),
    );
    let (ship, _, _) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(ship)
        .insert(PlayerSpaceshipMarker);
    let (target, _, target_controller) = spawn_ship(&mut app);
    app.world_mut()
        .entity_mut(target)
        .insert(Transform::from_xyz(0.0, 0.0, -800.0));
    settle(&mut app);
    app.world_mut()
        .entity_mut(target)
        .insert(LinearVelocity(Vec3::new(0.5, 0.0, 0.0)));
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
    let seed = |app: &App| {
        app.world()
            .get::<FlightPrediction>(ship)
            .map(|prediction| prediction.seed_time)
    };
    let shown = (0..200)
        .find_map(|_| {
            app.update();
            seed(&app)
        })
        .expect("a target that holds its attitude must be predicted");
    let start_rotation = app.world().get::<Rotation>(target).unwrap().0;

    let commanded_at = app.world().resource::<Time<Fixed>>().elapsed();
    app.world_mut()
        .entity_mut(target_controller)
        .insert(ControllerSectionRotationInput(Quat::from_rotation_y(
            std::f32::consts::FRAC_PI_2,
        )));
    let mut removed = false;
    let mut shown_again = None;
    for _ in 0..2000 {
        app.update();
        match seed(&app) {
            None => removed = true,
            Some(seed) if seed > commanded_at => {
                shown_again = Some(seed);
                break;
            }
            Some(seed) => assert!(
                !removed && seed == shown,
                "a path seeded before the turn command must not be shown again"
            ),
        }
    }
    let turned = app
        .world()
        .get::<Rotation>(target)
        .unwrap()
        .0
        .angle_between(start_rotation);
    println!("turned {turned} rad; removed: {removed}; shown again: {shown_again:?}");
    assert!(removed, "the turn must remove the shown path");
    assert!(turned > 1.0, "the flight computer must turn the target");
    assert!(
        shown_again.is_some(),
        "a path is shown again once the target holds its new attitude"
    );
}
