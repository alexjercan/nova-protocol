//! Live-tree tests for the Ship pane: section codes, the repair message
//! path, the scene's local-space placement and orbit input, and window-space
//! picking.

use bevy::{
    ecs::system::RunSystemOnce,
    input::InputPlugin,
    state::app::StatesPlugin,
    ui::{ComputedNode, UiGlobalTransform},
    ui_widgets::Activate,
};
use nova_events::prelude::EntityId;
use nova_gameplay::{prelude::*, PauseStates};
use nova_input::prelude::RegisterInputActions;
use nova_ship::prelude::*;
use nova_ui::{
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedFill, ThemedImageTint},
};

use super::{app::*, scene::*, sections::*, *};
use crate::{
    icons::{InterfaceIcons, SectionIconType},
    pane::InterfacePaneType,
    pointer_rig::{click_at, hear_ui_cues, pane_pointer_rig, settle, take_cues, PanePointerRig},
    terminal::NovaOsCloseTransition,
};

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
            SectionClass::Hull,
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
            SectionClass::Turret,
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
            SectionClass::Thruster,
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    (ship, hull, turret, thruster)
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
fn ship_repair_spends_plates_only_on_a_damaged_player_section() {
    // Repair spends exactly the selected whole plates, each restoring up to 20
    // HP. Live stock and Health are validated before either mutates. Every
    // refusal leaves Health, markers and both ships' inventories untouched.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ShipRuntime>();
    app.add_message::<SectionRepairCommand>();
    // A scheduled system keeps one reader cursor, so each frame reads only its
    // own commands; a `run_system_once` reader would replay older ones.
    app.add_systems(Update, apply_ship_section_commands);
    let (ship, hull, turret, thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();
    let repair = |app: &mut App, targets: &[(Entity, u32)]| {
        for &(target, requested_plates) in targets {
            app.world_mut().write_message(SectionRepairCommand {
                target,
                requested_plates,
            });
        }
        app.update();
    };
    let health = |app: &App, section: Entity| app.world().get::<Health>(section).unwrap().current;
    let plates = |app: &App, ship: Entity| {
        app.world()
            .get::<ShipInventory>(ship)
            .unwrap()
            .count(ItemType::HullPlate)
    };
    let note = |app: &App| {
        app.world()
            .resource::<ShipRuntime>()
            .note
            .clone()
            .map(|(text, _)| text)
    };
    let stock = |count| ShipInventory::new(400_000, [(ItemType::HullPlate, count)]);

    // No plates: the P key still sends, and the hull stays at 80/100.
    repair(&mut app, &[(hull, 1)]);
    assert_eq!(
        health(&app, hull),
        80.0,
        "a repair with no plates is not free"
    );
    assert_eq!(
        note(&app).as_deref(),
        Some("repair: no hull plates selected or in stock")
    );

    // Two commands in one run: the first spends 1 plate for 20 missing HP, the
    // second sees the committed full hull and spends nothing.
    app.world_mut().entity_mut(ship).insert(stock(2));
    repair(&mut app, &[(hull, 1), (hull, 1)]);
    assert_eq!(health(&app, hull), 100.0);
    assert_eq!(plates(&app, ship), 1);
    assert_eq!(
        note(&app).as_deref(),
        Some("repair: HULL-1 is at full integrity")
    );

    // PDC-1 at 12/60 accepts the selected one plate and spends it exactly.
    repair(&mut app, &[(turret, 1)]);
    assert_eq!(health(&app, turret), 32.0);
    assert!(app.world().get::<ShipInventory>(ship).unwrap().is_empty());
    assert_eq!(
        note(&app).as_deref(),
        Some("repaired PDC-1: 1 hull plate, 32/60 HP")
    );

    // A destroyed section and a collapse-disabled one with HP left refuse
    // without a spend, and keep their markers.
    app.world_mut().entity_mut(ship).insert(stock(5));
    app.world_mut().entity_mut(turret).insert((
        Health {
            current: 0.0,
            max: 60.0,
        },
        HealthZeroMarker,
        IntegrityDisabledMarker,
    ));
    app.world_mut().entity_mut(thruster).insert((
        Health {
            current: 50.0,
            max: 100.0,
        },
        IntegrityDisabledMarker,
    ));
    repair(&mut app, &[(turret, 1), (thruster, 1)]);
    assert_eq!(health(&app, turret), 0.0);
    assert_eq!(health(&app, thruster), 50.0);
    assert_eq!(plates(&app, ship), 5);
    assert!(app.world().entity(turret).contains::<HealthZeroMarker>());
    assert!(app
        .world()
        .entity(turret)
        .contains::<IntegrityDisabledMarker>());
    assert!(app
        .world()
        .entity(thruster)
        .contains::<IntegrityDisabledMarker>());
    assert_eq!(note(&app).as_deref(), Some("repair: THR-1 is destroyed"));

    // Another ship's section, a ship root and a despawned entity: silent, and
    // nothing changes on either ship.
    let other = app.world_mut().spawn((SpaceshipRootMarker, stock(4))).id();
    let foreign = app
        .world_mut()
        .spawn((
            SectionMarker,
            HullSectionMarker,
            SectionClass::Hull,
            SectionCode("HULL-9".to_string()),
            Health {
                current: 40.0,
                max: 100.0,
            },
            ChildOf(other),
        ))
        .id();
    let gone = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(gone);
    app.world_mut().resource_mut::<ShipRuntime>().note = None;
    repair(&mut app, &[(foreign, 1), (ship, 1), (gone, 1)]);
    assert_eq!(health(&app, foreign), 40.0);
    assert_eq!(plates(&app, other), 4);
    assert_eq!(plates(&app, ship), 5);
    assert_eq!(note(&app), None);
}

#[test]
fn ship_repair_spends_selected_quantity_and_refuses_stale_requests_atomically() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ShipRuntime>();
    app.add_message::<SectionRepairCommand>();
    app.add_systems(Update, apply_ship_section_commands);
    let (ship, _hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    let stock = |count| ShipInventory::new(400_000, [(ItemType::HullPlate, count)]);
    let health = |app: &App| app.world().get::<Health>(turret).unwrap().current;
    let plates = |app: &App| {
        app.world()
            .get::<ShipInventory>(ship)
            .unwrap()
            .count(ItemType::HullPlate)
    };
    let repair = |app: &mut App, requested_plates| {
        app.world_mut().write_message(SectionRepairCommand {
            target: turret,
            requested_plates,
        });
        app.update();
    };

    app.world_mut().entity_mut(ship).insert(stock(10));

    // Selected 1, an intermediate 2, and All (the 3 plates currently needed)
    // each spend exactly their request and restore 20 HP per plate.
    repair(&mut app, 1);
    assert_eq!(health(&app), 32.0);
    assert_eq!(plates(&app), 9);

    app.world_mut().get_mut::<Health>(turret).unwrap().current = 12.0;
    repair(&mut app, 2);
    assert_eq!(health(&app), 52.0);
    assert_eq!(plates(&app), 7);

    app.world_mut().get_mut::<Health>(turret).unwrap().current = 12.0;
    let all = plate_repair_limit(app.world().get::<Health>(turret), false, plates(&app));
    assert_eq!(all, 3);
    repair(&mut app, all);
    assert_eq!(health(&app), 60.0);
    assert_eq!(plates(&app), 4);

    // Stock falls after the quantity is requested but before the handler reads
    // it. The stale request refuses without a partial spend or Health change.
    app.world_mut().get_mut::<Health>(turret).unwrap().current = 12.0;
    app.world_mut().entity_mut(ship).insert(stock(3));
    app.world_mut().write_message(SectionRepairCommand {
        target: turret,
        requested_plates: 2,
    });
    app.world_mut().entity_mut(ship).insert(stock(1));
    app.update();
    assert_eq!(health(&app), 12.0);
    assert_eq!(plates(&app), 1);
    assert_eq!(
        app.world()
            .resource::<ShipRuntime>()
            .note
            .as_ref()
            .map(|(text, _)| text.as_str()),
        Some("repair: PDC-1 request exceeds live hull plate stock")
    );

    // Health changes after a 3-plate request is queued. Only one plate now
    // fits the missing integrity, so the full request refuses atomically.
    app.world_mut().entity_mut(ship).insert(stock(5));
    app.world_mut().get_mut::<Health>(turret).unwrap().current = 12.0;
    app.world_mut().write_message(SectionRepairCommand {
        target: turret,
        requested_plates: 3,
    });
    app.world_mut().get_mut::<Health>(turret).unwrap().current = 50.0;
    app.update();
    assert_eq!(health(&app), 50.0);
    assert_eq!(plates(&app), 5);
    assert_eq!(
        app.world()
            .resource::<ShipRuntime>()
            .note
            .as_ref()
            .map(|(text, _)| text.as_str()),
        Some("repair: PDC-1 request exceeds live missing integrity")
    );
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
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<ShipRuntime>();
    app.init_resource::<ActiveUiTheme>();
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);

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
            SectionClass::Turret,
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

/// The scene lives while the Ship pane is shown and goes with a switch to Map.
#[test]
fn ship_scene_follows_the_ship_pane() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<ShipRuntime>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);
    app.update();

    app.world_mut().run_system_once(manage_ship_scene).unwrap();
    assert!(app.world().resource::<ShipRuntime>().active);

    *app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Map;
    app.world_mut().run_system_once(manage_ship_scene).unwrap();
    assert!(!app.world().resource::<ShipRuntime>().active);
}

