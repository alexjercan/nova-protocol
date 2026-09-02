//! Static cross-checks over the built `shakedown_run` config: id wiring,
//! objective coverage, geometry and ship grades - no app, no simulation.

use super::*;

/// runtime (a handler that never fires). Cross-check: every id any
/// filter matches on or any despawn targets is spawned by some action
/// (object, area, or the two lazily spawned beacons and pirate).
#[test]
fn every_referenced_id_is_spawned() {
    let config = scenario();

    let mut spawned: Vec<String> = Vec::new();
    for action in all_actions(&config) {
        match action {
            EventActionConfig::SpawnScenarioObject(object) => {
                spawned.push(object.base.id.clone());
            }
            EventActionConfig::CreateScenarioArea(area) => {
                spawned.push(area.id.clone());
            }
            _ => {}
        }
    }

    let mut referenced: Vec<String> = Vec::new();
    for event in &config.events {
        for filter in &event.filters {
            if let EventFilterConfig::Entity(entity) = filter {
                referenced.extend(entity.id.clone());
                referenced.extend(entity.other_id.clone());
            }
        }
    }
    for action in all_actions(&config) {
        match action {
            EventActionConfig::DespawnScenarioObject(despawn) => {
                referenced.push(despawn.id.clone());
            }
            // Marker targets are id strings too - a typo'd attach is a
            // silently missing marker.
            EventActionConfig::ObjectiveMarkerAttach(attach) => {
                referenced.push(attach.target_id.clone());
            }
            EventActionConfig::ObjectiveMarkerDetach(detach) => {
                referenced.push(detach.target_id.clone());
            }
            _ => {}
        }
    }

    for id in &referenced {
        assert!(
            spawned.contains(id),
            "id '{}' is referenced by the script but never spawned; spawned: {:?}",
            id,
            spawned
        );
    }
}

