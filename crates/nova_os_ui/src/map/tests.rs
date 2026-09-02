//! Live-tree tests for the MAP app: contact derivation and codes, the rendered
//! table, `goto` routing into the autopilot, and the orbit/selection input.

use bevy::{
    ecs::system::RunSystemOnce,
    state::app::StatesPlugin,
    ui::{ComputedNode, UiGlobalTransform},
};
use nova_events::prelude::{EntityTypeName, ASTEROID_TYPE_NAME};
use nova_input::prelude::{BindingSpec, InputBindings, InputSource, RegisterInputActions};
use nova_ship::prelude::*;

use super::{app::*, contacts::*, scene::*, *};
use crate::pointer_rig::{
    click_at, glass_px, glass_uv_showing, image_px_shown_at, nova_os_pointer_rig, pointer_image_px,
    settle, NovaOsPointerRig,
};

/// The map readout and INFO cell render range through the shared
/// player-facing distance policy (1 world unit = 10 m), not raw `u`.
#[test]
fn map_range_renders_in_meters_and_kilometers() {
    let entity = Entity::PLACEHOLDER;
    // 50 world units = 500 m (below the km threshold).
    let near = MapContact {
        entity,
        kind: MapContactKind::Hostile,
        code: "HOST-1".to_string(),
        name: "RAIDER".to_string(),
        world_pos: Vec3::ZERO,
        range: 50.0,
        bearing_deg: 0.0,
        mark_deg: 0.0,
    };
    assert!(
        near.readout().contains("range 500 m,"),
        "near readout: {}",
        near.readout()
    );
    assert!(
        near.info_cell().starts_with("500 m  "),
        "near info cell: {}",
        near.info_cell()
    );

    // 150 world units = 1500 m -> 1.50 km.
    let far = MapContact {
        range: 150.0,
        ..near.clone()
    };
    assert!(
        far.readout().contains("range 1.50 km,"),
        "far readout: {}",
        far.readout()
    );

    // The own ship's zero-range placeholder also uses the new unit.
    let own = MapContact {
        kind: MapContactKind::OwnShip,
        range: 0.0,
        ..near
    };
    assert!(own.readout().contains("range 0 m,"), "{}", own.readout());
    assert_eq!(own.info_cell(), "range 0 m");
}

/// Spawn a scripted local-space scene: own ship at origin (facing -Z), a
/// hostile dead ahead, an objective to starboard, an asteroid astern.
fn scripted_world() -> (World, Entity, Entity) {
    let mut world = World::new();
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

#[test]
fn map_view_rows_render_contacts_and_empty_state() {
    let (mut world, _player, _raider) = scripted_world();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let rows = map_rows_from_contacts(&contacts);
    let joined: String = rows
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("LOCAL SPACE"));
    assert!(joined.contains("HOSTILE"));
    assert!(joined.contains("RAIDER"));
    assert!(joined.contains("OBJECTIVE"));

    // With only the own ship, the CLI reports no contacts.
    let own_only: Vec<MapContact> = contacts
        .into_iter()
        .filter(|c| c.kind == MapContactKind::OwnShip)
        .collect();
    let empty_rows = map_rows_from_contacts(&own_only);
    assert!(empty_rows.iter().any(|r| r.text.contains("no contacts")));
}

/// A denser world: two hostiles + two asteroids + one objective, so the
/// per-kind indices actually count up and can collide if minting is wrong.
fn crowded_world() -> World {
    let mut world = World::new();
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
}

