//! Live-tree tests for the SHIP app: section codes, the reload/repair command
//! and message paths, the detail rows, and the scene's local-space placement
//! and orbit input.

use bevy::{
    ecs::system::RunSystemOnce,
    input::InputPlugin,
    state::app::StatesPlugin,
    ui::{ComputedNode, UiGlobalTransform},
    ui_widgets::Activate,
};
use nova_events::prelude::EntityId;
use nova_ship::prelude::*;

use super::{app::*, scene::*, sections::*, *};
use crate::{
    pointer_rig::{
        click_at, glass_px, glass_uv_showing, image_px_shown_at, nova_os_pointer_rig,
        pointer_image_px, settle, NovaOsPointerRig,
    },
    terminal::{NOVA_OS_AMBER, NOVA_OS_PHOSPHOR},
};

/// A terminal with the `ship` command tree registered (so the arg-bearing
/// verbs resolve and set a pending invocation on submit).
fn ship_terminal() -> NovaOsTerminal {
    let mut registry = NovaOsCommandRegistry::default();
    registry.register(ship_command_tree());
    let mut terminal = NovaOsTerminal::default();
    terminal.set_commands(registry.specs());
    terminal
}

/// The arg-bearing ship verbs register their `<section>` argument hint, so
/// `<verb> help` renders a shell usage line naming the argument. Pins the
/// registration-site `.with_arg_hint("<section>")` wiring end to end (the
/// pure renderer is unit-tested in `nova_os`; this proves the ship tree
/// actually feeds it the hint).
#[test]
fn ship_verb_help_names_the_section_argument() {
    for verb in ["ship section", "ship reload", "ship repair"] {
        let mut terminal = ship_terminal();
        terminal.insert_text(&format!("{verb} help"));
        terminal.submit(&TerminalCommandSnapshot::default());
        let want = format!("Usage: {verb} <section>");
        assert!(
            terminal.scrollback().iter().any(|row| row.text == want),
            "{verb} help should render `{want}`",
        );
    }
}

/// Spawn a scripted player ship: a hull, a turret (with ammo, critically
/// damaged) and a healthy thruster. Sections carry NO `SectionCode` yet, so
/// `assign_section_codes` mints them (HULL-1 / PDC-1 / THR-1).
fn spawn_scripted_ship(world: &mut World) -> (Entity, Entity, Entity, Entity) {
    let ship = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ))
        .id();
    let section_base = |name: &str, id: &str, xyz: Vec3| {
        (
            SectionMarker,
            Name::new(name.to_string()),
            EntityId::new(id.to_string()),
            Transform::from_translation(xyz),
            GlobalTransform::from(Transform::from_translation(xyz)),
            SectionCollider::Cuboid { size: Vec3::ONE },
            ChildOf(ship),
        )
    };
    let hull = world
        .spawn((
            section_base("Block Hull", "cube_a", Vec3::ZERO),
            HullSectionMarker,
            SectionDamageClass::Hull,
            Health {
                current: 80.0,
                max: 100.0,
            },
        ))
        .id();
    let turret = world
        .spawn((
            section_base("Bow gun", "cube_b", Vec3::new(2.0, 0.0, 0.0)),
            TurretSectionMarker,
            SectionDamageClass::Turret,
            Health {
                current: 12.0,
                max: 60.0,
            },
            SectionAmmo {
                rounds: 2,
                capacity: 6,
            },
        ))
        .id();
    let thruster = world
        .spawn((
            section_base("Main drive", "cube_c", Vec3::new(-2.0, 0.0, 0.0)),
            ThrusterSectionMarker,
            SectionDamageClass::Thruster,
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    (ship, hull, turret, thruster)
}

fn scrollback_text(app: &App) -> String {
    app.world()
        .resource::<NovaOsTerminal>()
        .scrollback()
        .iter()
        .map(|row| row.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Submit a command line through the terminal (echo + resolve + queue), the
/// way a real Enter does, so `apply_ship_cli_commands` can drain it.
fn submit(app: &mut App, line: &str) {
    let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
    terminal.insert_text(line);
    terminal.submit(&TerminalCommandSnapshot::default());
}

#[test]
fn section_codes_assigned_and_resolve() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let (_ship, hull, turret, thruster) = spawn_scripted_ship(app.world_mut());

    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // Codes minted per kind, stable-indexed by the authored id order.
    assert_eq!(app.world().get::<SectionCode>(hull).unwrap().0, "HULL-1");
    assert_eq!(app.world().get::<SectionCode>(turret).unwrap().0, "PDC-1");
    assert_eq!(app.world().get::<SectionCode>(thruster).unwrap().0, "THR-1");

    // A code resolves back to its section, case-insensitively.
    let resolved = app
        .world_mut()
        .run_system_once(|s: ShipSections| s.resolve("pdc-1").map(|v| v.entity))
        .unwrap();
    assert_eq!(resolved, Some(turret));

    // A second run is idempotent: no new codes, same values.
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    assert_eq!(app.world().get::<SectionCode>(turret).unwrap().0, "PDC-1");
}

#[test]
fn ship_reload_and_repair_apply_through_command_handler() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ship_terminal());
    let (_ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // `ship repair HULL-1` restores integrity through the handler.
    submit(&mut app, "ship repair HULL-1");
    app.world_mut()
        .run_system_once(apply_ship_cli_commands)
        .unwrap();
    assert_eq!(app.world().get::<Health>(hull).unwrap().current, 100.0);
    assert!(scrollback_text(&app).contains("repaired HULL-1"));

    // `ship reload PDC-1` refills ammo through the same seam (lowercase id).
    submit(&mut app, "ship reload pdc-1");
    app.world_mut()
        .run_system_once(apply_ship_cli_commands)
        .unwrap();
    assert_eq!(app.world().get::<SectionAmmo>(turret).unwrap().rounds, 6);
    assert!(scrollback_text(&app).contains("reloaded PDC-1"));

    // Reload on a non-weapon is a friendly error, not a mutation.
    submit(&mut app, "ship reload HULL-1");
    app.world_mut()
        .run_system_once(apply_ship_cli_commands)
        .unwrap();
    assert!(scrollback_text(&app).contains("no ammo feed"));

    // An unknown code reports and lists the valid codes.
    submit(&mut app, "ship repair BOGUS-9");
    app.world_mut()
        .run_system_once(apply_ship_cli_commands)
        .unwrap();
    assert!(scrollback_text(&app).contains("no such section: BOGUS-9"));
}

#[test]
fn ship_section_detail_rows_from_live_data() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ship_terminal());
    spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    submit(&mut app, "ship section PDC-1");
    app.world_mut()
        .run_system_once(apply_ship_cli_commands)
        .unwrap();
    let text = scrollback_text(&app);
    assert!(text.contains("SECTION PDC-1 - Bow gun"), "{text}");
    assert!(text.contains("kind: turret"), "{text}");
    assert!(text.contains("ammo: 2/6"), "{text}");
    // 12/60 = 20% -> critical.
    assert!(text.contains("status: critical"), "{text}");
}