#[test]
fn ship_pane_renders_blocks_and_selects_section() {
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
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<ShipRuntime>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);
    app.add_message::<SectionRepairCommand>();

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
    // Under the command modal over the pane the key is text, not a cycle.
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Commands);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::BracketRight);
    app.world_mut().run_system_once(ship_input).unwrap();
    assert_eq!(
        app.world().resource::<ShipRuntime>().selected,
        before,
        "the modal over the pane takes no selection key"
    );
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::BracketRight);
        keys.press(KeyCode::BracketRight);
    }
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
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<ShipRuntime>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);
    app.add_message::<SectionRepairCommand>();

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
            SectionClass::Hull,
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
            SectionClass::Turret,
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
            SectionClass::Thruster,
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    (hull, turret, thruster)
}

/// A Ship pane fixture built from [`spawn_offset_ship`], with the scene already
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
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<ShipRuntime>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);
    app.add_message::<SectionRepairCommand>();

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

/// Advance the real clock and drive the camera `frames` times, so the
/// exponential center ease actually integrates over real time.
fn drive_frames(app: &mut App, frames: usize, dt: f32) {
    // The first real update only starts the clock, with a zero delta.
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(std::time::Duration::ZERO);
    for _ in 0..frames {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .update_with_duration(std::time::Duration::from_secs_f32(dt));
        app.world_mut().run_system_once(drive_ship_camera).unwrap();
    }
}

