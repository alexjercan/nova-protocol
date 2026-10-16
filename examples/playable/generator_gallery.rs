//! generator_gallery: the open world's ship and station generators, a row of
//! ships above a row of stations, driven from an egui window.
//!
//! The window picks what the rows show: ships, stations or both, the ship
//! role and the station archetype, intact or wrecked ships, how many of each
//! (1 to 7) across an advancement range, the seed, bare or clad, and an
//! optional style that replaces the role's or archetype's own. When a control
//! is released the rows are generated again through the world's own
//! `generate_ship`, `generate_wreck` and `generate_station`, every design is
//! run through the content lint (the station lint for a station), and the
//! scene is loaded again. A request the generator or the lint refuses shows
//! in red in the window and in the log; it does not stand. A station has no
//! wreck: Wreck changes the ships only.
//!
//! The snapshot is the one the open world arms with: built from the loaded
//! section packs. Every hull spawns inline with no controller, so the skin
//! and greebles are the real ones.
//!
//! Camera: drag with the left or right mouse button to orbit the rows, scroll
//! to zoom, `F` to fit both rows in view again. A new row fits itself. Click a
//! hull to orbit it alone, fitted to its clearance sphere; `F` goes back to
//! the rows. A new row keeps the focus on the hull in the same slot, and drops
//! it when that slot is gone.
//!
//! ```text
//! nix develop --command cargo run --example generator_gallery --features debug
//! ```
//!
//! The diagnostics (`F3`) draw each hull's clearance radius as one ring
//! square to the view and label it, above the ring, with its advancement, its
//! clearance radius, and its section, docking port, turret and flight
//! computer counts. The panel names the rows.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk stands five ships and five ring
//! stations across advancement 0 to 1, checks each one was generated and
//! passed the lint, shoots `generator-gallery.png` with `NOVA_CAPTURE=1`,
//! clicks the last ship through the click's own pick and shoots
//! `generator-gallery-focus.png`, asks for the ships as wrecks, checks them the
//! same way and that the focus stayed on the last ship, goes back to the rows,
//! then shoots the spire and the yard rows. Unset, no walk and no deadline
//! run.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
    window::PrimaryWindow,
};
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{
    nova_debug::{
        bevy_egui::{egui, input::EguiWantsInput, EguiContexts, EguiPrimaryContextPass},
        diagnostics,
    },
    prelude::*,
};
use nova_scenario::prelude::{lint_station_design, KnownSections, LintSeverity};
use nova_wfc::prelude::hull_errors;
use nova_world::prelude::CivilizationId;

/// The world seed of the pinned civilization every request names. Only a
/// failure line reads it.
const WORLD_SEED: u32 = 20_260_922;

/// The most hulls one row stands.
const MAX_ROW: usize = 7;

/// Clear space between the clearance spheres of two neighbouring hulls, and
/// between the two rows, in meters.
const STAND_GAP: Meters = Meters(40.0);

/// The space between the rows, as a share of the column pitch: room for the
/// station labels between them.
const ROW_GAP_SHARE: f32 = 0.4;

/// How much room a fit leaves round the rows, as a share of the distance
/// that would just hold them: room for the labels above the rings.
const FIT_MARGIN: f32 = 1.25;

/// The pixels a label takes above its ring: four lines and a gap.
const LABEL_LIFT_PX: f32 = 72.0;

/// The pixels a label is moved left to centre it on its hull.
const LABEL_CENTRE_PX: f32 = 70.0;

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

/// The most pixels the mouse may travel between a left press and its release
/// for the release to count as a click, not an orbit drag.
const CLICK_SLOP_PX: f32 = 4.0;

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
        "the world's generated hulls: click a hull to orbit it, F for all",
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

/// Which rows stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KindType {
    Ship,
    Station,
    Both,
}

impl KindType {
    fn ships(self) -> bool {
        self != Self::Station
    }

    fn stations(self) -> bool {
        self != Self::Ship
    }
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
    kind: KindType,
    role: ShipRoleType,
    archetype: StationArchetypeType,
    /// Whether the ships stand whole or wrecked. Stations stand whole.
    condition: ConditionType,
    /// How many hulls each row stands, 1 to [`MAX_ROW`].
    count: usize,
    /// The advancement of the first hull.
    from: f32,
    /// The advancement of the last hull. A row of one reads `from` only.
    to: f32,
    seed: u32,
    skin: SkinType,
    /// The style every hull wears instead of its role's or archetype's, by
    /// id.
    style: Option<String>,
}