#[test]
fn ship_verb_id_tab_completion() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ShipRuntime>();
    app.insert_resource(ship_terminal());
    spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    app.world_mut()
        .run_system_once(sync_ship_arg_completions)
        .unwrap();

    // `ship repair hu` Tab-completes to the HULL code (case-insensitive).
    let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
    terminal.insert_text("ship repair hu");
    assert!(terminal.complete());
    assert_eq!(terminal.prompt(), "ship repair HULL-1");
}

#[test]
fn ship_action_keys_mutate_through_message_handler() {
    // The in-app L/P keys raise a ShipSectionCommand; the handler applies it
    // and flashes the result note. Pins the message path at its own boundary
    // (deleting apply_ship_section_commands must fail this).
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ShipRuntime>();
    app.add_message::<ShipSectionCommand>();
    let (_ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // Repair the hull through the message handler.
    app.world_mut().write_message(ShipSectionCommand {
        target: hull,
        action: ShipAction::Repair,
    });
    app.world_mut()
        .run_system_once(apply_ship_section_commands)
        .unwrap();
    assert_eq!(app.world().get::<Health>(hull).unwrap().current, 100.0);
    let note = app.world().resource::<ShipRuntime>().note.clone();
    assert!(
        note.map(|(text, _)| text.contains("repaired HULL-1"))
            .unwrap_or(false),
        "the handler flashes the result on the panel note line",
    );

    // Reload the turret through the message handler.
    app.world_mut().write_message(ShipSectionCommand {
        target: turret,
        action: ShipAction::Reload,
    });
    app.world_mut()
        .run_system_once(apply_ship_section_commands)
        .unwrap();
    assert_eq!(app.world().get::<SectionAmmo>(turret).unwrap().rounds, 6);
}

#[test]
fn scene_blocks_use_local_space_when_ship_off_origin() {
    // Regression: the schematic scene is anchored at the origin and blocks sit
    // at each section's LOCAL offset; `project_ship_blips` projects that SAME
    // local offset, so blocks and blips stay aligned no matter where the ship
    // flies. Build the scene with the ship root far from the world origin and
    // assert the block sits at the local offset (not the off-origin world
    // position, which the old code projected the blips from).
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::NovaOs);
    app.init_resource::<ShipRuntime>();
    let mut terminal = ship_terminal();
    terminal.enter_app(SHIP_APP_ID);
    app.insert_resource(terminal);

    let ship_world = Vec3::new(500.0, -200.0, 900.0);
    let ship = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(ship_world),
            GlobalTransform::from(Transform::from_translation(ship_world)),
            Name::new("NOVA"),
        ))
        .id();
    let local_offset = Vec3::new(2.0, 0.0, 0.0);
    let turret = app
        .world_mut()
        .spawn((
            SectionMarker,
            Name::new("Bow gun"),
            EntityId::new("cube_b"),
            Transform::from_translation(local_offset),
            GlobalTransform::from(Transform::from_translation(ship_world + local_offset)),
            SectionCollider::Cuboid { size: Vec3::ONE },
            ChildOf(ship),
            TurretSectionMarker,
            SectionDamageClass::Turret,
            Health {
                current: 60.0,
                max: 60.0,
            },
        ))
        .id();
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    app.world_mut().run_system_once(manage_ship_scene).unwrap();

    // The block is placed at the LOCAL offset, not the off-origin world pos.
    let block_pos = app
        .world_mut()
        .query_filtered::<(&ShipBlock, &Transform), With<ShipBlock>>()
        .iter(app.world())
        .find(|(block, _)| block.section == turret)
        .map(|(_, t)| t.translation);
    assert_eq!(
        block_pos,
        Some(local_offset),
        "the schematic block sits at the LOCAL offset, in the same space the blips project from",
    );
}