/// Selecting a non-default section retargets the orbit center to that section's
/// LOCAL position, and `drive_ship_camera` eases the live center onto it. The
/// pane still OPENS framed on the whole-ship centroid. Fails if the ease is a
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

/// Selecting a section with no `ShipInventory` on the player ship (mid
/// transition, same window `update_ship_panel` already guards) must not panic
/// a `single().expect(..)` on the missing inventory; it treats stock as empty.
#[test]
fn ship_input_selection_repair_reset_survives_a_missing_player_inventory() {
    let (mut app, hull, _turret, _thruster) = offset_ship_app();
    let ship = app
        .world_mut()
        .query_filtered::<Entity, With<SpaceshipRootMarker>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(ship).remove::<ShipInventory>();

    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(hull);
    app.world_mut().run_system_once(ship_input).unwrap();

    let runtime = app.world().resource::<ShipRuntime>();
    assert_eq!(runtime.repair_target, Some(hull));
    assert_eq!(
        runtime.requested_plates, 0,
        "no inventory means no plates to request, not a panic"
    );
}

/// `T` re-frames the whole ship (center home, default angles and the opening
/// zoom) and the reframe STICKS: the selection reconcile does not chase the still-selected section
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

    // Nudge the angles and the zoom off the opening view so T's reset of
    // both is observable too.
    let opening_radius = ship_orbit(&mut app).radius;
    {
        let mut orbit = app
            .world_mut()
            .query_filtered::<&mut ShipOrbit, With<ShipCameraMarker>>()
            .single_mut(app.world_mut())
            .unwrap();
        orbit.theta += 0.5;
        orbit.phi = 0.3;
        orbit.radius = opening_radius * 0.5;
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
    assert!(
        (orbit.radius - opening_radius).abs() < 1e-4,
        "T restores the opening zoom, got {} want {opening_radius}",
        orbit.radius
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
    kind: SectionClass,
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
        bindings: None,
        inactive: false,
        zero_health: false,
        disabled: false,
    }
}

#[test]
fn mate_overlay_is_derived_from_live_section_link_points() {
    let mut left = view_fixture(SectionClass::Hull, None, None);
    left.link_points = unit_cube_link_points();
    let mut right = view_fixture(SectionClass::Hull, None, None);
    right.local.translation = Vec3::X;
    right.link_points = unit_cube_link_points();

    assert!(mate_edges_mesh(&[left.clone(), right.clone()]).is_some());
    left.link_points.clear();
    right.link_points.clear();
    assert!(mate_edges_mesh(&[left, right]).is_none());
}