impl Default for Controls {
    /// Five industrial ships above five ring stations across the whole
    /// advancement range, clad in their own styles: the rows the smoke walk
    /// checks.
    fn default() -> Self {
        Self {
            kind: KindType::Both,
            role: ShipRoleType::Industrial,
            archetype: StationArchetypeType::Ring,
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

/// The rows on stage: the controls they were built from, the ships and the
/// stations that stand, and every request the generator or the lint
/// refused.
#[derive(Resource, Clone, Debug)]
struct Row {
    controls: Controls,
    ships: Vec<Hull>,
    stations: Vec<Hull>,
    failures: Vec<String>,
}

impl Row {
    fn hulls(&self) -> impl Iterator<Item = &Hull> {
        self.ships.iter().chain(&self.stations)
    }
}

/// The orbit camera: a point it looks at, the yaw, pitch and distance it
/// stands at from it, and the spheres a fit holds in view.
#[derive(Resource)]
struct Orbit {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    /// Every standing hull's centre and clearance radius, in engine units:
    /// the ships first, then the stations.
    stands: Vec<(Vec3, f32)>,
    /// How many of `stands` are ships.
    ships: usize,
    /// The stand a fit holds alone, by index into `stands`. `None` fits
    /// every stand.
    focus: Option<usize>,
    /// Fit the stands in view on the next frame.
    fit: bool,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.35,
            distance: 100.0,
            stands: Vec::new(),
            ships: 0,
            focus: None,
            fit: false,
        }
    }
}

impl Orbit {
    /// The unit vector from the target to the eye.
    fn back(&self) -> Vec3 {
        let flat = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        flat * self.pitch.cos() + Vec3::Y * self.pitch.sin()
    }

    fn eye(&self) -> Vec3 {
        self.target + self.back() * self.distance
    }

    /// Look at the centre of the fitted stands' bounds from the distance that
    /// holds each of them inside a perspective view of vertical field of view
    /// `fov` and width over height `aspect`, with [`FIT_MARGIN`]. The fitted
    /// stands are the focused one, or every stand when none is focused.
    fn fit_to(&mut self, fov: f32, aspect: f32) {
        let spheres = match self.focus.and_then(|index| self.stands.get(index..=index)) {
            Some(focused) => focused,
            None => &self.stands,
        };
        let Some((low, high)) = spheres
            .iter()
            .map(|(centre, radius)| {
                (
                    *centre - Vec3::splat(*radius),
                    *centre + Vec3::splat(*radius),
                )
            })
            .reduce(|(low, high), (other_low, other_high)| {
                (low.min(other_low), high.max(other_high))
            })
        else {
            return;
        };
        self.target = (low + high) * 0.5;
        let back = self.back();
        let right = Vec3::Y.cross(back).normalize_or(Vec3::X);
        let up = back.cross(right);
        let half_tall = fov * 0.5;
        let half_wide = (half_tall.tan() * aspect).atan();
        // A sphere fits when its offset across the view plus its radius,
        // widened for the view's slant, stays inside the frustum at its depth.
        let mut distance: f32 = 1.0;
        for (centre, radius) in spheres {
            let offset = *centre - self.target;
            let nearer = offset.dot(back);
            for (across, half) in [(offset.dot(right), half_wide), (offset.dot(up), half_tall)] {
                let needed = (across.abs() + radius / half.cos()) / half.tan() + nearer;
                distance = distance.max(needed);
            }
        }
        self.distance = distance * FIT_MARGIN;
    }

    /// The stand whose clearance sphere `ray` enters first, by index.
    fn pick(&self, ray: Ray3d) -> Option<usize> {
        self.stands
            .iter()
            .enumerate()
            .filter_map(|(index, (centre, radius))| {
                let offset = *centre - ray.origin;
                let along = offset.dot(*ray.direction);
                let miss_squared = offset.length_squared() - along * along;
                let half_chord_squared = radius * radius - miss_squared;
                if half_chord_squared < 0.0 {
                    return None;
                }
                let exit = along + half_chord_squared.sqrt();
                (exit > 0.0).then_some((index, along - half_chord_squared.sqrt()))
            })
            .min_by(|(_, near), (_, other)| near.total_cmp(other))
            .map(|(index, _)| index)
    }