#[test]
fn map_view_table_aligns_kind_label_info_columns() {
    let mut world = crowded_world();
    world.run_system_once(assign_map_contact_codes).unwrap();
    let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
    let printed: Vec<String> = map_rows_from_contacts(&contacts)
        .into_iter()
        .map(|r| r.text)
        .collect();

    let header = printed
        .iter()
        .find(|r| r.starts_with("KIND"))
        .expect("a KIND/LABEL/INFO header row");
    assert!(header.contains("LABEL") && header.contains("INFO"));

    // Columns line up: the LABEL token starts at the SAME offset in the header
    // and in a data row (mirrors the `ship view` alignment assertion).
    let label_col = header.find("LABEL").unwrap();
    let hostile_row = printed
        .iter()
        .find(|r| r.starts_with("HOSTILE"))
        .expect("a hostile data row");
    assert!(
        hostile_row[label_col..].starts_with("HOST-"),
        "LABEL column is aligned: {hostile_row:?}",
    );
}

/// Register the `map`/`map goto` command tree into a bare terminal so `submit`
/// queues the gameplay invocation the handler drains.
fn terminal_with_map_goto() -> NovaOsTerminal {
    use nova_os::shell::{CommandArity, CommandDispatch, TerminalCommandSpec};
    let mut terminal = NovaOsTerminal::default();
    let mut specs = terminal.command_specs().to_vec();
    specs.push(TerminalCommandSpec {
        name: "map",
        summary: "Open the local-space map",
        arity: CommandArity::None,
        arg_hint: None,
        dispatch: CommandDispatch::App,
    });
    specs.push(TerminalCommandSpec {
        name: "map goto",
        summary: "Fly the ship to a contact label",
        arity: CommandArity::UpTo(1),
        arg_hint: Some("<label>"),
        dispatch: CommandDispatch::Gameplay,
    });
    terminal.set_nova_os_commands(specs);
    terminal
}

#[test]
fn map_goto_engages_autopilot_and_rejects_self_and_unknown() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
            MapContactCode("SELF".to_string()),
        ))
        .id();
    let raider = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Enemy,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
            Name::new("RAIDER"),
            MapContactCode("HOST-1".to_string()),
        ))
        .id();

    app.insert_resource(terminal_with_map_goto());

    let submit = |app: &mut App, line: &str| {
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        terminal.reset_prompt();
        terminal.insert_text(line);
        terminal.submit(&TerminalCommandSnapshot::default());
        app.world_mut()
            .run_system_once(apply_map_cli_commands)
            .unwrap();
    };

    // A real contact (case-insensitive): the autopilot targets the raider.
    submit(&mut app, "map goto host-1");
    let autopilot = app
        .world()
        .get::<Autopilot>(player)
        .expect("goto inserts an Autopilot on the player ship");
    assert!(
        matches!(autopilot.action, AutopilotAction::Goto { target } if target == raider),
        "the autopilot targets the labelled contact",
    );

    // Own ship: rejected, no autopilot change. Clear the autopilot first so we
    // can prove the SELF path does not set a new one.
    app.world_mut().entity_mut(player).remove::<Autopilot>();
    submit(&mut app, "map goto SELF");
    assert!(
        app.world().get::<Autopilot>(player).is_none(),
        "goto SELF must not engage an autopilot",
    );

    // Unknown label: rejected with an error row, still no autopilot.
    submit(&mut app, "map goto ZZZ");
    assert!(app.world().get::<Autopilot>(player).is_none());
    let printed: Vec<String> = app
        .world()
        .resource::<NovaOsTerminal>()
        .scrollback()
        .iter()
        .map(|r| r.text.clone())
        .collect();
    assert!(
        printed.iter().any(|r| r.contains("no such contact")),
        "an unknown label prints a not-found row: {printed:?}",
    );
}