#[test]
fn ship_app_launches_and_exits() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(PauseStates::NovaOs);
    app.init_resource::<ShipRuntime>();
    app.insert_resource(ship_terminal());

    // Bare `ship` launches the app (hands the screen over), not a CLI print.
    submit(&mut app, "ship");
    {
        let terminal = app.world().resource::<NovaOsTerminal>();
        assert_eq!(
            terminal.active_mode(),
            TerminalMode::App { id: SHIP_APP_ID }
        );
    }
    app.world_mut().run_system_once(manage_ship_scene).unwrap();
    assert!(app.world().resource::<ShipRuntime>().active);

    // Exiting returns to the prompt and tears the scene state down.
    app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
    app.world_mut().run_system_once(manage_ship_scene).unwrap();
    assert!(!app.world().resource::<ShipRuntime>().active);
    assert_eq!(
        app.world().resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::Prompt
    );
}

#[test]
fn ship_app_renders_blocks_and_selects_section() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        AssetPlugin::default(),
        InputPlugin,
    ));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::NovaOs);
    app.init_resource::<ShipRuntime>();
    app.add_message::<ShipSectionCommand>();

    let mut terminal = ship_terminal();
    terminal.enter_app(SHIP_APP_ID);
    app.insert_resource(terminal);

    let (_ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // Build the scene from the live section colliders.
    app.world_mut().run_system_once(manage_ship_scene).unwrap();
    {
        let runtime = app.world().resource::<ShipRuntime>();
        assert!(runtime.active);
        assert!(runtime.scene_root.is_some(), "scene root spawned");
        assert!(runtime.image.is_some(), "RTT image created");
        assert!(
            runtime.selected.is_some(),
            "a section is selected by default"
        );
    }
    let blocks = app
        .world_mut()
        .query_filtered::<(), With<ShipBlock>>()
        .iter(app.world())
        .count();
    assert_eq!(blocks, 3, "one proxy block per live section");

    // Inspecting does not mutate gameplay state: the hull HP is unchanged.
    assert_eq!(app.world().get::<Health>(hull).unwrap().current, 80.0);

    // Cycling the selection with `]` advances to another section. Drive the
    // real input system through `run_system_once` so the pressed edge is not
    // cleared by InputPlugin's PreUpdate pass before the system reads it
    // (`nextstate-input-test-needs-clear-and-two-updates`).
    let before = app.world().resource::<ShipRuntime>().selected;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::BracketRight);
    app.world_mut().run_system_once(ship_input).unwrap();
    let after = app.world().resource::<ShipRuntime>().selected;
    assert!(after.is_some() && after != before, "] cycles the selection");
}

/// LMB is the blip-SELECT button (the `Button` widget's Primary activation),
/// so it must NOT orbit-drag the camera - a small press-with-motion has to
/// stay a click, not become a drag that slides the blip out from under the
/// cursor. RMB remains the orbit-drag button.
#[test]
fn ship_orbit_drag_is_rmb_only() {
    use bevy::input::mouse::MouseMotion;

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        AssetPlugin::default(),
        InputPlugin,
    ));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::NovaOs);
    app.init_resource::<ShipRuntime>();
    app.add_message::<ShipSectionCommand>();

    let mut terminal = ship_terminal();
    terminal.enter_app(SHIP_APP_ID);
    app.insert_resource(terminal);

    spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    app.world_mut().run_system_once(manage_ship_scene).unwrap();

    let orbit_angles = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&ShipOrbit, With<ShipCameraMarker>>()
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
    app.world_mut().run_system_once(ship_input).unwrap();
    assert_eq!(
        orbit_angles(&mut app),
        before,
        "LMB drag must NOT orbit the ship camera (it selects blips)"
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
    app.world_mut().run_system_once(ship_input).unwrap();
    assert_ne!(
        orbit_angles(&mut app),
        before,
        "RMB drag must still orbit the ship camera"
    );
}