    /// Focus the stand under `cursor`, a point of `camera`'s viewport, and fit
    /// it on the next frame. A click on no stand keeps the focus. Returns the
    /// stand picked.
    fn click(&mut self, camera: &Camera, eye: &GlobalTransform, cursor: Vec2) -> Option<usize> {
        let picked = camera
            .viewport_to_world(eye, cursor)
            .ok()
            .and_then(|ray| self.pick(ray))?;
        self.focus = Some(picked);
        self.fit = true;
        Some(picked)
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
    let clearances = |hulls: &[Hull]| -> Vec<f32> {
        hulls
            .iter()
            .map(|hull| hull.clearance.to_engine())
            .collect()
    };
    let (ships, stations) = (clearances(&row.ships), clearances(&row.stations));
    // One column per advancement, the ship above the station. The rows are
    // set apart by a share of the column pitch, so the station labels clear
    // the ships at the zoom a fit gives.
    let widest = |radii: &[f32]| radii.iter().copied().fold(0.0, f32::max);
    let pitch = widest(&ships).max(widest(&stations)) * 2.0 + STAND_GAP.to_engine();
    let drop =
        widest(&ships) + widest(&stations) + STAND_GAP.to_engine().max(pitch * ROW_GAP_SHARE);
    let positions: Vec<Vec3> = stand_positions(ships.len(), pitch)
        .into_iter()
        .chain(
            stand_positions(stations.len(), pitch)
                .into_iter()
                .map(|position| position - Vec3::Y * drop),
        )
        .collect();
    orbit.stands = positions
        .iter()
        .copied()
        .zip(ships.iter().chain(&stations).copied())
        .collect();
    // The focus stays on the same slot, the ship or the station of its
    // column, while the new rows still stand one there.
    let old_ships = orbit.ships;
    orbit.focus = orbit.focus.and_then(|index| {
        if index < old_ships {
            (index < ships.len()).then_some(index)
        } else {
            let column = index - old_ships;
            (column < stations.len()).then_some(ships.len() + column)
        }
    });
    orbit.ships = ships.len();
    orbit.fit = true;
    let reach = orbit
        .stands
        .iter()
        .map(|(position, clearance)| position.length() + clearance)
        .fold(Meters(50.0).to_engine(), f32::max);

    let actions = row
        .hulls()
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
        description: "The world's generated ships and stations in two rows.".to_string(),
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

/// Generate and lint every ship and station `controls` asks for.
/// `generation` counts the rows stood, so no hull id of these rows names a
/// hull of the last ones.
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
    let count = |design: &ShipDesign, wanted: fn(&SectionKind) -> bool| {
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
    let hull = |name: String, advancement: f32, design: ShipDesign, clearance: Meters| Hull {
        name,
        advancement,
        clearance,
        sections: design.sections.len(),
        ports: count(&design, |kind| matches!(kind, SectionKind::Docking(_))),
        turrets: count(&design, |kind| matches!(kind, SectionKind::Turret(_))),
        computers: count(&design, |kind| matches!(kind, SectionKind::Controller(_))),
        design,
    };
    let known = KnownSections::from_configs(sections.iter());
    let mut ships = Vec::new();
    let mut stations = Vec::new();
    let mut failures = Vec::new();
    let advancements = controls.advancements();
    for (index, advancement) in advancements.iter().copied().enumerate() {
        if !controls.kind.ships() {
            break;
        }
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
        let name = format!("ship_{generation}_{index}");
        ships.push(hull(name, advancement, design, layout.clearance));
    }
    for (index, advancement) in advancements.iter().copied().enumerate() {
        if !controls.kind.stations() {
            break;
        }
        let request = StationLayoutRequest {
            seed: controls.seed,
            civilization,
            archetype: controls.archetype,
            advancement,
        };
        let layout = match generate_station(snapshot, request) {
            Ok(layout) => layout,
            Err(failure) => {
                failures.push(failure.to_string());
                continue;
            }
        };
        let keeper = Civilization {
            id: civilization,
            centroid: Meters3::ZERO,
            status: CivilizationStatusType::Living,
            advancement,
            role_preference: [1.0; 4],
            allegiance: Allegiance::Neutral,
        };
        let mut station = generated_station(snapshot, items, &layout, &keeper);
        station.hull.presentation.skin = controls.skin == SkinType::Clad;
        if let Some(style) = &controls.style {
            station.hull.presentation.style = Some(style.clone());
        }
        let name = format!("station_{generation}_{index}");
        let errors: Vec<String> =
            lint_station_design(&name, &station, &known, items, "generator_gallery")
                .into_iter()
                .filter(|issue| issue.severity == LintSeverity::Error)
                .map(|issue| issue.message)
                .collect();
        if !errors.is_empty() {
            failures.push(format!(
                "{} station at advancement {advancement:.2}, seed {}: the lint refuses it: {}",
                controls.archetype,
                controls.seed,
                errors.join("; ")
            ));
            continue;
        }
        stations.push(hull(name, advancement, station.hull, layout.clearance));
    }
    Row {
        controls: controls.clone(),
        ships,
        stations,
        failures,
    }
}

/// Where each of `count` hulls of a row stands, in engine units: one line
/// along X, centred on the origin, `pitch` apart.
fn stand_positions(count: usize, pitch: f32) -> Vec<Vec3> {
    let middle = (count as f32 - 1.0) * 0.5;
    (0..count)
        .map(|index| Vec3::new((index as f32 - middle) * pitch, 0.0, 0.0))
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
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -16.0))
        .default_width(320.0)
        .show(context, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut edit.kind, KindType::Ship, "Ship");
                ui.selectable_value(&mut edit.kind, KindType::Station, "Station");
                ui.selectable_value(&mut edit.kind, KindType::Both, "Both");
            });
            if edit.kind.ships() {
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
            }
            if edit.kind.stations() {
                ui.horizontal(|ui| {
                    ui.label("Archetype");
                    for archetype in StationArchetypeType::ALL {
                        ui.selectable_value(&mut edit.archetype, archetype, archetype.label());
                    }
                });
            }
            ui.add(egui::Slider::new(&mut edit.count, 1..=MAX_ROW).text("per row"));
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
                .unwrap_or_else(|| "own style".to_string());
            egui::ComboBox::from_label("style")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut edit.style, None, "own style");
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
                ui.label(format!(
                    "{} ships and {} stations stand",
                    row.ships.len(),
                    row.stations.len()
                ));
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

