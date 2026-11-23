//! faction_doctrines: the open world's generated hulls of one civilization at
//! a time, so each civilization's build doctrine reads against the next.
//!
//! One generated hull per role (civilian, industrial, scavenger, armored, left
//! to right) and the wreck of the armored one stand in a row, laid out by the
//! world's own `generate_ship` and `generate_wreck` for the civilization shown,
//! in its own palette, at one advancement. The stands and the camera do not
//! move between civilizations: the stand pitch and the camera come from the
//! advancement's ship ceiling, not from the hulls, so sizes compare.
//!
//! What to look at:
//!
//! 1. two civilizations' hulls of one role read apart by shape: long or
//!    stout, a waist or a shoulder, a pointed or a blunt bow;
//! 2. the hulls of one civilization look related across the roles;
//! 3. the wreck reads as a ruin of its civilization's armored hull.
//!
//! The label names the civilization and its doctrine: scale centre, broad and
//! compact shares, waist (negative) or shoulder, taper, favoured cube pattern
//! and ring width, drive pair share and nose divisor, then each stand's
//! section count.
//!
//! ```text
//! nix develop --command cargo run --example faction_doctrines --features debug
//! nix develop --command cargo run --example faction_doctrines --features debug -- --world 115 --advancement 1.0
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | ] | the next civilization of the lattice |
//! | [ | the previous civilization |
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk shows the first four civilizations,
//! or the comma list of lattice indices in `NOVA_DOCTRINE_WALK` (for example
//! `0,7,12`). For each it waits for the row and every greeble model, checks
//! that every stand stands in the civilization's palette with its design's
//! section count, and logs the doctrine. With `NOVA_CAPTURE=1` it shoots
//! `faction-doctrine-<index>.png`. Unset, no walk and no deadline run.

use std::sync::Arc;

use bevy::{core_pipeline::Skybox, prelude::*};
use clap::Parser;
use nova_gameplay::prelude::CivilizationId;
use nova_protocol::prelude::*;
use nova_wfc::prelude::lint_errors;

/// How far the lattice of civilizations reaches from the origin on each of
/// its two axes: 5 x 5 civilizations per world.
const REACH: i32 = 2;

/// The first hull seed a role's stand tries.
const FIRST_HULL_SEED: u32 = 1;

/// How many hull seeds a role tries before the example fails.
const HULL_SEED_SEARCH: u32 = 32;

/// Clear space added to the stand pitch, in engine units. Stands stand one
/// ship ceiling plus this apart, bows nearly forward: a generated hull is at
/// least twice as long as it is wide, so two neighbours' half-widths stay
/// under one ceiling.
const STAND_GAP: f32 = 2.0;

/// The stands of a row: four roles and the armored wreck.
const STANDS: usize = 5;

/// Every stand turns the same small yaw off the camera axis, for depth.
const SHIP_YAW: f32 = -0.25;

/// The camera looks down on the row at this elevation, in radians: the
/// plan view shows the waist, the taper and the bow; the slant shows depth.
const CAMERA_ELEVATION: f32 = 0.96;

/// The wreck's id and `Name`.
const WRECK_NAME: &str = "armored-wreck";

/// The looks backdrop, linear: a neutral light grey, as in
/// `faction_palettes`, so a dark hull reads as a silhouette.
const LOOKS_BACKDROP: Color = Color::linear_rgb(0.55, 0.55, 0.55);

/// The looks ambient floor: a face no rig light reaches keeps its plate lines.
const LOOKS_AMBIENT: f32 = 250.0;

#[derive(Parser)]
#[command(name = "faction_doctrines")]
#[command(version = "1.0.0")]
#[command(about = "The generated hulls of each civilization, one build doctrine at a time", long_about = None)]
struct Cli {
    /// The world seed of the civilization lattice.
    #[arg(long, default_value_t = 115)]
    world: u32,
    /// The lattice index to start on, from 0.
    #[arg(long, default_value_t = 0)]
    civilization: usize,
    /// The advancement every civilization builds at, in [0, 1].
    #[arg(long, default_value_t = 0.6)]
    advancement: f32,
}

