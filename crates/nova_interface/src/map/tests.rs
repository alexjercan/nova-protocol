//! Live-tree tests for the Map pane: contact derivation and codes, GOTO routing
//! into the autopilot, the orbit/selection input, and window-space picking.

use bevy::{
    camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo},
    ecs::system::RunSystemOnce,
    state::app::StatesPlugin,
    ui::{ComputedNode, InteractionDisabled, UiGlobalTransform},
    ui_widgets::Activate,
};
use nova_events::prelude::{
    EntityId, EntityTypeName, GeneratedBodyId, SectorCoord, ASTEROID_TYPE_NAME,
};
use nova_gameplay::{prelude::*, PauseStates};
use nova_input::prelude::{BindingSpec, InputBindings, InputSource, RegisterInputActions};
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{metrics, VerbGate},
    theme::{ActiveUiTheme, UiColorType},
    widget::{ThemedFill, ThemedText},
};
use nova_world_base::prelude::{
    enemy_neighbours, seeded_stance, standing, CivilizationField, CivilizationStatusType,
};

use super::{app::*, contacts::*, scene::*, *};
use crate::{
    icons::{BodyIconType, InterfaceIcons},
    pane::{ActionOutcomeType, InterfacePaneType},
    pointer_rig::{
        click_at, hear_ui_cues, move_cursor_to, pane_pointer_rig, settle, take_churn, take_cues,
        track_node_churn, NodeChurn, PanePointerRig, RIG_PANEL_MIN,
    },
    station::{apply_station_settle_commands, StanceSettleCommand},
    terminal::NovaOsCloseTransition,
    viewer::cycle_index,
};

/// The contact panel crosses the stored world-unit range to meters and
/// renders it through the shared distance policy, not raw `u`.
#[test]
fn map_range_renders_in_meters_and_kilometers() {
    let entity = Entity::PLACEHOLDER;
    // 50 world units = 500 m (below the km threshold).
    let near = MapContact {
        entity,
        kind: MapContactKind::Hostile,
        code: "HOST-1".to_string(),
        name: "RAIDER".to_string(),
        marker: None,
        world_pos: Vec3::ZERO,
        range: 50.0,
        bearing_deg: 0.0,
        mark_deg: 0.0,
        body: BodyIconType::Ship,
    };
    assert_eq!(near.range_text(), "500 m");

    // 150 world units = 1500 m -> 1.50 km.
    let far = MapContact {
        range: 150.0,
        ..near.clone()
    };
    assert_eq!(far.range_text(), "1.50 km");

    // The own ship's zero range also uses the new unit, and has no bearing.
    let own = MapContact {
        kind: MapContactKind::OwnShip,
        range: 0.0,
        ..near
    };
    assert_eq!(own.range_text(), "0 m");
    assert_eq!(own.bearing_text(), None);
    assert_eq!(far.bearing_text().as_deref(), Some("000 mark +00"));
}

/// Spawn a scripted local-space scene: own ship at origin (facing -Z), a
/// hostile dead ahead, an objective to starboard, an asteroid astern.
fn scripted_world() -> (World, Entity, Entity) {
    let mut world = World::new();
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<Encounters>();
    let player = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ))
        .id();
    let raider = world
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    world.spawn((
        ObjectiveMarkerTarget {
            label: "salvage".to_string(),
        },
        GlobalTransform::from(Transform::from_xyz(50.0, 0.0, 0.0)),
    ));
    world.spawn((
        EntityTypeName::new(ASTEROID_TYPE_NAME),
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 60.0)),
    ));
    (world, player, raider)
}

#[test]
fn map_contacts_report_kinds_range_and_bearing() {
    let (mut world, _player, raider) = scripted_world();
    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();

    // Own ship is enumerated first.
    assert_eq!(contacts[0].kind, MapContactKind::OwnShip);
    assert_eq!(contacts[0].range, 0.0);

    let find = |kind: MapContactKind| contacts.iter().find(|c| c.kind == kind).unwrap();
    let hostile = find(MapContactKind::Hostile);
    assert_eq!(hostile.entity, raider);
    assert!((hostile.range - 50.0).abs() < 0.01);
    // Dead ahead (-Z) reads bearing ~0.
    assert!(hostile.bearing_deg < 1.0 || hostile.bearing_deg > 359.0);

    let objective = find(MapContactKind::Objective);
    assert!((objective.range - 50.0).abs() < 0.01);
    // Starboard (+X) reads ~090.
    assert!((objective.bearing_deg - 90.0).abs() < 1.0);

    let asteroid = find(MapContactKind::Terrain);
    assert!((asteroid.range - 60.0).abs() < 0.01);
    // Astern (+Z) reads ~180.
    assert!((asteroid.bearing_deg - 180.0).abs() < 1.0);
}

/// A denser world: two hostiles + two asteroids + one objective, so the
/// per-kind indices actually count up and can collide if minting is wrong.
fn crowded_world() -> World {
    let mut world = World::new();
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<Encounters>();
    world.spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Name::new("NOVA"),
    ));
    for z in [-40.0, -80.0] {
        world.spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, z)),
            Name::new("RAIDER"),
        ));
    }
    for x in [30.0, 70.0] {
        world.spawn((
            EntityTypeName::new(ASTEROID_TYPE_NAME),
            GlobalTransform::from(Transform::from_xyz(x, 0.0, 0.0)),
        ));
    }
    world.spawn((
        ObjectiveMarkerTarget {
            label: "salvage".to_string(),
        },
        GlobalTransform::from(Transform::from_xyz(0.0, 40.0, 0.0)),
    ));
    world
}

#[test]
fn map_contact_codes_are_unique_and_stable() {
    let mut world = crowded_world();
    // Mint codes, then read them back off the contact model.
    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();

    let codes: Vec<String> = contacts.iter().map(|c| c.code.clone()).collect();
    let unique: std::collections::HashSet<&String> = codes.iter().collect();
    assert_eq!(
        unique.len(),
        codes.len(),
        "every contact code is unique: {codes:?}"
    );

    // The own ship is the bare SELF; each other kind counts from 1.
    assert!(codes.contains(&"SELF".to_string()));
    assert!(codes.contains(&"HOST-1".to_string()) && codes.contains(&"HOST-2".to_string()));
    assert!(codes.contains(&"AST-1".to_string()) && codes.contains(&"AST-2".to_string()));
    assert!(codes.contains(&"OBJ-1".to_string()));

    // Re-running the pass must NOT reassign or add codes (stable per session).
    world.run_system_once(assign_map_contact_codes).unwrap();
    let again = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let mut before = codes;
    let mut after: Vec<String> = again.iter().map(|c| c.code.clone()).collect();
    before.sort();
    after.sort();
    assert_eq!(before, after, "codes are stable across minting passes");

    // A Neutral answering the player reads Hostile but keeps its NEU code, so
    // the next Neutral must not mint that code again.
    let player = world
        .run_system_once(|c: MapContacts| c.player_frame().unwrap().0)
        .unwrap();
    let neutral = |world: &mut World| {
        world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Neutral,
                GlobalTransform::default(),
            ))
            .id()
    };
    let answering = neutral(&mut world);
    world.run_system_once(assign_map_contact_codes).unwrap();
    world
        .entity_mut(answering)
        .insert(RetaliationTarget(Some(player)));
    let streamed = neutral(&mut world);
    world.run_system_once(assign_map_contact_codes).unwrap();
    let code = |world: &World, entity| world.get::<MapContactCode>(entity).unwrap().0.clone();
    assert_eq!(code(&world, answering), "NEU-1");
    assert_eq!(code(&world, streamed), "NEU-2");
}

/// A ship, a rock and a station that wear an objective marker are each one
/// Objective contact that keeps its body icon and carries the label: the
/// ship and the rock read `OBJ-n`, the station its chart code. A marked own
/// ship stays the one `SELF` contact with the label and takes no `OBJ` index,
/// so the codes hold on the next pass and cycling reaches every contact.
#[test]
fn a_marked_body_is_one_objective_contact() {
    let mut world = World::new();
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<Encounters>();
    let player = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
            ObjectiveMarkerTarget {
                label: "JOB-0 Survive".into(),
            },
        ))
        .id();
    let ship = world
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            EntityId::new("caller"),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("CALLER"),
            ObjectiveMarkerTarget {
                label: "JOB-1 Answer".into(),
            },
        ))
        .id();
    let rock = world
        .spawn((
            EntityTypeName::new(ASTEROID_TYPE_NAME),
            EntityId::new("rock"),
            GlobalTransform::from(Transform::from_xyz(30.0, 0.0, 0.0)),
            ObjectiveMarkerTarget {
                label: "JOB-2 Recover".into(),
            },
        ))
        .id();
    let station = world
        .spawn((
            SpaceshipRootMarker,
            StationMarker,
            EntityId::new("outpost_prime"),
            GlobalTransform::from(Transform::from_xyz(0.0, 40.0, 0.0)),
            ObjectiveMarkerTarget {
                label: "JOB-3 Hand in".into(),
            },
        ))
        .id();

    world.run_system_once(assign_map_contact_codes).unwrap();
    let first = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    for (entity, kind, body, code, label) in [
        (
            player,
            MapContactKind::OwnShip,
            BodyIconType::Ship,
            "SELF",
            "JOB-0 Survive",
        ),
        (
            ship,
            MapContactKind::Objective,
            BodyIconType::Ship,
            "OBJ-1",
            "JOB-1 Answer",
        ),
        (
            rock,
            MapContactKind::Objective,
            BodyIconType::Asteroid,
            "OBJ-2",
            "JOB-2 Recover",
        ),
        (
            station,
            MapContactKind::Objective,
            BodyIconType::Station,
            "STN",
            "JOB-3 Hand in",
        ),
    ] {
        for (pass, list) in [(1, &first), (2, &contacts)] {
            let found: Vec<&MapContact> = list.iter().filter(|c| c.entity == entity).collect();
            assert_eq!(found.len(), 1, "{code} is one contact on pass {pass}");
            let contact = found[0];
            assert_eq!(
                (contact.kind, contact.body, contact.code.as_str()),
                (kind, body, code),
                "pass {pass}"
            );
            assert_eq!(contact.marker.as_deref(), Some(label));
        }
    }
    let codes: Vec<&String> = contacts.iter().map(|c| &c.code).collect();
    let unique: std::collections::HashSet<&&String> = codes.iter().collect();
    assert_eq!(unique.len(), codes.len(), "every code is unique: {codes:?}");

    // Forward cycling from the own ship, as `map_input` selects, visits every
    // contact once before it comes back.
    let mut selected = player;
    let mut visited = Vec::new();
    for _ in 0..contacts.len() {
        let current = contacts.iter().position(|c| c.entity == selected);
        let next = cycle_index(current, contacts.len(), true).unwrap();
        selected = contacts[next].entity;
        visited.push(selected);
    }
    assert_eq!(selected, player, "cycling returns to the own ship");
    let reached: std::collections::HashSet<Entity> = visited.iter().copied().collect();
    assert_eq!(
        reached.len(),
        contacts.len(),
        "cycling reaches every contact"
    );
}

/// A ship's code crosses the objective boundary with its marker: a marked
/// `NEU-1` reads `OBJ-1`, and once unmarked it takes the next free `NEU`
/// index. Its neighbour's code never moves.
#[test]
fn a_contact_code_follows_its_marker() {
    let mut world = World::new();
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<Encounters>();
    world.spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
    ));
    let neutral = |world: &mut World, id: &str| {
        world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Neutral,
                EntityId::new(id),
                GlobalTransform::default(),
            ))
            .id()
    };
    let marked = neutral(&mut world, "a");
    let neighbour = neutral(&mut world, "b");
    let codes = |world: &mut World| {
        world.run_system_once(assign_map_contact_codes).unwrap();
        let code = |entity| world.get::<MapContactCode>(entity).unwrap().0.clone();
        (code(marked), code(neighbour))
    };
    assert_eq!(codes(&mut world), ("NEU-1".into(), "NEU-2".into()));

    world.entity_mut(marked).insert(ObjectiveMarkerTarget {
        label: "JOB-1 Answer".into(),
    });
    assert_eq!(codes(&mut world), ("OBJ-1".into(), "NEU-2".into()));
    assert_eq!(codes(&mut world), ("OBJ-1".into(), "NEU-2".into()));

    world.entity_mut(marked).remove::<ObjectiveMarkerTarget>();
    assert_eq!(codes(&mut world), ("NEU-3".into(), "NEU-2".into()));
}

/// A found station draws the chart's cube at every side: its code is its
/// 1-based place in the chart's FOUND order, never spawn order, and it never
/// mints a [`MapContactCode`]. A ship beside it still mints `HOST-1`.
#[test]
fn a_station_draws_its_cube_and_chart_code_at_every_side() {
    let mut world = World::new();
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<Encounters>();
    world.spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
        Name::new("NOVA"),
    ));
    let spawn_station = |world: &mut World, id: GeneratedBodyId, allegiance: Allegiance| {
        world
            .spawn((
                SpaceshipRootMarker,
                StationMarker,
                id,
                allegiance,
                GlobalTransform::default(),
            ))
            .id()
    };
    let ally_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
    let hostile_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 1);
    let neutral_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 2);
    let ally = spawn_station(&mut world, ally_id.clone(), Allegiance::Player);
    let hostile = spawn_station(&mut world, hostile_id.clone(), Allegiance::Enemy);
    let neutral = spawn_station(&mut world, neutral_id.clone(), Allegiance::Neutral);
    let raider = world
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::default(),
            Name::new("RAIDER"),
        ))
        .id();

    // Found out of spawn order, so a correct index can only come from the
    // chart's found order.
    {
        let mut chart = world.resource_mut::<Chart>();
        chart.find_station(&neutral_id, false, false, None, 0.0);
        chart.find_station(&ally_id, false, false, None, 0.0);
        chart.find_station(&hostile_id, false, false, None, 0.0);
    }

    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let find = |entity: Entity| {
        contacts
            .iter()
            .find(|c| c.entity == entity)
            .unwrap_or_else(|| panic!("{entity:?} is not a contact"))
    };

    for (entity, want_kind, want_code) in [
        (neutral, MapContactKind::Neutral, "STN-1"),
        (ally, MapContactKind::Ally, "STN-2"),
        (hostile, MapContactKind::Hostile, "STN-3"),
    ] {
        let contact = find(entity);
        assert_eq!(contact.body, BodyIconType::Station, "{entity:?}");
        assert_eq!(contact.kind, want_kind, "{entity:?}");
        assert_eq!(contact.code, want_code, "{entity:?}");
        assert!(
            world.get::<MapContactCode>(entity).is_none(),
            "a station mints no ship code: {entity:?}"
        );
    }

    assert_eq!(
        find(raider).code,
        "HOST-1",
        "a plain ship still mints its usual code"
    );
}

