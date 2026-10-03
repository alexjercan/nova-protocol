//! Live-tree tests for the Map pane: contact derivation and codes, GOTO routing
//! into the autopilot, the orbit/selection input, and window-space picking.

use bevy::{
    camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo},
    ecs::system::RunSystemOnce,
    state::app::StatesPlugin,
    ui::{ComputedNode, InteractionDisabled, UiGlobalTransform},
};
use nova_events::prelude::{EntityTypeName, ASTEROID_TYPE_NAME};
use nova_gameplay::{prelude::*, PauseStates};
use nova_input::prelude::{BindingSpec, InputBindings, InputSource, RegisterInputActions};
use nova_ship::prelude::*;
use nova_ui::theme::ActiveUiTheme;

use super::{app::*, contacts::*, scene::*, *};
use crate::{
    icons::{BodyIconType, InterfaceIcons},
    pane::InterfacePaneType,
    pointer_rig::{
        click_at, hear_ui_cues, pane_pointer_rig, settle, take_churn, take_cues, track_node_churn,
        NodeChurn, PanePointerRig, RIG_PANEL_MIN,
    },
    terminal::NovaOsCloseTransition,
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
    assert_eq!(own.bearing_text(), "Bearing ---");
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

    // Docked to the selected contact in neutral: refused, and the note says
    // why instead of claiming a GOTO the helm would fly later.
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
    let note = |app: &App| {
        app.world()
            .resource::<MapRuntime>()
            .goto_note
            .as_ref()
            .map(|(note, _)| note.clone())
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
    // Under the command modal over the pane the key is text, not GOTO.
    press_goto_in(&mut app, PauseStates::Commands);
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(note(&app), None, "the modal over the pane takes no GOTO");
    press_goto_in(&mut app, PauseStates::Interface);
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(note(&app).as_deref(), Some("GOTO REFUSED: TAKE THE HELM"));

    // A pair that cannot be measured is refused for the fault, whoever holds
    // the helm.
    for helm in [DockedHelmType::Neutral, DockedHelmType::Held(player)] {
        let mut record = app
            .world_mut()
            .get_mut::<DockingConnection>(connection)
            .unwrap();
        record.helm = helm;
        record.measurement_fault = true;
        app.world_mut().run_system_once(map_input).unwrap();
        kept(&app);
        assert_eq!(note(&app).as_deref(), Some("GOTO REFUSED: HELM FAULT"));
    }
    app.world_mut()
        .get_mut::<DockingConnection>(connection)
        .unwrap()
        .measurement_fault = false;

    // Holding the helm, the selected contact is still the docked partner.
    app.world_mut()
        .get_mut::<DockedShip>(player)
        .unwrap()
        .drives = true;
    app.world_mut().run_system_once(map_input).unwrap();
    kept(&app);
    assert_eq!(note(&app).as_deref(), Some("GOTO REFUSED: DOCKED PARTNER"));

    app.world_mut().entity_mut(player).remove::<DockedShip>();
    app.world_mut().run_system_once(map_input).unwrap();
    assert_eq!(note(&app).as_deref(), Some("GOTO SET: RAIDER"));

    let autopilot = app
        .world()
        .get::<Autopilot>(player)
        .expect("GOTO inserts an Autopilot on the player ship");
    assert!(
        matches!(autopilot.action, AutopilotAction::Goto { target: t } if t == target),
        "the autopilot targets the selected contact",
    );
    assert_eq!(
        app.world().get::<TravelLock>(player),
        Some(&TravelLock(Some(target))),
        "an accepted GOTO designates its target"
    );
}

/// The GOTO button is the `map_goto` key's request, not a second path: it is
/// disabled on the own ship, and on a contact it sets the autopilot and travel
/// lock the key sets, which the panel then names as the destination.
#[test]
fn the_goto_button_sets_the_goto_the_key_sets() {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.add_plugins(StatesPlugin);
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<MapRuntime>();
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
        "No GOTO set"
    );

    let button = app
        .world_mut()
        .query_filtered::<Entity, With<MapGotoButton>>()
        .single(app.world())
        .unwrap();
    let disabled =
        |rig: &PanePointerRig| rig.app.world().get::<InteractionDisabled>(button).is_some();
    let goto = |rig: &PanePointerRig| {
        (
            rig.app.world().get::<Autopilot>(player).map(|a| a.action),
            rig.app.world().get::<TravelLock>(player).copied(),
        )
    };
    assert!(disabled(&rig), "nothing selected: no GOTO to request");

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
}

