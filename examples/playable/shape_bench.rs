//! shape_bench: a fixed roster of HAND-PLACED structures, clad, named and
//! reported - the bench a skin rule is judged on.
//!
//! `wfc_ships` shows whatever hulls the seed gives, and the editor needs every
//! shape rebuilt by hand. Neither lets a person flip through twenty deliberate
//! cases, and the case that matters most is the small hand-built one: the
//! owner's L measured 0% coplanar against 58.6% on the generated row, so the
//! generated row UNDERSTATES every shape defect a player will build. This
//! bench holds the deliberate cases still, dresses them in a style, and prints
//! the skin report next to the picture so the two are judged together.
//!
//! The subjects are STRUCTURE only, exactly as in `wfc_ships`: nothing here
//! places a plate. Each ship asks for cladding with one flag and the game
//! derives the whole skin from the sections at spawn (`nova_ship`'s
//! `shell_skin`), so the render and the report are both evidence about the
//! derivation rather than about this file.
//!
//! The report answers the two questions task 20260816-112429 closed into this
//! bench, per subject and in one run:
//!
//! 1. CREASES: the count of creased plates by relief class (`Spur`, `Step`,
//!    ...), so a render of the worst subjects - the small hand-built shapes
//!    where every corner falls - can be judged next to its numbers.
//! 2. STYLES: for EVERY authored style, the decoration count and the
//!    `near_fitting` taken-of-reach, read off the fitting-bearing subject and
//!    the rest of the roster. The scatter is hashed, not rolled, so the
//!    numbers for a style the row is not wearing are the numbers it would
//!    wear.
//!
//! Hand-run (free-fly with WASD, `L` cycles the look in place, `C` strips or
//! restores the cladding on the same subjects; grave/tilde cycles the game
//! HUD and the style readout follows it, capture frames exempt):
//! ```text
//! cargo run --example shape_bench --features debug
//! cargo run --example shape_bench --features debug -- --style salvage
//! cargo run --example shape_bench --features debug -- --bare
//! ```
//!
//! Two harnessed modes, the fleet's capture idiom:
//! - `NOVA_AUTOPILOT=1`: smoke path - load the roster, frame it, exit clean.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also shoot the clad roster (staged
//!   under `NOVA_CAPTURE_DIR`). `freeze_bodies` pins every subject, so two runs
//!   photograph the same attitude and a pair of shots is an A/B of the skin
//!   rather than of the clock.

use std::collections::HashSet;

use bevy::prelude::*;
use clap::Parser;
// Direct, not through `nova_protocol::nova_debug`: that path only exists under
// the `debug` feature, and `capturing()` gates the idle orbit and the readout
// in EVERY build.
use nova_debug::prelude::capturing;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "shape_bench")]
#[command(version = "1.0.0")]
#[command(about = "A fixed roster of hand-placed structures, clad and reported", long_about = None)]
struct Cli {
    /// Strip the cladding and show the bare structure.
    #[arg(long)]
    bare: bool,
    /// Start on this style id instead of the first the content offers. `L`
    /// cycles from wherever this leaves the roster.
    #[arg(long)]
    style: Option<String>,
}