/// An authored station - no [`GeneratedBodyId`], the chart only finds
/// generated ones - is still a contact: it shows as a Station body with the
/// bare `STN` stem, never hidden and never carrying a [`MapContactCode`],
/// even while the chart holds other, generated stations.
#[test]
fn an_authored_station_reads_stn_on_the_map() {
    let mut world = World::new();
    world.init_resource::<Chart>();
    world.init_resource::<Jobs>();
    world.init_resource::<Encounters>();
    world.spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
        Name::new("NOVA"),
    ));
    let generated_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
    world
        .resource_mut::<Chart>()
        .find_station(&generated_id, false, false, None, 0.0);
    let authored = world
        .spawn((
            SpaceshipRootMarker,
            StationMarker,
            EntityId::new("outpost_prime"),
            GlobalTransform::default(),
        ))
        .id();

    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let contact = contacts
        .iter()
        .find(|c| c.entity == authored)
        .expect("the authored station is a contact");
    assert_eq!(contact.body, BodyIconType::Station);
    assert_eq!(contact.code, "STN");
    assert!(
        world.get::<MapContactCode>(authored).is_none(),
        "a station mints no ship code"
    );
}

/// The scene lives while the Map pane is on screen, including under the
/// command modal that returns to it, and goes when the pane or the interface
/// does (headless: no render assets, so only the active flag toggles - the
/// scene build is skipped, but open/close is proven).
#[test]
fn map_scene_lives_while_the_map_pane_is_shown() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);
    app.update();

    let active = |app: &mut App| {
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        app.world().resource::<MapRuntime>().active
    };
    let set_pause = |app: &mut App, state: PauseStates| {
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(state);
        app.update();
    };

    assert!(!active(&mut app), "the Ship pane shows no map scene");
    *app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Map;
    assert!(active(&mut app), "the Map pane builds the scene");

    app.world_mut()
        .resource_mut::<NovaOsCloseTransition>()
        .return_to = PauseStates::Interface;
    set_pause(&mut app, PauseStates::Commands);
    assert!(
        active(&mut app),
        "the command modal over the pane keeps the scene for the way back"
    );

    set_pause(&mut app, PauseStates::Interface);
    set_pause(&mut app, PauseStates::Unpaused);
    assert!(!active(&mut app), "closing the interface tears it down");
}

/// With the asset stores present, opening the map actually builds the
/// schematic scene (camera + proxy meshes + RTT image) and the per-frame
/// systems run without panicking - the path a real GPU would render.
#[test]
fn map_scene_builds_and_drives_with_render_assets() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<ActiveUiTheme>();
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);

    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Name::new("NOVA"),
    ));

    // Build the scene.
    app.world_mut().run_system_once(manage_map_scene).unwrap();
    {
        let runtime = app.world().resource::<MapRuntime>();
        assert!(runtime.active);
        assert!(runtime.scene_root.is_some(), "scene root spawned");
        assert!(runtime.image.is_some(), "RTT image created");
        assert!(runtime.camera.is_some(), "map camera spawned");
    }
    // A camera entity carries the render layer + orbit.
    let cameras = app
        .world_mut()
        .query_filtered::<(), (With<MapCameraMarker>, With<MapOrbit>)>()
        .iter(app.world())
        .count();
    assert_eq!(cameras, 1, "exactly one orbit map camera");

    // The per-frame systems run without panicking (no viewport UI node here,
    // so projection/reconcile early-return, but the code path is exercised).
    app.world_mut()
        .run_system_once(reconcile_map_target)
        .unwrap();
    app.world_mut().run_system_once(drive_map_camera).unwrap();
    app.world_mut().run_system_once(project_map_blips).unwrap();

    // Switching to the Ship pane tears the scene down.
    *app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Ship;
    app.world_mut().run_system_once(manage_map_scene).unwrap();
    assert!(app.world().resource::<MapRuntime>().scene_root.is_none());
    let remaining = app
        .world_mut()
        .query_filtered::<(), With<MapCameraMarker>>()
        .iter(app.world())
        .count();
    assert_eq!(remaining, 0, "camera despawned on close");
}

#[test]
fn map_focus_follow_recenters_on_a_new_selection() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);

    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Name::new("NOVA"),
    ));
    let raider = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(90.0, 0.0, -30.0)),
            Name::new("RAIDER"),
        ))
        .id();

    // Build the scene (framed on the player).
    app.world_mut()
        .run_system_once(assign_map_contact_codes)
        .unwrap();
    app.world_mut().run_system_once(manage_map_scene).unwrap();

    // Select the raider: the orbit center + ring anchor snap onto it.
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
    app.world_mut().run_system_once(map_focus_follow).unwrap();

    let center = app
        .world_mut()
        .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
        .single(app.world())
        .unwrap()
        .center;
    assert!(
        center.distance(Vec3::new(90.0, 0.0, -30.0)) < 0.01,
        "the map recenters on the selected contact",
    );
    let anchor = app
        .world_mut()
        .query_filtered::<&Transform, With<MapFocusAnchor>>()
        .single(app.world())
        .unwrap()
        .translation;
    assert!(
        anchor.distance(Vec3::new(90.0, 0.0, -30.0)) < 0.01,
        "the ring anchor follows the focus",
    );
}

/// The viewers read ACTIONS, so a moved key moves the control. Before this the
/// map named `KeyCode::KeyG` in the system that read it: rebindable everywhere
/// except the interface the player flies with.
#[test]
fn a_rebound_goto_key_is_the_key_the_map_answers() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, bevy::input::InputPlugin));
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);

    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ))
        .id();
    let target = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    app.world_mut()
        .run_system_once(assign_map_contact_codes)
        .unwrap();
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.selected = Some(target);
    }
    app.world_mut().resource_mut::<InputBindings>().rebind(
        "map_goto",
        BindingSpec {
            keyboard: vec![InputSource::Keyboard(KeyCode::KeyJ)],
            gamepad: Vec::new(),
        },
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyG);
    app.world_mut().run_system_once(map_input).unwrap();
    assert!(
        app.world().get::<Autopilot>(player).is_none(),
        "the key that used to be GOTO does nothing once it is moved"
    );

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyJ);
    app.world_mut().run_system_once(map_input).unwrap();
    assert!(
        app.world().get::<Autopilot>(player).is_some(),
        "the key it was moved to sets the GOTO"
    );
}

#[test]
fn map_goto_sets_autopilot_on_the_player_ship() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, bevy::input::InputPlugin));
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);

    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ))
        .id();
    let target = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    let alt_target = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(50.0, 0.0, 0.0)),
            Name::new("RAIDER2"),
        ))
        .id();

    app.world_mut()
        .run_system_once(assign_map_contact_codes)
        .unwrap();
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.selected = Some(target);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyG);

    // A standing order and designation that every refusal below must leave
    // as they are.
    let prior = app.world_mut().spawn_empty().id();
    let standing = Autopilot::engage(AutopilotAction::Stop);
    app.world_mut()
        .entity_mut(player)
        .insert((standing, TravelLock(Some(prior))));
    let kept = |app: &App| {
        assert_eq!(app.world().get::<Autopilot>(player), Some(&standing));
        assert_eq!(
            app.world().get::<TravelLock>(player),
            Some(&TravelLock(Some(prior)))
        );
    };

    // Docked to the selected contact in neutral: refused with the deny cue
    // instead of claiming a GOTO the helm would fly later.
    let connection = app
        .world_mut()
        .spawn(DockingConnection {
            first_ship: player,
            first_section: Entity::PLACEHOLDER,
            second_ship: target,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    app.world_mut().entity_mut(player).insert(DockedShip {
        connection,
        helm: Quat::IDENTITY,
        drives: false,
    });
    // The cue the last press owes, taken so the next press starts clean.
    let take_cue = |app: &mut App| {
        std::mem::take(&mut app.world_mut().resource_mut::<MapRuntime>().cue).pending()
    };
    // A fresh GOTO edge in `state`: the frame clears the last edge.
    let press_goto_in = |app: &mut App, state: PauseStates| {
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(state);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyG);
        keys.press(KeyCode::KeyG);
    };
    let press_goto = |app: &mut App| press_goto_in(app, PauseStates::Interface);
    // Re-arm the standing order the next press must either keep or replace.
    let reset_order = |app: &mut App| {
        app.world_mut()
            .entity_mut(player)
            .insert((standing, TravelLock(Some(prior))));
    };
    let flying_to = |app: &App, target: Entity| {
        assert_eq!(
            app.world().get::<Autopilot>(player).map(|a| a.action),
            Some(AutopilotAction::Goto { target }),
            "the autopilot targets the queued contact",
        );
        assert_eq!(
            app.world().get::<TravelLock>(player),
            Some(&TravelLock(Some(target))),
            "an accepted GOTO designates its target"
        );
    };

    // Under the command modal over the pane the key is text, not GOTO.
    press_goto_in(&mut app, PauseStates::Commands);
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(
        take_cue(&mut app),
        None,
        "the modal over the pane takes no GOTO"
    );

    // Docked to the selected contact: refused, whatever holds the pair's
    // helm or whether it can be measured - a trip to the partner never
    // closes.
    press_goto(&mut app);
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(take_cue(&mut app), Some(ActionOutcomeType::Refused));

    app.world_mut()
        .get_mut::<DockedShip>(player)
        .unwrap()
        .drives = true;
    press_goto(&mut app);
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(take_cue(&mut app), Some(ActionOutcomeType::Refused));

    app.world_mut()
        .get_mut::<DockingConnection>(connection)
        .unwrap()
        .measurement_fault = true;
    press_goto(&mut app);
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(take_cue(&mut app), Some(ActionOutcomeType::Refused));

    // Docked to another body, without the helm: the GOTO is accepted and
    // waits for the undock (whoever, or nothing, holds that pair's helm).
    let partner = app.world_mut().spawn_empty().id();
    {
        let mut record = app
            .world_mut()
            .get_mut::<DockingConnection>(connection)
            .unwrap();
        record.second_ship = partner;
        record.measurement_fault = false;
    }
    app.world_mut()
        .get_mut::<DockedShip>(player)
        .unwrap()
        .drives = false;
    for (helm, fault) in [
        (DockedHelmType::Neutral, false),
        (DockedHelmType::Station(partner), false),
        (DockedHelmType::Neutral, true),
    ] {
        {
            let mut record = app
                .world_mut()
                .get_mut::<DockingConnection>(connection)
                .unwrap();
            record.helm = helm;
            record.measurement_fault = fault;
        }
        reset_order(&mut app);
        press_goto(&mut app);
        app.world_mut().run_system_once(map_input).unwrap();
        assert_eq!(
            take_cue(&mut app),
            None,
            "an accepted GOTO owes no deny cue"
        );
        flying_to(&app, target);
    }

    // Pressing GOTO again on the same selection cancels the queued order.
    press_goto(&mut app);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(
        app.world().get::<Autopilot>(player),
        None,
        "the toggle removes the autopilot"
    );
    assert_eq!(
        app.world().get::<TravelLock>(player),
        Some(&TravelLock(None)),
        "the lock named the cancelled target, so it clears too"
    );

    // A GOTO on the selected contact, but a lock naming a different body:
    // the toggle still cancels the autopilot, and leaves that lock alone.
    app.world_mut().entity_mut(player).insert((
        Autopilot::engage(AutopilotAction::Goto { target }),
        TravelLock(Some(prior)),
    ));
    press_goto(&mut app);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(app.world().get::<Autopilot>(player), None);
    assert_eq!(
        app.world().get::<TravelLock>(player),
        Some(&TravelLock(Some(prior))),
        "the lock does not name the cancelled target, so it stays"
    );

    // A GOTO on one contact, another selected: a press retargets instead of
    // cancelling, with the lock moved to the new selection.
    app.world_mut().entity_mut(player).insert((
        Autopilot::engage(AutopilotAction::Goto { target }),
        TravelLock(Some(target)),
    ));
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(alt_target);
    press_goto(&mut app);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(take_cue(&mut app), None);
    flying_to(&app, alt_target);
}

/// A GOTO set with the key stays set while the interface is open: the open
/// interface holds the clocks, so no fixed tick reaches the flight computer,
/// and the frames after the press still fly the autopilot to the target with
/// the panel naming it. The stand-in fixed system ends every maneuver, as the
/// flight computer does for a hull at rest.
#[test]
fn a_goto_set_on_the_map_stays_set_over_the_next_frames() {
    use std::time::Duration;

    use bevy::{
        input::{
            keyboard::{Key, KeyboardInput},
            ButtonState,
        },
        time::TimeUpdateStrategy,
    };
    use nova_gameplay::{prelude::ClockFreeze, GameStates};

    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.add_plugins((StatesPlugin, nova_ui::NovaUiPlugin));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.init_resource::<ClockFreeze>();
    app.insert_state(GameStates::Playing);
    app.init_state::<PauseStates>();
    crate::pane::add_interface_clock_hold(app);
    app.add_systems(
        FixedUpdate,
        |mut commands: Commands, q_flying: Query<Entity, With<Autopilot>>| {
            for ship in &q_flying {
                commands.entity(ship).remove::<Autopilot>();
            }
        },
    );
    app.register_input_actions(crate::bindings::interface_bindings());
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);
    app.add_systems(
        Update,
        (assign_map_contact_codes, map_input, update_map_panel).chain(),
    );
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
        ))
        .id();
    let raider = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.selected = Some(raider);
    }
    settle(app);
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    settle(app);

    let key = |app: &mut App, state: ButtonState| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyG,
            logical_key: Key::Character("g".into()),
            state,
            text: (state == ButtonState::Pressed).then(|| "g".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    };
    let flying = |app: &mut App| {
        (
            app.world().get::<Autopilot>(player).map(|a| a.action),
            panel_text(app.world_mut(), MapPanelField::Destination),
        )
    };
    let set = (
        Some(AutopilotAction::Goto { target: raider }),
        "GOTO HOST-1  500 m".to_string(),
    );

    key(app, ButtonState::Pressed);
    app.update();
    app.update();
    assert_eq!(flying(app), set, "the press sets the GOTO");
    for frame in 0..8 {
        if frame == 3 {
            key(app, ButtonState::Released);
        }
        app.update();
        assert_eq!(flying(app), set, "frame {frame} after the press");
    }
}