/// The scene lifecycle tracks the active NOVA OS surface (headless: no
/// render assets, so only the active flag toggles - the scene build is
/// skipped, but open/close is proven).
#[test]
fn map_scene_activates_with_the_app_surface() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(PauseStates::NovaOs);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.init_resource::<MapRuntime>();
    app.insert_resource(NovaOsTerminal::default());

    // At the prompt: inactive.
    app.update();
    app.world_mut().run_system_once(manage_map_scene).unwrap();
    assert!(!app.world().resource::<MapRuntime>().active);

    // Launch the map app: active.
    app.world_mut()
        .resource_mut::<NovaOsTerminal>()
        .enter_app(MAP_APP_ID);
    app.world_mut().run_system_once(manage_map_scene).unwrap();
    assert!(app.world().resource::<MapRuntime>().active);

    // Exit back to the terminal: inactive again.
    app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
    app.world_mut().run_system_once(manage_map_scene).unwrap();
    assert!(!app.world().resource::<MapRuntime>().active);
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
    app.insert_state(PauseStates::NovaOs);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.init_resource::<MapRuntime>();

    let mut terminal = NovaOsTerminal::default();
    terminal.enter_app(MAP_APP_ID);
    app.insert_resource(terminal);

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

    // Closing the app tears the scene down.
    app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
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
    app.insert_state(PauseStates::NovaOs);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.init_resource::<MapRuntime>();

    let mut terminal = NovaOsTerminal::default();
    terminal.enter_app(MAP_APP_ID);
    app.insert_resource(terminal);

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
/// except the computer the player flies with.
#[test]
fn a_rebound_goto_key_is_the_key_the_map_answers() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, bevy::input::InputPlugin));
    app.insert_state(PauseStates::NovaOs);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.init_resource::<MapRuntime>();

    let mut terminal = NovaOsTerminal::default();
    terminal.enter_app(MAP_APP_ID);
    app.insert_resource(terminal);

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
    app.insert_state(PauseStates::NovaOs);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.init_resource::<MapRuntime>();

    let mut terminal = NovaOsTerminal::default();
    terminal.enter_app(MAP_APP_ID);
    app.insert_resource(terminal);

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

    {
        let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
        runtime.active = true;
        runtime.selected = Some(target);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyG);

    app.world_mut().run_system_once(map_input).unwrap();

    let autopilot = app
        .world()
        .get::<Autopilot>(player)
        .expect("GOTO inserts an Autopilot on the player ship");
    assert!(
        matches!(autopilot.action, AutopilotAction::Goto { target: t } if t == target),
        "the autopilot targets the selected contact",
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
    app.insert_state(PauseStates::NovaOs);
    app.register_input_actions(crate::bindings::novaos_bindings());
    app.init_resource::<MapRuntime>();

    let mut terminal = NovaOsTerminal::default();
    terminal.enter_app(MAP_APP_ID);
    app.insert_resource(terminal);

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

/// Control is the app-EXIT chord, so every key the map reads must be withheld
/// while it is held - otherwise Ctrl+T both leaves the app and re-frames the
/// view behind it. `map_input` read `ButtonInput` directly and had no guard at
/// all; routing it through `NovaOsAppInput` is what supplies one. The `ship`
/// app carried the same bug (F34) and was fixed on its own.
#[test]
fn control_withholds_map_keys_because_it_is_the_exit_chord() {
    let mut app = map_input_app();

    let radius = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .map(|o| o.radius)
            .unwrap()
    };
    // Move the view off its default framing so a reset is observable.
    app.world_mut()
        .query_filtered::<&mut MapOrbit, With<MapCameraMarker>>()
        .single_mut(app.world_mut())
        .unwrap()
        .radius = MAP_RADIUS_DEFAULT * 0.5;
    let moved = radius(&mut app);

    // Ctrl+T: the chord belongs to the router, so T must not reach the app.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::KeyT);
    }
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(
        radius(&mut app),
        moved,
        "Ctrl+T is the exit chord and must not also re-frame the map"
    );

    // T alone still re-frames.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        // Release before clearing: `press` only raises a just_pressed edge for a
        // key that was not already down.
        keys.release(KeyCode::ControlLeft);
        keys.release(KeyCode::KeyT);
        keys.clear();
        keys.press(KeyCode::KeyT);
    }
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(
        radius(&mut app),
        MAP_RADIUS_DEFAULT,
        "T on its own re-frames the map"
    );
}