/// Subjects per row of the stand. Five small subjects fill a 16:9 frame at a
/// size where a single plate is still legible.
const COLUMNS: usize = 5;
/// Centre-to-centre spacing across a row, a little wider than the widest
/// subject (five cells) plus a plate on each flank.
const COLUMN_SPACING: f32 = 7.0;
/// Centre-to-centre spacing between rows, sized against subject LENGTH the
/// same way.
const ROW_SPACING: f32 = 8.0;
/// Every subject stands at the same quarter yaw, so the camera reads flank and
/// top at once and the fixed photo rig lights all of them the same way.
const SUBJECT_YAW: f32 = -0.5;
/// How far under a subject's stand its name hangs, in world units. Deep
/// enough to clear the tallest subject AND the fitted hull's drive plume,
/// which renders in front of a closer label.
const LABEL_DROP: f32 = 3.4;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let roster = Roster {
        clad: !cli.bare,
        style: 0,
    };
    let requested = cli.style.clone();
    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            bench_plugin(app, roster, StyleRequest(requested.clone()))
        })
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring, the wfc_ships pattern: run timeline + engine-bound
        // invariants so `probe run` grades this example. No frame-time capture
        // - a posed roster holds no steady-state load worth measuring.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // The HUD drops to cinematic only under capture, as in wfc_ships: a
        // hand-run keeps the level On so grave/tilde round-trips the readout
        // with the rest of the HUD.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        if capturing() {
            app.add_systems(Startup, hide_hud);
        }
        // A subject is a dynamic body, and in zero-g nothing damps a spawn
        // impulse: without this the roster drifts and turns at a rate set by
        // machine load, which is the exact defect this bench exists to close.
        // Ungated, as in wfc_ships: nothing flies these subjects, so a viewer
        // wants them still for the same reason a capture does.
        app.add_systems(Update, freeze_bodies);
        app.add_plugins(bench_script());
    }

    app.run()
}

fn bench_plugin(app: &mut App, roster: Roster, requested: StyleRequest) {
    app.insert_resource(roster);
    app.insert_resource(requested);
    // Enabled only for a hand-run: a capture composes its own frame.
    app.insert_resource(IdleOrbit::new(!capturing()));
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_roster);
    app.add_systems(
        Update,
        (
            restyle_on_key.run_if(in_state(GameStates::Playing)),
            report_skins.run_if(in_state(GameStates::Playing)),
            frame_new_camera,
            update_readout,
            place_labels,
            track_orbit_idle,
        ),
    );
    // PostUpdate, after the rig's own write and before the transform
    // propagates: the free-fly rig syncs the camera in PostUpdate, so an
    // Update system ordered against that set is ordered against nothing and
    // loses every frame.
    app.add_systems(
        PostUpdate,
        orbit_idle_camera
            .after(WASDCameraSystems::Sync)
            .before(TransformSystems::Propagate),
    );
}

/// What the stand shows: whether the subjects wear their skin, and WHICH look
/// they wear - an index into the merged catalog, exactly as in `wfc_ships`, so
/// this producer never has to know what a style is called.
#[derive(Resource, Clone, Copy)]
struct Roster {
    clad: bool,
    style: usize,
}

/// The style id `--style` asked for, before the content it names has loaded.
#[derive(Resource)]
struct StyleRequest(Option<String>);

/// The id of the style the clad subjects wear, read off the merged content.
type StyleId<'a> = Option<&'a str>;

/// The style at this index of the merged catalog, wrapped, so `L` is a plain
/// increment and a mod's fifth look joins the rotation with nothing changing.
fn style_at(styles: &GameStyles, index: usize) -> StyleId<'_> {
    if styles.is_empty() {
        return None;
    }
    styles
        .get(index % styles.len())
        .map(|style| style.id.as_str())
}

/// One fitting bolted onto a subject's hull: a catalog prototype at a cell,
/// turned so its single mounting socket lands on the hull face it bolts to.
struct Fitting {
    prototype: &'static str,
    cell: IVec3,
    rotation: Quat,
    /// Where inside its cell the part sits so its socket lands exactly on the
    /// cell face - zero for a part that fills its cell, a quarter cell along
    /// the mounting axis for the half-size PDC mount.
    offset: Vec3,
}

/// One named case: hull cubes plus whatever fittings the case is about.
struct Subject {
    id: &'static str,
    cells: Vec<IVec3>,
    fittings: Vec<Fitting>,
}

impl Subject {
    fn hull(id: &'static str, cells: Vec<IVec3>) -> Self {
        Self {
            id,
            cells,
            fittings: Vec::new(),
        }
    }
}

/// A solid block of hull cells, `size` cells along each axis from the origin.
fn block(size: IVec3) -> Vec<IVec3> {
    (0..size.x)
        .flat_map(|x| (0..size.y).flat_map(move |y| (0..size.z).map(move |z| IVec3::new(x, y, z))))
        .collect()
}

