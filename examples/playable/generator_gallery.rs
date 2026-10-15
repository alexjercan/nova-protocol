//! generator_gallery: the open world's ship generator, one row of hulls at a
//! time, driven from an egui window.
//!
//! The window picks what the row shows: the role, intact or wreck, how many
//! hulls (1 to 7) across an advancement range, the hull seed, bare or clad,
//! and an optional style that replaces the role's own. When a control is
//! released the row is generated again through the world's own
//! `generate_ship` or `generate_wreck`, every design is run through the
//! content lint, and the scene is loaded again. A request the generator or
//! the lint refuses shows in red in the window and in the log; it does not
//! stand.
//!
//! The snapshot is the one the open world arms with: built from the loaded
//! section packs. Every hull spawns inline with no controller, as the world
//! spawns its ships, so the skin and greebles are the real ones.
//!
//! Camera: drag with the left or right mouse button to orbit the row, scroll
//! to zoom.
//!
//! ```text
//! nix develop --command cargo run --example generator_gallery --features debug
//! ```
//!
//! The diagnostics (`F3`) label every hull with its advancement, its
//! clearance radius, and its section, docking port, turret and flight
//! computer counts. The panel names the row.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk stands five hulls across advancement
//! 0 to 1, checks each one was generated and passed the lint, shoots
//! `generator-gallery.png` with `NOVA_CAPTURE=1`, then asks for the same row
//! as wrecks and checks it the same way. Unset, no walk and no deadline run.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
};
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{
    nova_debug::{
        bevy_egui::{egui, input::EguiWantsInput, EguiContexts, EguiPrimaryContextPass},
        diagnostics,
    },
    prelude::*,
};
use nova_wfc::prelude::hull_errors;
use nova_world::prelude::{CivilizationId, ShipRoleType};

/// The world seed of the pinned civilization every request names. Only a
/// failure line reads it.
const WORLD_SEED: u32 = 20_260_922;

/// The most hulls one row stands.
const MAX_ROW: usize = 7;

/// Clear space between the clearance spheres of two neighbouring hulls, in
/// meters.
const STAND_GAP: Meters = Meters(40.0);

/// Every hull stands at the same yaw, bow turned a quarter toward the camera,
/// so its side and bow both show.
const HULL_YAW: f32 = std::f32::consts::PI - 0.5;

/// Orbit speed, radians per pixel of mouse drag.
const ORBIT_RADIANS_PER_PIXEL: f32 = 0.005;

/// Zoom per scroll line: the orbit distance is multiplied by this per line
/// toward the row.
const ZOOM_PER_LINE: f32 = 0.9;

/// Pixels a pixel-unit scroll counts as one line.
const PIXELS_PER_LINE: f32 = 40.0;

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new()
        .with_game_plugins(|app: &mut App| {
            app.insert_resource(Controls::default());
            app.init_resource::<Orbit>();
            app.add_systems(OnEnter(GameAssetsStates::Loaded), pin_snapshot);
            app.add_systems(
                Update,
                (
                    load_row.run_if(resource_exists::<Snapshot>),
                    orbit_camera.run_if(in_state(GameStates::Playing)),
                ),
            );
            app.add_systems(EguiPrimaryContextPass, controls_window);
        })
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "the world's generated hulls in one row",
    ));
    app.add_systems(
        PostUpdate,
        label_hulls
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );
    // Nothing flies the hulls; still hulls keep the labels on them.
    app.add_systems(Update, freeze_bodies);
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// Whether a hull stands whole or as a wreck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConditionType {
    Intact,
    Wreck,
}

/// Whether the hulls wear their derived skin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkinType {
    Bare,
    Clad,
}

/// What the window asks the row to be.
#[derive(Resource, Clone, Debug, PartialEq)]
struct Controls {
    role: ShipRoleType,
    condition: ConditionType,
    /// How many hulls stand, 1 to [`MAX_ROW`].
    count: usize,
    /// The advancement of the first hull.
    from: f32,
    /// The advancement of the last hull. A row of one reads `from` only.
    to: f32,
    seed: u32,
    skin: SkinType,
    /// The style every hull wears instead of its role's, by id.
    style: Option<String>,
}