/// Stand a map viewport up inside the rig's through-image content root,
/// clipped exactly as the app body's is, and return it.
fn rig_map_viewport(rig: &mut NovaOsPointerRig) -> Entity {
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
            BackgroundColor(MAP_VIEW_BG),
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
        world_pos: Vec3::ZERO,
        range: 100.0,
        bearing_deg: 0.0,
        mark_deg: 0.0,
    }
}

/// Put a real map blip (the production `spawn_blip` markup and its real
/// `Activate` observer) with its DOT centred on `image_px`.
fn rig_place_blip(
    rig: &mut NovaOsPointerRig,
    viewport: Entity,
    contact: &MapContact,
    image_px: Vec2,
) -> Entity {
    let blip = rig
        .app
        .world_mut()
        .run_system_once_with(
            |input: In<(Entity, MapContact)>, mut commands: Commands| {
                let (viewport, contact) = input.0;
                spawn_blip(&mut commands, viewport, &contact, Handle::default())
            },
            (viewport, contact.clone()),
        )
        .expect("spawning a blip through the production path");
    let mut node = rig
        .app
        .world_mut()
        .get_mut::<Node>(blip)
        .expect("the blip has a Node");
    node.left = Val::Px(image_px.x - MAP_BLIP_PX * 0.5);
    node.top = Val::Px(image_px.y - MAP_BLIP_PX * 0.5);
    settle(&mut rig.app);
    blip
}

/// DoD 1: a click at a known place on the glass selects the contact the CRT
/// is DISPLAYING there - in the middle of the viewport and out in a corner.
///
/// This is the failing test the bug was found with. Before the fix the
/// forwarded pointer applied the barrel INVERSE and ignored the shader's
/// overscan entirely, so the corner case selected nothing (the pointer landed
/// ~27 px away in x, more than twice the 12 px blip) while the centre case
/// passed - exactly the asymmetry the owner reported between the map (blips
/// spread across the viewport) and the ship app (blips clustered mid-screen).
///
/// Each case also carries a DECOY contact at the other test point, so
/// "selected something" cannot pass for "selected the right thing".
#[test]
fn map_contacts_select_where_the_crt_shows_them() {
    let centre_uv = Vec2::splat(0.5);
    // Far enough into the corner that the barrel residual is at its worst,
    // and still comfortably inside the glass.
    let corner_uv = Vec2::new(0.04, 0.05);

    for (what, aim_uv, decoy_uv) in [
        ("centre of the viewport", centre_uv, corner_uv),
        ("corner of the viewport", corner_uv, centre_uv),
    ] {
        let mut rig = nova_os_pointer_rig();
        rig.app.init_resource::<MapRuntime>();
        let viewport = rig_map_viewport(&mut rig);

        let target_id = rig.app.world_mut().spawn_empty().id();
        let decoy_id = rig.app.world_mut().spawn_empty().id();
        let target = rig_contact(target_id, "HOST-1");
        let decoy = rig_contact(decoy_id, "HOST-2");
        rig_place_blip(&mut rig, viewport, &target, image_px_shown_at(aim_uv));
        rig_place_blip(&mut rig, viewport, &decoy, image_px_shown_at(decoy_uv));

        click_at(&mut rig, glass_px(aim_uv));

        let selected = rig.app.world().resource::<MapRuntime>().selected;
        let intended = image_px_shown_at(aim_uv);
        assert_eq!(
            selected,
            Some(target_id),
            "clicking the {what} must select the contact the CRT shows there: \
             the forwarded pointer sat on image px {:?}, the blip is centred on \
             {intended:?}",
            pointer_image_px(&rig),
        );

        // ...and it must select it by landing on the DOT, not by drifting onto
        // some other part of the target. Without this the label pill's own
        // (deliberate) generosity would absorb a mis-mapped pointer and the
        // selection assertion above would pass right through the bug.
        let landed = pointer_image_px(&rig).expect("the pointer is on the image");
        assert!(
            landed.distance(intended) <= MAP_BLIP_PX * 0.5,
            "clicking the {what}, the forwarded pointer landed on image px \
             {landed:?}, {:.1} px from the {intended:?} the CRT displays there - \
             outside the {MAP_BLIP_PX} px dot it was aimed at",
            landed.distance(intended),
        );
    }
}