/// A wheel notch over the contact panel scrolls its body and leaves the
/// view's zoom alone; a notch away from the panel zooms the view and leaves
/// the body where it is. A selected station's stock list pushes the STANCES
/// block past a 720p panel, so its Pay fine is reached by the wheel; the
/// GOTO and Cancel row stays inside the panel over a body taller than it.
#[test]
fn a_wheel_over_the_map_panel_scrolls_it_and_leaves_the_chart_zoom() {
    use bevy::input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    };

    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.add_plugins(StatesPlugin);
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);
    app.add_systems(Update, (scroll_map_panel, map_input).chain());
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
        Name::new("NOVA"),
    ));
    let camera = app
        .world_mut()
        .spawn((
            MapCameraMarker,
            MapOrbit {
                theta: 0.0,
                phi: 0.5,
                radius: 400.0,
                center: Vec3::ZERO,
            },
            Transform::default(),
        ))
        .id();
    app.world_mut().resource_mut::<MapRuntime>().active = true;
    settle(app);
    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<MapPanelMarker>>()
        .single(app.world())
        .expect("the map pane spawns one contact panel");
    let body = app
        .world_mut()
        .query_filtered::<Entity, With<MapPanelBodyMarker>>()
        .single(app.world())
        .expect("the contact panel has one scrolling body");
    // Content taller than the panel, as a selected station's stock list is.
    app.world_mut().entity_mut(body).with_child(Node {
        height: Val::Px(3000.0),
        flex_shrink: 0.0,
        ..default()
    });
    settle(&mut rig.app);
    let wheel = |rig: &mut PanePointerRig| {
        rig.app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -3.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        settle(&mut rig.app);
    };
    let scrolled = |rig: &PanePointerRig| rig.app.world().get::<ScrollPosition>(body).unwrap().0.y;
    let verbs = [
        app_entity::<MapGotoButton>(&mut rig.app),
        app_entity::<MapCancelButton>(&mut rig.app),
    ];
    let verbs_inside = |rig: &PanePointerRig| {
        let panel = rig_rect(rig, panel);
        verbs.iter().all(|verb| {
            let verb = rig_rect(rig, *verb);
            verb.width() > 0.0
                && verb.height() > 0.0
                && panel.contains(verb.min)
                && panel.contains(verb.max)
        })
    };
    let radius = |rig: &PanePointerRig| rig.app.world().get::<MapOrbit>(camera).unwrap().radius;

    let rect = rig_rect(&rig, panel);
    assert!(
        verbs_inside(&rig),
        "GOTO and Cancel stand inside the panel under a body taller than it"
    );
    move_cursor_to(&mut rig, rect.center());
    wheel(&mut rig);
    assert!(scrolled(&rig) > 0.0, "the wheel over the panel scrolls it");
    assert!(verbs_inside(&rig), "and the row does not scroll with it");
    assert_eq!(radius(&rig), 400.0, "and leaves the zoom");

    let held = scrolled(&rig);
    move_cursor_to(&mut rig, Vec2::new(rect.max.x + 200.0, rect.center().y));
    wheel(&mut rig);
    assert!(radius(&rig) > 400.0, "the wheel off the panel zooms out");
    assert_eq!(scrolled(&rig), held, "and leaves the panel where it is");
}

/// The GOTO button is the `map_goto` key's request, not a second path: it is
/// disabled on the own ship, and on a contact it sets the autopilot and travel
/// lock the key sets, which the panel then names as the destination.
#[test]
fn the_goto_button_sets_the_goto_the_key_sets() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.add_plugins((StatesPlugin, nova_ui::NovaUiPlugin));
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);
    app.add_systems(
        Update,
        (assign_map_contact_codes, map_input, update_map_panel).chain(),
    );
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
        ))
        .id();
    let raider = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    app.world_mut().resource_mut::<MapRuntime>().active = true;
    settle(app);
    assert_eq!(
        panel_text(app.world_mut(), MapPanelField::Destination),
        "",
        "no GOTO set: the destination line reads nothing"
    );

    let button = app
        .world_mut()
        .query_filtered::<Entity, With<MapGotoButton>>()
        .single(app.world())
        .unwrap();
    let cancel_button = app
        .world_mut()
        .query_filtered::<Entity, With<MapCancelButton>>()
        .single(app.world())
        .unwrap();
    let disabled = |rig: &PanePointerRig| {
        rig.app.world().get::<VerbGate>(button) == Some(&VerbGate::Disabled)
            && rig.app.world().get::<InteractionDisabled>(button).is_some()
    };
    let cancel_disabled = |rig: &PanePointerRig| {
        rig.app.world().get::<VerbGate>(cancel_button) == Some(&VerbGate::Disabled)
            && rig
                .app
                .world()
                .get::<InteractionDisabled>(cancel_button)
                .is_some()
    };
    let goto = |rig: &PanePointerRig| {
        (
            rig.app.world().get::<Autopilot>(player).map(|a| a.action),
            rig.app.world().get::<TravelLock>(player).copied(),
        )
    };
    assert!(disabled(&rig), "nothing selected: no GOTO to request");
    assert!(
        cancel_disabled(&rig),
        "no GOTO flies: CANCEL has nothing to end"
    );

    rig.app.world_mut().resource_mut::<MapRuntime>().selected = Some(player);
    settle(&mut rig.app);
    assert!(disabled(&rig), "the own ship is no GOTO target");
    let at = rig_rect(&rig, button).center();
    click_at(&mut rig, at);
    assert_eq!(
        goto(&rig),
        (None, None),
        "a disabled button requests nothing"
    );

    rig.app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
    settle(&mut rig.app);
    assert!(!disabled(&rig), "a contact can be a GOTO target");
    // The contact's Bearing row moves the button down from the own ship's.
    let at = rig_rect(&rig, button).center();
    click_at(&mut rig, at);
    assert_eq!(
        goto(&rig),
        (
            Some(AutopilotAction::Goto { target: raider }),
            Some(TravelLock(Some(raider)))
        ),
    );
    assert_eq!(
        panel_text(rig.app.world_mut(), MapPanelField::Destination),
        "GOTO HOST-1  500 m"
    );
    assert!(!cancel_disabled(&rig), "a flying GOTO is CANCEL's to end");

    // The target leaves the map (destroyed, docked, out of range - whatever
    // the reason, the flown-to entity is no longer a contact); the panel
    // still names the queued order, and CANCEL still has it to end.
    rig.app.world_mut().entity_mut(raider).despawn();
    settle(&mut rig.app);
    assert_eq!(
        panel_text(rig.app.world_mut(), MapPanelField::Destination),
        "GOTO LOST"
    );
    assert!(!cancel_disabled(&rig), "CANCEL still ends the lost GOTO");

    let cancel_at = rig_rect(&rig, cancel_button).center();
    click_at(&mut rig, cancel_at);
    assert_eq!(
        rig.app.world().get::<Autopilot>(player),
        None,
        "CANCEL removes the autopilot even with its target gone"
    );
    assert_eq!(
        panel_text(rig.app.world_mut(), MapPanelField::Destination),
        ""
    );
    assert!(cancel_disabled(&rig), "nothing left to cancel");
}

/// A charted civilization shows its seeded generation potential in both
/// overview and the selected station's STANCES entry, never a ship rating.
#[test]
fn chart_stances_show_seeded_advancement_in_overview_and_selected_station() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.add_plugins(nova_ui::NovaUiPlugin);
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<WorldClock>();
    app.init_resource::<CivilizationStances>();
    app.add_systems(Update, update_map_stance_block);

    let civilization = CivilizationId {
        world_seed: 0,
        node: [4, 0, 0],
    };
    let station_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
    app.world_mut().resource_mut::<Chart>().find_station(
        &station_id,
        false,
        false,
        Some(civilization),
        0.0,
    );
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(spawn_map_stance_block);
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
    }
    settle(app);

    let assert_chip = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<(&Text, &ChildOf)>();
        let chips: Vec<(String, Entity)> = q
            .iter(world)
            .filter(|(text, _)| text.0.starts_with("ADV "))
            .map(|(text, parent)| (text.0.clone(), parent.parent()))
            .collect();
        assert_eq!(chips.len(), 1, "one advancement tag per entry");
        assert_eq!(chips[0].0, "ADV 13%");
        assert_eq!(
            world.get::<Node>(chips[0].1).map(|node| node.height),
            Some(Val::Px(metrics::TAG_H)),
            "the advancement tag keeps its tag node"
        );
    };
    assert_chip(app);

    app.world_mut().resource_mut::<MapRuntime>().chart_selected =
        Some(MapChartMarkType::Station(station_id));
    settle(app);
    assert_chip(app);
}

/// A STANCES detail line lays out at its text height in the overview, so it
/// draws.
#[test]
fn chart_stance_details_lay_out_at_their_text_height() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.add_plugins(nova_ui::NovaUiPlugin);
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<WorldClock>();
    app.init_resource::<CivilizationStances>();
    app.add_systems(Update, update_map_stance_block);

    let civilization = CivilizationId {
        world_seed: 0,
        node: [4, 0, 0],
    };
    app.world_mut()
        .resource_mut::<CivilizationStances>()
        .provoke(civilization, seeded_stance(&civilization), 0.0);
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(spawn_map_stance_block);
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
    }
    settle(app);

    let world = app.world_mut();
    let mut q = world.query::<(&Text, &ComputedNode)>();
    let heights: Vec<f32> = q
        .iter(world)
        .filter(|(text, _)| text.0.starts_with("HOSTILE "))
        .map(|(_, node)| node.size().y)
        .collect();
    assert_eq!(heights.len(), 1, "one HOSTILE detail line");
    assert!(
        heights[0] > 0.0,
        "the HOSTILE detail line laid out at zero height"
    );
}

/// A living civilization with a living neighbour its own seed drew Enemy,
/// and the fine [`enemy_neighbours`] reads for it.
fn civilization_with_enemy_neighbour() -> (CivilizationId, CivilizationId, u32) {
    for seed in 0..64u32 {
        let field = CivilizationField::new(seed);
        for x in -2..=2i32 {
            for y in -2..=2i32 {
                for z in -2..=2i32 {
                    let node = [x, y, z];
                    let civilization = field.civilization(node);
                    if civilization.status != CivilizationStatusType::Living {
                        continue;
                    }
                    let Some(enemy) = field
                        .neighbours(node)
                        .into_iter()
                        .find(|neighbour| neighbour.stance == StanceType::Enemy)
                    else {
                        continue;
                    };
                    let fine = enemy_neighbours(civilization.id, &CivilizationStances::default())
                        .into_iter()
                        .find_map(|(id, fine)| (id == enemy.id).then_some(fine))
                        .expect("a found Enemy neighbour reads back from its own civilization");
                    return (civilization.id, enemy.id, fine);
                }
            }
        }
    }
    panic!("no seed in [0, 64) holds a living civilization with a living Enemy neighbour");
}

/// The table stance `civilization` reads in the rig's saved table.
fn standing_table(rig: &PanePointerRig, civilization: CivilizationId) -> StanceType {
    let stances = rig.app.world().resource::<CivilizationStances>();
    let now = rig.app.world().resource::<WorldClock>().play_seconds;
    standing(stances, &civilization, now).table
}

/// The STANCES block's Pay fine button only moves the fine while the ship is
/// docked at the station that keeps the book, and each result plays one cue:
/// undocked, the button is hidden and a Dock line says where to pay; docked
/// with the balance short, the button is disabled, the fine reads red, and a
/// settle command is refused with one deny buzz and moves nothing; docked
/// with the fine in hand, no Dock line stands, and a click pays it both ways,
/// reports a Settle credit change on each ship's reports, settles the
/// civilization to Neutral and clicks once.
#[test]
fn the_chart_pays_a_fine_only_while_docked_at_the_selected_station() {
    let (civilization, enemy, fine) = civilization_with_enemy_neighbour();
    assert!(fine > 0, "a fine of nothing has no short balance to refuse");

    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    // The kit's gate and short-tone reconcilers.
    app.add_plugins(nova_ui::NovaUiPlugin);
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<WorldClock>();
    app.init_resource::<CivilizationStances>();
    app.add_message::<StanceSettleCommand>();
    app.add_systems(
        Update,
        (
            update_map_stance_block,
            apply_station_settle_commands,
            play_map_cue,
        )
            .chain(),
    );
    hear_ui_cues(app);
    #[derive(Resource, Default)]
    struct StanceSteps(Vec<StanceStep>);
    app.init_resource::<StanceSteps>();
    app.add_observer(
        |changed: On<StanceChanged>, mut steps: ResMut<StanceSteps>| {
            steps.0.push(changed.0);
        },
    );

    let station_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
    app.world_mut().resource_mut::<Chart>().find_station(
        &station_id,
        false,
        false,
        Some(civilization),
        0.0,
    );

    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| {
            spawn_map_stance_block(content);
        });

    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
            ShipCredits(fine),
            ShipInventoryReports::default(),
        ))
        .id();
    let station = app
        .world_mut()
        .spawn((
            StationMarker,
            TraderMarket {
                profile: None,
                cluster: ClusterType::AsteroidRich,
                civilization,
                role_preference: [0.0; 4],
                target: std::collections::BTreeMap::new(),
            },
            station_id.clone(),
            ShipCredits(0),
            ShipInventoryReports::default(),
            Name::new("STATION"),
        ))
        .id();

    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
        runtime.chart_selected = Some(MapChartMarkType::Station(station_id.clone()));
    }
    settle(&mut rig.app);

    // The rows are rebuilt whenever what they show changes, so the button
    // is looked up again after each change.
    // A station can neighbour several Enemy civilizations; the button is the
    // one on the row that names `enemy`.
    let find_button = |rig: &mut PanePointerRig| {
        let world = rig.app.world_mut();
        let cell = world
            .query::<(&Text, &ChildOf)>()
            .iter(world)
            .find_map(|(text, parent)| (text.0 == enemy.name()).then_some(parent.parent()))
            .expect("the enemy's Settle row names it");
        let row = world.get::<ChildOf>(cell).unwrap().parent();
        world
            .get::<Children>(row)
            .unwrap()
            .iter()
            .find(|child| {
                world
                    .get::<Name>(*child)
                    .is_some_and(|name| name.as_str().starts_with("MapStancePayFine"))
            })
            .expect("the Pay fine button spawned")
    };
    let gate = |rig: &mut PanePointerRig| {
        let button = find_button(rig);
        let world = rig.app.world();
        (
            world.get::<Node>(button).unwrap().display,
            world.get::<InteractionDisabled>(button).is_some(),
        )
    };
    let credits = |rig: &PanePointerRig, entity: Entity| {
        rig.app.world().get::<ShipCredits>(entity).unwrap().0
    };
    let settle_reports = |rig: &PanePointerRig, entity: Entity| {
        rig.app
            .world()
            .get::<ShipInventoryReports>(entity)
            .unwrap()
            .0
            .iter()
            .filter(|report| {
                matches!(
                    report,
                    ShipInventoryReport::CreditsChanged {
                        cause: CreditChangeCauseType::Settle,
                        ..
                    }
                )
            })
            .count()
    };
    let fine_tone = |rig: &mut PanePointerRig| {
        let fine_text = cr_text(u64::from(fine));
        rig.app
            .world_mut()
            .query::<(&Text, &ThemedText)>()
            .iter(rig.app.world())
            .find_map(|(text, themed)| (text.0 == fine_text).then_some(themed.color))
            .expect("the enemy's Settle row shows its fine")
    };
    let dock_line = |rig: &mut PanePointerRig| {
        rig.app
            .world_mut()
            .query::<&Text>()
            .iter(rig.app.world())
            .any(|text| text.0 == "DOCK STN-1")
    };
    assert!(take_cues(&mut rig.app).is_empty(), "the rig starts silent");

    assert_eq!(
        gate(&mut rig).0,
        Display::None,
        "undocked: the Pay fine button is hidden"
    );
    assert!(
        dock_line(&mut rig),
        "undocked: the Dock line says where to pay"
    );

    // Dock the player at the station, as `SettleDock` reads it.
    let connection = rig
        .app
        .world_mut()
        .spawn(DockingConnection {
            first_ship: player,
            first_section: Entity::PLACEHOLDER,
            second_ship: station,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    rig.app.world_mut().entity_mut(player).insert(DockedShip {
        connection,
        helm: Quat::IDENTITY,
        drives: false,
    });
    settle(&mut rig.app);
    assert_eq!(
        gate(&mut rig),
        (Display::Flex, false),
        "docked at the station with the fine in hand: the button is live"
    );

    // Docked with the balance one short, the button is disabled with the
    // fine in red, and a settle command raised past it is refused.
    rig.app
        .world_mut()
        .get_mut::<ShipCredits>(player)
        .unwrap()
        .0 = fine - 1;
    settle(&mut rig.app);
    assert_eq!(
        gate(&mut rig),
        (Display::Flex, true),
        "a short balance disables the button"
    );
    assert_eq!(
        fine_tone(&mut rig),
        UiColorType::Danger,
        "a short balance paints the fine red"
    );
    take_cues(&mut rig.app);
    rig.app.world_mut().write_message(StanceSettleCommand {
        civilization: enemy,
    });
    rig.app.update();
    assert_eq!(
        take_cues(&mut rig.app),
        [UiSfx::EditorDeny],
        "a refusal buzzes once"
    );
    settle(&mut rig.app);
    assert!(
        take_cues(&mut rig.app).is_empty(),
        "the refusal's buzz plays once"
    );
    assert_eq!(
        credits(&rig, player),
        fine - 1,
        "a refusal moves no credits"
    );
    assert_eq!(credits(&rig, station), 0, "a refusal moves no credits");
    assert_eq!(
        settle_reports(&rig, player),
        0,
        "a refusal reports no change"
    );
    assert_eq!(
        settle_reports(&rig, station),
        0,
        "a refusal reports no change"
    );
    assert_eq!(
        standing_table(&rig, enemy),
        StanceType::Enemy,
        "a refusal settles nothing"
    );

    rig.app
        .world_mut()
        .get_mut::<ShipCredits>(player)
        .unwrap()
        .0 = fine;
    settle(&mut rig.app);
    assert_eq!(
        gate(&mut rig),
        (Display::Flex, false),
        "the fine in hand again: the button is live"
    );
    assert_ne!(
        fine_tone(&mut rig),
        UiColorType::Danger,
        "the fine in hand: the fine no longer reads red"
    );
    assert!(!dock_line(&mut rig), "docked: no Dock line stands");
    take_cues(&mut rig.app);

    let button = find_button(&mut rig);
    let at = rig_rect(&rig, button).center();
    click_at(&mut rig, at);
    settle(&mut rig.app);

    assert_eq!(credits(&rig, player), 0, "the ship pays the whole fine");
    assert_eq!(credits(&rig, station), fine, "the station takes the fine");
    for entity in [player, station] {
        assert_eq!(
            settle_reports(&rig, entity),
            1,
            "{entity:?} carries one Settle credit change"
        );
    }
    assert_eq!(
        standing_table(&rig, enemy),
        StanceType::Neutral,
        "the settled civilization reads Neutral"
    );
    assert_eq!(
        rig.app.world().resource::<StanceSteps>().0,
        [StanceStep {
            civilization: enemy,
            from: StanceType::Enemy,
            to: StanceType::Neutral,
        }],
        "the paid fine raises one stance step and the refusals before it none"
    );
    assert_eq!(
        take_cues(&mut rig.app),
        [UiSfx::MenuFocus, UiSfx::MenuSelect],
        "the pointer's hover tick, then one settlement click: the press is silent"
    );
    settle(&mut rig.app);
    assert!(
        take_cues(&mut rig.app).is_empty(),
        "the settlement's click plays once"
    );
}

/// A Map with a live GOTO from the player at the origin to a raider 50 units
/// ahead, a trader off to the side, and a laid-out viewport.
fn goto_route_app() -> (App, Entity, Entity, Entity) {
    let mut app = map_input_app();
    app.insert_resource(InterfaceIcons::blank());
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(app.world())
        .unwrap();
    let raider = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    app.world_mut().spawn((
        SpaceshipRootMarker,
        Allegiance::Neutral,
        GlobalTransform::from(Transform::from_xyz(60.0, 0.0, 0.0)),
        Name::new("TRADER"),
    ));
    app.world_mut()
        .run_system_once(assign_map_contact_codes)
        .unwrap();
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyG);
    app.world_mut().run_system_once(map_input).unwrap();
    let viewport = app
        .world_mut()
        .spawn((MapViewportMarker, Node::default(), ComputedNode::DEFAULT))
        .id();
    app.world_mut().run_system_once(drive_map_camera).unwrap();
    (app, player, raider, viewport)
}