/// The conveyance choreography, pinned at the config level: every leg's target
/// is marked, hand-offs detach the previous marker, an attach that shares a
/// handler with its target's spawn comes AFTER the spawn (actions queue in list
/// order - an attach before the spawn resolves nothing), and the beat-4 GOTO
/// emphasis is cleared by the orbit handler.
#[test]
fn the_marker_rides_every_leg_and_hands_off() {
    let config = scenario();

    // Handler index -> (attach targets, detach targets) in order. A marker a
    // beat sets counts as the handler's: the chain belongs to the handler that
    // started it.
    let marker_ops = |event: &ScenarioEventConfig| {
        let mut attaches = Vec::new();
        let mut detaches = Vec::new();
        for action in &event.actions {
            action.walk(&mut |action| match action {
                EventActionConfig::ObjectiveMarkerAttach(attach) => {
                    attaches.push(attach.target_id.clone());
                }
                EventActionConfig::ObjectiveMarkerDetach(detach) => {
                    detaches.push(detach.target_id.clone());
                }
                _ => {}
            });
        }
        (attaches, detaches)
    };

    // The opening conversation hands off to objective 1: nothing is marked in
    // the OnStart FRAME (beacon 1 spawns lazily, on the hand-off beat of the
    // opening chain), and one handler's chain both spawns and marks beacon 1.
    let on_start = config
        .events
        .iter()
        .find(|event| matches!(event.name, EventConfig::OnStart))
        .unwrap();
    assert!(
        !on_start
            .actions
            .iter()
            .any(|action| matches!(action, EventActionConfig::ObjectiveMarkerAttach(_))),
        "OnStart marks nothing while the captain briefs"
    );
    let beacon_1_handler = config
        .events
        .iter()
        .find(|event| marker_ops(event).0.iter().any(|id| id == ID_BEACON_1))
        .expect("some handler marks beacon 1 after the opening");
    assert_eq!(
        marker_ops(beacon_1_handler).0,
        vec![ID_BEACON_1.to_string()]
    );

    // Attach-after-spawn ordering: in every FRAME that both spawns an object
    // and attaches a marker to it, the spawn comes first. A beat is a frame of
    // its own, and the hand-off beat is exactly this shape.
    for group in config
        .events
        .iter()
        .flat_map(ScenarioEventConfig::action_groups)
    {
        let mut spawned_so_far: Vec<&str> = Vec::new();
        // Ids spawned by OTHER frames before this one can run are not
        // checkable statically; restrict the ordering assert to ids this same
        // frame spawns.
        let spawned_here: Vec<String> = group
            .iter()
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => Some(object.base.id.clone()),
                _ => None,
            })
            .collect();
        for action in group {
            match action {
                EventActionConfig::SpawnScenarioObject(object) => {
                    spawned_so_far.push(object.base.id.as_str());
                }
                EventActionConfig::ObjectiveMarkerAttach(attach)
                    if spawned_here.contains(&attach.target_id) =>
                {
                    assert!(
                        spawned_so_far.contains(&attach.target_id.as_str()),
                        "attach to '{}' precedes its spawn in the same frame",
                        attach.target_id
                    );
                }
                _ => {}
            }
        }
    }

    // Hand-offs down the v2 leg chain: beacon 1 -> beacon 2 -> crates
    // -> beacon 3 -> beacon 4 -> planetoid -> derelict -> pirate ->
    // done (each attach handler detaches the previous leg's marker;
    // the crate markers die with their crates).
    let handler_with_attach = |target: &str| {
        config
            .events
            .iter()
            .find(|event| marker_ops(event).0.iter().any(|id| id == target))
            .unwrap_or_else(|| panic!("some handler attaches to '{}'", target))
    };
    assert_eq!(
        marker_ops(handler_with_attach(ID_BEACON_2)).1,
        vec![ID_BEACON_1.to_string()]
    );
    let crates_handler = handler_with_attach("crate_1");
    assert_eq!(marker_ops(crates_handler).1, vec![ID_BEACON_2.to_string()]);
    assert_eq!(
        marker_ops(crates_handler).0,
        vec!["crate_1", "crate_2", "crate_3"]
    );
    assert_eq!(
        marker_ops(handler_with_attach(ID_BEACON_3)).1,
        Vec::<String>::new()
    );
    assert_eq!(
        marker_ops(handler_with_attach(ID_BEACON_4)).1,
        vec![ID_BEACON_3.to_string()]
    );
    assert_eq!(
        marker_ops(handler_with_attach(ID_PLANETOID)).1,
        vec![ID_BEACON_4.to_string()]
    );
    assert_eq!(
        marker_ops(handler_with_attach(ID_DERELICT)).1,
        vec![ID_PLANETOID.to_string()]
    );
    assert_eq!(
        marker_ops(handler_with_attach(ID_PIRATE)).1,
        vec![ID_DERELICT.to_string()]
    );
    // The chain's last link is the KILL handler, which completes the fight
    // objective and drops the scavenger's marker. The run's completion note
    // is no longer part of it: it rides the outro's banner beat seconds
    // later, so the two live in different handlers now.
    let kill_handler = config
        .events
        .iter()
        .find(|event| {
            event.actions.iter().any(|action| {
                matches!(action, EventActionConfig::ObjectiveComplete(objective) if objective.id == OBJ_B12)
            })
        })
        .unwrap();
    assert_eq!(marker_ops(kill_handler).1, vec![ID_PIRATE.to_string()]);

    // Emphasis pairing: every emphasized verb is cleared downstream
    // (teardown covers death, but the happy path must not rely on it).
    // v2 sequences: RADAR for the first lock (cleared when it lands),
    // GOTO for the autopilot legs (cleared at the coast), RADAR again
    // for the combat rehearsal (cleared when the red lock lands).
    let mut set_verbs = Vec::new();
    let mut cleared_verbs = Vec::new();
    for action in all_actions(&config) {
        match action {
            EventActionConfig::HintEmphasisSet(set) => set_verbs.push(set.verb.clone()),
            EventActionConfig::HintEmphasisClear(clear) => cleared_verbs.push(clear.verb.clone()),
            _ => {}
        }
    }
    assert_eq!(
        set_verbs,
        vec!["RADAR".to_string(), "GOTO".to_string(), "RADAR".to_string()]
    );
    // Clears may EXCEED sets: the derelict-kill catch-all carries a
    // defensive RADAR clear for the skip path (clearing an unset
    // emphasis is a no-op) - the invariant is that every set verb has
    // a downstream clear, not a 1:1 pairing.
    assert_eq!(
        cleared_verbs,
        vec![
            "RADAR".to_string(),
            "GOTO".to_string(),
            "RADAR".to_string(),
            "RADAR".to_string(),
        ]
    );
    for verb in &set_verbs {
        assert!(cleared_verbs.contains(verb));
    }
}