/// The laid-out border box of a node in image space.
fn rig_rect(rig: &NovaOsPointerRig, entity: Entity) -> Rect {
    let world = rig.app.world();
    let node = world
        .get::<ComputedNode>(entity)
        .unwrap_or_else(|| panic!("{entity:?} never reached UI layout"));
    let xf = world
        .get::<UiGlobalTransform>(entity)
        .unwrap_or_else(|| panic!("{entity:?} has no UI transform"));
    Rect::from_center_size(xf.translation, node.size())
}

/// The blip's label node - its only child.
fn rig_label_of(rig: &NovaOsPointerRig, blip: Entity) -> Entity {
    let children = rig
        .app
        .world()
        .get::<Children>(blip)
        .expect("the blip has a label child");
    assert_eq!(
        children.len(),
        1,
        "the blip's hit target is its dot plus ONE label child"
    );
    children[0]
}

/// DoD 3: the label is as clickable as the dot, which means the two targets
/// TOUCH. The owner's comparison - "on labels, clicks work 99% of the time"
/// in the ship app - is the bar, and the ship app's label is a padded backing
/// pill starting 2 px from its dot. The map's bare text node started 4 px out
/// with no padding of its own, leaving a dead band between dot and label that
/// selects nothing, and a target tight to the glyph run on every side.
///
/// Both halves are read from the LIVE tree and clicked through the real
/// composite, so neither the gap nor the padding can be satisfied on paper.
#[test]
fn map_contact_label_and_dot_are_one_unbroken_target() {
    let aim_uv = Vec2::new(0.35, 0.45);
    let mut rig = nova_os_pointer_rig();
    rig.app.init_resource::<MapRuntime>();
    let viewport = rig_map_viewport(&mut rig);

    let target_id = rig.app.world_mut().spawn_empty().id();
    let contact = rig_contact(target_id, "HOST-1");
    let blip = rig_place_blip(&mut rig, viewport, &contact, image_px_shown_at(aim_uv));
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

    // A solid, padded backing box like the ship app's pill, not a box tight to
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
         target is tight to the glyph run, unlike the ship app's pill",
    );
    assert!(
        rig.app.world().get::<BackgroundColor>(label).is_some(),
        "the label has no backing fill, so there is nothing solid to aim at",
    );

    // Every point across the seam - dot centre, the gap between dot and
    // pill, label centre - selects the contact. Positions come from the live
    // rects, and the glass position from the shader reference, never the
    // production map. Swept at pixel CENTRES, since the shared edge itself is
    // a measure-zero boundary bevy's `contains_point` excludes from both
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
        let image_px = Vec2::new(x, y);
        rig.app.world_mut().resource_mut::<MapRuntime>().selected = None;
        click_at(&mut rig, glass_px(glass_uv_showing(image_px)));
        assert_eq!(
            rig.app.world().resource::<MapRuntime>().selected,
            Some(target_id),
            "clicking image px {image_px:?} - between the dot's centre and 6 px \
             into the label - must select the contact; the pointer landed on {:?}",
            pointer_image_px(&rig),
        );
        probed += 1;
    }
    assert!(
        probed >= 12,
        "the sweep only probed {probed} points - it is not crossing the seam"
    );
}

/// A viewport inset inside the content root, so its clip rect is a real
/// boundary rather than the image edge.
fn rig_inset_map_viewport(rig: &mut NovaOsPointerRig, inset: Rect) -> Entity {
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
            BackgroundColor(MAP_VIEW_BG),
        ))
        .id();
    rig.app
        .world_mut()
        .entity_mut(rig.content_root)
        .add_child(viewport);
    viewport
}