impl Default for Controls {
    /// Five industrial hulls across the whole advancement range, clad in
    /// their role's style: the row the smoke walk checks.
    fn default() -> Self {
        Self {
            role: ShipRoleType::Industrial,
            condition: ConditionType::Intact,
            count: 5,
            from: 0.0,
            to: 1.0,
            seed: 1,
            skin: SkinType::Clad,
            style: None,
        }
    }
}

impl Controls {
    /// The advancement of each hull, first to last.
    fn advancements(&self) -> Vec<f32> {
        if self.count <= 1 {
            return vec![self.from];
        }
        (0..self.count)
            .map(|index| self.from + (self.to - self.from) * index as f32 / (self.count - 1) as f32)
            .collect()
    }
}

/// The ship-part snapshot of the loaded catalog, built once when the assets
/// load, as the open world builds its own.
#[derive(Resource)]
struct Snapshot(ShipPartSnapshot);

/// One hull of the row, generated and linted.
#[derive(Clone, Debug)]
struct Hull {
    /// The scenario id it spawns under.
    name: String,
    advancement: f32,
    design: ShipDesign,
    clearance: Meters,
    sections: usize,
    ports: usize,
    turrets: usize,
    computers: usize,
}

/// The row on stage: the controls it was built from, the hulls that stand,
/// and every request the generator or the lint refused.
#[derive(Resource, Clone, Debug)]
struct Row {
    controls: Controls,
    hulls: Vec<Hull>,
    failures: Vec<String>,
}

/// The orbit camera: a point it looks at, and the yaw, pitch and distance it
/// stands at from it.
#[derive(Resource)]
struct Orbit {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.35,
            distance: 100.0,
        }
    }
}

impl Orbit {
    fn eye(&self) -> Vec3 {
        let flat = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        self.target + (flat * self.pitch.cos() + Vec3::Y * self.pitch.sin()) * self.distance
    }
}

/// Build the snapshot from the loaded section packs.
///
/// # Panics
///
/// When the packs are missing or refused: the open world would refuse to arm
/// on the same catalog.
fn pin_snapshot(mut commands: Commands, loaded: Option<Res<LoadedSectionPacks>>) {
    let Some(loaded) = loaded else {
        panic!("generator_gallery: no LoadedSectionPacks; the content merge refused the catalog");
    };
    let packs: Vec<ShipPartPack> = loaded
        .packs
        .iter()
        .map(|pack| ShipPartPack {
            id: pack.id.clone(),
            dependencies: pack.dependencies.clone(),
            sections: pack.sections.clone(),
        })
        .collect();
    let snapshot = ShipPartSnapshot::build(&packs).unwrap_or_else(|faults| {
        panic!(
            "generator_gallery: the loaded parts are refused:\n  {}",
            faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    });
    commands.insert_resource(Snapshot(snapshot));
}

/// Generate the row `controls` asks for, lint every design and stand it,
/// whenever the controls differ from the row on stage.
#[expect(clippy::too_many_arguments, reason = "one system stands the whole row")]
fn load_row(
    mut commands: Commands,
    controls: Res<Controls>,
    row: Option<Res<Row>>,
    snapshot: Res<Snapshot>,
    sections: Res<GameSections>,
    items: Res<GameItems>,
    game_assets: Res<GameAssets>,
    mut orbit: ResMut<Orbit>,
    mut generation: Local<u32>,
) {
    if row.is_some_and(|row| row.controls == *controls) {
        return;
    }
    *generation += 1;
    let row = generate_row(&controls, *generation, &snapshot.0, &sections, &items);
    for failure in &row.failures {
        error!("generator_gallery: {failure}");
    }
    let clearances: Vec<f32> = row
        .hulls
        .iter()
        .map(|hull| hull.clearance.to_engine())
        .collect();
    let positions = stand_positions(&clearances);
    let reach = positions
        .iter()
        .zip(&clearances)
        .map(|(position, clearance)| position.length() + clearance)
        .fold(Meters(50.0).to_engine(), f32::max);
    orbit.target = Vec3::ZERO;
    orbit.distance = reach * 1.7;

    let actions = row
        .hulls
        .iter()
        .zip(positions)
        .map(|(hull, position)| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: hull.name.clone(),
                    name: hull.name.clone(),
                    position: Meters3::from_engine(position),
                    rotation: Quat::from_rotation_y(HULL_YAW),
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::None,
                    allegiance: Some(Allegiance::Neutral),
                    design: ShipDesignSource::Inline(hull.design.clone()),
                    ..default()
                }),
            })
        })
        .chain(ThreePointRig::around("gallery", Meters3::ZERO, reach.max(1.0) * 0.5).actions())
        .collect();
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The world's generated hulls in one row.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "generator_gallery".to_string(),
            "Generator Gallery".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
    commands.insert_resource(row);
}

