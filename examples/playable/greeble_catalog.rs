//! greeble_catalog: every fixture model the merged styles own, stood in named
//! rows grouped by style - the catalog a greeble is judged on as an OBJECT
//! before placement rules ever touch it (task 20260816-203846, spec in
//! tasks/20260816-194637/GREEBLES.md section 6).
//!
//! The roster is RESOLVED, never listed: the example reads the merged
//! `GameStyles` after load and lays out every `StyleFixtureConfig` it finds,
//! so a mod's fifth style - or a vocabulary batch's new pieces - appears with
//! no code change. Each piece stands on a one-cell pedestal PLATE tinted with
//! its style's `Top` surface colour and roughness, because a greeble is only
//! judgeable against the plate it will stand on. Labels carry the fixture id
//! plus the authored collider extents and health, and the load log prints one
//! line per fixture - id, model path, collider, health, rule summary - so
//! this is the one place a reviewer sees model and rule together.
//!
//! Hand-run (free-fly with WASD; the wall idles on a slow orbit until the rig
//! is touched, and resumes after six quiet seconds):
//! ```text
//! cargo run --example greeble_catalog --features debug
//! ```
//! - arrows move the selection ring; Enter focuses the piece large on a
//!   turntable; left/right step within the style; Esc returns to the wall
//! - `L` snaps the camera to the next style row
//! - `C` toggles the pedestals (piece against void)
//! - `G` toggles a unit-cell wireframe, so the half-cell footprint budget the
//!   greeble README documents becomes checkable by eye
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the wall, frame it, walk the rows,
//!   exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot `greeble-catalog.png` (the
//!   wall), one `greeble-catalog-<style>.png` per row, and a
//!   `greeble-catalog-focus.png` of the turntable. The row shots are driven
//!   off the LOADED style list rather than a scripted name list, so a mod
//!   style gets its row shot with no code change either.

use bevy::prelude::*;
use clap::Parser;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the idle orbit in EVERY build.
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "greeble_catalog")]
#[command(version = "1.0.0")]
#[command(about = "Every style's fixture models in named rows, parts-preview style", long_about = None)]
struct Cli;

/// Centre-to-centre spacing across a row. Pieces are at most one cell wide, so
/// the gap is label room rather than model room.
const COLUMN_SPACING: f32 = 1.7;
/// Centre-to-centre spacing between style rows, deep enough that a back row's
/// labels clear the row in front.
const ROW_SPACING: f32 = 2.8;
/// Every piece stands at the same quarter yaw, so the camera reads two flanks
/// of each at once and the fixed photo rig lights all of them the same way.
const SUBJECT_YAW: f32 = -0.55;
/// How far under a piece its nameplate hangs, in engine world units - a display
/// offset in Bevy space, not an authored distance. Pieces are under
/// a cell tall, so a shallow drop keeps the label tight to its subject.
const LABEL_DROP: f32 = 0.55;
/// Extra drop every other column, so a back row's ids do not run together on
/// the wall shot: the columns project too close for one shared baseline.
const LABEL_STAGGER: f32 = 0.45;
/// The pedestal plate's thickness. A plate, not a plinth: its top face is the
/// `y = 0` mounting plane a greeble is authored against.
const PEDESTAL_HEIGHT: f32 = 0.12;
/// A plate's roughness/metallic where the style names no Top surface -
/// `nova_ship`'s own `SKIN_ROUGHNESS` / `SKIN_METALLIC` defaults, mirrored.
const BARE_PLATE_ROUGHNESS: f32 = 0.65;
const BARE_PLATE_METALLIC: f32 = 0.15;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(catalog_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring, the shape_bench pattern: run timeline + engine-bound
        // invariants so `probe run` grades this example. No frame-time capture
        // - a posed wall holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.insert_resource(RowWalk::default());
        app.add_systems(Update, drive_row_walk);
        app.add_plugins(catalog_script());
    }

    app.run()
}

fn catalog_plugin(app: &mut App) {
    // Armed only for a hand-run: a capture composes its own frame.
    app.insert_resource(IdleOrbit::new(!capturing()));
    app.insert_resource(CatalogMode::Wall);
    app.insert_resource(Selected { row: 0, column: 0 });
    app.insert_resource(CameraAim::default());
    app.insert_resource(ShowPedestals(true));
    app.insert_resource(ShowCellFrames(false));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_catalog);
    app.add_systems(
        Update,
        (
            (keyboard, rebuild_focus, sync_pedestals, update_readout).chain(),
            frame_new_camera,
            place_labels,
            spin_focused,
            draw_selection,
            draw_cell_frames,
            expire_row_aim,
            track_orbit_idle,
        ),
    );
    // PostUpdate, after the rig's own write and before the transform
    // propagates: the free-fly rig syncs the camera in PostUpdate, so an
    // Update system ordered against that set is ordered against nothing and
    // loses every frame. The aim runs after the orbit so a snapped row or a
    // focused piece wins over the turntable.
    app.add_systems(
        PostUpdate,
        (orbit_idle_camera, aim_camera)
            .chain()
            .after(WASDCameraSystems::Sync)
            .before(TransformSystems::Propagate),
    );
}