/// Lay the viewport out at `physical` px and 2x scale. With `camera_too` the
/// camera projects into it 1:1 in physical px, as the render target reconciler
/// leaves them; without, the camera is stale.
fn lay_out_map(app: &mut App, viewport: Entity, physical: UVec2, camera_too: bool) {
    let mut computed = app.world_mut().get_mut::<ComputedNode>(viewport).unwrap();
    computed.size = physical.as_vec2();
    computed.inverse_scale_factor = 0.5;
    if !camera_too {
        return;
    }
    let world = app.world_mut();
    let (mut camera, transform, mut global) = world
        .query_filtered::<(&mut Camera, &Transform, &mut GlobalTransform), With<MapCameraMarker>>()
        .single_mut(world)
        .unwrap();
    *global = GlobalTransform::from(*transform);
    camera.computed = ComputedCameraValues {
        clip_from_view: PerspectiveProjection {
            aspect_ratio: physical.x as f32 / physical.y as f32,
            ..default()
        }
        .get_clip_from_view(),
        target_info: Some(RenderTargetInfo {
            physical_size: physical,
            scale_factor: 2.0,
        }),
        ..default()
    };
}

/// Look straight down on `focus` from 20 units above it.
fn look_down_on(app: &mut App, focus: Vec3) {
    let world = app.world_mut();
    let mut global = world
        .query_filtered::<&mut GlobalTransform, With<MapCameraMarker>>()
        .single_mut(world)
        .unwrap();
    *global = GlobalTransform::from(
        Transform::from_translation(focus + Vec3::Y * 20.0).looking_at(focus, Vec3::Z),
    );
}

fn project_map_route_nodes(app: &mut App) {
    // The first pass spawns the nodes; the second places them.
    for _ in 0..2 {
        app.world_mut().run_system_once(project_map_blips).unwrap();
        app.world_mut().run_system_once(project_map_route).unwrap();
    }
}

fn px(val: Val) -> f32 {
    match val {
        Val::Px(px) => px,
        other => panic!("expected px, got {other:?}"),
    }
}

fn blip_centre(app: &App, contact: Entity) -> Vec2 {
    let blip = app.world().resource::<MapRuntime>().blips[&contact];
    let node = app.world().get::<Node>(blip).unwrap();
    Vec2::new(px(node.left), px(node.top)) + MAP_BLIP_PX * 0.5
}

/// A world point through the map camera, in logical px.
fn map_px(app: &mut App, world_pos: Vec3) -> Vec2 {
    let world = app.world_mut();
    let (camera, global) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<MapCameraMarker>>()
        .single(world)
        .unwrap();
    camera.world_to_viewport(global, world_pos).unwrap() * 0.5
}

/// The drawn ends and fill alpha of every shown route stroke, in no order.
fn route_strokes(app: &mut App) -> Vec<(Vec2, Vec2, f32)> {
    let world = app.world_mut();
    world
        .query_filtered::<(&Node, &UiTransform, &ThemedFill, &Visibility), With<MapRouteLine>>()
        .iter(world)
        .filter(|(_, _, _, visibility)| **visibility != Visibility::Hidden)
        .map(|(line, transform, fill, _)| {
            assert_eq!(
                fill.color,
                UiColorType::Accent,
                "every route stroke is accent"
            );
            let centre = Vec2::new(
                px(line.left) + px(line.width) * 0.5,
                px(line.top) + px(line.height) * 0.5,
            );
            let half =
                Vec2::new(transform.rotation.cos, transform.rotation.sin) * px(line.width) * 0.5;
            (centre - half, centre + half, fill.alpha)
        })
        .collect()
}

/// Every expected stroke is drawn, end to end within 0.01 px at its fill
/// alpha, and nothing else is.
fn assert_strokes(app: &mut App, expected: &[(Vec2, Vec2, f32)]) {
    let strokes = route_strokes(app);
    assert_eq!(
        strokes.len(),
        expected.len(),
        "drawn {strokes:?}, expected {expected:?}"
    );
    for (from, to, alpha) in expected {
        assert!(
            strokes.iter().any(|(a, b, fill)| a.distance(*from) < 0.01
                && b.distance(*to) < 0.01
                && fill == alpha),
            "no stroke runs {from:?} to {to:?} at {alpha}: {strokes:?}"
        );
    }
}

fn goto_tag(app: &mut App) -> (Vec2, Visibility) {
    let world = app.world_mut();
    let (marker, visibility) = world
        .query_filtered::<(&Node, &Visibility), With<MapGotoMarker>>()
        .single(world)
        .unwrap();
    (Vec2::new(px(marker.left), px(marker.top)), *visibility)
}

/// The `GOTO` tag's top-left for a target blip centred on `blip`.
fn tag_on(blip: Vec2) -> Vec2 {
    blip - Vec2::new(MAP_BLIP_PX * 0.5, MAP_BLIP_PX * 0.5 + MAP_GOTO_MARKER_PX)
}

fn predict(app: &mut App, ship: Entity, points: Vec<Vec3>, end: FlightPredictionEndType) {
    let sample_interval = 0.125;
    app.world_mut().entity_mut(ship).insert(FlightPrediction {
        final_point_time: points.len().saturating_sub(1) as f32 * sample_interval,
        points,
        flip_index: None,
        seed_time: std::time::Duration::ZERO,
        sample_interval,
        end,
        flown_point: Vec3::ZERO,
    });
}

/// The route starts at the player blip and runs through the projected
/// prediction points still ahead of the ship, dropping those `Time<Fixed>` has
/// flown past, and ends at the last one. It draws again after the map closes and reopens, at 2x scale, and
/// across a resize while another contact is selected, and the `GOTO` tag
/// stays on the target.
#[test]
fn the_goto_route_draws_the_predicted_path_from_the_ship_blip() {
    let (mut app, player, raider, viewport) = goto_route_app();
    let points = vec![
        Vec3::new(1.0, 0.0, -2.0),
        Vec3::new(5.0, 0.0, -10.0),
        Vec3::new(12.0, 0.0, -20.0),
        Vec3::new(15.0, 0.0, -35.0),
    ];
    predict(
        &mut app,
        player,
        points.clone(),
        FlightPredictionEndType::Completed,
    );

    // Close and reopen: the selection does not survive, the GOTO does.
    for pane in [InterfacePaneType::Ship, InterfacePaneType::Map] {
        *app.world_mut().resource_mut::<InterfacePaneType>() = pane;
        app.world_mut().run_system_once(manage_map_scene).unwrap();
    }
    assert_eq!(app.world().resource::<MapRuntime>().selected, None);
    let trader = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "TRADER").then_some(entity));
    app.world_mut().resource_mut::<MapRuntime>().selected = trader;
    app.world_mut().run_system_once(drive_map_camera).unwrap();

    // Point `i` is `i` sample intervals after the seed; the seed itself is
    // where the ship is now, so the blip stands in for it.
    let expected = |app: &mut App, ahead: &[Vec3]| {
        let mut ends = vec![blip_centre(app, player)];
        ends.extend(ahead.iter().map(|point| map_px(app, *point)));
        ends.windows(2)
            .map(|pair| (pair[0], pair[1], 0.8))
            .collect::<Vec<_>>()
    };
    lay_out_map(&mut app, viewport, UVec2::new(1600, 1200), true);
    project_map_route_nodes(&mut app);
    let path = expected(&mut app, &points[1..]);
    assert_strokes(&mut app, &path);
    assert!(
        blip_centre(&app, player).distance(map_px(&mut app, points[3])) > 20.0,
        "the rig spreads the path"
    );
    let raider_blip = blip_centre(&app, raider);
    assert_eq!(
        goto_tag(&mut app),
        (tag_on(raider_blip), Visibility::Inherited),
        "the tag sits on the GOTO target, not the selection"
    );

    // One sample interval flown: the first point ahead is behind the ship.
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .advance_by(std::time::Duration::from_millis(130));
    project_map_route_nodes(&mut app);
    let path = expected(&mut app, &points[2..]);
    assert_strokes(&mut app, &path);

    // A resize: while the camera still projects into the old size the route
    // holds, then it follows the prediction in the new layout.
    lay_out_map(&mut app, viewport, UVec2::new(1200, 1200), false);
    project_map_route_nodes(&mut app);
    assert_strokes(&mut app, &path);
    lay_out_map(&mut app, viewport, UVec2::new(1200, 1200), true);
    project_map_route_nodes(&mut app);
    let resized = expected(&mut app, &points[2..]);
    assert_ne!(resized, path, "the resize moves the projection");
    assert_strokes(&mut app, &resized);
}

/// Zoomed close with the path running through the view: every stroke stops
/// at the viewport rather than rotating an offscreen-wide UI node through it.
#[test]
fn the_goto_route_strokes_stay_inside_a_zoomed_viewport() {
    let (mut app, player, _, viewport) = goto_route_app();
    let points: Vec<Vec3> = (0..=6)
        .map(|i| Vec3::new(0.0, 0.0, -10.0 * i as f32))
        .collect();
    predict(&mut app, player, points, FlightPredictionEndType::Completed);
    lay_out_map(&mut app, viewport, UVec2::new(1200, 1200), true);
    look_down_on(&mut app, Vec3::new(0.0, 0.0, -25.0));
    project_map_route_nodes(&mut app);

    let (start, end) = (
        map_px(&mut app, Vec3::ZERO),
        map_px(&mut app, Vec3::new(0.0, 0.0, -60.0)),
    );
    assert!(
        (start.y < 0.0 && end.y > 600.0) || (end.y < 0.0 && start.y > 600.0),
        "the path crosses the viewport: {start:?} to {end:?}"
    );
    let strokes = route_strokes(&mut app);
    let inset = MAP_ROUTE_PX * 0.5;
    assert!(
        strokes
            .iter()
            .flat_map(|(a, b, _)| [a, b])
            .all(|p| p.x >= inset && p.y >= inset && p.x <= 600.0 - inset && p.y <= 600.0 - inset),
        "a zoomed stroke escapes the viewport: {strokes:?}"
    );
    let drawn: f32 = strokes.iter().map(|(a, b, _)| a.distance(*b)).sum();
    assert!(drawn > 500.0, "the crossing stays drawn: {drawn} px");
}