/// Quarter-cell offset of the half-size PDC mount along its mounting axis.
const PDC_SEAT: f32 = 0.25;

/// The fixed roster. Every case is deliberate:
///
/// - `owners_l` is the owner's own build, cell for cell (three flat, two
///   vertical on the right end) - the shape that measured 0% coplanar.
/// - the runs vary length and thickness, because a surface only grows an
///   interior at three cells across.
/// - `tee`, `cross`, `lone_cell`, `plate_open` and `inside_corner` are the
///   remaining primitive neighbourhoods a plate can stand in.
/// - `fitted_hull` carries a drive, a bay and two PDC mounts on four different
///   faces: the subject the style regressions (`near_fitting` above all) are
///   read off.
fn roster_subjects() -> Vec<Subject> {
    let owners_l = vec![
        IVec3::new(0, 0, 0),
        IVec3::new(1, 0, 0),
        IVec3::new(2, 0, 0),
        IVec3::new(2, 1, 0),
        IVec3::new(2, 2, 0),
    ];
    let mut tee = block(IVec3::new(5, 1, 1));
    tee.extend((1..4).map(|z| IVec3::new(2, 0, z)));
    let mut cross = block(IVec3::new(5, 1, 1))
        .into_iter()
        .map(|cell| cell + IVec3::new(0, 0, 2))
        .collect::<Vec<_>>();
    cross.extend((0..5).filter(|z| *z != 2).map(|z| IVec3::new(2, 0, z)));
    let mut inside_corner = block(IVec3::new(3, 1, 3));
    inside_corner.extend((0..3).flat_map(|x| (1..3).map(move |y| IVec3::new(x, y, 0))));

    vec![
        Subject::hull("owners_l", owners_l),
        Subject::hull("lone_cell", vec![IVec3::ZERO]),
        Subject::hull("run_2", block(IVec3::new(2, 1, 1))),
        Subject::hull("run_5", block(IVec3::new(5, 1, 1))),
        Subject::hull("run_5_thick", block(IVec3::new(5, 2, 2))),
        Subject::hull("tee", tee),
        Subject::hull("cross", cross),
        Subject::hull("plate_open", block(IVec3::new(4, 1, 4))),
        Subject::hull("inside_corner", inside_corner),
        Subject {
            id: "fitted_hull",
            cells: block(IVec3::new(3, 2, 3)),
            fittings: vec![
                // The drive bolts by its forward end and exhausts +Z, so
                // unrotated on the aft face its lane points into open space.
                Fitting {
                    prototype: "basic_thruster_section",
                    cell: IVec3::new(1, 0, 3),
                    rotation: Quat::IDENTITY,
                    offset: Vec3::ZERO,
                },
                // The bay's -Z bow face is a hole, not a socket: unrotated on
                // the bow it mates the hull behind it and fires forward, with
                // the torpedo born two cells out into vacuum.
                Fitting {
                    prototype: "torpedo_section",
                    cell: IVec3::new(1, 0, -1),
                    rotation: Quat::IDENTITY,
                    offset: Vec3::ZERO,
                },
                // A mount bolts down by -Y and traverses out of +Y: one on the
                // roof as authored...
                Fitting {
                    prototype: "pdc_kinetic_turret_section",
                    cell: IVec3::new(1, 2, 1),
                    rotation: Quat::IDENTITY,
                    offset: Vec3::NEG_Y * PDC_SEAT,
                },
                // ...and one on the starboard flank, rolled so the base plate
                // faces -X and the gun stands out of +X.
                Fitting {
                    prototype: "pdc_kinetic_turret_section",
                    cell: IVec3::new(3, 0, 1),
                    rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                    offset: Vec3::NEG_X * PDC_SEAT,
                },
            ],
        },
    ]
}