#[test]
fn block_fill_follows_family_not_status() {
    // The block FILL names the section family, never its status: a critical
    // turret and a healthy turret share one fill handle, and the hull has its
    // own.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.init_asset::<Image>();
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<ShipRuntime>();
    app.init_resource::<ActiveUiTheme>();
    app.init_resource::<NovaOsCloseTransition>();
    app.insert_resource(InterfacePaneType::Ship);

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
            SectionClass::Hull,
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
            SectionClass::Turret,
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
    let healthy_turret = app
        .world_mut()
        .spawn((
            section("Stern gun", "cube_c", Vec3::new(-3.0, 0.0, 0.0)),
            TurretSectionMarker,
            SectionClass::Turret,
            Health {
                current: 60.0,
                max: 60.0,
            },
            SectionAmmo {
                rounds: 6,
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
        turret_fill,
        fill_of(&mut app, healthy_turret),
        "a critical turret's fill is the SAME handle as a healthy turret's",
    );
    assert_ne!(hull_fill, turret_fill, "a turret's fill is not the hull's");
}

#[test]
fn badge_shows_family_icon_status_pip_and_code() {
    // A section badge is its family icon, a pip in its status colour and a
    // code label; HP and ammo live in the section panel.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();

    let viewport = app.world_mut().spawn_empty().id();
    // 12/60 = 20% -> critical.
    let view = view_fixture(
        SectionClass::Turret,
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
            spawn_ship_blip(&mut commands, viewport, &view, &InterfaceIcons::blank())
        })
        .unwrap();

    let children: Vec<Entity> = app
        .world()
        .get::<Children>(blip)
        .expect("the badge has children")
        .iter()
        .collect();
    let tint = children
        .iter()
        .find_map(|child| app.world().get::<ThemedImageTint>(*child))
        .expect("the badge shows its family icon");
    assert_eq!(
        tint.color,
        UiColor::Danger,
        "a turret wears the weapon tint"
    );
    let pip = children
        .iter()
        .filter(|child| app.world().get::<ShipStatusPip>(**child).is_some())
        .find_map(|child| app.world().get::<ThemedFill>(*child))
        .expect("the badge has a status pip");
    assert_eq!(
        pip.color,
        UiColor::Danger,
        "a critical section's pip reads danger"
    );

    let texts: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert_eq!(
        texts,
        vec!["PDC-1".to_string()],
        "the label carries only the code"
    );
}

#[test]
fn docking_and_mining_sections_have_distinct_legend_icons() {
    let docking = SectionIconType::of(SectionClass::Docking);
    let mining = SectionIconType::of(SectionClass::Mining);
    let intake = SectionIconType::of(SectionClass::CargoIntake);
    assert_ne!(docking, mining);
    assert_ne!(mining, SectionIconType::of(SectionClass::Hull));
    assert_ne!(intake, SectionIconType::of(SectionClass::Hull));
    assert_eq!(docking.label(), "Docking");
    assert_eq!(mining.label(), "Mining");
    assert_eq!(intake.label(), "Cargo Intake");
    assert!(SectionIconType::ALL.contains(&docking));
    assert!(SectionIconType::ALL.contains(&mining));
    assert!(SectionIconType::ALL.contains(&intake));
}

#[test]
fn unknown_health_reads_nominal() {
    // A section with no `Health` reads nominal (green), never a misleading
    // damaged state; `status`/`status_color` drive the badge pip and the
    // panel condition bar.
    let unknown = view_fixture(SectionClass::Thruster, None, None);
    assert_eq!(unknown.status(), "nominal");
    assert_eq!(unknown.status_color(), UiColor::Nominal);
}

#[test]
fn panel_action_state_gates_repair() {
    let hp = |current, max| Some(Health { current, max });
    let ammo = Some(SectionAmmo {
        rounds: 2,
        capacity: 6,
    });
    let reason = |a: &PanelActions| a.reason.clone().unwrap_or_default();

    // Damaged hull with plates: repairable, no reason.
    let hull = view_fixture(SectionClass::Hull, hp(80.0, 100.0), None);
    let a = panel_action_state(&hull, 1, 12);
    assert!(a.repair_enabled, "a damaged hull with plates is repairable");
    assert!(a.reason.is_none(), "no disabled reason: {:?}", a.reason);

    // No plates: refused with a reason.
    let a = panel_action_state(&hull, 1, 0);
    assert!(!a.repair_enabled);
    assert_eq!(reason(&a), "repair: no hull plates selected or in stock");

    // Full integrity refuses regardless of stock.
    let full = view_fixture(SectionClass::Hull, hp(100.0, 100.0), None);
    let a = panel_action_state(&full, 1, 0);
    assert!(!a.repair_enabled);
    assert_eq!(reason(&a), "repair: PDC-1 is at full integrity");

    // Damaged turret with plates: repairable, no reason.
    let turret = view_fixture(SectionClass::Turret, hp(12.0, 60.0), ammo);
    let a = panel_action_state(&turret, 1, 12);
    assert!(
        a.repair_enabled,
        "a damaged turret with plates is repairable"
    );
    assert!(a.reason.is_none(), "no disabled reason: {:?}", a.reason);
    assert!(panel_action_state(&turret, 2, 12).repair_enabled);
    let all = plate_repair_limit(turret.health.as_ref(), turret.disabled, 12);
    assert_eq!(all, 3, "All is the affordable missing-integrity limit");
    assert!(panel_action_state(&turret, all, 12).repair_enabled);
    assert_eq!(
        reason(&panel_action_state(&turret, 4, 12)),
        "repair: PDC-1 request exceeds live missing integrity"
    );

    // Destroyed at 0 HP, or disabled by a collapse with HP left.
    let dead = view_fixture(SectionClass::Turret, hp(0.0, 60.0), ammo);
    let a = panel_action_state(&dead, 1, 12);
    assert!(!a.repair_enabled);
    assert_eq!(reason(&a), "repair: PDC-1 is destroyed");
    let mut collapsed = view_fixture(SectionClass::Turret, hp(30.0, 60.0), ammo);
    collapsed.disabled = true;
    assert_eq!(
        reason(&panel_action_state(&collapsed, 1, 12)),
        "repair: PDC-1 is destroyed"
    );

    // A section with no health at all refuses with a reason.
    let ghost = view_fixture(SectionClass::Turret, None, ammo);
    let a = panel_action_state(&ghost, 1, 12);
    assert!(!a.repair_enabled);
    assert_eq!(reason(&a), "repair: PDC-1 has no integrity to restore");
}

#[test]
fn panel_detail_text_covers_live_fields() {
    let mut turret = view_fixture(
        SectionClass::Turret,
        Some(Health {
            current: 12.0,
            max: 60.0,
        }),
        Some(SectionAmmo {
            rounds: 2,
            capacity: 6,
        }),
    );
    turret.bindings = Some(vec![KeyCode::KeyK.into(), MouseButton::Left.into()]);
    assert_eq!(
        panel_status_text(&turret),
        "Weapon  Condition 20%  critical",
        "12/60 -> 20%"
    );
    let text = panel_detail_text(&turret);
    assert!(
        text.contains(kind_description(SectionClass::Turret)),
        "{text}"
    );
    assert!(text.contains("12/60 HP"), "{text}");
    assert!(text.contains("Ammunition: 2 / 6 rounds"), "{text}");
    assert!(text.contains("Control: K / LMB"), "{text}");
}

#[test]
fn panel_buttons_raise_section_repair_command() {
    // Each button's `Activate` observer routes a SectionRepairCommand for the
    // selected section, or arms a rebind, and clicks once - but only when the
    // panel marked that action enabled. A disabled or modal-blocked press stays
    // silent. Pins every button entry point at its own boundary
    // (`pin-each-caller-not-just-shared-core`).
    // A world trigger leaves the observers' commands queued, so each cue check
    // flushes first; a silent check without the flush would pass vacuously.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
    app.insert_state(PauseStates::Interface);
    app.init_resource::<ShipRuntime>();
    app.add_message::<SectionRepairCommand>();
    // A scheduled handler keeps one reader cursor, so a spent repair is never
    // replayed by a later step.
    app.add_systems(Update, apply_ship_section_commands);
    hear_ui_cues(&mut app);
    let (ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // --- Repair button on the hull (starts at 80/100). ---
    app.world_mut()
        .entity_mut(ship)
        .insert(ShipInventory::new(400_000, [(ItemType::HullPlate, 1)]));
    let repair = app
        .world_mut()
        .spawn(ShipPanelButton::Repair)
        .observe(on_ship_repair_button)
        .id();
    {
        let mut runtime = app.world_mut().resource_mut::<ShipRuntime>();
        runtime.selected = Some(hull);
        runtime.repair_target = Some(hull);
        runtime.requested_plates = 1;
        runtime.panel_repair_enabled = false;
    }
    app.world_mut().trigger(Activate { entity: repair });
    app.update();
    assert_eq!(
        app.world().get::<Health>(hull).unwrap().current,
        80.0,
        "a disabled repair button writes no command",
    );
    app.world_mut().flush();
    assert!(
        take_cues(&mut app).is_empty(),
        "a disabled repair stays silent"
    );

    app.world_mut()
        .resource_mut::<ShipRuntime>()
        .panel_repair_enabled = true;
    app.world_mut().trigger(Activate { entity: repair });
    app.update();
    assert_eq!(
        app.world().get::<Health>(hull).unwrap().current,
        100.0,
        "an enabled repair button routes through the SectionRepairCommand seam",
    );
    app.world_mut().flush();
    assert_eq!(take_cues(&mut app), [UiSfx::MenuSelect]);

    // --- Rebind button on the turret, the caller that arms a capture. ---
    let rebind = app
        .world_mut()
        .spawn(ShipPanelButton::Rebind)
        .observe(on_ship_rebind_button)
        .id();
    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);

    app.world_mut().trigger(Activate { entity: rebind });
    assert_eq!(
        app.world().resource::<ShipRuntime>().rebinding,
        None,
        "a disabled rebind button arms nothing",
    );
    app.world_mut().flush();
    assert!(
        take_cues(&mut app).is_empty(),
        "a disabled rebind stays silent"
    );

    app.world_mut()
        .resource_mut::<ShipRuntime>()
        .panel_rebind_enabled = true;
    app.world_mut().trigger(Activate { entity: rebind });
    assert_eq!(
        app.world().resource::<ShipRuntime>().rebinding,
        Some(turret),
        "an enabled rebind button arms a capture on the selection",
    );
    app.world_mut().flush();
    assert_eq!(take_cues(&mut app), [UiSfx::MenuSelect]);
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
    app.insert_resource(InterfaceIcons::blank());
    let (ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .entity_mut(ship)
        .insert(ShipInventory::new(400_000, [(ItemType::HullPlate, 1)]));
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    // Build the panel subtree under a root, the way `spawn_body` does.
    let root = app.world_mut().spawn_empty().id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .entity(root)
                .with_children(|parent| spawn_ship_panel(parent, &InterfaceIcons::blank()));
        })
        .unwrap();

    {
        let mut runtime = app.world_mut().resource_mut::<ShipRuntime>();
        runtime.active = true;
        runtime.selected = Some(hull);
        runtime.repair_target = Some(hull);
        runtime.requested_plates = 1;
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
    let status = field_text(&mut app, ShipPanelField::Status);
    assert!(
        status.starts_with("Hull  Condition"),
        "the status line names the family and condition: {status}"
    );
    let title = field_text(&mut app, ShipPanelField::Title);
    assert!(title.contains("HULL-1"), "title reflects the code: {title}");

    // A damaged hull with plates caches repair ENABLED for the observer.
    let runtime = app.world().resource::<ShipRuntime>();
    assert!(
        runtime.panel_repair_enabled,
        "a damaged hull with plates is repairable -> repair enabled",
    );
}

/// Build the same panel subtree as [`update_ship_panel_reflects_selection`],
/// but without a `PlayerSpaceshipMarker` entity: `SpaceshipRootMarker`
/// requires `ShipInventory`, so the only way to lack one is to lack the
/// player ship itself, as during a transition between ships.
fn spawn_ship_panel_app_without_inventory() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();
    app.init_resource::<ShipRuntime>();
    app.insert_resource(InterfaceIcons::blank());
    let (ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .entity_mut(ship)
        .remove::<PlayerSpaceshipMarker>();
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    let root = app.world_mut().spawn_empty().id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .entity(root)
                .with_children(|parent| spawn_ship_panel(parent, &InterfaceIcons::blank()));
        })
        .unwrap();
    (app, hull)
}