/// The ambush choreography: the pirate is NOT part of the opening
/// spawn set - it enters in exactly one later handler (the salvage
/// completion), patrolling the debris cluster, passive by
/// construction (patrol AI engages only inside AI_ENGAGE_RANGE or
/// when damaged).
#[test]
fn pirate_spawns_late_at_the_debris_cluster() {
    let config = scenario();

    let on_start_spawns: Vec<&ScenarioObjectConfig> = config
        .events
        .iter()
        .filter(|event| matches!(event.name, EventConfig::OnStart))
        .flat_map(|event| event.actions.iter())
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => Some(object),
            _ => None,
        })
        .collect();
    assert!(
        on_start_spawns
            .iter()
            .all(|object| object.base.id != ID_PIRATE),
        "the pirate must not be in the opening spawn set"
    );

    let pirate_spawns: Vec<&ScenarioObjectConfig> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) if object.base.id == ID_PIRATE => {
                Some(object)
            }
            _ => None,
        })
        .collect();
    assert_eq!(pirate_spawns.len(), 1, "exactly one pirate spawn action");

    let pirate = pirate_spawns[0];
    let ScenarioObjectKind::Spaceship(ship) = &pirate.kind else {
        panic!("the pirate is a spaceship");
    };
    let SpaceshipController::AI(ai) = &ship.controller else {
        panic!("the pirate is AI-controlled");
    };
    assert!(!ai.patrol.is_empty(), "the pirate patrols");
    for waypoint in &ai.patrol {
        assert!(
            waypoint.distance(DEBRIS_CENTER) < Meters(1_000.0),
            "patrol waypoint {:?} is over the debris cluster",
            waypoint
        );
    }
}