/// One subject as a ship: hull cubes and fittings, centred on its own bounding
/// box so the stand yaw turns it about its middle, with the skin flag and the
/// style id the game derives the whole look from.
fn subject_ship(subject: &Subject, roster: Roster, style: StyleId) -> SpaceshipConfig {
    let cells = subject
        .cells
        .iter()
        .chain(subject.fittings.iter().map(|fitting| &fitting.cell));
    let (low, high) = cells.fold((IVec3::MAX, IVec3::MIN), |(low, high), cell| {
        (low.min(*cell), high.max(*cell))
    });
    let centre = (low + high).as_vec3() * 0.5;

    let mut sections = Vec::new();
    for (index, cell) in subject.cells.iter().enumerate() {
        sections.push(SpaceshipSectionConfig {
            id: format!("hull_{index}"),
            position: cell.as_vec3() - centre,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("reinforced_hull_section".to_string()),
            modifications: vec![],
        });
    }
    for (index, fitting) in subject.fittings.iter().enumerate() {
        sections.push(SpaceshipSectionConfig {
            id: format!("fitting_{index}"),
            position: fitting.cell.as_vec3() + fitting.offset - centre,
            rotation: fitting.rotation,
            source: SectionSource::Prototype(fitting.prototype.to_string()),
            modifications: vec![],
        });
    }

    SpaceshipConfig {
        allegiance: None,
        // Scenery: these are subjects, not craft. Nothing flies them.
        controller: SpaceshipController::None,
        hull: ShipSource::Inline(ShipHull {
            sections,
            // The whole of the skin is the game's business, derived from the
            // sections above at spawn - which is what makes the clad frame
            // evidence rather than a picture of what this file decided.
            skin: roster.clad,
            style: roster.clad.then_some(style).flatten().map(str::to_string),
            ..default()
        }),
        ..default()
    }
}

/// Where one subject stands: filled row by row, each row centred on its own
/// count so a short last row does not hang off one side.
fn stand_position(index: usize, subjects: usize) -> Vec3 {
    let row = index / COLUMNS;
    let rows = subjects.div_ceil(COLUMNS);
    let in_row = (subjects - row * COLUMNS).min(COLUMNS);
    let column = index % COLUMNS;
    Vec3::new(
        (column as f32 - (in_row as f32 - 1.0) * 0.5) * COLUMN_SPACING,
        0.0,
        (row as f32 - (rows as f32 - 1.0) * 0.5) * ROW_SPACING,
    )
}

/// The stage: the roster under the game's own sky and the repo's standard
/// three-point rig, checked by the game's own content gate and by the
/// clearance rule before it is posed.
fn bench_row(
    game_assets: &GameAssets,
    sections: &GameSections,
    roster: Roster,
    style: StyleId,
) -> ScenarioConfig {
    let subjects = roster_subjects();
    for subject in &subjects {
        refuse_blocked(subject, sections);
    }
    let count = subjects.len();
    let ships = subjects.iter().enumerate().map(|(index, subject)| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: subject.id.to_string(),
                name: subject.id.to_string(),
                position: stand_position(index, count),
                rotation: Quat::from_rotation_y(SUBJECT_YAW),
            },
            kind: ScenarioObjectKind::Spaceship(subject_ship(subject, roster, style)),
        })
    });

    let scenario = ScenarioConfig {
        description: "A fixed roster of hand-placed structures for judging the skin".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: ships
                .chain(ThreePointRig::around("photo", Vec3::ZERO, 3.0).actions())
                .collect(),
        }],
        ..ScenarioConfig::new(
            "shape_bench".to_string(),
            "Shape Bench".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    refuse_broken(&scenario, sections);
    scenario
}

/// The six cardinal faces, in the order `ShipExit` indexes them (`nova_ship`'s
/// `FACES`).
const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// The index in [`FACES`] of the cardinal `direction` points along.
fn face_index(direction: Vec3) -> Option<usize> {
    FACES.iter().position(|face| face.dot(direction) > 0.999)
}