#[test]
fn update_ship_panel_shows_no_section_without_a_player_inventory() {
    // Mid-transition the Ship pane can turn active before the player ship's
    // ShipInventory lands; the panel must fall back to the no-section state
    // instead of panicking a `single().expect(..)` on the missing inventory.
    let (mut app, hull) = spawn_ship_panel_app_without_inventory();
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
    assert_eq!(field_text(&mut app, ShipPanelField::Title), "No section");
    assert!(!app.world().resource::<ShipRuntime>().panel_repair_enabled);
}

#[test]
fn update_ship_panel_preview_names_the_exact_repair_refusal() {
    // A section already at full integrity refuses on `Full`, not on a
    // quantity/stock mismatch the preview used to claim unconditionally.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();
    app.init_resource::<ShipRuntime>();
    app.insert_resource(InterfaceIcons::blank());
    let (ship, _hull, _turret, thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .entity_mut(ship)
        .insert(ShipInventory::new(400_000, [(ItemType::HullPlate, 1)]));
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    let root = app.world_mut().spawn_empty().id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .entity(root)
                .with_children(|parent| spawn_ship_panel(parent, &InterfaceIcons::blank()));
        })
        .unwrap();

    {
        let mut runtime = app.world_mut().resource_mut::<ShipRuntime>();
        runtime.active = true;
        runtime.selected = Some(thruster);
        runtime.repair_target = Some(thruster);
        runtime.requested_plates = 1;
    }
    app.world_mut().run_system_once(update_ship_panel).unwrap();

    let preview = app
        .world_mut()
        .query::<(&ShipPanelField, &Text)>()
        .iter(app.world())
        .find(|(field, _)| **field == ShipPanelField::RepairPreview)
        .map(|(_, text)| text.0.clone())
        .expect("preview field text");
    assert!(
        preview.ends_with("repair: THR-1 is at full integrity"),
        "preview names the Full refusal instead of a generic mismatch: {preview}"
    );
}