/// Spawn a player ship whose root is OFF-origin and rotated, with each
/// section's world pose (`GlobalTransform`) deliberately DIFFERENT from its
/// local `Transform`. The orbit recenter reads the LOCAL frame (the scene is
/// built from `Transform`, like the blips), so a regression that read the
/// world frame would retarget to the wrong place - invisible at the origin
/// (`spatial-fixture-off-the-trivial-point`). Returns (hull, turret, thruster);
/// local translations are hull (3,2,-1), turret (9,2,-1), thruster (-3,2,-1),
/// so the centroid is (3,2,-1) and the turret is well off it.
fn spawn_offset_ship(world: &mut World) -> (Entity, Entity, Entity) {
    let root_world = Transform::from_translation(Vec3::new(120.0, -40.0, 75.0))
        .with_rotation(Quat::from_rotation_y(0.8));
    let ship = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            root_world,
            GlobalTransform::from(root_world),
            Name::new("NOVA"),
        ))
        .id();
    // Local Transform is the scene frame; GlobalTransform is the genuinely
    // composed world pose (`root_world * local`), so the two frames are
    // consistent AND clearly distinct - the recenter must read local.
    let section = |name: &str, id: &str, local: Vec3| {
        (
            SectionMarker,
            Name::new(name.to_string()),
            EntityId::new(id.to_string()),
            Transform::from_translation(local),
            GlobalTransform::from(root_world.mul_transform(Transform::from_translation(local))),
            SectionCollider::Cuboid { size: Vec3::ONE },
            ChildOf(ship),
        )
    };
    let hull = world
        .spawn((
            section("Block Hull", "cube_a", Vec3::new(3.0, 2.0, -1.0)),
            HullSectionMarker,
            SectionDamageClass::Hull,
            Health {
                current: 80.0,
                max: 100.0,
            },
        ))
        .id();
    let turret = world
        .spawn((
            section("Bow gun", "cube_b", Vec3::new(9.0, 2.0, -1.0)),
            TurretSectionMarker,
            SectionDamageClass::Turret,
            Health {
                current: 12.0,
                max: 60.0,
            },
            SectionAmmo {
                rounds: 2,
                capacity: 6,
            },
        ))
        .id();
    let thruster = world
        .spawn((
            section("Main drive", "cube_c", Vec3::new(-3.0, 2.0, -1.0)),
            ThrusterSectionMarker,
            SectionDamageClass::Thruster,
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    (hull, turret, thruster)
}

/// A ship app fixture built from [`spawn_offset_ship`], with the scene already
/// managed. Returns the app plus (hull, turret, thruster).
fn offset_ship_app() -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        AssetPlugin::default(),
        InputPlugin,
    ));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::NovaOs);
    app.init_resource::<ShipRuntime>();
    app.add_message::<ShipSectionCommand>();

    let mut terminal = ship_terminal();
    terminal.enter_app(SHIP_APP_ID);
    app.insert_resource(terminal);

    let (hull, turret, thruster) = spawn_offset_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    app.world_mut().run_system_once(manage_ship_scene).unwrap();
    (app, hull, turret, thruster)
}

fn ship_orbit(app: &mut App) -> ShipOrbit {
    *app.world_mut()
        .query_filtered::<&ShipOrbit, With<ShipCameraMarker>>()
        .single(app.world())
        .unwrap()
}

/// Advance the generic clock and drive the camera `frames` times, so the
/// exponential center ease actually integrates over simulated time.
fn drive_frames(app: &mut App, frames: usize, dt: f32) {
    for _ in 0..frames {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(dt));
        app.world_mut().run_system_once(drive_ship_camera).unwrap();
    }
}

/// Selecting a non-default section retargets the orbit center to that section's
/// LOCAL position, and `drive_ship_camera` eases the live center onto it. The
/// app still OPENS framed on the whole-ship centroid. Fails if the ease is a
/// no-op (`test-the-wiring-system-not-just-its-pure-helpers`).
#[test]
fn ship_orbit_recenters_on_selected_section() {
    let (mut app, _hull, turret, _thruster) = offset_ship_app();

    let centroid = Vec3::new(3.0, 2.0, -1.0);
    let orbit = ship_orbit(&mut app);
    assert!(
        orbit.center.abs_diff_eq(centroid, 1e-4) && orbit.center_target.abs_diff_eq(centroid, 1e-4),
        "app opens framed on the whole-ship centroid, not a section"
    );

    // Idling with no selection change must NOT drift off the whole-ship view
    // (the default selection is treated as already centered at home).
    app.world_mut().run_system_once(ship_input).unwrap();
    drive_frames(&mut app, 30, 1.0 / 60.0);
    let orbit = ship_orbit(&mut app);
    assert!(
        orbit.center.abs_diff_eq(centroid, 1e-3),
        "no selection change keeps the center on the whole-ship centroid, got {:?}",
        orbit.center
    );

    // Select the turret and reconcile: the center RETARGETS to the turret's
    // LOCAL translation (9,2,-1), not its world pose.
    let turret_local = Vec3::new(9.0, 2.0, -1.0);
    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);
    app.world_mut().run_system_once(ship_input).unwrap();
    let orbit = ship_orbit(&mut app);
    assert_eq!(
        orbit.centered_on,
        Some(turret),
        "reconcile records the new selection"
    );
    assert!(
        orbit.center_target.abs_diff_eq(turret_local, 1e-4),
        "center_target follows the selected section's LOCAL position, got {:?}",
        orbit.center_target
    );
    // The eased center has not jumped yet - it must be driven there over frames.
    assert!(
        orbit.center.abs_diff_eq(centroid, 1e-3),
        "center does not snap on selection; it eases"
    );

    // Drive the camera: the live center converges onto the target and leaves
    // the centroid behind. This is the assertion that fails if the ease no-ops.
    drive_frames(&mut app, 60, 1.0 / 60.0);
    let orbit = ship_orbit(&mut app);
    assert!(
        orbit.center.abs_diff_eq(turret_local, 1e-2),
        "eased center reaches the selected section, got {:?}",
        orbit.center
    );
    assert!(
        orbit.center.distance(centroid) > 1.0,
        "eased center actually moved off the centroid (ease is not a no-op)"
    );
}