/// Refuse a subject whose fittings cannot fire where they stand, through the
/// same reading and the same rule the game uses (`read_structure` +
/// `blocked_exits`). A bench subject with a blocked exit is a broken subject,
/// not something to photograph around.
fn refuse_blocked(subject: &Subject, sections: &GameSections) {
    let config_of = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("shape_bench: no section prototype '{id}'"))
    };
    let hull = config_of("reinforced_hull_section");
    let mut placed: Vec<PlacedPart> = subject
        .cells
        .iter()
        .map(|cell| PlacedPart {
            position: cell.as_vec3(),
            rotation: Quat::IDENTITY,
            link_points: &hull.base.link_points,
            exit: None,
        })
        .collect();
    for fitting in &subject.fittings {
        let config = config_of(fitting.prototype);
        placed.push(PlacedPart {
            position: fitting.cell.as_vec3() + fitting.offset,
            rotation: fitting.rotation,
            link_points: &config.base.link_points,
            exit: exit_normal(&config.kind),
        });
    }

    let (structure, _, cells) = read_structure(&placed);
    let exits: Vec<ShipExit> = placed
        .iter()
        .zip(&cells)
        .filter_map(|(part, cell)| {
            let out = face_index(part.rotation * part.exit?)?;
            Some(ShipExit { cell: *cell, out })
        })
        .collect();
    let blocked = blocked_exits(&structure, &exits);
    assert!(
        blocked.is_empty(),
        "shape_bench: subject `{}` blocks its own exits: {blocked:?}",
        subject.id,
    );
}

/// Run the roster through the game's OWN content gate before posing it, the
/// `wfc_ships` claim held to by the hand-placed cases too: what stands on the
/// bench is a ship the game would accept.
fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::default(),
        &HashSet::from([scenario.id.clone()]),
    );
    let errors: Vec<&str> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "shape_bench: the roster holds content the game would refuse:\n  {}",
        errors.join("\n  ")
    );
    info!(
        "shape_bench: {} subject(s) lint clean, {} warning(s)",
        roster_subjects().len(),
        issues.len(),
    );
}

fn load_roster(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    requested: Res<StyleRequest>,
    mut roster: ResMut<Roster>,
) {
    if let Some(id) = requested.0.as_deref() {
        match styles.iter().position(|style| style.id == id) {
            Some(index) => roster.style = index,
            // Loud, not silent: a typo would otherwise photograph the first
            // look and read as the one that was asked for.
            None => panic!("--style '{id}' is not in the merged content"),
        }
    }
    let style = style_at(&styles, roster.style);
    commands.trigger(LoadScenario(bench_row(
        &game_assets,
        &sections,
        *roster,
        style,
    )));
    spawn_readout(&mut commands);
    spawn_labels(&mut commands);
}

/// `L` steps to the next authored look on the SAME subjects, and `C` strips or
/// restores the cladding on the spot - both through the `LoadScenario` path,
/// which tears the old roster down for us.
fn restyle_on_key(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    mut roster: ResMut<Roster>,
) {
    if keyboard.just_pressed(KeyCode::KeyL) {
        roster.style = roster.style.wrapping_add(1);
        // A style change is invisible on a bare roster, so asking for one asks
        // for the skin back.
        roster.clad = true;
    } else if keyboard.just_pressed(KeyCode::KeyC) {
        roster.clad = !roster.clad;
    } else {
        return;
    }
    let style = style_at(&styles, roster.style);
    commands.trigger(LoadScenario(bench_row(
        &game_assets,
        &sections,
        *roster,
        style,
    )));
}

/// Marks the style readout.
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
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Written every frame rather than on `Roster` change, for the same reason as
/// `wfc_ships`: the readout is spawned by a command in the same run as the
/// roster's first change, so a change-gated write never runs again.
///
/// The readout follows the grave/tilde HUD cycle in a hand-run, so "no hud"
/// clears the top of the frame. Captures are exempt: they run at cinematic
/// from startup, and the readout is what names the look a frame shows.
fn update_readout(
    roster: Res<Roster>,
    styles: Res<GameStyles>,
    hud: Res<HudVisibility>,
    mut q_readout: Query<(&mut Text, &mut Visibility), With<StyleReadout>>,
) {
    let dress = if roster.clad {
        format!(
            "clad: {}",
            style_at(&styles, roster.style).unwrap_or("bare")
        )
    } else {
        "bare".to_string()
    };
    let line = format!("Shape bench - {dress} - [L] look  [C] cladding");
    let shown = capturing() || hud.shows();
    for (mut text, mut visibility) in &mut q_readout {
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if text.as_str() != line {
            **text = line.clone();
        }
    }
}