/// A long curved prediction - a GOTO flown to its arrival, thousands of
/// points - draws exactly its decimated points from the player blip to the
/// arrival point: no stroke runs on to the target blip.
#[test]
fn the_goto_route_is_the_decimated_predicted_path_and_nothing_else() {
    let (mut app, player, raider, viewport) = goto_route_app();
    // 2001 points bowing 20 u off the line from the player at the origin to
    // (0, 0, -46), 4 u short of the raider at (0, 0, -50): the standoff the
    // leg stops at.
    let points: Vec<Vec3> = (0..=2000)
        .map(|i| {
            let t = i as f32 / 2000.0;
            Vec3::new(20.0 * (t * std::f32::consts::PI).sin(), 0.0, -46.0 * t)
        })
        .collect();
    predict(
        &mut app,
        player,
        points.clone(),
        FlightPredictionEndType::Completed,
    );
    lay_out_map(&mut app, viewport, UVec2::new(1600, 1200), true);
    project_map_route_nodes(&mut app);

    // The seed point is where the ship is now, so the blip stands in for it.
    let decimated = decimate_path(&points, MAP_ROUTE_SEGMENTS + 1);
    assert_eq!(decimated.len(), MAP_ROUTE_SEGMENTS + 1);
    let mut ends = vec![blip_centre(&app, player)];
    ends.extend(decimated[1..].iter().map(|point| map_px(&mut app, *point)));
    let expected: Vec<_> = ends
        .windows(2)
        .map(|pair| (pair[0], pair[1], 0.8))
        .collect();
    assert_strokes(&mut app, &expected);
    let raider_blip = blip_centre(&app, raider);
    let arrival = map_px(&mut app, *points.last().unwrap());
    assert!(
        arrival.distance(raider_blip) > 5.0,
        "the arrival stands off the target blip"
    );
    assert!(
        route_strokes(&mut app)
            .iter()
            .all(|(_, to, _)| to.distance(raider_blip) > 1.0),
        "no stroke runs to the target blip"
    );
}

/// A fresh GOTO draws no route until its first prediction lands, however its
/// target moves, then exactly the predicted path. A prediction flown past its
/// off-grid final point, or removed, draws nothing again. The tag stays on the
/// live GOTO target throughout; a cancelled GOTO, or another order with a
/// prediction of its own, draws neither, and the travel lock outlives the
/// GOTO.
/// A GOTO on a body the Map does not plot, such as a scenario beacon, has no
/// tag to place, but its route is the predicted flight all the same.
#[test]
fn a_goto_on_a_body_off_the_map_draws_its_predicted_route_without_a_tag() {
    let (mut app, player, _, viewport) = goto_route_app();
    let beacon = app
        .world_mut()
        .spawn(GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -40.0)))
        .id();
    app.world_mut()
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
    let points = vec![
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, -15.0),
        Vec3::new(0.0, 0.0, -35.0),
    ];
    predict(
        &mut app,
        player,
        points.clone(),
        FlightPredictionEndType::Completed,
    );
    lay_out_map(&mut app, viewport, UVec2::new(1600, 1200), true);
    project_map_route_nodes(&mut app);
    let blip = blip_centre(&app, player);
    let (middle, end) = (map_px(&mut app, points[1]), map_px(&mut app, points[2]));
    assert_strokes(&mut app, &[(blip, middle, 0.8), (middle, end, 0.8)]);
    let world = app.world_mut();
    let tag = *world
        .query_filtered::<&Visibility, With<MapGotoMarker>>()
        .single(world)
        .unwrap();
    assert_eq!(tag, Visibility::Hidden, "no blip, no tag");
}

/// A GOTO on a body the Map does not plot, such as a scenario beacon locked
/// in flight, is live: the panel names it and its range as for a contact. It
/// reads `GOTO LOST` only once the target is gone, the rule the autopilot's
/// own target-gone check uses.
#[test]
fn a_goto_on_a_body_off_the_map_reads_its_name_and_range_until_it_is_gone() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.add_systems(Update, (assign_map_contact_codes, update_map_panel).chain());
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
        ))
        .id();
    let beacon = app
        .world_mut()
        .spawn((
            EntityTypeName::new(nova_events::prelude::BEACON_TYPE_NAME),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -210.0)),
            Name::new("Helm Beacon"),
        ))
        .id();
    app.world_mut()
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
    app.world_mut().resource_mut::<MapRuntime>().active = true;
    settle(app);
    let plotted = app
        .world_mut()
        .run_system_once(move |contacts: MapContacts| {
            contacts
                .collect()
                .iter()
                .any(|contact| contact.entity == beacon)
        })
        .unwrap();
    assert!(!plotted, "the beacon is not a map contact");
    assert_eq!(
        panel_text(app.world_mut(), MapPanelField::Destination),
        "GOTO Helm Beacon  2.10 km"
    );

    app.world_mut()
        .entity_mut(beacon)
        .insert(GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -90.0)));
    settle(app);
    assert_eq!(
        panel_text(app.world_mut(), MapPanelField::Destination),
        "GOTO Helm Beacon  900 m",
        "the range is live"
    );

    app.world_mut().entity_mut(beacon).despawn();
    settle(app);
    assert_eq!(
        panel_text(app.world_mut(), MapPanelField::Destination),
        "GOTO LOST"
    );
}

/// The wheel pulls back far enough to see the end of the GOTO route, the
/// decimated predicted path the Map draws, even when no contact is out
/// there: a route to a beacon the Map does not plot ran off the viewport.
#[test]
fn the_map_zoom_reaches_the_end_of_the_goto_route() {
    use bevy::input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
    };

    let mut app = map_input_app();
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(app.world())
        .unwrap();
    let beacon = app
        .world_mut()
        .spawn((
            EntityTypeName::new(nova_events::prelude::BEACON_TYPE_NAME),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -3000.0)),
            Name::new("Helm Beacon"),
        ))
        .id();
    app.world_mut()
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target: beacon }));
    // A bent leg whose far end is not the target: the reach follows the path.
    let end = Vec3::new(400.0, 0.0, -2900.0);
    predict(
        &mut app,
        player,
        vec![Vec3::ZERO, Vec3::new(300.0, 0.0, -1500.0), end],
        FlightPredictionEndType::Completed,
    );
    let radius = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .unwrap()
            .radius
    };
    let pull_back = |app: &mut App| {
        for _ in 0..200 {
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: -3.0,
                window: Entity::PLACEHOLDER,
                phase: TouchPhase::Moved,
            });
            app.world_mut().run_system_once(map_input).unwrap();
        }
    };
    pull_back(&mut app);
    assert_eq!(
        radius(&mut app),
        map_radius_max(end.length()),
        "the wheel stops where the route's end is in reach, not at the contacts"
    );

    // With the GOTO gone the reach is the contacts' again.
    app.world_mut().entity_mut(player).remove::<Autopilot>();
    pull_back(&mut app);
    assert_eq!(radius(&mut app), map_radius_max(0.0));
}

#[test]
fn a_goto_draws_no_route_until_its_prediction_lands() {
    let (mut app, player, raider, viewport) = goto_route_app();
    lay_out_map(&mut app, viewport, UVec2::new(1600, 1200), true);
    project_map_route_nodes(&mut app);
    assert_eq!(route_strokes(&mut app), [], "no prediction, no route");
    let fresh = blip_centre(&app, raider);
    assert_eq!(
        goto_tag(&mut app),
        (tag_on(fresh), Visibility::Inherited),
        "the tag marks the target at once"
    );

    app.world_mut()
        .entity_mut(raider)
        .insert(GlobalTransform::from(Transform::from_xyz(20.0, 0.0, -40.0)));
    project_map_route_nodes(&mut app);
    let raider_blip = blip_centre(&app, raider);
    assert!(
        raider_blip.distance(fresh) > 20.0,
        "the target moves on screen"
    );
    assert_eq!(
        route_strokes(&mut app),
        [],
        "a moving target draws no route"
    );

    let points = vec![Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0)];
    predict(
        &mut app,
        player,
        points.clone(),
        FlightPredictionEndType::Completed,
    );
    project_map_route_nodes(&mut app);
    let (blip, end) = (blip_centre(&app, player), map_px(&mut app, points[1]));
    assert_strokes(&mut app, &[(blip, end, 0.8)]);

    // The leg ends at 0.0625 s, between two samples. Past that tick the end
    // point is flown although its grid time, 0.125 s, is still ahead.
    app.world_mut()
        .get_mut::<FlightPrediction>(player)
        .unwrap()
        .final_point_time = 0.0625;
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .advance_by(std::time::Duration::from_millis(70));
    project_map_route_nodes(&mut app);
    assert_eq!(route_strokes(&mut app), [], "a flown path draws nothing");
    assert_eq!(
        goto_tag(&mut app),
        (tag_on(raider_blip), Visibility::Inherited),
        "the tag does not follow the prediction"
    );

    app.world_mut()
        .entity_mut(player)
        .remove::<FlightPrediction>();
    project_map_route_nodes(&mut app);
    assert_eq!(route_strokes(&mut app), []);
    assert_eq!(
        goto_tag(&mut app),
        (tag_on(raider_blip), Visibility::Inherited),
        "the GOTO is still live"
    );

    predict(
        &mut app,
        player,
        points.clone(),
        FlightPredictionEndType::Completed,
    );
    app.world_mut().entity_mut(player).remove::<Autopilot>();
    project_map_route_nodes(&mut app);
    assert_eq!(
        route_strokes(&mut app),
        [],
        "a cancelled GOTO draws no path"
    );
    assert_eq!(goto_tag(&mut app).1, Visibility::Hidden);

    app.world_mut()
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    project_map_route_nodes(&mut app);
    assert_eq!(route_strokes(&mut app), [], "another order draws no path");
    assert_eq!(goto_tag(&mut app).1, Visibility::Hidden);
    assert_eq!(
        app.world().get::<TravelLock>(player),
        Some(&TravelLock(Some(raider))),
        "the travel lock outlives the GOTO"
    );
}

/// LMB is the contact-SELECT click (the blip `Button` widget's Primary
/// activation), so it must NOT orbit-drag the map camera - otherwise a small
/// press-with-motion drags the view and the blip slips out from under the
/// cursor before the click lands. RMB stays the orbit-drag button.
fn map_input_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        AssetPlugin::default(),
        bevy::input::InputPlugin,
    ));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Map);

    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Name::new("NOVA"),
    ));
    app.world_mut().run_system_once(manage_map_scene).unwrap();
    app
}

#[test]
fn map_orbit_drag_is_rmb_only() {
    use bevy::input::mouse::MouseMotion;

    let mut app = map_input_app();

    let orbit_angles = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .map(|o| (o.theta, o.phi))
            .unwrap()
    };
    let before = orbit_angles(&mut app);

    // Hold LMB and sweep the mouse: the camera must not orbit.
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().write_message(MouseMotion {
        delta: Vec2::new(60.0, 40.0),
    });
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(
        orbit_angles(&mut app),
        before,
        "LMB drag must NOT orbit the map camera (it selects contacts)"
    );

    // Hold RMB and sweep the same delta: the camera must orbit.
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Right);
    app.world_mut().write_message(MouseMotion {
        delta: Vec2::new(60.0, 40.0),
    });
    app.world_mut().run_system_once(map_input).unwrap();
    assert_ne!(
        orbit_angles(&mut app),
        before,
        "RMB drag must still orbit the map camera"
    );
}

/// An uncharted generated station's blip reads the bare `STN` stem; once the
/// chart finds it, [`project_map_blips`] relabels the same blip with its
/// chart code `STN-n`.
#[test]
fn a_station_blip_relabels_from_stn_to_its_chart_code_when_the_chart_finds_it() {
    let mut app = map_input_app();
    app.insert_resource(InterfaceIcons::blank());
    let station_id = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
    app.world_mut().spawn((
        SpaceshipRootMarker,
        StationMarker,
        station_id.clone(),
        GlobalTransform::default(),
    ));
    app.world_mut()
        .spawn((MapViewportMarker, Node::default(), ComputedNode::DEFAULT));

    let station_codes = |app: &mut App| -> Vec<String> {
        blip_label_texts(app.world_mut())
            .into_iter()
            .filter(|text| text.starts_with("STN"))
            .collect()
    };

    app.world_mut().run_system_once(project_map_blips).unwrap();
    assert_eq!(
        station_codes(&mut app),
        vec!["STN".to_string()],
        "uncharted, the blip reads the bare stem"
    );

    app.world_mut()
        .resource_mut::<Chart>()
        .find_station(&station_id, false, false, None, 0.0);
    app.world_mut().run_system_once(project_map_blips).unwrap();
    assert_eq!(
        station_codes(&mut app),
        vec!["STN-1".to_string()],
        "the chart's find relabels the live blip"
    );
}

/// The chart view's way back to its overview: a first click on a mark
/// selects it and moves the orbit centre onto its cell with the radius and
/// angles kept, a second click clears it with the view kept, and Reframe
/// clears a selection and frames the whole grid.
#[test]
fn a_second_click_on_the_selected_chart_mark_and_reframe_return_the_chart_to_its_overview() {
    let mut app = map_input_app();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.chart = true;
        runtime.chart_player_cell = Some(SectorCoord::new(4, 1, 4));
        runtime.chart_layer = 1;
    }
    let mark = MapChartMarkType::Cell(SectorCoord::new(6, 2, 3));
    let label = app
        .world_mut()
        .spawn(MapChartLabel {
            anchor: Vec3::ZERO,
            mark: Some(mark.clone()),
        })
        .observe(on_map_chart_label_click)
        .id();
    let orbit = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .map(|orbit| (orbit.center, orbit.radius, orbit.theta, orbit.phi))
            .unwrap()
    };
    let selected = |app: &App| app.world().resource::<MapRuntime>().chart_selected.clone();
    let (_, radius, theta, phi) = orbit(&mut app);
    let mark_cell = Vec3::new(2.0, 1.0, -1.0) * MAP_CHART_CELL;

    app.world_mut().trigger(Activate { entity: label });
    app.world_mut().flush();
    assert_eq!(selected(&app), Some(mark.clone()), "a first click selects");
    assert_eq!(
        app.world().resource::<MapRuntime>().chart_layer,
        2,
        "the grid follows the focused mark's layer"
    );
    assert_eq!(
        orbit(&mut app),
        (mark_cell, radius, theta, phi),
        "a first click moves the centre onto the mark's cell and keeps the zoom and angles"
    );

    app.world_mut().trigger(Activate { entity: label });
    app.world_mut().flush();
    assert_eq!(selected(&app), None, "a second click clears the selection");
    assert_eq!(
        orbit(&mut app),
        (mark_cell, radius, theta, phi),
        "a second click keeps the view"
    );

    app.world_mut().trigger(Activate { entity: label });
    app.world_mut().flush();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(selected(&app), None, "Reframe clears the selection");
    assert_eq!(
        app.world().resource::<MapRuntime>().chart_layer,
        1,
        "Reframe returns to the player's layer"
    );
    assert_eq!(
        orbit(&mut app),
        (
            Vec3::ZERO,
            MAP_CHART_RADIUS,
            MAP_THETA_DEFAULT,
            MAP_PHI_DEFAULT
        ),
        "Reframe frames the whole grid"
    );
}