/// `T` re-frames the whole ship (center home + default angles) and the reframe
/// STICKS: the selection reconcile does not chase the still-selected section
/// back on the next frame.
#[test]
fn ship_reset_reframes_whole_ship_and_sticks() {
    let (mut app, _hull, turret, _thruster) = offset_ship_app();
    let centroid = Vec3::new(3.0, 2.0, -1.0);
    let turret_local = Vec3::new(9.0, 2.0, -1.0);

    // Select the turret and ease the center onto it.
    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);
    app.world_mut().run_system_once(ship_input).unwrap();
    drive_frames(&mut app, 60, 1.0 / 60.0);
    assert!(
        ship_orbit(&mut app).center.abs_diff_eq(turret_local, 1e-2),
        "precondition: center is on the turret before reset"
    );

    // Nudge the angles off default so T's angle reset is observable too.
    {
        let mut orbit = app
            .world_mut()
            .query_filtered::<&mut ShipOrbit, With<ShipCameraMarker>>()
            .single_mut(app.world_mut())
            .unwrap();
        orbit.theta += 0.5;
        orbit.phi = 0.3;
    }

    // Press T: retarget home, reset angles, and consume the selection.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    app.world_mut().run_system_once(ship_input).unwrap();
    let orbit = ship_orbit(&mut app);
    assert!(
        orbit.center_target.abs_diff_eq(centroid, 1e-4),
        "T retargets the center to the whole-ship centroid, got {:?}",
        orbit.center_target
    );
    assert_eq!(
        orbit.centered_on,
        Some(turret),
        "T consumes the selection so the reframe is not chased back"
    );
    assert!(
        (orbit.theta - SHIP_THETA_DEFAULT).abs() < 1e-4
            && (orbit.phi - SHIP_PHI_DEFAULT).abs() < 1e-4,
        "T restores the default orbit angles"
    );

    // Release T and reconcile again with the turret STILL selected: the center
    // target must stay home, not snap back to the turret.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyT);
    app.world_mut().run_system_once(ship_input).unwrap();
    drive_frames(&mut app, 60, 1.0 / 60.0);
    let orbit = ship_orbit(&mut app);
    assert!(
        orbit.center_target.abs_diff_eq(centroid, 1e-4),
        "the whole-ship reframe STICKS while the section stays selected"
    );
    assert!(
        orbit.center.abs_diff_eq(centroid, 1e-2),
        "the eased center settles back on the centroid, got {:?}",
        orbit.center
    );
}

/// A bare [`ShipSectionView`] for the pure-helper tests.
fn view_fixture(
    kind: SectionDamageClass,
    health: Option<Health>,
    ammo: Option<SectionAmmo>,
) -> ShipSectionView {
    ShipSectionView {
        entity: Entity::PLACEHOLDER,
        code: "PDC-1".to_string(),
        kind,
        name: "Bow gun".to_string(),
        local: Transform::default(),
        half_extents: Vec3::ONE,
        link_points: Vec::new(),
        health,
        ammo,
        inactive: false,
        zero_health: false,
    }
}

#[test]
fn mate_overlay_is_derived_from_live_section_link_points() {
    let mut left = view_fixture(SectionDamageClass::Hull, None, None);
    left.link_points = unit_cube_link_points();
    let mut right = view_fixture(SectionDamageClass::Hull, None, None);
    right.local.translation = Vec3::X;
    right.link_points = unit_cube_link_points();

    assert!(mate_edges_mesh(&[left.clone(), right.clone()]).is_some());
    left.link_points.clear();
    right.link_points.clear();
    assert!(mate_edges_mesh(&[left, right]).is_none());
}

#[test]
fn kind_glyph_distinct_per_kind() {
    use bevy::platform::collections::HashSet;
    let kinds = [
        SectionDamageClass::Hull,
        SectionDamageClass::Thruster,
        SectionDamageClass::Controller,
        SectionDamageClass::Turret,
        SectionDamageClass::Torpedo,
    ];
    let glyphs: Vec<&str> = kinds.iter().map(|&k| kind_glyph(k)).collect();
    assert!(
        glyphs.iter().all(|g| !g.is_empty()),
        "every kind has a glyph: {glyphs:?}"
    );
    let unique: HashSet<&&str> = glyphs.iter().collect();
    assert_eq!(
        unique.len(),
        kinds.len(),
        "each kind's glyph is distinct: {glyphs:?}"
    );
}