/// Generate and lint every hull `controls` asks for. `generation` counts the
/// rows stood, so no hull id of this row names a hull of the last one.
fn generate_row(
    controls: &Controls,
    generation: u32,
    snapshot: &ShipPartSnapshot,
    sections: &GameSections,
    items: &GameItems,
) -> Row {
    let civilization = CivilizationId {
        world_seed: WORLD_SEED,
        node: [0, 0, 0],
    };
    let mut hulls = Vec::new();
    let mut failures = Vec::new();
    for (index, advancement) in controls.advancements().into_iter().enumerate() {
        let request = ShipLayoutRequest {
            seed: controls.seed,
            civilization,
            role: controls.role,
            advancement,
        };
        let layout = match controls.condition {
            ConditionType::Intact => generate_ship(snapshot, request),
            ConditionType::Wreck => generate_wreck(snapshot, request),
        };
        let layout = match layout {
            Ok(layout) => layout,
            Err(failure) => {
                failures.push(failure.to_string());
                continue;
            }
        };
        let mut design = layout.design;
        design.presentation.skin = controls.skin == SkinType::Clad;
        if let Some(style) = &controls.style {
            design.presentation.style = Some(style.clone());
        }
        let name = format!("hull_{generation}_{index}");
        let errors = hull_errors(&design, sections, items);
        if !errors.is_empty() {
            failures.push(format!(
                "{} at advancement {advancement:.2}, seed {}: the lint refuses it: {}",
                controls.role.label(),
                controls.seed,
                errors.join("; ")
            ));
            continue;
        }
        let count = |wanted: fn(&SectionKind) -> bool| {
            design
                .sections
                .iter()
                .filter(|section| match &section.source {
                    SectionSource::Prototype { id, .. } => sections
                        .get_section(id)
                        .is_some_and(|config| wanted(&config.kind)),
                    SectionSource::Inline(config) => wanted(&config.kind),
                })
                .count()
        };
        hulls.push(Hull {
            name,
            advancement,
            clearance: layout.clearance,
            sections: design.sections.len(),
            ports: count(|kind| matches!(kind, SectionKind::Docking(_))),
            turrets: count(|kind| matches!(kind, SectionKind::Turret(_))),
            computers: count(|kind| matches!(kind, SectionKind::Controller(_))),
            design,
        });
    }
    Row {
        controls: controls.clone(),
        hulls,
        failures,
    }
}

/// Where each hull of the row stands, in engine units: one line along X,
/// centred on the origin, every cell as wide as the widest clearance sphere.
fn stand_positions(clearances: &[f32]) -> Vec<Vec3> {
    let pitch = clearances.iter().copied().fold(0.0, f32::max) * 2.0 + STAND_GAP.to_engine();
    let count = clearances.len() as f32;
    (0..clearances.len())
        .map(|index| Vec3::new((index as f32 - (count - 1.0) * 0.5) * pitch, 0.0, 0.0))
        .collect()
}