/// Step 5 of the task: a warp fix that still loses edge contacts to clipping
/// is not a fix. The map viewport is `Overflow::clip()`, and bevy's UI picking
/// respects clip rects, so a blip straddling the viewport edge is pickable
/// over its UNCLIPPED part and dead over the clipped part.
///
/// Both halves are asserted: the visible half must select (that is the bug
/// this guards), and the clipped half must NOT (otherwise the test would pass
/// against a build that ignores clipping entirely).
#[test]
fn map_contacts_straddling_the_viewport_edge_are_pickable_over_their_visible_half() {
    let inset = Rect::new(120.0, 90.0, 900.0, 600.0);
    let mut rig = nova_os_pointer_rig();
    rig.app.init_resource::<MapRuntime>();
    let viewport = rig_inset_map_viewport(&mut rig, inset);

    let target_id = rig.app.world_mut().spawn_empty().id();
    let contact = rig_contact(target_id, "HOST-1");
    // Straddle the viewport's right edge: half the dot inside, half outside.
    // `place` is viewport-local; the rect below is in image space.
    let blip = rig_place_blip(
        &mut rig,
        viewport,
        &contact,
        Vec2::new(inset.max.x - inset.min.x, 200.0),
    );
    let dot = rig_rect(&rig, blip);
    assert!(
        dot.min.x < inset.max.x && dot.max.x > inset.max.x,
        "the rig meant to straddle the clip edge at x {} but the dot is {dot:?}",
        inset.max.x,
    );

    let probe = |rig: &mut NovaOsPointerRig, at: Vec2| {
        rig.app.world_mut().resource_mut::<MapRuntime>().selected = None;
        click_at(rig, glass_px(glass_uv_showing(at)));
        rig.app.world().resource::<MapRuntime>().selected
    };

    let inside = Vec2::new(inset.max.x - 3.5, dot.center().y);
    assert_eq!(
        probe(&mut rig, inside),
        Some(target_id),
        "the visible half of an edge contact (image px {inside:?}) must still \
         select it; the pointer landed on {:?}",
        pointer_image_px(&rig),
    );

    let outside = Vec2::new(inset.max.x + 3.5, dot.center().y);
    assert_eq!(
        probe(&mut rig, outside),
        None,
        "the clipped half (image px {outside:?}) draws nothing, so it must not \
         select either - otherwise this test does not exercise clipping",
    );
}

/// The overlap path: map contacts drift, so a label routinely lies over a
/// neighbouring dot. UI picking resolves that by stacking order, and the
/// TOPMOST node wins - deterministically, not by accident. Pinned so the
/// bigger label pill this task introduced cannot quietly start swallowing its
/// neighbours' clicks in some other order.
#[test]
fn overlapping_map_contacts_select_the_topmost() {
    let aim_uv = Vec2::new(0.45, 0.5);
    let mut rig = nova_os_pointer_rig();
    rig.app.init_resource::<MapRuntime>();
    let viewport = rig_map_viewport(&mut rig);

    let under_id = rig.app.world_mut().spawn_empty().id();
    let over_id = rig.app.world_mut().spawn_empty().id();
    let at = image_px_shown_at(aim_uv);
    // Spawned first = lower in the UI stack; the second sits exactly on top.
    rig_place_blip(&mut rig, viewport, &rig_contact(under_id, "HOST-1"), at);
    rig_place_blip(&mut rig, viewport, &rig_contact(over_id, "HOST-2"), at);

    click_at(&mut rig, glass_px(aim_uv));
    assert_eq!(
        rig.app.world().resource::<MapRuntime>().selected,
        Some(over_id),
        "two contacts stacked on the same pixel resolve to the topmost, not to \
         whichever the hit test happened to visit first",
    );
}