#[test]
fn update_ship_panel_quantity_drops_with_the_live_max_after_a_full_repair() {
    // A full repair processed by `apply_ship_section_commands` fills the
    // section but leaves `requested_plates` untouched; the panel's displayed
    // quantity must track the new (zero) live max rather than restate the
    // outstanding request.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();
    app.init_resource::<ShipRuntime>();
    app.insert_resource(InterfaceIcons::blank());
    app.add_message::<SectionRepairCommand>();
    app.add_systems(Update, apply_ship_section_commands);
    let (ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
    app.world_mut()
        .entity_mut(ship)
        .insert(ShipInventory::new(400_000, [(ItemType::HullPlate, 1)]));
    app.world_mut()
        .run_system_once(assign_section_codes)
        .unwrap();

    let root = app.world_mut().spawn_empty().id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands
                .entity(root)
                .with_children(|parent| spawn_ship_panel(parent, &InterfaceIcons::blank()));
        })
        .unwrap();

    {
        let mut runtime = app.world_mut().resource_mut::<ShipRuntime>();
        runtime.active = true;
        runtime.selected = Some(hull);
        runtime.repair_target = Some(hull);
        runtime.requested_plates = 1;
    }
    // Hull starts at 80/100: one plate (20 HP) fills it exactly.
    app.world_mut().write_message(SectionRepairCommand {
        target: hull,
        requested_plates: 1,
    });
    app.update();
    assert_eq!(app.world().get::<Health>(hull).unwrap().current, 100.0);
    assert_eq!(
        app.world().resource::<ShipRuntime>().requested_plates,
        1,
        "the processed command's quantity is not silently rewritten"
    );

    app.world_mut().run_system_once(update_ship_panel).unwrap();

    let quantity = app
        .world_mut()
        .query::<(&ShipPanelField, &Text)>()
        .iter(app.world())
        .find(|(field, _)| **field == ShipPanelField::RepairQuantity)
        .map(|(_, text)| text.0.clone())
        .expect("quantity field text");
    assert_eq!(
        quantity, "Selected: 0 hull plates",
        "quantity reflects the new live max (0), not the stale request"
    );
    assert_eq!(
        app.world().resource::<ShipRuntime>().requested_plates,
        1,
        "the panel refresh must not mutate the outstanding request state"
    );
}