/// One fixture as the wall shows it: the authored data the labels, the report
/// and the focus view all read from.
struct CatalogPiece {
    id: String,
    model: AssetRef<WorldAsset>,
    collider: Vec3,
    health: f32,
    /// Where the piece stands on the wall.
    stand: Vec3,
}

/// One style's row of the wall.
struct CatalogRow {
    style: String,
    /// The pedestal material this row's plates wear - the style's own `Top`
    /// surface, or the built-in bare plate where the style names none.
    material: Handle<StandardMaterial>,
    pieces: Vec<CatalogPiece>,
}

/// The resolved catalog, in merged-content order (authored looks first,
/// placeholder last - the order `GameStyles` already holds).
#[derive(Resource)]
struct Catalog {
    rows: Vec<CatalogRow>,
    /// One unit plate mesh shared by every pedestal.
    pedestal_mesh: Handle<Mesh>,
}

impl Catalog {
    fn piece(&self, row: usize, column: usize) -> Option<&CatalogPiece> {
        self.rows.get(row)?.pieces.get(column)
    }
}

/// Where one piece stands: rows along Z, each row centred on its own count.
fn stand_position(row: usize, rows: usize, column: usize, in_row: usize) -> Vec3 {
    Vec3::new(
        (column as f32 - (in_row as f32 - 1.0) * 0.5) * COLUMN_SPACING,
        0.0,
        (row as f32 - (rows as f32 - 1.0) * 0.5) * ROW_SPACING,
    )
}

/// A one-line reading of a fixture's scatter rule - the placement half of the
/// report, so the catalog shows model and rule together.
fn rule_summary(rule: &ScatterRule) -> String {
    let relief = if rule.relief.is_empty() {
        "any".to_string()
    } else {
        rule.relief
            .iter()
            .map(|relief| relief.name())
            .collect::<Vec<_>>()
            .join("+")
    };
    let near = rule
        .near_fitting
        .map(|near| format!(", near_fitting {near}"))
        .unwrap_or_default();
    format!(
        "relief {relief}, seat {:?}, align {:?}, chance {:.2}, stride {}, patch {}{near}",
        rule.seat, rule.align, rule.chance, rule.stride, rule.patch,
    )
}

/// The stage: the game's own sky and the repo's standard three-point rig, with
/// NO ships - every subject is a display entity this example owns.
fn catalog_stage(game_assets: &GameAssets) -> ScenarioConfig {
    ScenarioConfig {
        description: "Every style's fixture models in named rows".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ThreePointRig::around("photo", Meters3::ZERO, 3.0).actions(),
        }],
        ..ScenarioConfig::new(
            "greeble_catalog".to_string(),
            "Greeble Catalog".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Marks a pedestal plate, so `C` can strip them all.
#[derive(Component)]
struct Pedestal;

fn load_catalog(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    styles: Res<GameStyles>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(catalog_stage(&game_assets)));

    let pedestal_mesh = meshes.add(Cuboid::new(1.0, PEDESTAL_HEIGHT, 1.0));
    let rows_total = styles.len();
    let widest = styles
        .iter()
        .map(|style| style.fixtures.len())
        .max()
        .unwrap_or(1);
    let mut rows = Vec::new();
    for (row, style) in styles.iter().enumerate() {
        let top = style.surface(ShellSurface::Top);
        let material = materials.add(StandardMaterial {
            base_color: top.map_or(ShellSurface::Top.colour(), |dress| dress.color),
            perceptual_roughness: top.map_or(BARE_PLATE_ROUGHNESS, |dress| dress.roughness),
            metallic: top.map_or(BARE_PLATE_METALLIC, |dress| dress.metallic),
            ..default()
        });

        let in_row = style.fixtures.len();
        let mut pieces = Vec::new();
        for (column, fixture) in style.fixtures.iter().enumerate() {
            let stand = stand_position(row, rows_total, column, in_row);
            info!(
                "greeble_catalog: [{}] {}: model {}, collider {:.2} x {:.2} x {:.2}, \
                 health {:.0}, {}",
                style.id,
                fixture.id,
                fixture.model.path().unwrap_or("<handle>"),
                fixture.collider.x,
                fixture.collider.y,
                fixture.collider.z,
                fixture.health,
                rule_summary(&fixture.scatter),
            );
            spawn_stand(
                &mut commands,
                &asset_server,
                fixture,
                stand,
                &pedestal_mesh,
                &material,
            );
            let drop = LABEL_DROP + (column % 2) as f32 * LABEL_STAGGER;
            spawn_label(
                &mut commands,
                stand + Vec3::NEG_Y * drop,
                &fixture.id,
                &format!(
                    "{:.2} x {:.2} x {:.2}  hp {:.0}",
                    fixture.collider.x, fixture.collider.y, fixture.collider.z, fixture.health
                ),
            );
            pieces.push(CatalogPiece {
                id: fixture.id.clone(),
                model: fixture.model.clone(),
                collider: fixture.collider,
                health: fixture.health,
                stand,
            });
        }
        spawn_row_header(&mut commands, row, rows_total, style, in_row, widest);
        rows.push(CatalogRow {
            style: style.id.clone(),
            material,
            pieces,
        });
    }
    info!(
        "greeble_catalog: {} style(s), {} fixture(s) resolved from the merged content",
        rows.len(),
        rows.iter().map(|row| row.pieces.len()).sum::<usize>(),
    );

    commands.insert_resource(Catalog {
        rows,
        pedestal_mesh,
    });
    spawn_readout(&mut commands);
}