/// What the rows are laid out from: the base snapshot, the lattice of
/// civilizations and the advancement.
#[derive(Resource, Clone)]
struct Yard {
    snapshot: Arc<ShipPartSnapshot>,
    civilizations: Vec<CivilizationId>,
    advancement: f32,
    /// The stand pitch, from the advancement's ship ceiling.
    pitch: f32,
}

/// One stand of the row: its id and `Name`, where it stands, its design and
/// whether it is the wreck.
#[derive(Clone)]
struct Stand {
    name: String,
    position: Vec3,
    design: ShipDesign,
    wreck: bool,
}

/// The row of the civilization shown.
#[derive(Resource, Clone)]
struct Stage {
    index: usize,
    civilization: CivilizationId,
    doctrine: BuildDoctrine,
    stands: Vec<Stand>,
}

/// Where the camera stands and how large the light rig is: the same for
/// every civilization.
#[derive(Resource, Clone, Copy)]
struct View {
    light_scale: f32,
    camera_position: Meters3,
}

/// The lattice index of the civilization shown.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct Shown(usize);

/// The label naming the civilization and its doctrine.
#[derive(Component)]
struct DoctrineLabel;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    assert!(
        (0.0..=1.0).contains(&cli.advancement),
        "faction_doctrines: --advancement {} lies outside [0, 1]",
        cli.advancement
    );
    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&["base"]);
    let snapshot = ShipPartSnapshot::build(&packs, &nova_authoring::generation::build_palettes())
        .unwrap_or_else(|faults| {
            panic!(
                "faction_doctrines: the base snapshot is refused:\n  {}",
                faults
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )
        });
    let civilizations: Vec<CivilizationId> = (-REACH..=REACH)
        .flat_map(|x| {
            (-REACH..=REACH).map(move |y| CivilizationId {
                world_seed: cli.world,
                node: [x, y, 0],
            })
        })
        .collect();
    assert!(
        cli.civilization < civilizations.len(),
        "faction_doctrines: --civilization {} is past the {} of the lattice",
        cli.civilization,
        civilizations.len()
    );
    let ceiling = ship_ceiling(cli.advancement).to_engine();
    let pitch = ceiling + STAND_GAP;
    let yard = Yard {
        snapshot: Arc::new(snapshot),
        civilizations,
        advancement: cli.advancement,
        pitch,
    };
    let half_span = pitch * (STANDS as f32 - 1.0) * 0.5 + ceiling * 0.5;
    let distance = half_span * 1.35;
    let view = View {
        light_scale: ceiling * 0.5,
        // Above the bows: -Z is ahead of an unturned hull.
        camera_position: Meters3::from_engine(Vec3::new(
            0.0,
            distance * CAMERA_ELEVATION.sin(),
            -distance * CAMERA_ELEVATION.cos(),
        )),
    };
    let stage = lay_out_the_row(&yard, cli.civilization);

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(yard.clone());
            app.insert_resource(view);
            app.insert_resource(stage.clone());
            app.insert_resource(Shown(cli.civilization));
            app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
            app.add_systems(
                Update,
                (
                    change_civilization_on_key.run_if(in_state(GameStates::Playing)),
                    reload_on_change.run_if(in_state(GameStates::Playing)),
                    looks_rig,
                    label_the_doctrine,
                ),
            );
            app.add_observer(stand_the_wreck);
        })
        .build();
    // Nothing flies the row; a still row compares across civilizations.
    app.add_systems(Update, freeze_bodies);
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }
    app.add_plugins(smoke::script());
    app.run()
}

/// The row of the civilization at lattice `index`: one hull per role it
/// builds at the yard's advancement and the wreck of its armored hull, each
/// in its slot, so a role the civilization does not build leaves its slot
/// empty.
///
/// # Panics
///
/// When no hull seed of the search range lays out a role the civilization
/// builds, or the wreck fails.
fn lay_out_the_row(yard: &Yard, index: usize) -> Stage {
    let civilization = yard.civilizations[index];
    let snapshot = &yard.snapshot;
    let roles = snapshot.eligible_roles(civilization, yard.advancement);
    let middle = (STANDS as f32 - 1.0) * 0.5;
    let slot = |at: usize| Vec3::NEG_X * ((at as f32 - middle) * yard.pitch);
    let mut stands = Vec::new();
    for (at, role) in ShipRoleType::ALL.into_iter().enumerate() {
        if !roles.contains(&role) {
            continue;
        }
        let (request, layout) = first_hull_layout(snapshot, civilization, role, yard.advancement);
        stands.push(Stand {
            name: role.label().to_string(),
            position: slot(at),
            design: layout.design,
            wreck: false,
        });
        if role == ShipRoleType::Armored {
            let wreck = generate_wreck(snapshot, request)
                .unwrap_or_else(|failure| panic!("faction_doctrines: wreck: {failure}"));
            stands.push(Stand {
                name: WRECK_NAME.to_string(),
                position: slot(STANDS - 1),
                design: wreck.design,
                wreck: true,
            });
        }
    }
    Stage {
        index,
        civilization,
        doctrine: BuildDoctrine::of(civilization),
        stands,
    }
}