/// Chart layer keys step the focused Y grid once per press, discard a selected
/// mark, and Reframe restores the player's layer without changing local Map.
#[test]
fn chart_layer_steps_once_per_press_and_reframe_returns_to_the_player_layer() {
    let mut app = map_input_app();
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.chart = true;
        runtime.chart_layer = 4;
        runtime.chart_player_cell = Some(SectorCoord::new(2, 4, 3));
        runtime.chart_selected = Some(MapChartMarkType::Cell(SectorCoord::new(2, 4, 3)));
    }
    let center = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .unwrap()
            .center
    };
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyX);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(app.world().resource::<MapRuntime>().chart_layer, 5);
    assert_eq!(app.world().resource::<MapRuntime>().chart_selected, None);
    assert_eq!(center(&mut app).y, MAP_CHART_CELL);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(
        app.world().resource::<MapRuntime>().chart_layer,
        5,
        "holding X does not repeat"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyX);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyZ);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(app.world().resource::<MapRuntime>().chart_layer, 4);
    assert_eq!(center(&mut app).y, 0.0);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyZ);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyX);
    app.world_mut().run_system_once(map_input).unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(app.world().resource::<MapRuntime>().chart_layer, 4);
    assert_eq!(
        center(&mut app),
        Vec3::ZERO,
        "Reframe resets the layer and orbit"
    );
}

/// A held pan moves the map focus by real time, not by frame count. The
/// interface pauses `Time<Virtual>`, and a per-frame floor on that zero delta
/// made one second of W cover four times the ground at 120 Hz that it covered
/// at 30 Hz.
#[test]
fn a_held_pan_covers_the_same_ground_at_30_and_120_hz() {
    let pan_for_one_second = |hz: u32| {
        let mut app = map_input_app();
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        // The first real update only starts the clock, with a zero delta.
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .update_with_duration(std::time::Duration::ZERO);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.world_mut().run_system_once(drive_map_camera).unwrap();
        let focus = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
                .single(app.world())
                .map(|orbit| orbit.center)
                .unwrap()
        };
        let start = focus(&mut app);
        for _ in 0..hz {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .update_with_duration(std::time::Duration::from_secs_f64(1.0 / f64::from(hz)));
            app.world_mut().run_system_once(map_input).unwrap();
            app.world_mut().run_system_once(drive_map_camera).unwrap();
        }
        focus(&mut app).distance(start)
    };

    let at_30 = pan_for_one_second(30);
    let at_120 = pan_for_one_second(120);
    assert!(at_30 > 0.0, "a held pan moves the focus");
    assert!(
        (at_120 - at_30).abs() <= at_30 * 1e-3,
        "one second of pan covers {at_30} at 30 Hz and {at_120} at 120 Hz"
    );
}

/// Stand a map viewport up inside the rig's content root, clipped exactly as
/// the pane body's is, and return it.
fn rig_map_viewport(rig: &mut PanePointerRig) -> Entity {
    let viewport = rig
        .app
        .world_mut()
        .spawn((
            MapViewportMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                // The production body clips its viewport; a fix that only
                // works on an unclipped viewport is not a fix.
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    rig.app
        .world_mut()
        .entity_mut(rig.content_root)
        .add_child(viewport);
    viewport
}

fn rig_contact(entity: Entity, code: &str) -> MapContact {
    MapContact {
        entity,
        kind: MapContactKind::Hostile,
        code: code.to_string(),
        name: code.to_string(),
        marker: None,
        world_pos: Vec3::ZERO,
        range: 100.0,
        bearing_deg: 0.0,
        mark_deg: 0.0,
        body: BodyIconType::Ship,
    }
}

/// Put a real map blip (the production `spawn_blip` markup and its real
/// `Activate` observer) with its tile centred on viewport pixel `at`, shown
/// the way the projection shows it.
fn rig_place_blip(
    rig: &mut PanePointerRig,
    viewport: Entity,
    contact: &MapContact,
    at: Vec2,
) -> Entity {
    let blip = rig
        .app
        .world_mut()
        .run_system_once_with(
            |input: In<(Entity, MapContact)>, mut commands: Commands| {
                let (viewport, contact) = input.0;
                spawn_blip(&mut commands, viewport, &contact, &InterfaceIcons::blank())
            },
            (viewport, contact.clone()),
        )
        .expect("spawning a blip through the production path");
    let mut node = rig
        .app
        .world_mut()
        .get_mut::<Node>(blip)
        .expect("the blip has a Node");
    node.left = Val::Px(at.x - MAP_BLIP_PX * 0.5);
    node.top = Val::Px(at.y - MAP_BLIP_PX * 0.5);
    // The projection reveals a blip once it has placed it.
    *rig.app
        .world_mut()
        .get_mut::<Visibility>(blip)
        .expect("the blip has a Visibility") = Visibility::Inherited;
    settle(&mut rig.app);
    blip
}

/// The laid-out border box of a node in window space.
fn rig_rect(rig: &PanePointerRig, entity: Entity) -> Rect {
    let world = rig.app.world();
    let node = world
        .get::<ComputedNode>(entity)
        .unwrap_or_else(|| panic!("{entity:?} never reached UI layout"));
    let xf = world
        .get::<UiGlobalTransform>(entity)
        .unwrap_or_else(|| panic!("{entity:?} has no UI transform"));
    Rect::from_center_size(xf.translation, node.size())
}

/// The blip's label node - the child that is not the drawn icon.
fn rig_label_of(rig: &PanePointerRig, blip: Entity) -> Entity {
    let world = rig.app.world();
    let children = world
        .get::<Children>(blip)
        .expect("the blip has a label child");
    let labels: Vec<Entity> = children
        .iter()
        .filter(|child| world.get::<ImageNode>(*child).is_none())
        .collect();
    assert_eq!(
        labels.len(),
        1,
        "the blip's target carries its icon plus ONE label child"
    );
    labels[0]
}

/// DoD 3: the label is as clickable as the dot, which means the two targets
/// TOUCH. The owner's comparison - "on labels, clicks work 99% of the time"
/// in the Ship pane - is the bar, and the Ship pane's label is a padded backing
/// pill starting 2 px from its dot. The map's bare text node started 4 px out
/// with no padding of its own, leaving a dead band between dot and label that
/// selects nothing, and a target tight to the glyph run on every side.
///
/// Both halves are read from the LIVE tree and clicked through real window
/// picking, so neither the gap nor the padding can be satisfied on paper.
#[test]
fn map_contact_label_and_dot_are_one_unbroken_target() {
    let mut rig = pane_pointer_rig();
    rig.app.init_resource::<MapRuntime>();
    rig.app.init_resource::<Chart>();
    let viewport = rig_map_viewport(&mut rig);

    let target_id = rig.app.world_mut().spawn_empty().id();
    let contact = rig_contact(target_id, "HOST-1");
    let blip = rig_place_blip(&mut rig, viewport, &contact, Vec2::new(448.0, 324.0));
    let label = rig_label_of(&rig, blip);

    let dot = rig_rect(&rig, blip);
    let label_box = rig_rect(&rig, label);
    assert!(
        label_box.min.x <= dot.max.x + 0.01,
        "the label starts at x {} but the dot ends at x {} - {:.1} px of dead \
         band between the two halves of one target",
        label_box.min.x,
        dot.max.x,
        label_box.min.x - dot.max.x,
    );

    // A solid, padded backing box like the Ship pane's pill, not a box tight to
    // the glyph run: the label must be taller than its own text.
    let label_frame = {
        let computed = rig
            .app
            .world()
            .get::<ComputedNode>(label)
            .expect("the label reached UI layout");
        computed.padding.min_inset
            + computed.padding.max_inset
            + computed.border.min_inset
            + computed.border.max_inset
    };
    assert!(
        label_frame.x > 0.0 && label_frame.y > 0.0,
        "the label carries no padding of its own ({label_frame:?}) - its hit \
         target is tight to the glyph run, unlike the Ship pane's pill",
    );
    assert!(
        rig.app.world().get::<BackgroundColor>(label).is_some(),
        "the label has no backing fill, so there is nothing solid to aim at",
    );

    // Every point across the seam - dot centre, the gap between dot and
    // pill, label centre - selects the contact. Positions come from the live
    // window-space rects. Swept at pixel CENTRES, since the shared edge itself
    // is a measure-zero boundary bevy's `contains_point` excludes from both
    // rects.
    let y = dot.center().y;
    let first = dot.center().x + 0.5;
    let last = label_box.min.x + 6.0;
    // Counted, not accumulated: the step is a whole pixel, so the sweep is
    // `first + i` and the float never carries rounding from one probe to the
    // next.
    let steps = ((last - first).floor() as i32 + 1).max(0);
    let mut probed = 0;
    for step in 0..steps {
        let x = first + step as f32;
        let window_px = Vec2::new(x, y);
        rig.app.world_mut().resource_mut::<MapRuntime>().selected = None;
        click_at(&mut rig, window_px);
        assert_eq!(
            rig.app.world().resource::<MapRuntime>().selected,
            Some(target_id),
            "clicking window px {window_px:?} - between the dot's centre and 6 px \
             into the label - must select the contact",
        );
        probed += 1;
    }
    assert!(
        probed >= 12,
        "the sweep only probed {probed} points - it is not crossing the seam"
    );
}

/// A viewport inset inside the content root, so its clip rect is a real
/// boundary rather than the content edge.
fn rig_inset_map_viewport(rig: &mut PanePointerRig, inset: Rect) -> Entity {
    let viewport = rig
        .app
        .world_mut()
        .spawn((
            MapViewportMarker,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(inset.min.x),
                top: Val::Px(inset.min.y),
                width: Val::Px(inset.width()),
                height: Val::Px(inset.height()),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    rig.app
        .world_mut()
        .entity_mut(rig.content_root)
        .add_child(viewport);
    viewport
}

/// A contact at the viewport edge must not be lost to clipping. The map
/// viewport is `Overflow::clip()`, and bevy's UI picking respects clip
/// rects, so a blip straddling the viewport edge is pickable over its
/// UNCLIPPED part and dead over the clipped part.
///
/// Both halves are asserted: the visible half must select (that is the bug
/// this guards), and the clipped half must NOT (otherwise the test would pass
/// against a build that ignores clipping entirely).
#[test]
fn map_contacts_straddling_the_viewport_edge_are_pickable_over_their_visible_half() {
    let inset = Rect::new(120.0, 90.0, 900.0, 600.0);
    let mut rig = pane_pointer_rig();
    rig.app.init_resource::<MapRuntime>();
    rig.app.init_resource::<Chart>();
    let viewport = rig_inset_map_viewport(&mut rig, inset);

    let target_id = rig.app.world_mut().spawn_empty().id();
    let contact = rig_contact(target_id, "HOST-1");
    // Straddle the viewport's right edge: half the dot inside, half outside.
    // `place` is viewport-local; the rects below are in window space.
    let blip = rig_place_blip(
        &mut rig,
        viewport,
        &contact,
        Vec2::new(inset.max.x - inset.min.x, 200.0),
    );
    let edge = RIG_PANEL_MIN.x + inset.max.x;
    let dot = rig_rect(&rig, blip);
    assert!(
        dot.min.x < edge && dot.max.x > edge,
        "the rig meant to straddle the clip edge at x {edge} but the dot is {dot:?}",
    );

    let probe = |rig: &mut PanePointerRig, at: Vec2| {
        rig.app.world_mut().resource_mut::<MapRuntime>().selected = None;
        click_at(rig, at);
        rig.app.world().resource::<MapRuntime>().selected
    };

    let inside = Vec2::new(edge - 3.5, dot.center().y);
    assert_eq!(
        probe(&mut rig, inside),
        Some(target_id),
        "the visible half of an edge contact (window px {inside:?}) must still \
         select it",
    );

    let outside = Vec2::new(edge + 3.5, dot.center().y);
    assert_eq!(
        probe(&mut rig, outside),
        None,
        "the clipped half (window px {outside:?}) draws nothing, so it must not \
         select either - otherwise this test does not exercise clipping",
    );
}

/// The overlap path: map contacts drift, so a label routinely lies over a
/// neighbouring dot. UI picking resolves that by stacking order, and the
/// TOPMOST node wins - deterministically, not by accident. Pinned so the
/// bigger label pill cannot quietly start swallowing its neighbours' clicks
/// in some other order.
#[test]
fn overlapping_map_contacts_select_the_topmost() {
    let mut rig = pane_pointer_rig();
    rig.app.init_resource::<MapRuntime>();
    rig.app.init_resource::<Chart>();
    hear_ui_cues(&mut rig.app);
    let viewport = rig_map_viewport(&mut rig);

    let under_id = rig.app.world_mut().spawn_empty().id();
    let over_id = rig.app.world_mut().spawn_empty().id();
    let at = Vec2::new(576.0, 360.0);
    // Spawned first = lower in the UI stack; the second sits exactly on top.
    rig_place_blip(&mut rig, viewport, &rig_contact(under_id, "HOST-1"), at);
    rig_place_blip(&mut rig, viewport, &rig_contact(over_id, "HOST-2"), at);

    click_at(&mut rig, RIG_PANEL_MIN + at);
    assert_eq!(
        rig.app.world().resource::<MapRuntime>().selected,
        Some(over_id),
        "two contacts stacked on the same pixel resolve to the topmost, not to \
         whichever the hit test happened to visit first",
    );
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    // The selected contact again changes nothing and stays silent.
    click_at(&mut rig, RIG_PANEL_MIN + at);
    assert!(take_cues(&mut rig.app).is_empty());
}

/// A scenario spread over 20 km has to be reachable. The old fixed 520 unit
/// ceiling put a contact that far out permanently past the wheel, on a map
/// whose whole job is showing where things are.
#[test]
fn the_map_frames_and_reaches_the_live_contact_spread() {
    // Twenty kilometres, in world units.
    let spread = Meters(20_000.0).to_engine();

    let framing = map_radius_default(spread);
    assert!(
        framing > spread,
        "the default framing stands back from the spread, got {framing} for {spread}"
    );
    assert!(
        map_radius_max(spread) > framing,
        "the wheel still has room past the opening frame"
    );

    // A tight scene keeps the composition the map has always opened at.
    assert_eq!(map_radius_default(0.0), MAP_RADIUS_DEFAULT_MIN);
    assert_eq!(map_radius_default(10.0), MAP_RADIUS_DEFAULT_MIN);
}

/// The floor rings are a scale reading, so they land on round metric steps at
/// whatever scale the map is currently framed at.
#[test]
fn the_floor_rings_land_on_round_metric_steps() {
    let rings = map_ring_radii(MAP_RADIUS_DEFAULT_MIN);
    assert_eq!(
        rings.map(|ring| Meters::from_engine(ring).0),
        [500.0, 1_000.0, 1_500.0],
        "the default framing reads in half-kilometres"
    );

    // Ten times the framing moves the ladder up a decade, not off it.
    let wide = map_ring_radii(MAP_RADIUS_DEFAULT_MIN * 10.0);
    assert_eq!(
        wide.map(|ring| Meters::from_engine(ring).0),
        [5_000.0, 10_000.0, 15_000.0]
    );

    // Evenly spaced, always: the rings are a ruler.
    for rings in [rings, wide] {
        assert!((rings[1] - rings[0] - (rings[2] - rings[1])).abs() < 1e-3);
    }
}

/// The focus hub marks the focused BODY, so it is the size of that body: a
/// fixed 16 m sphere buried a skiff and vanished inside a planetoid.
#[test]
fn the_focus_hub_is_the_size_of_what_it_marks() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(PauseStates::Interface);
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.world_mut().resource_mut::<MapRuntime>().active = true;

    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Name::new("NOVA"),
    ));
    let carrier = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            GlobalTransform::from(Transform::from_xyz(300.0, 0.0, 0.0)),
            Name::new("CARRIER"),
            HullEnvelopeRadius(19.53),
        ))
        .id();
    app.world_mut().spawn((
        MapCameraMarker,
        MapOrbit {
            theta: MAP_THETA_DEFAULT,
            phi: MAP_PHI_DEFAULT,
            radius: MAP_RADIUS_DEFAULT_MIN,
            center: Vec3::ZERO,
        },
    ));
    app.world_mut()
        .spawn((MapFocusAnchor, Transform::default()));
    let hub = app
        .world_mut()
        .spawn((MapFocusHub, Transform::default()))
        .id();

    // Nothing focused: the hub is the floor, a mark on the map floor.
    app.world_mut().run_system_once(map_focus_follow).unwrap();
    let empty = app.world().get::<Transform>(hub).unwrap().scale.x;
    assert!((empty - MAP_HUB_MIN.to_engine()).abs() < 1e-4);

    // Focused on the carrier: the hub IS the carrier's envelope.
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(carrier);
    app.world_mut().run_system_once(map_focus_follow).unwrap();
    let framed = app.world().get::<Transform>(hub).unwrap().scale.x;
    assert!((framed - 19.53).abs() < 1e-4, "hub scale {framed}");
    assert!(framed > empty);
}

