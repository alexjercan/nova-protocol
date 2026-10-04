//! The flight prediction against the flight it predicts.

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
    let goal = Vec3::new(0.0, 0.0, -5000.0);
    let well_center = Vec3::new(95.0, 0.0, -300.0);
    let gravity = GravitySettings::default();
    let well_data = nova_gameplay::gravity::GravityWell::from_mass(8000.0, 40.0, &gravity);
    // (verb is GOTO, off-axis well present, shipped-scale turn authority)
    for (goto, with_well, low_turn_authority) in [
        (true, false, false),
        (true, true, false),
        (true, false, true),
        (true, true, true),
        (false, false, true),
    ] {
        let case = format!("goto={goto} well={with_well} low_turn={low_turn_authority}");
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
                Transform::from_translation(well_center),
                well_data.clone(),
            ));
        }
        let (ship, _, controller) = spawn_ship(&mut app);
        if low_turn_authority {
            app.world_mut()
                .get_mut::<PDController>(controller)
                .unwrap()
                .max_angular_acceleration = 0.5;
        }
        app.world_mut()
            .entity_mut(ship)
            .insert(PlayerSpaceshipMarker);
        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(goal),
                GlobalTransform::from(Transform::from_translation(goal)),
            ))
            .id();
        settle(&mut app);
        let action = if goto {
            AutopilotAction::Goto { target }
        } else {
            // Coasting sideways and fast: STOP flips the hull through 90
            // degrees at the slow rate, brakes on the drive and settles on
            // the RCS.
            app.world_mut()
                .entity_mut(ship)
                .insert(LinearVelocity(Vec3::new(30.0, 0.0, 0.0)));
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
        let mut order_braked = false;
        let mut was_braking = false;
        let mut braking_entries = 0;
        let mut reengaged_at = None;
        for update in 1..=8000 {
            app.update();
            entered_well |= app.world().get::<DominantWell>(ship).is_some();
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
        assert!(
            app.world().get::<FlightPrediction>(ship).is_none(),
            "{case}: a released leg keeps no prediction"
        );
        // The completing tick flies from the state after the update before it.
        let live_end = released_at - 1;

        let mut max_point_error = 0.0f32;
        let mut max_tick_error = 0.0f32;
        let mut covered = vec![false; live_end + 1];
        for prediction in &predictions {
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
        assert!(
            covered[1..].iter().all(|&covered| covered),
            "{case}: predictions must cover every flown tick after the first"
        );
        assert!(
            max_tick_error <= 1.0,
            "{case}: the drawn prediction strays {max_tick_error} u from the flown path"
        );
    }
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
    // paused run took frames.
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
}