/// The route and its `GOTO` tag follow the player's live autopilot GOTO: they
/// draw again after the map closes and reopens, meet the player and target
/// blips in logical px at 2x scale and across a resize while another contact
/// is selected, and hide when the GOTO is cancelled or replaced while the
/// travel lock stays.
#[test]
fn the_goto_route_follows_the_live_autopilot_across_a_reopen() {
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
    let trader = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            GlobalTransform::from(Transform::from_xyz(60.0, 0.0, 0.0)),
            Name::new("TRADER"),
        ))
        .id();
    app.world_mut()
        .run_system_once(assign_map_contact_codes)
        .unwrap();
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyG);
    app.world_mut().run_system_once(map_input).unwrap();

    // Close and reopen: the selection does not survive, the GOTO does.
    for pane in [InterfacePaneType::Ship, InterfacePaneType::Map] {
        *app.world_mut().resource_mut::<InterfacePaneType>() = pane;
        app.world_mut().run_system_once(manage_map_scene).unwrap();
    }
    assert_eq!(app.world().resource::<MapRuntime>().selected, None);
    app.world_mut().resource_mut::<MapRuntime>().selected = Some(trader);

    // A laid-out viewport at 2x scale and a camera that projects into it 1:1
    // in physical px, as the render target reconciler leaves them. The blips
    // and the route both convert to logical px, so they meet only if both do.
    let viewport = app
        .world_mut()
        .spawn((MapViewportMarker, Node::default(), ComputedNode::DEFAULT))
        .id();
    app.world_mut().run_system_once(drive_map_camera).unwrap();
    let lay_out = |app: &mut App, physical: UVec2, camera_too: bool| {
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
    };
    let project = |app: &mut App| {
        // The first pass spawns the nodes; the second places them.
        for _ in 0..2 {
            app.world_mut().run_system_once(project_map_blips).unwrap();
            app.world_mut().run_system_once(project_map_route).unwrap();
        }
    };
    let px = |val: Val| match val {
        Val::Px(px) => px,
        other => panic!("expected px, got {other:?}"),
    };
    let blip_centre = |app: &App, contact: Entity| {
        let blip = app.world().resource::<MapRuntime>().blips[&contact];
        let node = app.world().get::<Node>(blip).unwrap();
        Vec2::new(px(node.left), px(node.top)) + MAP_BLIP_PX * 0.5
    };
    let route_ends = |app: &mut App| {
        let world = app.world_mut();
        let (line, transform, visibility) = world
            .query_filtered::<(&Node, &UiTransform, &Visibility), With<MapRouteLine>>()
            .single(world)
            .unwrap();
        assert_eq!(*visibility, Visibility::Inherited, "the route draws");
        let centre = Vec2::new(
            px(line.left) + px(line.width) * 0.5,
            px(line.top) + px(line.height) * 0.5,
        );
        let half = Vec2::new(transform.rotation.cos, transform.rotation.sin) * px(line.width) * 0.5;
        (centre - half, centre + half)
    };
    let assert_route_meets_blips = |app: &mut App| {
        let blips = (blip_centre(app, player), blip_centre(app, raider));
        let ends = route_ends(app);
        assert!(
            ends.0.distance(blips.0) < 0.01 && ends.1.distance(blips.1) < 0.01,
            "the route runs {ends:?}, the blips sit at {blips:?}"
        );
        assert!(
            blips.0.distance(blips.1) > 20.0,
            "the rig spreads the blips"
        );
    };

    lay_out(&mut app, UVec2::new(1600, 1200), true);
    project(&mut app);
    assert_route_meets_blips(&mut app);
    let to_blip = blip_centre(&app, raider);
    let world = app.world_mut();
    let (marker, marker_vis, children) = world
        .query_filtered::<(&Node, &Visibility, &Children), With<MapGotoMarker>>()
        .single(world)
        .unwrap();
    assert_eq!(*marker_vis, Visibility::Inherited);
    assert_eq!(
        px(marker.left),
        to_blip.x - MAP_BLIP_PX * 0.5,
        "the tag sits on the GOTO target, not the selection"
    );
    let tag: Vec<String> = children
        .iter()
        .filter_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
        .collect();
    assert_eq!(tag, ["GOTO"], "the destination is named in text");

    // A resize: while the camera still projects into the old size the route
    // holds, then it meets the blips in the new layout.
    let before = route_ends(&mut app);
    lay_out(&mut app, UVec2::new(1200, 1200), false);
    project(&mut app);
    assert_eq!(route_ends(&mut app), before, "a stale camera moves nothing");
    lay_out(&mut app, UVec2::new(1200, 1200), true);
    project(&mut app);
    assert_route_meets_blips(&mut app);
    assert_ne!(route_ends(&mut app), before, "the route follows the resize");

    // Cancelled, then replaced by another order: no route, lock untouched.
    let shown = |app: &mut App| {
        let world = app.world_mut();
        world
            .query_filtered::<&Visibility, Or<(With<MapRouteLine>, With<MapGotoMarker>)>>()
            .iter(world)
            .map(|visibility| *visibility != Visibility::Hidden)
            .collect::<Vec<_>>()
    };
    app.world_mut().entity_mut(player).remove::<Autopilot>();
    project(&mut app);
    assert_eq!(
        shown(&mut app),
        [false, false],
        "a cancelled GOTO draws no route"
    );
    app.world_mut()
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    project(&mut app);
    assert_eq!(
        shown(&mut app),
        [false, false],
        "another order draws no route"
    );
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
    assert_eq!(panel_text(world, MapPanelField::Code), "No contact");
    assert_eq!(panel_text(world, MapPanelField::Range), "");
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
        (MapPanelField::Range, "Range 500 m"),
        (MapPanelField::Bearing, "Bearing 000 mark +00"),
        (MapPanelField::Note, "Hostile contact."),
    ] {
        assert_eq!(panel_text(world, field), want, "{field:?}");
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