/// The request and layout of the first hull seed of the search range that
/// lays out.
fn first_hull_layout(
    snapshot: &ShipPartSnapshot,
    civilization: CivilizationId,
    role: ShipRoleType,
    advancement: f32,
) -> (ShipLayoutRequest, ShipLayout) {
    let mut last_failure = None;
    for seed in FIRST_HULL_SEED..FIRST_HULL_SEED + HULL_SEED_SEARCH {
        let request = ShipLayoutRequest {
            seed,
            civilization,
            role,
            advancement,
            size: ShipSizeType::Advancement,
            spare_drive: ShipSpareDriveType::Without,
        };
        match generate_ship(snapshot, request) {
            Ok(layout) => return (request, layout),
            Err(failure) => last_failure = Some(failure),
        }
    }
    panic!(
        "faction_doctrines: no {} hull seed lays out: {}",
        role.label(),
        last_failure.expect("the search range is non-empty"),
    );
}

/// The ship config of `stand`, in the palette the generator dressed it in.
fn stand_config(stand: &Stand) -> SpaceshipConfig {
    SpaceshipConfig {
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Neutral),
        lootable: stand.wreck,
        design: ShipDesignSource::Inline(stand.design.clone()),
        ..default()
    }
}

/// Load the row once the assets are in.
fn load_row(
    commands: Commands,
    stage: Res<Stage>,
    view: Res<View>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    game_assets: Res<GameAssets>,
) {
    load_scenario(commands, &stage, &view, &sections, &styles, &game_assets);
}

/// Load `stage`'s row under the three-point rig. The wreck stands when the
/// scenario is loaded, as a derelict.
fn load_scenario(
    mut commands: Commands,
    stage: &Stage,
    view: &View,
    sections: &GameSections,
    styles: &GameStyles,
    game_assets: &GameAssets,
) {
    let ships = stage
        .stands
        .iter()
        .filter(|stand| !stand.wreck)
        .map(|stand| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: stand.name.clone(),
                    name: stand.name.clone(),
                    position: Meters3::from_engine(stand.position),
                    rotation: Quat::from_rotation_y(SHIP_YAW),
                },
                kind: ScenarioObjectKind::Spaceship(stand_config(stand)),
            })
        });
    let camera = EventActionConfig::SetCamera(SetCameraActionConfig {
        position: view.camera_position,
        look_at: Meters3::ZERO,
        blend: None,
    });
    let actions = ships
        .chain(ThreePointRig::around("stand", Meters3::ZERO, view.light_scale).actions())
        .chain(std::iter::once(camera))
        .collect();
    let scenario = ScenarioConfig {
        description: "Generated hulls of one civilization under its build doctrine".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "faction_doctrines".to_string(),
            "Faction Doctrines".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    let errors = lint_errors(&scenario, sections, &GameItems::default(), styles);
    assert!(
        errors.is_empty(),
        "faction_doctrines: a stand is content the game would refuse:\n  {}",
        errors.join("\n  ")
    );
    commands.trigger(LoadScenario(scenario));
}

/// Stand the wreck as the open world spawns a derelict: with
/// [`DerelictShipMarker`] in the root's bundle. A scenario action has no
/// derelict field.
fn stand_the_wreck(_: On<ScenarioLoaded>, mut commands: Commands, stage: Res<Stage>) {
    for stand in stage.stands.iter().filter(|stand| stand.wreck) {
        commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: stand.name.clone(),
                name: stand.name.clone(),
                position: Meters3::from_engine(stand.position),
                rotation: Quat::from_rotation_y(SHIP_YAW),
            }),
            spaceship_scenario_object(stand_config(stand)),
            DerelictShipMarker,
        ));
    }
}