/// One wall stand: the pedestal plate and the fixture model on it, posed at
/// the shared presentation yaw.
fn spawn_stand(
    commands: &mut Commands,
    asset_server: &AssetServer,
    fixture: &StyleFixtureConfig,
    stand: Vec3,
    pedestal_mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
) {
    commands
        .spawn((
            Name::new(fixture.id.clone()),
            Transform::from_translation(stand).with_rotation(Quat::from_rotation_y(SUBJECT_YAW)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Top face at y = 0: the plate the piece is authored to stand on.
            parent.spawn((
                Pedestal,
                Mesh3d(pedestal_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::NEG_Y * (PEDESTAL_HEIGHT * 0.5)),
            ));
            // The model exactly as the skin spawns it: the authored glb in the
            // plate's own frame, +Y out, foot at y = 0.
            parent.spawn((
                WorldAssetRoot(fixture.model.resolve(asset_server)),
                Transform::IDENTITY,
                Visibility::default(),
            ));
        });
}

/// Which view is up: the wall of rows, or one piece large on the turntable.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum CatalogMode {
    Wall,
    Focused,
}

/// The piece the selection ring sits on - and the one Enter focuses.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct Selected {
    row: usize,
    column: usize,
}

/// Whether the pedestal plates show (`C`: piece against void).
#[derive(Resource)]
struct ShowPedestals(bool);

/// Whether the unit-cell wireframes show (`G`: the footprint budget by eye).
#[derive(Resource)]
struct ShowCellFrames(bool);

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    catalog: Option<Res<Catalog>>,
    mut mode: ResMut<CatalogMode>,
    mut selected: ResMut<Selected>,
    mut aim: ResMut<CameraAim>,
    mut pedestals: ResMut<ShowPedestals>,
    mut frames: ResMut<ShowCellFrames>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        pedestals.0 = !pedestals.0;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        frames.0 = !frames.0;
    }
    let Some(catalog) = catalog else { return };
    if catalog.rows.is_empty() {
        return;
    }

    if keys.just_pressed(KeyCode::KeyL) {
        let next = match aim.target {
            AimTarget::Row(row) => (row + 1) % catalog.rows.len(),
            _ => selected.row,
        };
        *aim = CameraAim::row(next);
    }

    match *mode {
        CatalogMode::Wall => {
            let mut row = selected.row as isize;
            let mut column = selected.column as isize;
            if keys.just_pressed(KeyCode::ArrowLeft) {
                column -= 1;
            }
            if keys.just_pressed(KeyCode::ArrowRight) {
                column += 1;
            }
            if keys.just_pressed(KeyCode::ArrowUp) {
                row -= 1;
            }
            if keys.just_pressed(KeyCode::ArrowDown) {
                row += 1;
            }
            let row = row.clamp(0, catalog.rows.len() as isize - 1) as usize;
            let in_row = catalog.rows[row].pieces.len().max(1);
            let column = column.clamp(0, in_row as isize - 1) as usize;
            let next = Selected { row, column };
            if *selected != next {
                *selected = next;
            }
            if keys.just_pressed(KeyCode::Enter) && !catalog.rows[row].pieces.is_empty() {
                *mode = CatalogMode::Focused;
                *aim = CameraAim::focus();
            }
        }
        CatalogMode::Focused => {
            let in_row = catalog.rows[selected.row].pieces.len().max(1) as isize;
            let mut column = selected.column as isize;
            if keys.just_pressed(KeyCode::ArrowLeft) {
                column -= 1;
            }
            if keys.just_pressed(KeyCode::ArrowRight) {
                column += 1;
            }
            let column = column.rem_euclid(in_row) as usize;
            if selected.column != column {
                selected.column = column;
            }
            if keys.just_pressed(KeyCode::Escape) {
                *mode = CatalogMode::Wall;
                // Free, not a snap back: the camera parks where it is and the
                // idle orbit resumes from that bearing after its quiet spell.
                *aim = CameraAim::default();
            }
        }
    }
}