/// Both the player and the scavenger fly the cargoa corvette (base
/// craft-ships-into-base prototypes) and the rehearsal hulk is an inert
/// three-cell light hull; the scavenger is scavenger-grade - the
/// SetHealth-nerfed hull, mounts included. Resolves each section's prototype ref
/// against the base catalog to read its kind, and honors SetHealth overrides.
#[test]
fn ships_are_corvettes_and_the_pirate_is_scavenger_grade() {
    let config = scenario();

    let ships: Vec<(&str, &SpaceshipConfig)> = all_actions(&config)
        .into_iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => match &object.kind {
                ScenarioObjectKind::Spaceship(ship) => Some((object.base.id.as_str(), ship)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(ships.len(), 3, "player, inert hulk, and pirate");

    let hulk = ships
        .iter()
        .find(|(id, _)| *id == ID_DERELICT)
        .expect("the rehearsal hulk is a ship")
        .1;
    assert!(matches!(&hulk.controller, SpaceshipController::None));
    let hulk_sections = crate::generation::spawned_ship_sections(hulk);
    assert_eq!(hulk_sections.len(), 3);
    assert!(hulk_sections.iter().all(|section| {
        matches!(&section.source, SectionSource::Prototype(id) if id == "light_hull_section")
    }));

    let pirate_config = ships.iter().find(|(id, _)| *id == ID_PIRATE).unwrap().1;
    let SpaceshipController::AI(pirate_ai) = &pirate_config.controller else {
        panic!("the pirate is AI-controlled");
    };
    assert_eq!(
        pirate_ai.leash,
        Some(PIRATE_LEASH_RADIUS),
        "the scavenger is leashed to the debris field (playtest round 3)"
    );

    // Each spawn REFERENCES a catalog ship, so the section list is a join.
    let ships: Vec<(&str, Vec<SpaceshipSectionConfig>)> = ships
        .into_iter()
        .filter(|(id, _)| *id != ID_DERELICT)
        .map(|(id, ship)| (id, crate::generation::spawned_ship_sections(ship)))
        .collect();

    // The corvette's sections reference base catalog prototypes; resolve them.
    let catalog = crate::base_content::sections::section_catalog(
        &crate::base_content::assets::BaseContentAssets::from_paths(),
    );
    let resolve = |section: &SpaceshipSectionConfig| -> SectionConfig {
        match &section.source {
            SectionSource::Inline(config) => config.clone(),
            SectionSource::Prototype(id) => catalog
                .iter()
                .find(|c| c.base.id == *id)
                .unwrap_or_else(|| panic!("unknown prototype '{id}'"))
                .clone(),
        }
    };
    // Effective health = a SetHealth modification if present, else the
    // prototype's own (an AI corvette nerfs its hull this way).
    let effective_hp = |s: &SpaceshipSectionConfig| -> f32 {
        s.modifications
            .iter()
            .rev()
            .find_map(|m| match m {
                SectionModification::SetHealth(h) => Some(*h),
                _ => None,
            })
            .unwrap_or_else(|| resolve(s).base.health)
    };
    let max_turret_hp = |ship: &[SpaceshipSectionConfig]| -> f32 {
        ship.iter()
            .filter(|s| matches!(resolve(s).kind, SectionKind::Turret(_)))
            .map(effective_hp)
            .fold(0.0_f32, f32::max)
    };
    let max_hull_hp = |ship: &[SpaceshipSectionConfig]| -> f32 {
        ship.iter()
            .filter(|s| matches!(resolve(s).kind, SectionKind::Hull(_)))
            .map(effective_hp)
            .fold(0.0_f32, f32::max)
    };

    // Every corvette carries its two turret cubes and no torpedo bay.
    for (id, ship) in &ships {
        let turrets = ship
            .iter()
            .filter(|s| matches!(resolve(s).kind, SectionKind::Turret(_)))
            .count();
        assert_eq!(turrets, 2, "'{}' is a corvette with two turret modules", id);
        assert!(
            !ship
                .iter()
                .any(|s| matches!(resolve(s).kind, SectionKind::Torpedo(_))),
            "'{}' has no torpedo bay",
            id
        );
    }

    // Every semantic part belongs to the authoritative structural graph.
    for (id, ship) in &ships {
        let points: Vec<_> = ship
            .iter()
            .map(|section| SectionLinkPoints(resolve(section).base.link_points))
            .collect();
        let placed: Vec<_> = ship
            .iter()
            .zip(&points)
            .map(|(section, link_points)| PlacedSectionLinkPoints {
                position: section.position,
                rotation: section.rotation,
                link_points,
            })
            .collect();
        derive_link_point_graph(&placed)
            .unwrap_or_else(|errors| panic!("'{id}' has an invalid parts graph: {errors:?}"));
    }

    let player = &ships.iter().find(|(id, _)| *id == ID_PLAYER).unwrap().1;
    let pirate = &ships.iter().find(|(id, _)| *id == ID_PIRATE).unwrap().1;
    // Both fly the SAME gun now - there is one turret in the catalog - so the
    // scavenger grade lives in the mount, not the round: its guns are quicker
    // to shoot off, not softer to be shot by.
    assert!(
        max_turret_hp(pirate) < max_turret_hp(player),
        "the scavenger's gun mounts are flimsier than the player's"
    );
    assert!(
        max_hull_hp(pirate) < max_hull_hp(player),
        "the scavenger's hull is squishier than the player's"
    );
}

/// Beat 4's geometry, against the planetoid's ONE SOI and its worst-seed
/// orbit ring.
///
/// The SOI is a property of the authored mass alone
/// (`sqrt(mu / soi_cutoff_accel)`, GravityWell::from_mass), so it is the same
/// number on every load - this test used to sweep a 5.6-9.6 km range because
/// the old derivation multiplied the geometric radius, which the noise mesh
/// puts anywhere in ASTEROID_GEOMETRIC_FACTOR_MIN..MAX times the nominal one.
/// That seed spread still governs the ORBIT ring, which parks at
/// orbit_clearance_factor(1.5) * (body_radius + surface_margin(10 m)) off the
/// GEOMETRIC surface (the authored-vs-derived lesson: an earlier cut hardcoded
/// an observed 4.0-4.55 band, real seeds reach 5.64, and the ring landed
/// outside the old 1.6 km gate - a silent beat-4 softlock). So: one SOI, worst
/// seed for the ring.
#[test]
fn beat4_geometry_holds_against_the_planetoid_soi() {
    const ORBIT_CLEARANCE: f32 = 1.5;
    // `GravitySettings::surface_margin`, the engine's one world unit.
    const SURFACE_MARGIN: Meters = Meters(10.0);

    let gravity = nova_gameplay::prelude::GravitySettings::default();
    // Mirrors GravityWell::from_mass. `mu` and the surface-gravity cap stay on
    // the engine side (u^3/s^2 and u/s^2), so the geometric radius crosses back
    // for this one comparison: the mass is under the escapability cap on the
    // smallest mesh seed (10 u/s^2 * 70^2 = 49 000), so no clamp bites and the
    // SOI really is seed-independent - assert that, because a heavier planetoid
    // would quietly go back to being a per-seed lottery.
    let smallest_geometric = (PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MIN).to_engine();
    assert!(
        PLANETOID_MASS <= gravity.max_surface_gravity * smallest_geometric * smallest_geometric,
        "the planetoid's mass ({PLANETOID_MASS}) must stay under the surface-gravity cap \
         on the SMALLEST mesh seed, or its SOI varies with the seed again"
    );
    let soi = Meters::from_engine((PLANETOID_MASS / gravity.soi_cutoff_accel).sqrt());
    let widest_body = PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX;
    let widest_ring = (widest_body + SURFACE_MARGIN) * ORBIT_CLEARANCE;

    // Beacon 3 (the FIRST lock target, beat sheet v2): its GOTO leg is
    // the gravity-free rehearsal, so it must clear the SOI - and stay
    // within the DEFAULT beacon lock range of the
    // debris cluster, where the lesson is taught (BEACON_LOCK_SIGNATURE
    // 200 m * signature_range_per_unit 30 = 6 km; both cited constants).
    let beacon_3_planetoid = BEACON_3_POS.distance(PLANETOID_POS);
    assert!(
        beacon_3_planetoid > soi + Meters(400.0),
        "beacon 3 ({:.0} m from the planetoid) must clear the \
         planetoid SOI ({:.0} m)",
        beacon_3_planetoid.get(),
        soi.get()
    );
    let default_lock_range = Meters(200.0) * 30.0;
    let cluster_to_beacon_3 = DEBRIS_CENTER.distance(BEACON_3_POS);
    assert!(
        cluster_to_beacon_3 < default_lock_range - Meters(1_000.0),
        "beacon 3 ({:.0} m from the cluster) must be well inside \
         the default beacon lock range ({:.0} m)",
        cluster_to_beacon_3.get(),
        default_lock_range.get()
    );

    // Beacon 4 (the waypoint target): inside the SOI with margin so
    // the ORBIT hint lights on arrival, outside the widest
    // orbit ring, and its 700 m trigger must stay CLEAR of the coast
    // ring - a player still inside a trigger when its OnEnter beat
    // arms misses the CollisionStart (the already-inside trap, same
    // rule as the crate sensors below).
    let beacon_4_distance = BEACON_4_POS.distance(PLANETOID_POS);
    assert!(
        beacon_4_distance < soi * 0.75,
        "beacon 4 ({:.0} m) sits inside the SOI \
         ({:.0} m) with margin, so the ORBIT hint lights on arrival",
        beacon_4_distance.get(),
        soi.get()
    );
    assert!(
        beacon_4_distance > widest_ring + Meters(300.0),
        "beacon 4 ({:.0} m) clears the widest orbit ring \
         ({:.0} m)",
        beacon_4_distance.get(),
        widest_ring.get()
    );
    // The NOMINAL beacon-4 park (arrival_standoff on the approach
    // side) must sit outside the ring so the coast exists on the
    // happy path; a player who ends up inside the ring anyway still
    // advances, because a SPAWNED area fires OnEnter for bodies it
    // lands on (pinned in nova_scenario's area tests - the ring
    // spawns with its beat).
    let standoff =
        Meters::from_engine(nova_ship::prelude::FlightSettings::default().arrival_standoff);
    assert!(
        COAST_RING_RADIUS < beacon_4_distance + standoff - Meters(200.0),
        "the coast ring ({:.0} m) leaves the nominal park \
         ({:.0} m) outside it",
        COAST_RING_RADIUS.get(),
        (beacon_4_distance + standoff).get()
    );
    // The waypoint LEG must be lockable: beacon 4 authors its own
    // signature (BEACON_4_LOCK_SIGNATURE * 30, the range model - a plain
    // ratio, so 30 m of range per meter of signature).
    let waypoint_leg = BEACON_3_POS.distance(BEACON_4_POS);
    let beacon_4_lock_range = BEACON_4_LOCK_SIGNATURE * 30.0;
    assert!(
        waypoint_leg < beacon_4_lock_range - Meters(500.0),
        "the waypoint leg ({:.0} m) fits beacon 4's authored lock range \
         ({:.0} m) with margin",
        waypoint_leg.get(),
        beacon_4_lock_range.get()
    );

    // The coast ring: outside the widest orbit ring (the held orbit
    // must stay INSIDE the ring, or breaking away could not be
    // detected by OnExit - and a swing outside during capture would
    // fire it early, though the beat guard eats that), inside the SOI
    // (the coast is FELT).
    assert!(
        COAST_RING_RADIUS > widest_ring + Meters(200.0),
        "the coast ring ({:.0} m) clears the widest orbit ring \
         ({:.0} m)",
        COAST_RING_RADIUS.get(),
        widest_ring.get()
    );
    assert!(
        COAST_RING_RADIUS < soi - Meters(500.0),
        "the coast ring ({:.0} m) sits well inside the SOI \
         ({:.0} m)",
        COAST_RING_RADIUS.get(),
        soi.get()
    );

    // The derelict: a DYNAMIC body - inside the SOI it would fall into
    // the planetoid; it must hold still by the old salvage field.
    let derelict_distance = DERELICT_POS.distance(PLANETOID_POS);
    assert!(
        derelict_distance > soi + Meters(400.0),
        "the derelict ({:.0} m from the planetoid) must clear the \
         planetoid SOI ({:.0} m)",
        derelict_distance.get(),
        soi.get()
    );

    // Playtest round 2 finding 1: the debris cluster (and every crate
    // in it) must sit OUTSIDE the SOI - the salvage beat is
    // flown by hand, and fighting gravity while weaving crates reads
    // as a bug, not a challenge.
    let cluster_distance = DEBRIS_CENTER.distance(PLANETOID_POS);
    assert!(
        cluster_distance > soi + Meters(400.0),
        "the debris cluster ({:.0} m from the planetoid) must clear \
         the planetoid SOI ({:.0} m)",
        cluster_distance.get(),
        soi.get()
    );
    for (i, crate_pos) in CRATE_POSITIONS.iter().enumerate() {
        let distance = crate_pos.distance(PLANETOID_POS);
        assert!(
            distance > soi + Meters(400.0),
            "crate_{} ({:.0} m) sits outside the SOI \
             with margin",
            i + 1,
            distance.get()
        );
    }

    // Review (adapted): the beacon triggers must CONTAIN the GOTO park point
    // (playtest finding 2) - the autopilot stops arrival_standoff from an
    // unsized target, and a smaller trigger parks the ship outside its own
    // objective.
    assert!(
        BEACON_AREA_RADIUS > standoff + Meters(100.0),
        "beacon trigger ({:.0} m) must contain the GOTO park point \
         (standoff {:.0} m) with margin",
        BEACON_AREA_RADIUS.get(),
        standoff.get()
    );
    // No crate sensor reachable from inside beacon 2's trigger:
    // the beat 2->3 flip happens inside beacon 2's area, and a player
    // already parked inside a crate sensor when the pickups arm would
    // miss its CollisionStart.
    for (i, crate_pos) in CRATE_POSITIONS.iter().enumerate() {
        let distance = crate_pos.distance(BEACON_2_POS);
        assert!(
            distance > BEACON_AREA_RADIUS + CRATE_AREA_RADIUS,
            "crate_{} ({:.0} m from beacon 2) must not overlap beacon 2's \
             trigger volume",
            i + 1,
            distance.get()
        );
    }
}

/// The salvage crates must be spread far enough apart that each pickup
/// registers as its own moment: the old ~290-370 m scatter let a fast pass
/// sweep two 80 m sensors almost at once. Pin every pair at >= 5x the pickup
/// radius center-to-center - a clear gap between sensor surfaces (2*radius), so
/// you cannot collect two without a deliberate second approach. A future
/// re-cram fails here.
#[test]
fn crates_are_spaced_for_distinct_pickups() {
    let min_separation = 5.0 * CRATE_AREA_RADIUS;
    for (i, a) in CRATE_POSITIONS.iter().enumerate() {
        for (j, b) in CRATE_POSITIONS.iter().enumerate().skip(i + 1) {
            let separation = a.distance(*b);
            assert!(
                separation >= min_separation,
                "crate_{} and crate_{} are {:.0} m apart - too close for \
                 distinct pickups (need >= {:.0} m, 5x the {:.0} m \
                 pickup radius)",
                i + 1,
                j + 1,
                separation.get(),
                min_separation.get(),
                CRATE_AREA_RADIUS.get()
            );
        }
    }
}

/// Distance from a point to the SURFACE of an axis-aligned knot box (0 inside).
/// A rock is sampled uniformly in the whole box, so the box - not its centre -
/// is what has to stay off a pocket.
fn distance_to_box(point: Meters3, center: Meters3, half_extent: Meters3) -> Meters {
    Meters(
        ((point - center).get().abs() - half_extent.get())
            .max(Vec3::ZERO)
            .length(),
    )
}

/// Distance from a segment to a knot box - an autopilot leg is a line the ship
/// actually flies, not two endpoints. Distance to a convex set is convex and
/// the segment is affine in `t`, so the composition is convex and a ternary
/// search lands on the true minimum.
fn segment_distance_to_box(
    from: Meters3,
    to: Meters3,
    center: Meters3,
    half_extent: Meters3,
) -> Meters {
    let at = |t: f32| distance_to_box(Meters3(from.get().lerp(to.get(), t)), center, half_extent);
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..80 {
        let third = (hi - lo) / 3.0;
        if at(lo + third) < at(hi - third) {
            hi -= third;
        } else {
            lo += third;
        }
    }
    at(lo)
}

/// The slalom belt must not fill the air a beat needs. Measured as the ROCK
/// sees it: the distance from the pocket to the knot BOX surface, minus the
/// widest rock's collider (`BELT_ROCK_RADIUS.1 * ASTEROID_GEOMETRIC_FACTOR_MAX`,
/// not its nominal radius), must leave a 200 m margin. A centre-to-centre check
/// against the widest half-extent is not the same test - it passed knot 2 by
/// 160 m while that box's worst-case rock reached 60 m INSIDE beacon 1's
/// trigger. Second half: the far parallax layer stays under
/// `GravitySettings::min_well_radius`, so no belt rock gets the default well
/// and sprays gravity over the legs authored to be gravity-free.
#[test]
fn belt_knots_keep_every_beat_pocket_clear() {
    const ORBIT_CLEARANCE: f32 = 1.5;
    // `GravitySettings::surface_margin`, the engine's one world unit.
    const SURFACE_MARGIN: Meters = Meters(10.0);
    const POCKET_MARGIN: Meters = Meters(200.0);

    let widest_body = PLANETOID_NOMINAL_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX;
    let widest_ring = (widest_body + SURFACE_MARGIN) * ORBIT_CLEARANCE;
    let pockets: [(&str, Meters3, Meters); 9] = [
        ("the player spawn", PLAYER_SPAWN, Meters(600.0)),
        ("beacon 1", BEACON_1_POS, BEACON_AREA_RADIUS),
        ("beacon 2", BEACON_2_POS, BEACON_AREA_RADIUS),
        ("beacon 3", BEACON_3_POS, BEACON_AREA_RADIUS),
        ("beacon 4", BEACON_4_POS, BEACON_AREA_RADIUS),
        ("the debris cluster", DEBRIS_CENTER, Meters(900.0)),
        ("the derelict", DERELICT_POS, Meters(400.0)),
        ("the pirate spawn", PIRATE_SPAWN, Meters(400.0)),
        ("the planetoid orbit ring", PLANETOID_POS, widest_ring),
    ];

    // A rock's own body, not its authored radius: the noise mesh reaches up to
    // ASTEROID_GEOMETRIC_FACTOR_MAX times nominal.
    let rock_reach = BELT_ROCK_RADIUS.1 * ASTEROID_GEOMETRIC_FACTOR_MAX;

    for knot in &BELT_KNOTS {
        for (name, center, radius) in &pockets {
            let box_surface = distance_to_box(*center, knot.center, knot.half_extent);
            let clearance = box_surface - *radius - rock_reach;
            assert!(
                clearance > POCKET_MARGIN,
                "{} leaves {name} only {:.0} m of air (its box surface is \
                 {:.0} m out, {name} needs {:.0} m and a rock reaches \
                 {:.0} m) - the floor is {:.0} m",
                knot.id_prefix,
                clearance.get(),
                box_surface.get(),
                radius.get(),
                rock_reach.get(),
                POCKET_MARGIN.get()
            );
        }
    }

    // The AUTOPILOT legs: beat 4 hands the ship to the computer, so the player
    // cannot dodge anything parked on the line. Every knot clears each leg by
    // its own reach plus the margin.
    let legs: [(&str, Meters3, Meters3); 3] = [
        ("the GOTO leg to beacon 3", DEBRIS_CENTER, BEACON_3_POS),
        ("the waypoint run to beacon 4", BEACON_3_POS, BEACON_4_POS),
        ("the run in to the orbit", BEACON_4_POS, PLANETOID_POS),
    ];
    for knot in &BELT_KNOTS {
        for (name, from, to) in &legs {
            let clearance =
                segment_distance_to_box(*from, *to, knot.center, knot.half_extent) - rock_reach;
            assert!(
                clearance > POCKET_MARGIN,
                "{} leaves {name} only {:.0} m of air - an autopilot corridor \
                 needs {:.0} m the player cannot dodge into",
                knot.id_prefix,
                clearance.get(),
                POCKET_MARGIN.get()
            );
        }
    }

    // The far parallax ring is seeded, so a single rock cannot be aimed away
    // from a beat: its HOLE must contain the whole playable volume instead.
    let farthest_beat = pockets
        .iter()
        .map(|(_, center, radius)| center.distance(PLANETOID_POS) + *radius)
        .fold(Meters::ZERO, Meters::max);
    assert!(
        BELT_FAR_RING.0 > farthest_beat + Meters(1_000.0),
        "the far belt ring starts at {:.0} m from the planetoid, inside the playable \
         volume (which reaches {:.0} m)",
        BELT_FAR_RING.0.get(),
        farthest_beat.get()
    );

    let gravity = nova_gameplay::prelude::GravitySettings::default();
    let min_well_radius = Meters::from_engine(gravity.min_well_radius);
    assert!(
        BELT_FAR_RADIUS.1 < min_well_radius,
        "the far belt layer's biggest rock ({:.0} m) must stay under min_well_radius \
         ({:.0} m), or all {BELT_FAR_COUNT} of them get the default well",
        BELT_FAR_RADIUS.1.get(),
        min_well_radius.get()
    );
}

/// Player death restarts the run (linger: Enter confirms).
#[test]
fn player_death_routes_back_to_shakedown() {
    let config = scenario();

    let death_routes: Vec<&NextScenarioActionConfig> = config
        .events
        .iter()
        .filter(|event| {
            matches!(event.name, EventConfig::OnDestroyed)
                && event.filters.iter().any(|filter| {
                    matches!(
                        filter,
                        EventFilterConfig::Entity(entity)
                            if entity.id.as_deref() == Some(ID_PLAYER)
                    )
                })
        })
        .flat_map(|event| event.actions.iter())
        .filter_map(|action| match action {
            EventActionConfig::NextScenario(next) => Some(next),
            _ => None,
        })
        .collect();

    assert_eq!(death_routes.len(), 1);
    assert_eq!(death_routes[0].scenario_id, SHAKEDOWN_SCENARIO_ID);
    assert!(death_routes[0].linger, "Enter confirms the restart");
}