/// `]` shows the next civilization of the lattice, `[` the previous one.
fn change_civilization_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    yard: Res<Yard>,
    mut shown: ResMut<Shown>,
) {
    let count = yard.civilizations.len();
    let step = if keys.just_pressed(KeyCode::BracketRight) {
        1
    } else if keys.just_pressed(KeyCode::BracketLeft) {
        count - 1
    } else {
        return;
    };
    shown.0 = (shown.0 + step) % count;
}

/// Lay out and load the row again when the civilization shown changes.
fn reload_on_change(
    commands: Commands,
    yard: Res<Yard>,
    view: Res<View>,
    shown: Res<Shown>,
    mut stage: ResMut<Stage>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    game_assets: Res<GameAssets>,
) {
    if stage.index == shown.0 {
        return;
    }
    *stage = lay_out_the_row(&yard, shown.0);
    load_scenario(commands, &stage, &view, &sections, &styles, &game_assets);
}

/// The looks light: a flat grey backdrop in place of the skybox and an
/// ambient floor. Runs each frame, since the loader hangs the skybox on the
/// scenario camera some frames after the camera is up.
fn looks_rig(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Camera, Has<Skybox>), With<ScenarioCameraMarker>>,
) {
    for (entity, mut camera, skybox) in &mut cameras {
        if skybox {
            camera.clear_color = ClearColorConfig::Custom(LOOKS_BACKDROP);
            commands
                .entity(entity)
                .remove::<Skybox>()
                .insert(AmbientLight {
                    brightness: LOOKS_AMBIENT,
                    ..default()
                });
        }
    }
}

/// The label text for `stage` of `count` civilizations.
fn doctrine_text(stage: &Stage, count: usize) -> String {
    let BuildDoctrine {
        scale,
        broad,
        compact,
        waist,
        taper,
        pattern,
        ring_width,
        drive_pair,
        nose,
    } = stage.doctrine;
    let [x, y, _] = stage.civilization.node;
    let stands = stage
        .stands
        .iter()
        .map(|stand| format!("{} {}", stand.name, stand.design.sections.len()))
        .collect::<Vec<_>>()
        .join("  ");
    format!(
        "{} ({}/{count}, world {}, node {x},{y})   [ ] civilization\n\
         scale {scale:.2}  broad {broad:.2}  compact {compact:.2}  waist {waist:+.2}  \
         taper {taper:.2}  pattern {pattern:?} rings {ring_width}  drive pairs {drive_pair:.2}  \
         nose /{nose}\n\
         sections: {stands}",
        stage.civilization.name(),
        stage.index + 1,
        stage.civilization.world_seed,
    )
}