/// The ship viewport, standing in the rig's content root.
fn rig_ship_viewport(rig: &mut PanePointerRig) -> Entity {
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
        kind: SectionClass::Hull,
        name: code.to_string(),
        local: Transform::IDENTITY,
        half_extents: Vec3::splat(0.5),
        link_points: Vec::new(),
        health: None,
        ammo: None,
        bindings: None,
        inactive: false,
        zero_health: false,
        disabled: false,
    }
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

/// The Ship pane is the OTHER caller of this hit-target shape, so it is
/// pinned end to end here rather than left to the map's coverage
/// (`pin-each-caller-not-just-shared-core`). Same three checks: the pill
/// touches its dot, and dot / seam / label all select the section through
/// real window picking.
#[test]
fn ship_section_label_and_dot_are_one_unbroken_target() {
    let mut rig = pane_pointer_rig();
    rig.app.init_resource::<ShipRuntime>();
    hear_ui_cues(&mut rig.app);
    let viewport = rig_ship_viewport(&mut rig);

    let section = rig.app.world_mut().spawn_empty().id();
    let view = rig_section_view(section, "HULL-1");
    let at = Vec2::new(384.0, 432.0);
    let dot_entity = rig
        .app
        .world_mut()
        .run_system_once_with(
            |input: In<(Entity, ShipSectionView)>, mut commands: Commands| {
                let (viewport, view) = input.0;
                spawn_ship_blip(&mut commands, viewport, &view, &InterfaceIcons::blank())
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
        node.left = Val::Px(at.x - SHIP_BLIP_PX * 0.5);
        node.top = Val::Px(at.y - SHIP_BLIP_PX * 0.5);
        // The projection reveals a badge once it has placed it.
        *rig.app
            .world_mut()
            .get_mut::<Visibility>(dot_entity)
            .expect("the blip has a Visibility") = Visibility::Inherited;
    }
    settle(&mut rig.app);

    // The code pill exists only for the SELECTION, so select the section the
    // way a click would and let the production system reveal it.
    rig.app.world_mut().resource_mut::<ShipRuntime>().selected = Some(section);
    rig.app
        .world_mut()
        .run_system_once(label_the_selected_section)
        .expect("labelling the selection");
    settle(&mut rig.app);

    let label = {
        let world = rig.app.world();
        let children = world
            .get::<Children>(dot_entity)
            .expect("the blip has a label child");
        let labels: Vec<Entity> = children
            .iter()
            .filter(|child| world.get::<ShipBlipLabel>(*child).is_some())
            .collect();
        assert_eq!(labels.len(), 1, "the dot carries ONE code pill");
        labels[0]
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

    // The same seam sweep the map pane gets, at pixel centres (the shared edge
    // itself is a measure-zero boundary `contains_point` excludes).
    let y = dot.center().y;
    let first = dot.center().x + 0.5;
    let last = pill.min.x + 6.0;
    // Counted, not accumulated - see the map pane's twin of this sweep.
    let steps = ((last - first).floor() as i32 + 1).max(0);
    let mut probed = 0;
    for step in 0..steps {
        let x = first + step as f32;
        let window_px = Vec2::new(x, y);
        rig.app.world_mut().resource_mut::<ShipRuntime>().selected = None;
        click_at(&mut rig, window_px);
        assert_eq!(
            rig.app.world().resource::<ShipRuntime>().selected,
            Some(section),
            "clicking window px {window_px:?} - between the dot's centre and 6 px \
             into the pill - must select the section",
        );
        probed += 1;
    }
    assert!(
        probed >= 12,
        "the sweep only probed {probed} points - it is not crossing the seam"
    );
    // Each probe selected the section afresh, so each clicked once.
    assert_eq!(take_cues(&mut rig.app), vec![UiSfx::MenuSelect; probed]);
    // The selected section again changes nothing and stays silent.
    click_at(&mut rig, Vec2::new(first, y));
    assert!(take_cues(&mut rig.app).is_empty());
}

/// The carrier's inspector used to draw 2081 code pills at once. Every
/// section keeps its clickable dot and its class glyph; only the selection
/// spells out its code.
#[test]
fn only_the_selected_section_spells_out_its_code() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Font>();
    app.init_resource::<ShipRuntime>();

    let viewport = app.world_mut().spawn_empty().id();
    let mut blips = Vec::new();
    for (index, kind) in [
        SectionClass::Hull,
        SectionClass::Thruster,
        SectionClass::Turret,
    ]
    .into_iter()
    .enumerate()
    {
        let mut view = view_fixture(kind, None, None);
        view.entity = app.world_mut().spawn_empty().id();
        view.code = format!("{}-{index}", code_prefix(kind));
        let spawned = view.clone();
        let blip = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_ship_blip(&mut commands, viewport, &spawned, &InterfaceIcons::blank())
            })
            .unwrap();
        blips.push((view.entity, blip, kind));
    }

    let labelled = |app: &mut App| -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<&Visibility, With<ShipBlipLabel>>()
            .iter(world)
            .filter(|visibility| **visibility != Visibility::Hidden)
            .count()
    };

    // Nothing selected: every pill is down, and every section still has the
    // dot that selects it.
    app.world_mut()
        .run_system_once(label_the_selected_section)
        .unwrap();
    assert_eq!(
        labelled(&mut app),
        0,
        "an unselected schematic shows no codes"
    );
    let dots = app
        .world_mut()
        .query_filtered::<(), With<ShipBlip>>()
        .iter(app.world())
        .count();
    assert_eq!(dots, blips.len(), "every section keeps its clickable dot");

    // Selecting one raises exactly its pill.
    let (section, blip, _) = blips[1];
    app.world_mut().resource_mut::<ShipRuntime>().selected = Some(section);
    app.world_mut()
        .run_system_once(label_the_selected_section)
        .unwrap();
    assert_eq!(labelled(&mut app), 1, "only the selection is spelled out");
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(blip)
        .expect("the blip has children")
        .iter()
        .collect();
    assert!(
        children.iter().any(|child| {
            app.world().get::<ShipBlipLabel>(*child).is_some()
                && app.world().get::<Visibility>(*child) != Some(&Visibility::Hidden)
        }),
        "the raised pill belongs to the selected section"
    );
}