#[test]
fn blocks_stay_uniform_green_regardless_of_status() {
    // Regression pin on DECISION.md: the block FILL colour no longer encodes
    // status, so a critically-damaged section and a healthy one share the same
    // uniform-green fill material handle.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::NovaOs);
    app.init_resource::<ShipRuntime>();
    let mut terminal = ship_terminal();
    terminal.enter_app(SHIP_APP_ID);
    app.insert_resource(terminal);

    // Off-origin root (spatial-fixture-off-the-trivial-point).
    let ship_world = Vec3::new(500.0, -200.0, 900.0);
    let ship = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(ship_world),
            GlobalTransform::from(Transform::from_translation(ship_world)),
            Name::new("NOVA"),
        ))
        .id();
    let section = |name: &str, id: &str, offset: Vec3| {
        (
            SectionMarker,
            Name::new(name.to_string()),
            EntityId::new(id.to_string()),
            Transform::from_translation(offset),
            GlobalTransform::from(Transform::from_translation(ship_world + offset)),
            SectionCollider::Cuboid { size: Vec3::ONE },
            ChildOf(ship),
        )
    };
    let hull = app
        .world_mut()
        .spawn((
            section("Block Hull", "cube_a", Vec3::ZERO),
            HullSectionMarker,
            SectionDamageClass::Hull,
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let turret = app
        .world_mut()
        .spawn((
            section("Bow gun", "cube_b", Vec3::new(3.0, 0.0, 0.0)),
            TurretSectionMarker,
            SectionDamageClass::Turret,
            // 12/60 = critical.
            Health {
                current: 12.0,
                max: 60.0,
            },
            SectionAmmo {
                rounds: 2,
                capacity: 6,
            },
        ))
        .id();
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    app.world_mut().run_system_once(manage_ship_scene).unwrap();

    let fill_of = |app: &mut App, sect: Entity| -> Handle<StandardMaterial> {
        app.world_mut()
            .query::<(&ShipBlock, &MeshMaterial3d<StandardMaterial>)>()
            .iter(app.world())
            .find(|(b, _)| b.section == sect)
            .map(|(_, m)| m.0.clone())
            .expect("block fill for section")
    };
    let hull_fill = fill_of(&mut app, hull);
    let turret_fill = fill_of(&mut app, turret);
    assert_eq!(
        hull_fill, turret_fill,
        "a critical section's fill uses the SAME uniform-green handle as a nominal one",
    );
}

#[test]
fn blip_is_status_dot_with_labelled_marker() {
    // The blip is a status-coloured dot + a labelled marker; the integrity bar
    // and ammo pips are gone (HP/ammo live in the inspector panel now). A
    // critical turret's dot reads amber, and no ammo pips remain.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();

    let viewport = app.world_mut().spawn_empty().id();
    // 12/60 = 20% -> critical -> amber dot.
    let view = view_fixture(
        SectionDamageClass::Turret,
        Some(Health {
            current: 12.0,
            max: 60.0,
        }),
        Some(SectionAmmo {
            rounds: 2,
            capacity: 6,
        }),
    );
    let blip = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_ship_blip(&mut commands, viewport, &view, Handle::default())
        })
        .unwrap();

    // The dot fill reflects status (critical -> amber).
    let dot_bg = app
        .world()
        .get::<BackgroundColor>(blip)
        .expect("dot background")
        .0;
    assert_eq!(
        dot_bg, NOVA_OS_AMBER,
        "a critical section's dot reads amber"
    );

    // Some descendant label carries the kind glyph + code, and no ammo pips
    // survive anywhere on the blip.
    let label = format!("{} {}", kind_glyph(SectionDamageClass::Turret), "PDC-1");
    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(
        texts.contains(&label),
        "the blip label reads '{label}', got {texts:?}",
    );
    assert!(
        !texts.iter().any(|t| t.contains('●') || t.contains('○')),
        "no ammo pips remain on the blip: {texts:?}",
    );
}

#[test]
fn unknown_health_reads_nominal() {
    // A section with no `Health` reads nominal (green), never a misleading
    // damaged state - the edge the deleted bar/pips test used to pin, kept
    // because `status`/`status_color` now drive the blip dot and the panel.
    let unknown = view_fixture(SectionDamageClass::Thruster, None, None);
    assert_eq!(unknown.status(), "nominal");
    assert_eq!(unknown.status_color(), NOVA_OS_PHOSPHOR);
}