/// The controls window. Edits land in [`Controls`] once no mouse button is
/// held, so a dragged slider regenerates the row once, on release.
fn controls_window(
    mut contexts: EguiContexts,
    mut controls: ResMut<Controls>,
    mut pending: Local<Option<Controls>>,
    row: Option<Res<Row>>,
    styles: Option<Res<GameStyles>>,
) -> Result {
    let context = contexts.ctx_mut()?;
    let mut edit = pending.take().unwrap_or_else(|| controls.clone());
    egui::Window::new("Generator")
        .default_pos(egui::pos2(900.0, 40.0))
        .default_width(320.0)
        .show(context, |ui| {
            ui.horizontal(|ui| {
                ui.label("Role");
                for role in ShipRoleType::ALL {
                    ui.selectable_value(&mut edit.role, role, role.label());
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut edit.condition, ConditionType::Intact, "Intact");
                ui.selectable_value(&mut edit.condition, ConditionType::Wreck, "Wreck");
            });
            ui.add(egui::Slider::new(&mut edit.count, 1..=MAX_ROW).text("hulls"));
            let from_label = if edit.count > 1 {
                "advancement from"
            } else {
                "advancement"
            };
            ui.add(egui::Slider::new(&mut edit.from, 0.0..=1.0).text(from_label));
            if edit.count > 1 {
                ui.add(egui::Slider::new(&mut edit.to, 0.0..=1.0).text("to"));
            }
            ui.horizontal(|ui| {
                ui.label("seed");
                ui.add(egui::DragValue::new(&mut edit.seed));
                if ui.button("Reroll").clicked() {
                    edit.seed = reroll(edit.seed);
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut edit.skin, SkinType::Bare, "Bare");
                ui.selectable_value(&mut edit.skin, SkinType::Clad, "Clad");
            });
            let shown = edit
                .style
                .clone()
                .unwrap_or_else(|| "role style".to_string());
            egui::ComboBox::from_label("style")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut edit.style, None, "role style");
                    for style in styles.iter().flat_map(|styles| styles.0.iter()) {
                        ui.selectable_value(
                            &mut edit.style,
                            Some(style.id.clone()),
                            style.id.as_str(),
                        );
                    }
                });
            if let Some(row) = &row {
                ui.separator();
                ui.label(format!("{} hulls stand", row.hulls.len()));
                for failure in &row.failures {
                    ui.colored_label(egui::Color32::from_rgb(230, 80, 70), failure);
                }
            }
        });
    if context.input(|input| input.pointer.any_down()) {
        *pending = Some(edit);
    } else if edit != *controls {
        *controls = edit;
    }
    Ok(())
}

/// The next seed after `seed`: one LCG step, so a reroll is repeatable.
fn reroll(seed: u32) -> u32 {
    seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
}

/// Orbit the scenario camera round the row: drag to turn, scroll to zoom.
/// The pose is pinned on the camera, so the free-fly rig never moves it.
fn orbit_camera(
    mut commands: Commands,
    mut orbit: ResMut<Orbit>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    egui_input: Option<Res<EguiWantsInput>>,
    q_camera: Query<(Entity, Option<&ScriptedCameraPose>), With<ScenarioCameraMarker>>,
) {
    let free = egui_input.is_none_or(|input| !input.wants_pointer_input());
    if free && (buttons.pressed(MouseButton::Left) || buttons.pressed(MouseButton::Right)) {
        orbit.yaw -= motion.delta.x * ORBIT_RADIANS_PER_PIXEL;
        orbit.pitch = (orbit.pitch + motion.delta.y * ORBIT_RADIANS_PER_PIXEL).clamp(-1.4, 1.4);
    }
    if free && scroll.delta.y != 0.0 {
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
        };
        orbit.distance *= ZOOM_PER_LINE.powf(lines);
    }
    let pose = ScriptedCameraPose {
        position: Meters3::from_engine(orbit.eye()),
        look_at: Meters3::from_engine(orbit.target),
    };
    for (camera, worn) in &q_camera {
        if worn.is_none_or(|worn| worn.position != pose.position || worn.look_at != pose.look_at) {
            commands
                .entity(camera)
                .remove::<WASDCameraController>()
                .insert(pose);
        }
    }
}