/// A subject's nameplate, anchored under its stand in world space.
#[derive(Component)]
struct SubjectLabel(Vec3);

/// The width the label centres its text in, in logical pixels.
const LABEL_WIDTH: f32 = 220.0;

/// One nameplate per subject. Spawned ONCE, off the fixed stand layout rather
/// than off the entities: the anchor is a property of the bench, so the labels
/// survive every restyle reload untouched.
fn spawn_labels(commands: &mut Commands) {
    let subjects = roster_subjects();
    let count = subjects.len();
    for (index, subject) in subjects.iter().enumerate() {
        let anchor = stand_position(index, count) + Vec3::NEG_Y * LABEL_DROP;
        commands
            .spawn((
                SubjectLabel(anchor),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-1000.0),
                    top: Val::Px(-1000.0),
                    width: Val::Px(LABEL_WIDTH),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(subject.id),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.9, 0.95)),
                    // A back row's label lands on the hull behind it, and
                    // white on lit plate is unreadable; the scrim hugs the
                    // text and keeps every name legible.
                    Node {
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                ));
            });
    }
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

/// Print the skin report for every subject, once per spawned roster.
///
/// The reading is the game's own (`read_structure` off the live sections, then
/// `derive_skin` + `read_plates` + `skin_report`), so the numbers describe the
/// skin on screen rather than a parallel derivation. Per subject:
///
/// - the summary line, and the CREASED count by relief class (inherited
///   question 1: do creased plates read as a defect on the worst subjects?);
/// - one line per authored style with the decoration count, the `near_fitting`
///   taken-of-reach and the full per-fixture tally (inherited question 2: the
///   three unjudged style regressions). Every style is reported, not just the
///   worn one - the scatter is hashed off the cell, so the numbers hold.
fn report_skins(
    mut reported: Local<Vec<Entity>>,
    roots: Query<(Entity, &Name, &Children), With<SpaceshipRootMarker>>,
    q_section: Query<(&Transform, &SectionLinkPoints, Option<&SectionExit>), With<SectionMarker>>,
    styles: Res<GameStyles>,
) {
    for (root, name, children) in &roots {
        if reported.contains(&root) {
            continue;
        }
        let placed: Vec<PlacedPart> = children
            .iter()
            .filter_map(|child| q_section.get(child).ok())
            .map(|(pose, sockets, exit)| PlacedPart {
                position: pose.translation,
                rotation: pose.rotation,
                link_points: sockets.as_slice(),
                exit: exit.map(|exit| exit.0),
            })
            .collect();
        // Sections spawn a command flush after the root; come back for this
        // subject once they stand.
        if placed.is_empty() {
            continue;
        }
        reported.push(root);

        let (structure, _, _) = read_structure(&placed);
        let plates = derive_skin(&structure);
        let readings = read_plates(&structure, &plates);
        let bare = skin_report(&structure, &plates, &readings, None);

        let creased_total: usize = bare.plates.iter().filter(|row| !row.coplanar).count();
        let by_relief: Vec<String> = RELIEFS
            .iter()
            .map(|relief| {
                let of = || {
                    bare.plates
                        .iter()
                        .filter(|row| row.reading.relief == *relief)
                };
                format!(
                    "{} {}/{}",
                    relief.name(),
                    of().filter(|row| !row.coplanar).count(),
                    of().count(),
                )
            })
            .collect();
        info!(
            "shape_bench: subject `{}`: {} plate(s), {creased_total} creased; \
             creased/total by relief: {}",
            name.as_str(),
            bare.plates.len(),
            by_relief.join(", "),
        );
        info!(
            "shape_bench: subject `{}`: {}",
            name.as_str(),
            skin_summary(&bare),
        );

        for style in styles.iter() {
            let report = skin_report(&structure, &plates, &readings, Some(style));
            let (near_taken, near_reach) = style
                .fixtures
                .iter()
                .zip(&report.rules)
                .filter(|(fixture, _)| fixture.scatter.near_fitting.is_some())
                .fold((0, 0), |(taken, reach), (_, rule)| {
                    (taken + rule.taken, reach + rule.reach)
                });
            let taken: Vec<usize> = report.rules.iter().map(|rule| rule.taken).collect();
            let reach: Vec<usize> = report.rules.iter().map(|rule| rule.reach).collect();
            info!(
                "shape_bench: subject `{}` style `{}`: {} piece(s); near_fitting \
                 {near_taken} hit(s) of {near_reach} reach; {}",
                name.as_str(),
                style.id,
                report.decor,
                decor_tally(style, &taken, &reach),
            );
        }
    }
}