/// Orbit the scenario camera round the rows: drag to turn, scroll to zoom,
/// click a hull to orbit it alone, `F` to fit the rows in view again. A new
/// row fits itself. A press that starts over the egui window never picks. The
/// pose is pinned on the camera, so the free-fly rig never moves it.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reads every camera input"
)]
fn orbit_camera(
    mut commands: Commands,
    mut orbit: ResMut<Orbit>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    egui_input: Option<Res<EguiWantsInput>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<
        (
            Entity,
            &Camera,
            &GlobalTransform,
            Option<&ScriptedCameraPose>,
            Option<&Projection>,
        ),
        With<ScenarioCameraMarker>,
    >,
    mut press_travel: Local<Option<f32>>,
) {
    let typing = egui_input
        .as_ref()
        .is_some_and(|input| input.wants_keyboard_input());
    let free = egui_input
        .as_ref()
        .is_none_or(|input| !input.wants_pointer_input());
    // A left press that ends within the slop is a click; the travel counts
    // only presses that started off the egui window.
    if buttons.just_pressed(MouseButton::Left) {
        *press_travel = free.then_some(0.0);
    }
    if let Some(travel) = press_travel.as_mut() {
        *travel += motion.delta.length();
    }
    if buttons.just_released(MouseButton::Left)
        && press_travel
            .take()
            .is_some_and(|travel| travel <= CLICK_SLOP_PX)
    {
        let cursor = q_window.single().ok().and_then(Window::cursor_position);
        if let (Some(cursor), Some((_, camera, eye, _, _))) = (cursor, q_camera.iter().next()) {
            orbit.click(camera, eye, cursor);
        }
    }
    if !typing && keys.just_pressed(KeyCode::KeyF) {
        orbit.focus = None;
        orbit.fit = true;
    }
    if orbit.fit {
        let view = q_camera
            .iter()
            .find_map(|(_, _, _, _, projection)| match projection {
                Some(Projection::Perspective(perspective)) => {
                    Some((perspective.fov, perspective.aspect_ratio))
                }
                _ => None,
            });
        if let Some((fov, aspect)) = view {
            orbit.fit_to(fov, aspect);
            orbit.fit = false;
        }
    }
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
    for (camera, _, _, worn, _) in &q_camera {
        if worn.is_none_or(|worn| worn.position != pose.position || worn.look_at != pose.look_at) {
            commands
                .entity(camera)
                .remove::<WASDCameraController>()
                .insert(pose);
        }
    }
}