/// Name the civilization and its doctrine in the top left corner.
fn label_the_doctrine(
    mut commands: Commands,
    stage: Res<Stage>,
    yard: Res<Yard>,
    mut q_label: Query<&mut Text, With<DoctrineLabel>>,
) {
    let text = doctrine_text(&stage, yard.civilizations.len());
    if let Some(mut label) = q_label.iter_mut().next() {
        if label.0 != text {
            label.0 = text;
        }
        return;
    }
    commands.spawn((
        Name::new("Doctrine Label"),
        DoctrineLabel,
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::BLACK),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

mod smoke {
    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// The lattice indices the walk shows when `NOVA_DOCTRINE_WALK` is unset.
    const WALK: [usize; 4] = [0, 1, 2, 3];

    /// Seconds a row gets to lay out, load, clad and dress on a software GPU.
    const SETUP_SECS: f32 = 300.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;

    /// The lattice indices the walk shows: the comma list in
    /// `NOVA_DOCTRINE_WALK`, or [`WALK`].
    fn walk() -> Vec<usize> {
        match std::env::var("NOVA_DOCTRINE_WALK") {
            Ok(list) => list
                .split(',')
                .map(str::trim)
                .filter(|index| !index.is_empty())
                .map(|index| {
                    index.parse().unwrap_or_else(|_| {
                        panic!("faction_doctrines: NOVA_DOCTRINE_WALK index '{index}'")
                    })
                })
                .collect(),
            Err(_) => WALK.to_vec(),
        }
    }

    /// Whether `entity` has a mesh under it.
    fn has_mesh(world: &World, entity: Entity) -> bool {
        world.get::<Mesh3d>(entity).is_some()
            || world
                .get::<Children>(entity)
                .is_some_and(|children| children.iter().any(|child| has_mesh(world, child)))
    }

    /// The row of lattice `index` stands, and every greeble model is in.
    fn dressed(index: usize) -> Arc<dyn Fn(&World) -> bool + Send + Sync> {
        Arc::new(move |world: &World| {
            let stage = world.resource::<Stage>();
            if stage.index != index {
                return false;
            }
            let Some(mut roots) = world.try_query_filtered::<&Name, With<SpaceshipRootMarker>>()
            else {
                return false;
            };
            let names: Vec<String> = roots.iter(world).map(ToString::to_string).collect();
            let Some(mut decor) = world.try_query_filtered::<Entity, With<ShipDecorMarker>>()
            else {
                return false;
            };
            names.len() == stage.stands.len()
                && stage.stands.iter().all(|stand| names.contains(&stand.name))
                && decor.iter(world).all(|piece| has_mesh(world, piece))
        })
    }

    /// Every stand stands in its civilization's palette with its design's
    /// section count. Logs the doctrine and the row.
    fn assert_the_row(world: &mut World) {
        let stage = world.resource::<Stage>().clone();
        let yard = world.resource::<Yard>().clone();
        let palette = yard.snapshot.palette(stage.civilization).clone();
        let roots: Vec<(Entity, String, ShipStyle)> = world
            .query_filtered::<(Entity, &Name, &ShipStyle), With<SpaceshipRootMarker>>()
            .iter(world)
            .map(|(entity, name, style)| (entity, name.to_string(), style.clone()))
            .collect();
        let sections: Vec<Entity> = world
            .query_filtered::<Entity, With<SectionMarker>>()
            .iter(world)
            .collect();
        for stand in &stage.stands {
            let (root, _, style) = roots
                .iter()
                .find(|(_, name, _)| *name == stand.name)
                .unwrap_or_else(|| panic!("faction_doctrines: {} does not stand", stand.name));
            let worn = style
                .palette
                .as_ref()
                .unwrap_or_else(|| panic!("faction_doctrines: {} wears no palette", stand.name));
            assert_eq!(
                worn.id, palette,
                "faction_doctrines: {} palette",
                stand.name
            );
            let spawned = sections
                .iter()
                .filter(|section| is_under(world, **section, *root))
                .count();
            assert_eq!(
                spawned,
                stand.design.sections.len(),
                "faction_doctrines: {} sections",
                stand.name
            );
        }
        info!(
            "faction_doctrines: PASS {}: {}",
            stage.index,
            doctrine_text(&stage, yard.civilizations.len()).replace('\n', " | ")
        );
    }

    /// Whether `entity` is `root` or under it.
    fn is_under(world: &World, entity: Entity, root: Entity) -> bool {
        let mut at = entity;
        loop {
            if at == root {
                return true;
            }
            match world.get::<ChildOf>(at) {
                Some(parent) => at = parent.parent(),
                None => return false,
            }
        }
    }

    /// The smoke walk: each civilization of [`walk`] checked and shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let mut script = AutopilotPlugin::<GameStates>::new()
            .step("faction doctrines: reach the row")
            .until(scenario_is_built())
            .deadline(SETUP_SECS)
            .add();
        for index in walk() {
            let shot = format!("faction-doctrine-{index}.png");
            script = script
                .step(format!("faction doctrines: show {index}"))
                .on_enter(move |world: &mut World| {
                    world.resource_mut::<Shown>().0 = index;
                })
                .until(dressed(index))
                .deadline(SETUP_SECS)
                .add()
                .step(format!("faction doctrines: settle {index}"))
                .until(frames(SETTLE_FRAMES))
                .add()
                .step(format!("faction doctrines: check {index}"))
                .on_enter(assert_the_row)
                .add()
                .step(format!("faction doctrines: shoot {index}"))
                .on_enter({
                    let shot = shot.clone();
                    move |world: &mut World| {
                        hide_hud(world);
                        shoot(world, &shot);
                    }
                })
                .until(shot_written(shot))
                .deadline(SHOT_SECS)
                .add();
        }
        script
    }
}