/// The label a shown Range or Bearing kv row reads, or `None` while the row
/// hides.
fn slot_label(world: &mut World, field: MapPanelField) -> Option<String> {
    let shown = world
        .query::<(&MapPanelSlot, &Node)>()
        .iter(world)
        .find(|(slot, _)| slot.0 == field)
        .map(|(_, node)| node.display != Display::None)
        .unwrap_or_else(|| panic!("the panel has a {field:?} row"));
    let label = world
        .query::<(&MapPanelLabel, &Text)>()
        .iter(world)
        .find(|(label, _)| label.0 == field)
        .map(|(_, text)| text.0.clone())
        .unwrap_or_else(|| panic!("the {field:?} row has a label"));
    shown.then_some(label)
}

/// The contact panel's text lines, by field.
fn panel_text(world: &mut World, field: MapPanelField) -> String {
    world
        .query::<(&MapPanelField, &Text)>()
        .iter(world)
        .find(|(each, _)| **each == field)
        .map(|(_, text)| text.0.clone())
        .unwrap_or_else(|| panic!("the panel has a {field:?} line"))
}

#[test]
fn a_selected_contact_fills_the_panel_in_place() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.add_systems(Update, (assign_map_contact_codes, update_map_panel).chain());
    track_node_churn(app);
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
        Name::new("NOVA"),
    ));
    let raider = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
        ))
        .id();
    app.world_mut().resource_mut::<MapRuntime>().active = true;
    settle(app);
    let world = app.world_mut();
    assert_eq!(panel_text(world, MapPanelField::Code), "");
    assert_eq!(panel_text(world, MapPanelField::Range), "");
    assert_eq!(slot_label(world, MapPanelField::Range), None);
    take_churn(app);

    app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
    settle(app);
    let churn = take_churn(app);
    assert_eq!((churn.spawned, churn.despawned), (0, 0), "{churn:?}");
    let world = app.world_mut();
    for (field, want) in [
        (MapPanelField::Code, "HOST-1"),
        (MapPanelField::Name, "RAIDER"),
        (MapPanelField::Kind, "HOSTILE"),
        (MapPanelField::Range, "500 m"),
        (MapPanelField::Bearing, "000 mark +00"),
        (MapPanelField::Note, ""),
    ] {
        assert_eq!(panel_text(world, field), want, "{field:?}");
    }
    // Range and Bearing are kv rows: the label is the row's, not the value's.
    for (field, label) in [
        (MapPanelField::Range, "Range"),
        (MapPanelField::Bearing, "Bearing"),
    ] {
        assert_eq!(
            slot_label(world, field).as_deref(),
            Some(label),
            "{field:?}"
        );
    }
    let icon = world
        .query_filtered::<&Visibility, With<MapPanelIcon>>()
        .single(world)
        .unwrap();
    assert_eq!(*icon, Visibility::Inherited);

    // A held selection writes nothing.
    settle(app);
    assert_eq!(take_churn(app), NodeChurn::default());
}

/// The chart overview's Code and Range fields read blank with nothing
/// selected, since the contact panel header renders unreliably there; a
/// selected station still reads its chart code and its cell in Range.
#[test]
fn chart_selection_fills_code_and_range_in_place() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.add_systems(Update, update_map_panel);
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
        Name::new("NOVA"),
    ));
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
    }
    settle(app);
    let world = app.world_mut();
    assert_eq!(panel_text(world, MapPanelField::Code), "");
    assert_eq!(panel_text(world, MapPanelField::Range), "");

    let station_id = GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_0_0_0_station", 0);
    app.world_mut()
        .resource_mut::<Chart>()
        .find_station(&station_id, false, false, None, 0.0);
    app.world_mut().resource_mut::<MapRuntime>().chart_selected =
        Some(MapChartMarkType::Station(station_id.clone()));
    settle(app);
    let world = app.world_mut();
    assert_eq!(panel_text(world, MapPanelField::Code), "STN-1");
    assert_eq!(
        panel_text(world, MapPanelField::Range),
        station_id.origin.to_string()
    );
    // The local view's Range row names the chart mark's cell here, and a
    // station mark has nothing for the Bearing row.
    assert_eq!(
        slot_label(world, MapPanelField::Range).as_deref(),
        Some("Cell")
    );
    assert_eq!(slot_label(world, MapPanelField::Bearing), None);

    let mut q = world.query::<(&MapPanelField, &ThemedText, &ComputedNode)>();
    for (field, themed, computed) in q.iter(world) {
        if *field == MapPanelField::Code || *field == MapPanelField::Range {
            assert_eq!(themed.color, UiColorType::Text, "{field:?}");
            assert!((themed.alpha - 1.0).abs() < 1e-6, "{field:?}");
            assert!(computed.size().y > 0.0, "{field:?} laid out at zero height");
        }
    }
}

/// The header node (icon frame, Code/Name/Kind, Range/Bearing and Note)
/// hides as one with nothing selected, on local Map and in the chart
/// overview alike, and shows on a selected contact or chart mark.
#[test]
fn map_header_hides_only_with_nothing_selected() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.add_systems(Update, update_map_panel);
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
        ))
        .id();
    app.world_mut().resource_mut::<MapRuntime>().active = true;
    settle(app);

    fn header_display(world: &mut World) -> Display {
        world
            .query_filtered::<&Node, With<MapPanelHeaderMarker>>()
            .single(world)
            .unwrap()
            .display
    }
    assert_eq!(
        header_display(app.world_mut()),
        Display::None,
        "hidden on local Map with nothing selected"
    );
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(player);
    settle(app);
    assert_eq!(
        header_display(app.world_mut()),
        Display::Flex,
        "shown on local Map with a contact selected"
    );

    app.world_mut().resource_mut::<MapRuntime>().chart = true;
    settle(app);
    assert_eq!(
        header_display(app.world_mut()),
        Display::None,
        "hidden in the chart overview with nothing selected"
    );

    let station_id = GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_0_0_0_station", 0);
    app.world_mut()
        .resource_mut::<Chart>()
        .find_station(&station_id, false, false, None, 0.0);
    app.world_mut().resource_mut::<MapRuntime>().chart_selected =
        Some(MapChartMarkType::Station(station_id));
    settle(app);
    assert_eq!(
        header_display(app.world_mut()),
        Display::Flex,
        "shown again on a selected chart mark"
    );
}

/// The OBJECTIVES heading carries the focused chart layer; stepping the
/// layer alone, with no objective or goal change, still rebuilds the heading
/// off the cached rows.
#[test]
fn chart_objectives_heading_follows_the_focused_layer() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.init_resource::<GoalProgress>();
    app.init_resource::<GoalRecord>();
    app.add_systems(Update, update_map_chart_objectives);
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    app.world_mut()
        .resource_mut::<Chart>()
        .visit(SectorCoord::new(0, 2, 0), vec![], 0.0);
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
        runtime.chart_layer = 2;
    }
    settle(app);

    /// The heading's title and the aside beside it in the head row.
    fn heading(world: &mut World) -> (String, String) {
        let (text, row) = world
            .query::<(&Text, &ChildOf)>()
            .iter(world)
            .find_map(|(text, child_of)| {
                (text.0 == "OBJECTIVES").then(|| (text.0.clone(), child_of.parent()))
            })
            .expect("the OBJECTIVES heading spawned");
        let aside = world
            .get::<Children>(row)
            .expect("the heading sits in its head row")
            .iter()
            .filter_map(|child| world.get::<Text>(child))
            .map(|text| text.0.clone())
            .find(|aside| *aside != text)
            .expect("the head row carries the layer");
        (text, aside)
    }
    assert_eq!(
        heading(app.world_mut()),
        ("OBJECTIVES".to_string(), "Layer 2".to_string())
    );

    app.world_mut().resource_mut::<MapRuntime>().chart_layer = 3;
    settle(app);
    assert_eq!(
        heading(app.world_mut()),
        ("OBJECTIVES".to_string(), "Layer 3".to_string())
    );
}

/// A marked contact's Note is its marker label, then one line per held job
/// that sends the player to that body, in accept order: a poster lists each
/// done job it pays, a caller its undone job. A scenario marker on a body
/// with no generated id reads its label alone.
#[test]
fn a_marked_job_body_lists_every_matching_job() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.add_systems(Update, (assign_map_contact_codes, update_map_panel).chain());
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    app.world_mut().spawn((
        SpaceshipRootMarker,
        PlayerSpaceshipMarker,
        GlobalTransform::default(),
        Name::new("NOVA"),
    ));
    let poster = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
    let caller = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_derelict", 0);
    let job = |slot, kind| Job {
        id: JobId::slot(poster.clone(), slot, 0),
        kind,
        target: poster.clone(),
        leg_m: 0,
        threat: 0,
        reward: JobReward::flat(100),
    };
    let ore = |slot, count| {
        job(
            slot,
            JobType::Procure {
                item: ItemDesignId::from("CarbonOre"),
                count,
            },
        )
    };
    let first = ore(0, 4);
    let distress = job(
        1,
        JobType::Distress {
            caller: caller.clone(),
        },
    );
    let second = ore(2, 2);
    let mut jobs = app.world_mut().resource_mut::<Jobs>();
    for held in [&first, &distress, &second] {
        jobs.accept(held.clone());
    }
    jobs.mark_done(&first.id);
    jobs.mark_done(&second.id);

    let station = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            StationMarker,
            Allegiance::Neutral,
            poster.clone(),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("KESTREL"),
            ObjectiveMarkerTarget {
                label: "JOB-1, JOB-3 Hand in".into(),
            },
        ))
        .id();
    let caller_ship = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            caller,
            EntityId::new("caller"),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 50.0)),
            Name::new("CALLER"),
            ObjectiveMarkerTarget {
                label: "JOB-2 Answer".into(),
            },
        ))
        .id();
    let drone = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            EntityId::new("drone"),
            GlobalTransform::from(Transform::from_xyz(50.0, 0.0, 0.0)),
            Name::new("DRONE"),
            ObjectiveMarkerTarget {
                label: "Disable the drone".into(),
            },
        ))
        .id();
    app.world_mut().resource_mut::<MapRuntime>().active = true;

    for (entity, code, name, note) in [
        (
            station,
            "STN",
            "KESTREL",
            "Objective: JOB-1, JOB-3 Hand in\n\
             JOB-1 Bring 4 CarbonOre: done\n\
             JOB-3 Bring 2 CarbonOre: done"
                .to_string(),
        ),
        (
            caller_ship,
            "OBJ-1",
            "CALLER",
            format!(
                "Objective: JOB-2 Answer\nJOB-2 Answer a distress call: {}",
                distress.task(&GameItems::default())
            ),
        ),
        (
            drone,
            "OBJ-2",
            "DRONE",
            "Objective: Disable the drone".to_string(),
        ),
    ] {
        app.world_mut().resource_mut::<MapRuntime>().selected = Some(entity);
        settle(app);
        let world = app.world_mut();
        for (field, want) in [
            (MapPanelField::Code, code),
            (MapPanelField::Name, name),
            (MapPanelField::Kind, "OBJECTIVE"),
            (MapPanelField::Note, note.as_str()),
        ] {
            assert_eq!(panel_text(world, field), want, "{name} {field:?}");
        }
    }
}

/// Every label text under the map blips, read from the live tree.
fn blip_label_texts(world: &mut World) -> Vec<String> {
    let blips: Vec<Entity> = world
        .query_filtered::<Entity, With<MapBlip>>()
        .iter(world)
        .collect();
    let mut texts = Vec::new();
    for blip in blips {
        let mut pending = vec![blip];
        while let Some(entity) = pending.pop() {
            if let Some(text) = world.get::<Text>(entity) {
                texts.push(text.0.clone());
            }
            if let Some(children) = world.get::<Children>(entity) {
                pending.extend(children.iter());
            }
        }
    }
    texts
}

/// A streamed ship's `Name` says whose it is and what it was, which is for
/// inspection, not for a distant label. A ship whose spawn lands after this
/// frame's minting must not reach a blip under its name: it waits for its
/// minted code, and the panel still reads its name once it is selected.
#[test]
fn a_ship_that_lands_after_minting_never_labels_its_blip_with_its_name() {
    const WRECK: &str = "Halcor derelict, former scavenger";
    let (mut world, _player, _raider) = scripted_world();
    world.run_system_once(assign_map_contact_codes).unwrap();
    let viewport = world.spawn(MapViewportMarker).id();
    let wreck = world
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -400.0)),
            Name::new(WRECK),
        ))
        .id();

    // What `project_map_blips` does on a contact's first sight.
    let first_sight = |world: &mut World| {
        world
            .run_system_once_with(
                |viewport: In<Entity>, contacts: MapContacts, mut commands: Commands| {
                    for contact in contacts.collect() {
                        spawn_blip(&mut commands, *viewport, &contact, &InterfaceIcons::blank());
                    }
                },
                viewport,
            )
            .unwrap();
    };
    first_sight(&mut world);
    let leaked: Vec<String> = blip_label_texts(&mut world)
        .into_iter()
        .filter(|text| text.to_uppercase().contains("HALCOR"))
        .collect();
    assert!(
        leaked.is_empty(),
        "a blip is labelled with the wreck's name: {leaked:?}"
    );

    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let contact = contacts
        .iter()
        .find(|contact| contact.entity == wreck)
        .expect("the wreck is a contact once its code is minted");
    assert_eq!(contact.code, "NEU-1");
    assert_eq!(contact.name, WRECK, "the panel still reads its name");
}