/// Show or hide every pedestal to match the toggle - the focus pedestal too,
/// which is why this syncs instead of flipping on the key press.
fn sync_pedestals(
    pedestals: Res<ShowPedestals>,
    mut q_pedestal: Query<&mut Visibility, With<Pedestal>>,
) {
    let want = if pedestals.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut q_pedestal {
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Where the focused piece stands: well above the wall, so the turntable shot
/// reads against sky rather than against the rows.
const FOCUS_STAND: Vec3 = Vec3::new(0.0, 30.0, 0.0);
/// The presentation size the focused piece is scaled toward, in engine world
/// units: a mesh scale target, not an authored distance.
const FOCUS_FIT: f32 = 1.5;
/// Radians per second the focused piece turns at.
const FOCUS_SPIN_RATE: f32 = 0.6;

/// A spawned focus entity; cleared whenever the focus changes or closes.
#[derive(Component)]
struct FocusItem;

/// The focused piece's turntable.
#[derive(Component)]
struct FocusSpin;

/// How large the focused piece stands: fitted by its authored collider, which
/// is the size the GAME thinks the piece is - so a collider that badly
/// misfits its model is visible right here.
fn focus_scale(collider: Vec3) -> f32 {
    (FOCUS_FIT / collider.max_element().max(0.05)).clamp(1.0, 12.0)
}

/// Respawn the focus turntable whenever the focused piece changes; tear it
/// down when the mode returns to the wall.
fn rebuild_focus(
    mut commands: Commands,
    mode: Res<CatalogMode>,
    selected: Res<Selected>,
    catalog: Option<Res<Catalog>>,
    asset_server: Res<AssetServer>,
    pedestals: Res<ShowPedestals>,
    existing: Query<Entity, With<FocusItem>>,
    mut last: Local<Option<(CatalogMode, Selected)>>,
) {
    let Some(catalog) = catalog else { return };
    let now = (*mode, *selected);
    if *last == Some(now) {
        return;
    }
    *last = Some(now);
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if *mode != CatalogMode::Focused {
        return;
    }
    let Some(piece) = catalog.piece(selected.row, selected.column) else {
        return;
    };
    let row = &catalog.rows[selected.row];
    let scale = focus_scale(piece.collider);
    info!(
        "greeble_catalog: focus [{}] {} at x{scale:.1}, hp {:.0}",
        row.style, piece.id, piece.health
    );
    commands
        .spawn((
            FocusItem,
            FocusSpin,
            Name::new(format!("focus_{}", piece.id)),
            Transform::from_translation(FOCUS_STAND).with_scale(Vec3::splat(scale)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            let mut pedestal = parent.spawn((
                Pedestal,
                Mesh3d(catalog.pedestal_mesh.clone()),
                MeshMaterial3d(row.material.clone()),
                Transform::from_translation(Vec3::NEG_Y * (PEDESTAL_HEIGHT * 0.5)),
            ));
            // Born matching the toggle, so `C` off does not flash a plate in.
            if !pedestals.0 {
                pedestal.insert(Visibility::Hidden);
            }
            parent.spawn((
                WorldAssetRoot(piece.model.resolve(&asset_server)),
                Transform::IDENTITY,
                Visibility::default(),
            ));
        });
}

/// Turntable for the focused piece.
fn spin_focused(time: Res<Time>, mut spinners: Query<&mut Transform, With<FocusSpin>>) {
    for mut transform in &mut spinners {
        transform.rotate_y(FOCUS_SPIN_RATE * time.delta_secs());
    }
}

/// A flat gold frame around the selected stand. Hand-run chrome only: a
/// capture composes clean frames.
fn draw_selection(
    mut gizmos: Gizmos,
    mode: Res<CatalogMode>,
    selected: Res<Selected>,
    catalog: Option<Res<Catalog>>,
) {
    if capturing() || *mode != CatalogMode::Wall {
        return;
    }
    let Some(piece) = catalog
        .as_ref()
        .and_then(|catalog| catalog.piece(selected.row, selected.column))
    else {
        return;
    };
    gizmos.cube(
        Transform::from_translation(piece.stand)
            .with_rotation(Quat::from_rotation_y(SUBJECT_YAW))
            .with_scale(Vec3::new(1.2, 0.02, 1.2)),
        Color::srgb(1.0, 0.85, 0.3),
    );
}

/// The unit cell over every stand (`G`): one wireframe cube per plate, so the
/// half-cell footprint budget is judged by eye instead of by trust.
fn draw_cell_frames(
    mut gizmos: Gizmos,
    frames: Res<ShowCellFrames>,
    mode: Res<CatalogMode>,
    selected: Res<Selected>,
    catalog: Option<Res<Catalog>>,
) {
    if capturing() || !frames.0 {
        return;
    }
    let Some(catalog) = catalog else { return };
    let grey = Color::srgba(0.55, 0.65, 0.7, 0.6);
    for row in &catalog.rows {
        for piece in &row.pieces {
            gizmos.cube(
                Transform::from_translation(piece.stand + Vec3::Y * 0.5)
                    .with_rotation(Quat::from_rotation_y(SUBJECT_YAW)),
                grey,
            );
        }
    }
    if *mode == CatalogMode::Focused {
        if let Some(piece) = catalog.piece(selected.row, selected.column) {
            let scale = focus_scale(piece.collider);
            gizmos.cube(
                Transform::from_translation(FOCUS_STAND + Vec3::Y * (scale * 0.5))
                    .with_scale(Vec3::splat(scale)),
                grey,
            );
        }
    }
}

/// A nameplate anchored to a world position.
#[derive(Component)]
struct SubjectLabel(Vec3);

/// The width a label centres its text in, in logical pixels.
const LABEL_WIDTH: f32 = 170.0;

fn spawn_label(commands: &mut Commands, anchor: Vec3, id: &str, note: &str) {
    commands
        .spawn((
            SubjectLabel(anchor),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-1000.0),
                top: Val::Px(-1000.0),
                width: Val::Px(LABEL_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            for (text, size, colour) in [
                (id, 12.0, Color::srgb(0.85, 0.9, 0.95)),
                (note, 9.0, Color::srgb(0.55, 0.65, 0.7)),
            ] {
                parent.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(colour),
                    // A back row's label lands on lit hull; the scrim keeps
                    // every name legible.
                    Node {
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(0.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ));
            }
        });
}

/// The style's name in a shared margin column at the wall's left edge -
/// aligned to the WIDEST row rather than its own, so no header ever lands in
/// a neighbouring row's label band.
fn spawn_row_header(
    commands: &mut Commands,
    row: usize,
    rows: usize,
    style: &ShipStyleConfig,
    in_row: usize,
    widest: usize,
) {
    let left = -((widest.max(1) as f32 - 1.0) * 0.5) * COLUMN_SPACING - 1.6;
    let anchor = Vec3::new(left, 0.35, stand_position(row, rows, 0, 1).z);
    spawn_label(commands, anchor, &style.id, &format!("{} piece(s)", in_row));
}

/// Project each nameplate under its subject, whatever the camera is doing.
fn place_labels(
    camera: Query<(&Camera, &GlobalTransform), With<ScenarioCameraMarker>>,
    mut labels: Query<(&SubjectLabel, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    for (label, mut node) in &mut labels {
        match camera.world_to_viewport(camera_transform, label.0) {
            Ok(position) => {
                node.left = Val::Px(position.x - LABEL_WIDTH * 0.5);
                node.top = Val::Px(position.y);
            }
            Err(_) => {
                node.left = Val::Px(-1000.0);
            }
        }
    }
}

/// Marks the status readout.
#[derive(Component)]
struct StyleReadout;

fn spawn_readout(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                StyleReadout,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Written every frame rather than on change, the shape_bench reason: the
/// readout is spawned by a command in the same run as the first selection
/// change, so a change-gated write never runs again.
fn update_readout(
    mode: Res<CatalogMode>,
    selected: Res<Selected>,
    catalog: Option<Res<Catalog>>,
    mut q_readout: Query<&mut Text, With<StyleReadout>>,
) {
    let Some(catalog) = catalog else { return };
    let subject = catalog
        .piece(selected.row, selected.column)
        .map(|piece| format!("{} / {}", catalog.rows[selected.row].style, piece.id))
        .unwrap_or_else(|| "empty".to_string());
    let line = match *mode {
        CatalogMode::Wall => format!(
            "Greeble catalog - {subject} - [arrows] select  [Enter] focus  [L] row  \
             [C] pedestals  [G] cell"
        ),
        CatalogMode::Focused => {
            format!("Greeble catalog - FOCUS {subject} - [left/right] step  [Esc] wall")
        }
    };
    for mut text in &mut q_readout {
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// What the wall camera aims at: the middle of the stand.
const CAMERA_TARGET: Vec3 = Vec3::ZERO;

/// The wall's camera span, off the RESOLVED catalog: a sixth style deepens the
/// stand and the framing follows.
fn wall_span(catalog: &Catalog) -> f32 {
    let columns = catalog
        .rows
        .iter()
        .map(|row| row.pieces.len())
        .max()
        .unwrap_or(1) as f32
        + 1.0;
    let rows = catalog.rows.len() as f32 + 1.0;
    (columns * COLUMN_SPACING).max(rows * ROW_SPACING).max(8.0)
}

/// Where the wall camera stands: backed off far enough to hold the grid AND
/// the header margin beside the front row, high and in front so it reads the
/// pieces and their plates at once.
fn wall_camera_position(catalog: &Catalog) -> Vec3 {
    let span = wall_span(catalog);
    Vec3::new(0.0, span * 0.42, span * 0.80)
}

/// One row's framing: close enough that a half-cell piece and its label read.
fn row_view(catalog: &Catalog, row: usize) -> (Vec3, Vec3) {
    let rows = catalog.rows.len();
    let in_row = catalog.rows[row].pieces.len().max(1);
    let target = stand_position(row, rows, 0, 1);
    let width = ((in_row as f32 + 1.0) * COLUMN_SPACING).max(4.5);
    (target + Vec3::new(0.0, width * 0.45, width * 0.62), target)
}

/// The focus turntable's framing: slightly above the piece, sky behind it.
fn focus_view() -> (Vec3, Vec3) {
    (
        FOCUS_STAND + Vec3::new(0.0, 1.2, 3.4),
        FOCUS_STAND + Vec3::new(0.0, 0.55, 0.0),
    )
}

/// Frame every camera the loader spawns, so the catalog comes up composed
/// instead of on the loader's default perch.
fn frame_new_camera(
    catalog: Option<Res<Catalog>>,
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    let Some(catalog) = catalog else { return };
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(wall_camera_position(&catalog))
            .looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// What the camera is pinned on beyond the orbit: a snapped row or the focus
/// turntable. `Free` hands it back to the free-fly rig and the idle orbit.
#[derive(Clone, Copy, PartialEq)]
enum AimTarget {
    Free,
    Row(usize),
    Focus,
}

/// The pinned camera target and how long a row snap has held.
#[derive(Resource)]
struct CameraAim {
    target: AimTarget,
    held_secs: f32,
}

impl Default for CameraAim {
    fn default() -> Self {
        Self {
            target: AimTarget::Free,
            held_secs: 0.0,
        }
    }
}

impl CameraAim {
    fn row(row: usize) -> Self {
        Self {
            target: AimTarget::Row(row),
            held_secs: 0.0,
        }
    }

    fn focus() -> Self {
        Self {
            target: AimTarget::Focus,
            held_secs: 0.0,
        }
    }

    fn active(&self) -> bool {
        self.target != AimTarget::Free
    }
}

/// A row snap borrows the orbit's re-arm rhythm: hold the row for the quiet
/// spell, then hand the camera back so the wall goes on turning. Focus never
/// expires - Esc is its exit.
fn expire_row_aim(mut aim: ResMut<CameraAim>, time: Res<Time>) {
    if let AimTarget::Row(_) = aim.target {
        aim.held_secs += time.delta_secs();
        if aim.held_secs >= ORBIT_RESUME_SECS {
            *aim = CameraAim::default();
        }
    }
}

/// Pin the camera on the aim, every frame while one is active - after the
/// orbit in the same PostUpdate slot, so a snap or a focus wins the write.
fn aim_camera(
    aim: Res<CameraAim>,
    catalog: Option<Res<Catalog>>,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    let Some(catalog) = catalog else { return };
    let (position, target) = match aim.target {
        AimTarget::Free => return,
        AimTarget::Row(row) if row < catalog.rows.len() => row_view(&catalog, row),
        AimTarget::Row(_) => return,
        AimTarget::Focus => focus_view(),
    };
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(position).looking_at(target, Vec3::Y);
    }
}

/// Radians per second the idle orbit turns at.
const ORBIT_RATE: f32 = 0.25;

/// How much further out the orbit stands than the composed front-on framing,
/// so the corner pieces stay in frame on the broadside pass.
const ORBIT_STANDOFF: f32 = 1.35;

/// Seconds the free-fly rig must sit untouched before the orbit re-arms.
///
/// Six: long enough that a viewer pausing over a detail is not yanked away
/// the moment their hands leave the keys, short enough that a parked window
/// goes back to turning before it reads as frozen.
const ORBIT_RESUME_SECS: f32 = 6.0;

/// The idle orbit's state: whether it may ever run, how long the free-fly rig
/// has sat untouched, and the bearing the orbit stands at.
///
/// The angle is a PHASE that is stepped, not read off the clock: the clock
/// keeps counting while the viewer flies, so `elapsed * ORBIT_RATE` would
/// teleport a re-armed camera onto whatever bearing it had drifted to.
/// Holding the phase, and re-deriving it from the parked camera on each
/// re-arm, is what lets the orbit pick up from where the viewer left it.
#[derive(Resource)]
struct IdleOrbit {
    /// Never set under a capture: a capture composes its own frame, and an
    /// orbit under it would photograph a different attitude every run.
    enabled: bool,
    /// Seconds since the free-fly rig last reported input.
    idle_secs: f32,
    /// The orbit's current azimuth around [`CAMERA_TARGET`], in radians.
    angle: f32,
    /// Whether the orbit owned the camera last frame, so the first re-armed
    /// frame can read the parked bearing before the orbit writes over it.
    driving: bool,
}

impl IdleOrbit {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            // Born idle-for-long-enough, so a fresh hand-run orbits at once.
            idle_secs: ORBIT_RESUME_SECS,
            angle: 0.0,
            driving: false,
        }
    }
}

/// Hand back the camera the moment the free-fly rig is asked for anything,
/// and count the quiet seconds that re-arm the orbit once the flying stops.
///
/// Reads the rig's own input component rather than the keyboard, so it cannot
/// disagree with what actually moves the camera. An ACTIVE AIM counts as
/// touch: a snapped row or a focused piece is the viewer looking at something,
/// and the orbit yanking the camera off it would defeat both keys.
fn track_orbit_idle(
    mut orbit: ResMut<IdleOrbit>,
    time: Res<Time>,
    aim: Res<CameraAim>,
    q_input: Query<&WASDCameraInput>,
) {
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched || aim.active() {
        orbit.idle_secs = 0.0;
    } else {
        // Saturated at the threshold: the timer is a re-arm gate, not a
        // stopwatch, so there is nothing to count past it.
        orbit.idle_secs = (orbit.idle_secs + time.delta_secs()).min(ORBIT_RESUME_SECS);
    }
}

/// Turn the wall on a slow turntable while nobody is flying. The CAMERA
/// orbits rather than the subjects, which hold the composition the grid exists
/// for. Runs after the free-fly rig writes its transform, because that rig
/// writes every frame and would otherwise win.
///
/// On re-arm the azimuth is read off the parked camera's own xz offset, so
/// the orbit drifts on from wherever the viewer left it. Radius and height
/// SNAP back to the composed standoff - the one framing known to hold the
/// whole wall - rather than easing out from a camera flown in close.
fn orbit_idle_camera(
    mut orbit: ResMut<IdleOrbit>,
    time: Res<Time>,
    catalog: Option<Res<Catalog>>,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !orbit.enabled {
        return;
    }
    let Some(catalog) = catalog else { return };
    if orbit.idle_secs < ORBIT_RESUME_SECS {
        orbit.driving = false;
        return;
    }
    if !orbit.driving {
        let Some(parked) = q_camera.iter().next() else {
            return;
        };
        let offset = parked.translation - CAMERA_TARGET;
        orbit.angle = offset.x.atan2(offset.z);
        orbit.driving = true;
    }
    orbit.angle += time.delta_secs() * ORBIT_RATE;
    let stand = wall_camera_position(&catalog);
    let radius = Vec2::new(stand.x, stand.z).length() * ORBIT_STANDOFF;
    for mut transform in &mut q_camera {
        *transform = Transform::from_translation(Vec3::new(
            radius * orbit.angle.sin(),
            stand.y,
            radius * orbit.angle.cos(),
        ))
        .looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Ceiling on the whole row walk. Generous because the row count is only
/// known at runtime; a single stuck shot still fails fast on its own
/// [`SHOT_DEADLINE_SECS`] inside [`drive_row_walk`].
#[cfg(feature = "debug")]
const ROW_WALK_DEADLINE_SECS: f32 = 180.0;

/// The per-row capture walk's state. Script-armed; driven by
/// [`drive_row_walk`], which poses, settles, shoots and awaits one row at a
/// time - off the LOADED catalog, so a mod's row is walked too.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct RowWalk {
    active: bool,
    done: bool,
    row: usize,
    /// Settle frames left before this row's shot; `None` when not yet posed.
    settle: Option<u32>,
    /// The shot awaiting its ack, and the seconds it has been awaited.
    awaiting: Option<(String, f32)>,
}

#[cfg(feature = "debug")]
fn drive_row_walk(world: &mut World) {
    let (active, done) = {
        let walk = world.resource::<RowWalk>();
        (walk.active, walk.done)
    };
    if !active || done {
        return;
    }
    let rows = world.resource::<Catalog>().rows.len();
    let advance = |world: &mut World| {
        let mut walk = world.resource_mut::<RowWalk>();
        walk.row += 1;
        walk.settle = None;
        walk.awaiting = None;
        if walk.row >= rows {
            walk.done = true;
            info!("greeble_catalog: row walk complete ({rows} row(s))");
        }
    };

    if let Some((name, waited)) = world.resource::<RowWalk>().awaiting.clone() {
        let wrote = world
            .get_resource::<CaptureLog>()
            .is_some_and(|log| log.wrote(&name));
        if wrote {
            advance(world);
            return;
        }
        let waited = waited + world.resource::<Time>().delta_secs();
        assert!(
            waited < SHOT_DEADLINE_SECS,
            "greeble_catalog: shot {name} not written within {SHOT_DEADLINE_SECS}s"
        );
        world.resource_mut::<RowWalk>().awaiting = Some((name, waited));
        return;
    }

    let (row, settle) = {
        let walk = world.resource::<RowWalk>();
        (walk.row, walk.settle)
    };
    if row >= rows {
        world.resource_mut::<RowWalk>().done = true;
        return;
    }
    match settle {
        None => {
            let (position, target) = row_view(world.resource::<Catalog>(), row);
            pose_camera(
                world,
                Meters3::from_engine(position),
                Meters3::from_engine(target),
            );
            world.resource_mut::<RowWalk>().settle = Some(SETTLE_FRAMES);
        }
        Some(0) => {
            let style = world.resource::<Catalog>().rows[row].style.clone();
            let name = format!("greeble-catalog-{style}.png");
            shoot(world, &name);
            if capturing() {
                world.resource_mut::<RowWalk>().awaiting = Some((name, 0.0));
            } else {
                // The smoke path still walks and frames every row; only the
                // ack has nothing to wait for.
                advance(world);
            }
        }
        Some(frames_left) => {
            world.resource_mut::<RowWalk>().settle = Some(frames_left - 1);
        }
    }
}

/// Pose the harness camera on the whole wall.
#[cfg(feature = "debug")]
fn frame_wall(world: &mut World) {
    let position = wall_camera_position(world.resource::<Catalog>());
    pose_camera(
        world,
        Meters3::from_engine(position),
        Meters3::from_engine(CAMERA_TARGET),
    );
}

/// The driven walk: load the wall, frame it, shoot it, then walk the rows.
#[cfg(feature = "debug")]
fn catalog_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the catalog")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the wall")
        .on_enter(|world: &mut World| frame_wall(world))
        .until(frames(SETTLE_FRAMES * 2))
        .add()
        .step("shoot the wall")
        .on_enter(|world: &mut World| shoot(world, "greeble-catalog.png"))
        .until(shot_written("greeble-catalog.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("walk the style rows")
        .on_enter(|world: &mut World| world.resource_mut::<RowWalk>().active = true)
        .until(resource_where::<RowWalk>(|walk| walk.done))
        .deadline(ROW_WALK_DEADLINE_SECS)
        .add()
        // The parts_viewer idiom: drive the mode resource directly, so the
        // focus path - respawn, turntable, pedestal, framing - is walked and
        // graded on every harness run, not only by hand.
        .step("focus the selected piece")
        .on_enter(|world: &mut World| {
            *world.resource_mut::<CatalogMode>() = CatalogMode::Focused;
            *world.resource_mut::<CameraAim>() = CameraAim::focus();
            // The row walk left a ScriptedCameraPose pinned on the camera;
            // repin it on the turntable so the enforcer and the aim agree.
            let (position, target) = focus_view();
            pose_camera(
                world,
                Meters3::from_engine(position),
                Meters3::from_engine(target),
            );
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the focus")
        .on_enter(|world: &mut World| shoot(world, "greeble-catalog-focus.png"))
        .until(shot_written("greeble-catalog-focus.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