/// What the camera aims at: the middle of the stand.
const CAMERA_TARGET: Vec3 = Vec3::ZERO;

/// Where the camera stands: backed off far enough to hold the fixed grid, high
/// and in front so it reads flank and top at once.
fn camera_position() -> Vec3 {
    let subjects = roster_subjects().len();
    let columns = subjects.min(COLUMNS) as f32 + 1.0;
    let rows = subjects.div_ceil(COLUMNS) as f32 + 1.0;
    let span = (columns * COLUMN_SPACING).max(rows * ROW_SPACING);
    Vec3::new(0.0, span * 0.34, span * 0.58)
}

/// Frame every camera the loader spawns, so a restyle comes back to the same
/// composition instead of the loader's default perch.
fn frame_new_camera(
    mut q_camera: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut q_camera {
        *transform =
            Transform::from_translation(camera_position()).looking_at(CAMERA_TARGET, Vec3::Y);
    }
}

/// Radians per second the idle orbit turns at.
const ORBIT_RATE: f32 = 0.25;

/// How much further out the orbit stands than the composed front-on framing,
/// so the corner subjects stay in frame on the broadside pass.
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
/// disagree with what actually moves the camera - and so a binding change does
/// not silently leave the orbit fighting the player.
fn track_orbit_idle(
    mut orbit: ResMut<IdleOrbit>,
    time: Res<Time>,
    q_input: Query<&WASDCameraInput>,
) {
    let touched = q_input
        .iter()
        .any(|input| input.pan != Vec2::ZERO || input.wasd != Vec2::ZERO || input.vertical != 0.0);
    if touched {
        orbit.idle_secs = 0.0;
    } else {
        // Saturated at the threshold: the timer is a re-arm gate, not a
        // stopwatch, so there is nothing to count past it.
        orbit.idle_secs = (orbit.idle_secs + time.delta_secs()).min(ORBIT_RESUME_SECS);
    }
}

/// Turn the roster on a slow turntable while nobody is flying. The CAMERA
/// orbits rather than the subjects, which hold the composition the grid exists
/// for. Runs after the free-fly rig writes its transform, because that rig
/// writes every frame and would otherwise win.
///
/// On re-arm the azimuth is read off the parked camera's own xz offset, so
/// the orbit drifts on from wherever the viewer left it. Radius and height
/// SNAP back to the composed standoff - the one framing known to hold the
/// whole roster - rather than easing out from a camera flown in close.
fn orbit_idle_camera(
    mut orbit: ResMut<IdleOrbit>,
    time: Res<Time>,
    mut q_camera: Query<&mut Transform, With<ScenarioCameraMarker>>,
) {
    if !orbit.enabled {
        return;
    }
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
    let stand = camera_position();
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

/// Seconds a step may sit before the run aborts naming it (llvmpipe headroom).
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 30.0;

/// Pose the harness camera on the stand.
#[cfg(feature = "debug")]
fn frame_stand(world: &mut World) {
    pose_camera(world, camera_position(), CAMERA_TARGET);
}

/// The driven walk: load the roster, frame it, shoot it.
#[cfg(feature = "debug")]
fn bench_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the roster")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("frame the roster")
        .on_enter(|world: &mut World| frame_stand(world))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the roster")
        .on_enter(|world: &mut World| shoot(world, "shape-bench.png"))
        .until(shot_written("shape-bench.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