/// Label every standing hull and name the row in the panel.
fn label_hulls(
    row: Option<Res<Row>>,
    q_root: Query<(&EntityId, &GlobalTransform), With<SpaceshipRootMarker>>,
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
) {
    let Some(row) = row else {
        panel.line("generating the row");
        return;
    };
    let controls = &row.controls;
    panel.line(format!(
        "{} {} x{}, seed {}",
        controls.role.label(),
        match controls.condition {
            ConditionType::Intact => "intact",
            ConditionType::Wreck => "wreck",
        },
        controls.count,
        controls.seed
    ));
    if !row.failures.is_empty() {
        panel.line(format!("{} refused, see the window", row.failures.len()));
    }
    for (id, transform) in &q_root {
        let Some(hull) = row.hulls.iter().find(|hull| hull.name == id.0) else {
            continue;
        };
        let centre = transform.translation();
        let radius = hull.clearance.to_engine();
        diagnostics::sphere(&mut gizmos, centre, radius, palette::MUTED);
        panel.label(
            centre + Vec3::Y * radius,
            format!(
                "a {:.2}  r {:.0} m\n{} sections\n{} ports  {} turrets  {} computers",
                hull.advancement,
                hull.clearance.0,
                hull.sections,
                hull.ports,
                hull.turrets,
                hull.computers
            ),
        );
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// Seconds the row gets to generate, load and clad on a software GPU.
    const SETUP_SECS: f32 = 300.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the view gets before the shot.
    const SETTLE_FRAMES: u32 = 20;
    /// The PNG of the row.
    const SHOT: &str = "generator-gallery.png";

    /// Every hull of the row stands under its root.
    fn row_standing(world: &World) -> bool {
        let Some(row) = world.get_resource::<Row>() else {
            return false;
        };
        let Some(mut query) = world.try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
        else {
            return false;
        };
        let standing: Vec<&str> = query.iter(world).map(|id| id.0.as_str()).collect();
        row.hulls
            .iter()
            .all(|hull| standing.contains(&hull.name.as_str()))
    }

    /// The default row: five hulls across advancement 0 to 1, every one
    /// generated and passed by the lint.
    fn assert_the_row(world: &mut World) {
        let row = world.resource::<Row>();
        assert!(
            row.failures.is_empty(),
            "generator gallery: refused:\n  {}",
            row.failures.join("\n  ")
        );
        assert_eq!(
            row.hulls.len(),
            row.controls.count,
            "generator gallery: one hull per requested advancement"
        );
        let advancements: Vec<String> = row
            .hulls
            .iter()
            .map(|hull| format!("{:.2}", hull.advancement))
            .collect();
        info!(
            "generator gallery: PASS {} hulls at advancement {}",
            row.hulls.len(),
            advancements.join(", ")
        );
    }

    /// The smoke walk: the default row generates, lints and stands, and a
    /// frame with the diagnostics up is shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("generator gallery: stand the row")
            .until(and(Arc::new(row_standing), scenario_camera_present()))
            .deadline(SETUP_SECS)
            .add()
            .step("generator gallery: check the row")
            .on_enter(assert_the_row)
            .add()
            .step("generator gallery: settle the view")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("generator gallery: shoot the row")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("generator gallery: ask for the wrecks")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Controls>().condition = ConditionType::Wreck;
            })
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<Row>()
                    .is_some_and(|row| row.controls.condition == ConditionType::Wreck)
                    && row_standing(world)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("generator gallery: check the wrecks")
            .on_enter(assert_the_row)
            .add()
    }
}