/// Ring every standing hull square to the view, label it above its ring,
/// and name the rows in the panel.
fn label_hulls(
    row: Option<Res<Row>>,
    q_root: Query<(&EntityId, &GlobalTransform), With<SpaceshipRootMarker>>,
    q_camera: Query<(&Camera, &GlobalTransform, &Projection), With<ScenarioCameraMarker>>,
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
) {
    let Some(row) = row else {
        panel.line("generating the rows");
        return;
    };
    let controls = &row.controls;
    if controls.kind.ships() {
        panel.line(format!(
            "ships: {} {} x{}, seed {}",
            controls.role.label(),
            match controls.condition {
                ConditionType::Intact => "intact",
                ConditionType::Wreck => "wreck",
            },
            controls.count,
            controls.seed
        ));
    }
    if controls.kind.stations() {
        panel.line(format!(
            "stations: {} x{}, seed {}",
            controls.archetype, controls.count, controls.seed
        ));
    }
    if !row.failures.is_empty() {
        panel.line(format!("{} refused, see the window", row.failures.len()));
    }
    let Some((camera, eye, projection)) = q_camera.iter().next() else {
        return;
    };
    let Projection::Perspective(perspective) = projection else {
        return;
    };
    let tall_px = camera
        .logical_viewport_size()
        .map_or(1080.0, |size| size.y.max(1.0));
    let (up, right) = (eye.up().as_vec3(), eye.right().as_vec3());
    for (id, transform) in &q_root {
        let Some(hull) = row.hulls().find(|hull| hull.name == id.0) else {
            continue;
        };
        let centre = transform.translation();
        let radius = hull.clearance.to_engine();
        let toward = centre - eye.translation();
        diagnostics::ring(&mut gizmos, centre, toward, radius, palette::MUTED);
        // A label hangs down and right from its anchor: lift it clear of the
        // ring and move it left to centre it.
        let per_px = 2.0 * toward.length() * (perspective.fov * 0.5).tan() / tall_px;
        panel.label(
            centre + up * (radius + LABEL_LIFT_PX * per_px) - right * LABEL_CENTRE_PX * per_px,
            format!(
                "a {:.2}  r {:.0} m\n{} sections\n{} ports  {} turrets\n{} computers",
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
    /// The PNG of the ships above the ring stations.
    const SHOT: &str = "generator-gallery.png";
    /// The PNG of the last ship alone, after a click on it.
    const FOCUS_SHOT: &str = "generator-gallery-focus.png";
    /// The PNG of the wrecks above the spire stations.
    const SPIRE_SHOT: &str = "generator-gallery-spire.png";
    /// The PNG of the wrecks above the yard stations.
    const YARD_SHOT: &str = "generator-gallery-yard.png";

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
        row.hulls()
            .all(|hull| standing.contains(&hull.name.as_str()))
    }

    /// The rows the controls ask for stand: `condition` ships and `archetype`
    /// stations.
    fn rows_standing(
        condition: ConditionType,
        archetype: StationArchetypeType,
    ) -> Arc<dyn Fn(&World) -> bool + Send + Sync> {
        Arc::new(move |world: &World| {
            world.get_resource::<Row>().is_some_and(|row| {
                row.controls.condition == condition && row.controls.archetype == archetype
            }) && row_standing(world)
        })
    }

    /// Click the last ship where the camera shows its centre, through the
    /// click's own pick, and check the pick names it.
    fn click_last_ship(world: &mut World) {
        let last = world.resource::<Orbit>().ships - 1;
        let centre = world.resource::<Orbit>().stands[last].0;
        let mut q_camera =
            world.query_filtered::<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>();
        let (camera, eye) = q_camera
            .iter(world)
            .next()
            .expect("generator gallery: no scenario camera to click through");
        let (camera, eye) = (camera.clone(), *eye);
        let cursor = camera
            .world_to_viewport(&eye, centre)
            .expect("generator gallery: the last ship is off screen");
        let picked = world.resource_mut::<Orbit>().click(&camera, &eye, cursor);
        assert_eq!(
            picked,
            Some(last),
            "generator gallery: a click on the last ship at {cursor} picks it"
        );
        info!("generator gallery: PASS the click at {cursor} focuses ship {last}");
    }

    /// The focus stayed on the last ship through the new rows.
    fn assert_focus_kept(world: &mut World) {
        let orbit = world.resource::<Orbit>();
        assert_eq!(
            orbit.focus,
            Some(orbit.ships - 1),
            "generator gallery: the new rows keep the focus on the last ship"
        );
        info!("generator gallery: PASS the wrecks keep the focus on the last ship");
    }

    /// Fit the rows again, as `F` does.
    fn back_to_rows(world: &mut World) {
        let mut orbit = world.resource_mut::<Orbit>();
        orbit.focus = None;
        orbit.fit = true;
    }

    /// Both rows: five ships and five stations across advancement 0 to 1,
    /// every one generated and passed by the lint.
    fn assert_the_rows(world: &mut World) {
        let row = world.resource::<Row>();
        assert!(
            row.failures.is_empty(),
            "generator gallery: refused:\n  {}",
            row.failures.join("\n  ")
        );
        for (what, hulls) in [("ships", &row.ships), ("stations", &row.stations)] {
            assert_eq!(
                hulls.len(),
                row.controls.count,
                "generator gallery: one of the {what} per requested advancement"
            );
            let advancements: Vec<String> = hulls
                .iter()
                .map(|hull| format!("{:.2} ({:.0} m)", hull.advancement, hull.clearance.0))
                .collect();
            info!(
                "generator gallery: PASS {} {what} at advancement {}",
                hulls.len(),
                advancements.join(", ")
            );
        }
    }

    /// The smoke walk: the default rows generate, lint and stand, and a frame
    /// with the diagnostics up is shot; then the wrecks, the spires and the
    /// yards the same way.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("generator gallery: stand the rows")
            .until(and(Arc::new(row_standing), scenario_camera_present()))
            .deadline(SETUP_SECS)
            .add()
            .step("generator gallery: check the rows")
            .on_enter(assert_the_rows)
            .add()
            .step("generator gallery: settle the view")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("generator gallery: shoot the row")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("generator gallery: click the last ship")
            .on_enter(click_last_ship)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("generator gallery: shoot the last ship")
            .on_enter(|world: &mut World| shoot(world, FOCUS_SHOT))
            .until(shot_written(FOCUS_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("generator gallery: ask for the wrecks")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Controls>().condition = ConditionType::Wreck;
            })
            .until(rows_standing(
                ConditionType::Wreck,
                StationArchetypeType::Ring,
            ))
            .deadline(SETUP_SECS)
            .add()
            .step("generator gallery: check the wrecks")
            .on_enter(|world: &mut World| {
                assert_the_rows(world);
                assert_focus_kept(world);
                back_to_rows(world);
            })
            .add()
            .step("generator gallery: ask for the spires")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Controls>().archetype = StationArchetypeType::Spire;
            })
            .until(rows_standing(
                ConditionType::Wreck,
                StationArchetypeType::Spire,
            ))
            .deadline(SETUP_SECS)
            .add()
            .step("generator gallery: check the spires")
            .on_enter(assert_the_rows)
            .add()
            .step("generator gallery: settle the spires")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("generator gallery: shoot the spires")
            .on_enter(|world: &mut World| shoot(world, SPIRE_SHOT))
            .until(shot_written(SPIRE_SHOT))
            .deadline(SHOT_SECS)
            .add()
            .step("generator gallery: ask for the yards")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Controls>().archetype = StationArchetypeType::Yard;
            })
            .until(rows_standing(
                ConditionType::Wreck,
                StationArchetypeType::Yard,
            ))
            .deadline(SETUP_SECS)
            .add()
            .step("generator gallery: check the yards")
            .on_enter(assert_the_rows)
            .add()
            .step("generator gallery: settle the yards")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("generator gallery: shoot the yards")
            .on_enter(|world: &mut World| shoot(world, YARD_SHOT))
            .until(shot_written(YARD_SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