const CHART_EDGE: ChartSectorEdge = ChartSectorEdge(nova_events::prelude::Meters(32_000.0));

fn chart_station_id() -> GeneratedBodyId {
    GeneratedBodyId::new(SectorCoord::new(4, 0, -1), "cluster_4_0_n1_station", 0)
}

/// A ledger whose station record for `id` froze at `at`, frozen by the
/// production path and carried through the record's RON form: no ledger
/// body has a public constructor outside `nova_world`.
fn frozen_station_ledger(id: &GeneratedBodyId, at: Vec3) -> nova_world::prelude::FrozenSectors {
    use nova_scenario::prelude::{freeze_station, spaceship_scenario_object, SpaceshipConfig};
    let mut world = World::new();
    let station = world
        .spawn((
            spaceship_scenario_object(SpaceshipConfig::default()),
            ShipInventory::new(&nova_gameplay::test_support::test_items(), 10_000, []),
            DamageMarks::default(),
            RcsBudget::default(),
            StationMarker,
            StationPost::Fixed {
                position: Vec3::ZERO,
            },
            StationServices(Vec::new()),
            StationBuilds(Vec::new()),
            StationAdvancement(0.0),
            StationPdcReserve::full(),
            StationStorage::new(0),
        ))
        .id();
    let body = nova_world::prelude::FrozenBodyType::Station(
        Box::new(freeze_station(&world, station).expect("a bare station is settled")),
        Vec::new(),
    );
    ron::from_str(&format!(
        "{{{}: Visited([(id: Some({}), name: None, transform: {}, visibility: None, \
         motion: None, body: {}, frozen_at: 0.0)])}}",
        ron::to_string(&id.origin).unwrap(),
        ron::to_string(&id.to_string()).unwrap(),
        ron::to_string(&Transform::from_translation(at)).unwrap(),
        ron::to_string(&body).unwrap(),
    ))
    .unwrap()
}

/// In the chart view GOTO on a cell flies `GotoPos` at its centre, on a
/// station out of the window flies the pose its record froze at, and on a
/// live station flies a `Goto` of it with the travel lock at once; the ship
/// wears the waypoint each time. GOTO again on the flown mark, or CANCEL,
/// ends it. A wreck, a job, an uncharted station, a charted station with no
/// record, the docked partner and a world with no cell edge each play the
/// deny cue and set nothing.
#[test]
fn a_chart_goto_flies_the_mark_of_a_station_or_cell_and_refuses_the_rest() {
    let mut app = map_input_app();
    app.insert_resource(CHART_EDGE);
    app.init_resource::<nova_world::prelude::FrozenSectors>();
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(app.world())
        .unwrap();
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
    }
    let select = |app: &mut App, mark: MapChartMarkType| {
        app.world_mut().resource_mut::<MapRuntime>().chart_selected = Some(mark);
    };
    let press = |app: &mut App| {
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyG);
        keys.press(KeyCode::KeyG);
        app.world_mut().run_system_once(map_input).unwrap();
    };
    let take_cue = |app: &mut App| {
        std::mem::take(&mut app.world_mut().resource_mut::<MapRuntime>().cue).pending()
    };
    let flying = |app: &App| {
        (
            app.world().get::<Waypoint>(player).cloned(),
            app.world().get::<Autopilot>(player).map(|a| a.action),
        )
    };
    let refused = |app: &mut App, case: &str| {
        assert_eq!(take_cue(app), Some(ActionOutcomeType::Refused), "{case}");
        assert_eq!(flying(app), (None, None), "{case}: nothing set");
    };

    // A cell: its centre.
    let cell = SectorCoord::new(-3, 0, 2);
    let centre = cell.centre(CHART_EDGE.0).to_engine();
    select(&mut app, MapChartMarkType::Cell(cell));
    press(&mut app);
    assert_eq!(take_cue(&mut app), None);
    assert_eq!(
        flying(&app),
        (
            Some(Waypoint {
                target: WaypointTargetType::Sector(cell),
                mark: centre,
            }),
            Some(AutopilotAction::GotoPos { position: centre })
        )
    );
    // GOTO again on the flown mark ends it.
    press(&mut app);
    assert_eq!(flying(&app), (None, None), "the toggle ends the waypoint");

    // Nothing to fly to.
    let station = chart_station_id();
    let wreck = GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_1_0_0_wreck", 0);
    select(&mut app, MapChartMarkType::Wreck(wreck.clone()));
    press(&mut app);
    refused(&mut app, "a wreck");
    select(
        &mut app,
        MapChartMarkType::Job(JobId {
            poster: station.clone(),
            source: JobSourceType::Slot {
                slot: 0,
                posting: 0,
            },
        }),
    );
    press(&mut app);
    refused(&mut app, "a job");
    select(&mut app, MapChartMarkType::Station(station.clone()));
    press(&mut app);
    refused(&mut app, "an uncharted station");
    app.world_mut()
        .resource_mut::<Chart>()
        .find_station(&station, false, false, None, 0.0);
    press(&mut app);
    refused(&mut app, "a charted station with no frozen record");

    // Frozen out of the window: where it froze.
    let pose = Vec3::new(12_000.0, -30.0, -9_000.0);
    app.insert_resource(frozen_station_ledger(&station, pose));
    press(&mut app);
    assert_eq!(take_cue(&mut app), None);
    assert_eq!(
        flying(&app),
        (
            Some(Waypoint {
                target: WaypointTargetType::Station(station.clone()),
                mark: pose,
            }),
            Some(AutopilotAction::GotoPos { position: pose })
        )
    );
    // CANCEL ends it from the chart.
    app.world_mut()
        .resource_mut::<MapRuntime>()
        .cancel_requested = true;
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(flying(&app), (None, None), "CANCEL ends the waypoint");

    // Live: a GOTO of the station with the lock on it.
    let live_at = Vec3::new(0.0, 0.0, -400.0);
    let live = app
        .world_mut()
        .spawn((
            StationMarker,
            station.clone(),
            GlobalTransform::from_translation(live_at),
        ))
        .id();
    press(&mut app);
    assert_eq!(take_cue(&mut app), None);
    assert_eq!(
        flying(&app),
        (
            Some(Waypoint {
                target: WaypointTargetType::Station(station.clone()),
                mark: live_at,
            }),
            Some(AutopilotAction::Goto { target: live })
        )
    );
    assert_eq!(
        app.world().get::<TravelLock>(player),
        Some(&TravelLock(Some(live)))
    );
    press(&mut app);
    assert_eq!(flying(&app), (None, None), "the toggle ends the hand-off");
    assert_eq!(
        app.world().get::<TravelLock>(player),
        Some(&TravelLock(None)),
        "the lock named the cancelled station"
    );

    // Docked to it: the trip never closes.
    let connection = app
        .world_mut()
        .spawn(DockingConnection {
            first_ship: player,
            first_section: Entity::PLACEHOLDER,
            second_ship: live,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    app.world_mut().entity_mut(player).insert(DockedShip {
        connection,
        helm: Quat::IDENTITY,
        drives: false,
    });
    press(&mut app);
    refused(&mut app, "the docked partner");

    // A fixed scene has no cell edge.
    app.world_mut().remove_resource::<ChartSectorEdge>();
    select(&mut app, MapChartMarkType::Cell(cell));
    press(&mut app);
    refused(&mut app, "no cell edge");
}

/// The chart's GOTO and CANCEL show for a station or a cell and hide for a
/// wreck, a job and nothing selected; a world with no cell edge offers no
/// chart GOTO. The destination line names a waypoint by its chart code and
/// the range to its mark, in the chart and the local view alike.
#[test]
fn chart_goto_gates_follow_the_mark_and_the_destination_reads_the_chart_code() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
    app.init_resource::<Chart>();
    app.init_resource::<Jobs>();
    app.init_resource::<Encounters>();
    app.insert_resource(CHART_EDGE);
    app.add_systems(Update, update_map_panel);
    let content = rig.content_root;
    app.world_mut()
        .commands()
        .entity(content)
        .with_children(|content| spawn_map_panel(content, &InterfaceIcons::blank()));
    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::default(),
            Name::new("NOVA"),
        ))
        .id();
    let station = chart_station_id();
    app.world_mut()
        .resource_mut::<Chart>()
        .find_station(&station, false, false, None, 0.0);
    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.chart = true;
    }
    let gates = |app: &mut App, mark: Option<MapChartMarkType>| {
        app.world_mut().resource_mut::<MapRuntime>().chart_selected = mark;
        settle(app);
        let world = app.world_mut();
        let goto = world
            .query_filtered::<&VerbGate, With<MapGotoButton>>()
            .single(world)
            .unwrap()
            .clone();
        let cancel = world
            .query_filtered::<&VerbGate, With<MapCancelButton>>()
            .single(world)
            .unwrap()
            .clone();
        (goto, cancel)
    };
    let cell = MapChartMarkType::Cell(SectorCoord::new(4, 0, -1));
    let wreck = MapChartMarkType::Wreck(GeneratedBodyId::new(
        SectorCoord::new(1, 0, 0),
        "cluster_1_0_0_wreck",
        0,
    ));
    let job = MapChartMarkType::Job(JobId {
        poster: station.clone(),
        source: JobSourceType::Slot {
            slot: 0,
            posting: 0,
        },
    });
    let station_mark = MapChartMarkType::Station(station.clone());
    for mark in [station_mark.clone(), cell.clone()] {
        assert_eq!(
            gates(app, Some(mark.clone())),
            (VerbGate::Ready, VerbGate::Disabled),
            "{mark:?}"
        );
    }
    for mark in [Some(wreck), Some(job), None] {
        assert_eq!(
            gates(app, mark.clone()),
            (VerbGate::Hidden, VerbGate::Hidden),
            "{mark:?}"
        );
    }
    assert_eq!(panel_text(app.world_mut(), MapPanelField::Destination), "");

    // Flying a waypoint: CANCEL ends it, and the line reads the chart code.
    let mark = Vec3::new(0.0, 0.0, -12_840.0);
    app.world_mut().entity_mut(player).insert((
        Waypoint {
            target: WaypointTargetType::Station(station.clone()),
            mark,
        },
        Autopilot::engage(AutopilotAction::GotoPos { position: mark }),
    ));
    assert_eq!(
        gates(app, Some(cell.clone())),
        (VerbGate::Ready, VerbGate::Ready)
    );
    assert_eq!(
        panel_text(app.world_mut(), MapPanelField::Destination),
        "GOTO STN-1  128.40 km"
    );
    app.world_mut().resource_mut::<MapRuntime>().chart = false;
    settle(app);
    assert_eq!(
        panel_text(app.world_mut(), MapPanelField::Destination),
        "GOTO STN-1  128.40 km",
        "the local view reads the same line"
    );
    let world = app.world_mut();
    let cancel = world
        .query_filtered::<&VerbGate, With<MapCancelButton>>()
        .single(world)
        .unwrap()
        .clone();
    assert_eq!(cancel, VerbGate::Ready);

    // No cell edge: no chart GOTO.
    app.world_mut().resource_mut::<MapRuntime>().chart = true;
    app.world_mut().remove_resource::<ChartSectorEdge>();
    assert_eq!(gates(app, Some(station_mark)).0, VerbGate::Hidden);
}

/// A waypoint's `GotoPos` draws no route until its prediction lands, then
/// exactly the predicted path, and the local view tags the mark. The chart
/// view draws no route at all and tags the waypoint's label beside it.
#[test]
fn a_waypoint_goto_draws_its_prediction_alone_and_the_chart_tags_its_label() {
    let (mut app, player, _raider, viewport) = goto_route_app();
    let mark = Vec3::new(10.0, 0.0, -40.0);
    let station = chart_station_id();
    app.world_mut().entity_mut(player).insert((
        Waypoint {
            target: WaypointTargetType::Station(station.clone()),
            mark,
        },
        Autopilot::engage(AutopilotAction::GotoPos { position: mark }),
    ));
    lay_out_map(&mut app, viewport, UVec2::new(1600, 1200), true);
    project_map_route_nodes(&mut app);
    assert_eq!(route_strokes(&mut app), [], "no prediction, no route");
    let at = map_px(&mut app, mark);
    assert_eq!(
        goto_tag(&mut app),
        (tag_on(at), Visibility::Inherited),
        "the tag marks the mark at once"
    );

    let points = vec![Vec3::ZERO, Vec3::new(5.0, 0.0, -20.0)];
    predict(
        &mut app,
        player,
        points.clone(),
        FlightPredictionEndType::Completed,
    );
    project_map_route_nodes(&mut app);
    let (blip, end) = (blip_centre(&app, player), map_px(&mut app, points[1]));
    assert_strokes(&mut app, &[(blip, end, 0.8)]);

    // The chart: no route, the tag on the label of the waypoint's mark.
    let label = app
        .world_mut()
        .spawn((
            MapChartLabel {
                anchor: Vec3::ZERO,
                mark: Some(MapChartMarkType::Station(station)),
            },
            Node {
                left: Val::Px(300.0),
                top: Val::Px(200.0),
                ..default()
            },
            ComputedNode {
                size: Vec2::new(80.0, 36.0),
                inverse_scale_factor: 0.5,
                ..ComputedNode::DEFAULT
            },
            Visibility::Inherited,
        ))
        .id();
    app.world_mut().resource_mut::<MapRuntime>().chart = true;
    project_map_route_nodes(&mut app);
    assert_eq!(route_strokes(&mut app), [], "the chart draws no route");
    assert_eq!(
        goto_tag(&mut app),
        (
            Vec2::new(300.0 + 40.0 + metrics::GAP_S, 200.0),
            Visibility::Inherited
        ),
        "the tag stands on the label's row, past its right edge"
    );
    app.world_mut().entity_mut(label).insert(Visibility::Hidden);
    project_map_route_nodes(&mut app);
    assert_eq!(goto_tag(&mut app).1, Visibility::Hidden, "no label, no tag");
}

/// The one entity with `T`.
fn app_entity<T: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<T>>()
        .single(app.world())
        .unwrap_or_else(|_| panic!("one {}", std::any::type_name::<T>()))
}

/// The Map names a rock and a world as the target inset captions them, by
/// the body's own `Name`: a streamed rock by its kind, a world by its name.
#[test]
fn the_map_names_an_asteroid_by_its_type_and_a_planet_by_its_name_as_the_lock_does() {
    let (mut world, _player, _raider) = scripted_world();
    let rock = world
        .spawn((
            EntityTypeName::new(ASTEROID_TYPE_NAME),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 90.0)),
            Name::new("Ice asteroid"),
        ))
        .id();
    let planet = world
        .spawn((
            EntityTypeName::new(nova_events::prelude::PLANET_TYPE_NAME),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -900.0)),
            Name::new("Halmar I"),
        ))
        .id();
    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let name_of = |entity: Entity| {
        contacts
            .iter()
            .find(|contact| contact.entity == entity)
            .map(|contact| (contact.kind, contact.name.as_str()))
    };
    assert_eq!(
        name_of(rock),
        Some((MapContactKind::Terrain, "Ice asteroid"))
    );
    assert_eq!(
        name_of(planet),
        Some((MapContactKind::Planet, "Halmar I"))
    );
}