#[test]
fn panel_action_state_gates_repair_and_reload() {
    // Hull: repairable, no ammo feed -> reload disabled with the handler's text.
    let hull = view_fixture(
        SectionDamageClass::Hull,
        Some(Health {
            current: 80.0,
            max: 100.0,
        }),
        None,
    );
    let a = panel_action_state(&hull);
    assert!(a.repair_enabled, "a hull with HP is repairable");
    assert!(!a.reload_enabled, "a hull has no ammo feed");
    assert!(
        a.reason.as_deref().unwrap().contains("no ammo feed"),
        "{:?}",
        a.reason
    );

    // Armed turret with ammo + HP: both enabled, no reason.
    let turret = view_fixture(
        SectionDamageClass::Turret,
        Some(Health {
            current: 12.0,
            max: 60.0,
        }),
        Some(SectionAmmo {
            rounds: 2,
            capacity: 6,
        }),
    );
    let a = panel_action_state(&turret);
    assert!(a.repair_enabled && a.reload_enabled, "armed turret: both");
    assert!(a.reason.is_none(), "no disabled reason: {:?}", a.reason);

    // Weapon with ammo but NO health: reload enabled, repair disabled w/ reason.
    let ghost = view_fixture(
        SectionDamageClass::Turret,
        None,
        Some(SectionAmmo {
            rounds: 0,
            capacity: 6,
        }),
    );
    let a = panel_action_state(&ghost);
    assert!(a.reload_enabled && !a.repair_enabled);
    assert!(
        a.reason
            .as_deref()
            .unwrap()
            .contains("no integrity to restore"),
        "{:?}",
        a.reason
    );
}

#[test]
fn panel_detail_text_covers_live_fields() {
    let turret = view_fixture(
        SectionDamageClass::Turret,
        Some(Health {
            current: 12.0,
            max: 60.0,
        }),
        Some(SectionAmmo {
            rounds: 2,
            capacity: 6,
        }),
    );
    let text = panel_detail_text(&turret);
    assert!(text.contains("kind: turret"), "{text}");
    assert!(
        text.contains(kind_description(SectionDamageClass::Turret)),
        "{text}"
    );
    assert!(text.contains("20%"), "12/60 -> 20%: {text}");
    assert!(text.contains("status: critical"), "{text}");
    assert!(text.contains("ammo: 2/6"), "{text}");
}

#[test]
fn panel_buttons_raise_ship_section_command() {
    // Each button's `Activate` observer routes a ShipSectionCommand for the
    // selected section - but only when the panel marked that action enabled.
    // Pins BOTH button entry points at their own boundary
    // (`pin-each-caller-not-just-shared-core`).
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ShipRuntime>();
    app.add_message::<ShipSectionCommand>();
    let (_ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // --- Reload button on the turret (starts at 2/6). ---
    let reload = app
        .world_mut()
        .spawn(ShipPanelButton::Reload)
        .observe(on_ship_reload_button)
        .id();
    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);

    // Disabled: activating the button is a no-op (ammo unchanged at 2).
    app.world_mut()
        .resource_mut::<ShipRuntime>()
        .panel_reload_enabled = false;
    app.world_mut().trigger(Activate { entity: reload });
    app.world_mut()
        .run_system_once(apply_ship_section_commands)
        .unwrap();
    assert_eq!(
        app.world().get::<SectionAmmo>(turret).unwrap().rounds,
        2,
        "a disabled reload button writes no command",
    );

    // Enabled: activating routes the command and refills ammo to capacity.
    app.world_mut()
        .resource_mut::<ShipRuntime>()
        .panel_reload_enabled = true;
    app.world_mut().trigger(Activate { entity: reload });
    app.world_mut()
        .run_system_once(apply_ship_section_commands)
        .unwrap();
    assert_eq!(
        app.world().get::<SectionAmmo>(turret).unwrap().rounds,
        6,
        "an enabled reload button routes through the ShipSectionCommand seam",
    );

    // --- Repair button on the hull (starts at 80/100), the other caller. ---
    let repair = app
        .world_mut()
        .spawn(ShipPanelButton::Repair)
        .observe(on_ship_repair_button)
        .id();
    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(hull);

    app.world_mut()
        .resource_mut::<ShipRuntime>()
        .panel_repair_enabled = false;
    app.world_mut().trigger(Activate { entity: repair });
    app.world_mut()
        .run_system_once(apply_ship_section_commands)
        .unwrap();
    assert_eq!(
        app.world().get::<Health>(hull).unwrap().current,
        80.0,
        "a disabled repair button writes no command",
    );

    app.world_mut()
        .resource_mut::<ShipRuntime>()
        .panel_repair_enabled = true;
    app.world_mut().trigger(Activate { entity: repair });
    app.world_mut()
        .run_system_once(apply_ship_section_commands)
        .unwrap();
    assert_eq!(
        app.world().get::<Health>(hull).unwrap().current,
        100.0,
        "an enabled repair button routes through the ShipSectionCommand seam",
    );
}

#[test]
fn update_ship_panel_reflects_selection() {
    // The live refresh system wires the pure helpers into the panel tree and
    // caches the button-enabled flags the observers read. Reverting it to a
    // no-op must fail this (the detail text would stay the placeholder and the
    // flags stay false).
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();
    app.init_resource::<ShipRuntime>();
    let (_ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // Build the panel subtree under a root, the way `spawn_body` does.
    let root = app.world_mut().spawn_empty().id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .entity(root)
                .with_children(|parent| spawn_ship_panel(parent, Handle::default()));
        })
        .unwrap();

    {
        let mut runtime = app.world_mut().resource_mut::<ShipRuntime>();
        runtime.active = true;
        runtime.selected = Some(hull);
    }
    app.world_mut().run_system_once(update_ship_panel).unwrap();

    let field_text = |app: &mut App, want: ShipPanelField| -> String {
        app.world_mut()
            .query::<(&ShipPanelField, &Text)>()
            .iter(app.world())
            .find(|(field, _)| **field == want)
            .map(|(_, text)| text.0.clone())
            .expect("panel field text")
    };
    let detail = field_text(&mut app, ShipPanelField::Detail);
    assert!(
        detail.contains("kind: hull"),
        "detail reflects kind: {detail}"
    );
    assert!(detail.contains("status:"), "detail has status: {detail}");
    let title = field_text(&mut app, ShipPanelField::Title);
    assert!(title.contains("HULL-1"), "title reflects the code: {title}");

    // A hull caches reload DISABLED and repair ENABLED for the observers.
    let runtime = app.world().resource::<ShipRuntime>();
    assert!(
        !runtime.panel_reload_enabled,
        "a hull has no ammo feed -> reload disabled",
    );
    assert!(
        runtime.panel_repair_enabled,
        "a hull with HP is repairable -> repair enabled",
    );
}

// -----------------------------------------------------------------------
// Clicking sections through the CRT composite
// -----------------------------------------------------------------------

/// The ship viewport, standing in the rig's through-image content root.
fn rig_ship_viewport(rig: &mut NovaOsPointerRig) -> Entity {
    let viewport = rig
        .app
        .world_mut()
        .spawn((
            ShipViewportMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
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

fn rig_section_view(entity: Entity, code: &str) -> ShipSectionView {
    ShipSectionView {
        entity,
        code: code.to_string(),
        kind: SectionDamageClass::Hull,
        name: code.to_string(),
        local: Transform::IDENTITY,
        half_extents: Vec3::splat(0.5),
        link_points: Vec::new(),
        health: None,
        ammo: None,
        inactive: false,
        zero_health: false,
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

/// The `ship` app is the OTHER caller of this hit-target shape, so it is
/// pinned end to end here rather than left to the map's coverage
/// (`pin-each-caller-not-just-shared-core`). Same three checks: the pill
/// touches its dot, and dot / seam / label all select the section through the
/// real CRT composite.
#[test]
fn ship_section_label_and_dot_are_one_unbroken_target() {
    let aim_uv = Vec2::new(0.3, 0.6);
    let mut rig = nova_os_pointer_rig();
    rig.app.init_resource::<ShipRuntime>();
    let viewport = rig_ship_viewport(&mut rig);

    let section = rig.app.world_mut().spawn_empty().id();
    let view = rig_section_view(section, "HULL-1");
    let image_px = image_px_shown_at(aim_uv);
    let dot_entity = rig
        .app
        .world_mut()
        .run_system_once_with(
            |input: In<(Entity, ShipSectionView)>, mut commands: Commands| {
                let (viewport, view) = input.0;
                spawn_ship_blip(&mut commands, viewport, &view, Handle::default())
            },
            (viewport, view),
        )
        .expect("spawning a ship blip through the production path");
    {
        let mut node = rig
            .app
            .world_mut()
            .get_mut::<Node>(dot_entity)
            .expect("the blip has a Node");
        node.left = Val::Px(image_px.x - SHIP_BLIP_PX * 0.5);
        node.top = Val::Px(image_px.y - SHIP_BLIP_PX * 0.5);
    }
    settle(&mut rig.app);

    let label = {
        let children = rig
            .app
            .world()
            .get::<Children>(dot_entity)
            .expect("the blip has a label child");
        assert_eq!(children.len(), 1, "the dot's only child is its label pill");
        children[0]
    };
    let dot = rig_rect(&rig, dot_entity);
    let pill = rig_rect(&rig, label);
    assert!(
        pill.min.x <= dot.max.x + 0.01,
        "the label pill starts at x {} but the dot ends at x {} - {:.1} px of \
         dead band between the two halves of one target",
        pill.min.x,
        dot.max.x,
        pill.min.x - dot.max.x,
    );

    // The same seam sweep the map app gets, at pixel centres (the shared edge
    // itself is a measure-zero boundary `contains_point` excludes).
    let y = dot.center().y;
    let first = dot.center().x + 0.5;
    let last = pill.min.x + 6.0;
    // Counted, not accumulated - see the map app's twin of this sweep.
    let steps = ((last - first).floor() as i32 + 1).max(0);
    let mut probed = 0;
    for step in 0..steps {
        let x = first + step as f32;
        let at = Vec2::new(x, y);
        rig.app.world_mut().resource_mut::<ShipRuntime>().selected = None;
        click_at(&mut rig, glass_px(glass_uv_showing(at)));
        assert_eq!(
            rig.app.world().resource::<ShipRuntime>().selected,
            Some(section),
            "clicking image px {at:?} - between the dot's centre and 6 px into \
             the pill - must select the section; the pointer landed on {:?}",
            pointer_image_px(&rig),
        );
        probed += 1;
    }
    assert!(
        probed >= 12,
        "the sweep only probed {probed} points - it is not crossing the seam"
    );
}
