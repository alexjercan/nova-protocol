//! faction_palettes: the open world's generated hulls in each faction
//! palette, under the looks light on a grey backdrop.
//!
//! One generated hull per role (civilian, industrial, scavenger, armored, left
//! to right) and the wreck of the armored one stand in a row, laid out by the
//! world's own `generate_ship` and `generate_wreck` for one civilization. The
//! open world dresses every hull of a civilization in that civilization's
//! palette; here every hull wears the palette shown, so the same hulls compare
//! across the catalog. The role keeps its tone in each palette: civilian light,
//! industrial matte, scavenger dull, armored dark.
//!
//! What to look at:
//!
//! 1. the hue reads as one faction across the row, and the tone as the role;
//! 2. the civilian hull's keyed greebles (stripe, fin, door, windows, tank)
//!    wear the hull's own top colour; magenta anywhere is a missed tint;
//! 3. the greeble habit: a dense palette adds pieces, a sparse one leaves
//!    plates bare, and a palette's accent pieces show on the hulls of their
//!    style.
//!
//! ```text
//! nix develop --command cargo run --example faction_palettes --features debug
//! nix develop --command cargo run --example faction_palettes --features debug -- --palette indigo
//! ```
//!
//! | key | what it does |
//! | - | - |
//! | ] | the next palette of the catalog |
//! | [ | the previous palette |
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk shows four palettes. For each it waits
//! for the row and every greeble model, checks that every hull wears the
//! palette, that each hull's plates carry the palette's top colour for its
//! role, that the civilian hull's keyed greebles took the tint, and that no
//! mesh is magenta. With `NOVA_CAPTURE=1` it shoots
//! `faction-palette-<id>.png`. Unset, no walk and no deadline run.

use bevy::{core_pipeline::Skybox, prelude::*};
use clap::Parser;
use nova_gameplay::prelude::CivilizationId;
use nova_protocol::prelude::*;
use nova_wfc::prelude::lint_errors;

/// The world seed of the civilization the row is laid out for.
const WORLD_SEED: u32 = 115;

/// The first hull seed a role's stand tries.
const FIRST_HULL_SEED: u32 = 1;

/// How many hull seeds a role tries before the example fails.
const HULL_SEED_SEARCH: u32 = 32;

/// The advancement steps a role's highest eligible advancement is searched on.
const ADVANCEMENT_STEPS: u32 = 20;

/// Clear space between two neighbouring stands' clearance spheres, in engine
/// units.
const STAND_GAP: f32 = 2.0;

/// Every stand faces the same quarter yaw.
const SHIP_YAW: f32 = -0.5;

/// The wreck's id and `Name`.
const WRECK_NAME: &str = "armored-wreck";

/// The looks backdrop, linear: a neutral light grey, as in
/// `screenshot_base_fleet --looks`, so a dark hull reads as a silhouette.
const LOOKS_BACKDROP: Color = Color::linear_rgb(0.55, 0.55, 0.55);

/// The looks ambient floor: a face no rig light reaches keeps its plate lines.
const LOOKS_AMBIENT: f32 = 250.0;

#[derive(Parser)]
#[command(name = "faction_palettes")]
#[command(version = "1.0.0")]
#[command(about = "The generated hulls of one civilization in each faction palette", long_about = None)]
struct Cli {
    /// The palette to start on. The civilization's own when not given.
    #[arg(long)]
    palette: Option<String>,
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

/// The row, laid out once at boot, and where the camera stands.
#[derive(Resource, Clone)]
struct Stage {
    stands: Vec<Stand>,
    light_scale: f32,
    camera_position: Meters3,
}

/// The palette the row wears.
#[derive(Resource, Clone, PartialEq, Eq)]
struct Shown(FactionPaletteDesignId);

/// The label naming the palette shown.
#[derive(Component)]
struct PaletteLabel;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&["base"]);
    let snapshot = ShipPartSnapshot::build(&packs, &nova_authoring::generation::build_palettes())
        .unwrap_or_else(|faults| {
            panic!(
                "faction_palettes: the base snapshot is refused:\n  {}",
                faults
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n  ")
            )
        });
    let civilization = CivilizationId {
        world_seed: WORLD_SEED,
        node: [0, 0, 0],
    };
    let start = cli.palette.map_or_else(
        || snapshot.palette(civilization).clone(),
        |id| id.as_str().into(),
    );
    let stage = lay_out_the_row(&snapshot, civilization);

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(stage.clone());
            app.insert_resource(Shown(start.clone()));
            app.add_systems(OnEnter(GameAssetsStates::Loaded), load_row);
            app.add_systems(
                Update,
                (
                    change_palette_on_key.run_if(in_state(GameStates::Playing)),
                    reload_on_change.run_if(in_state(GameStates::Playing)),
                    looks_rig,
                    label_the_palette,
                ),
            );
            app.add_observer(stand_the_wreck);
        })
        .build();
    // Nothing flies the row; a still row compares across palettes.
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

/// The row of one hull per role and the wreck of the armored one, each at
/// the highest advancement its civilization builds the role at.
///
/// # Panics
///
/// When a role has no eligible advancement, no hull seed of the search range
/// lays out, or the wreck fails.
fn lay_out_the_row(snapshot: &ShipPartSnapshot, civilization: CivilizationId) -> Stage {
    let mut stands = Vec::new();
    let mut clearances = Vec::new();
    let mut armored = None;
    for role in ShipRoleType::ALL {
        let advancement = highest_eligible_advancement(snapshot, civilization, role);
        let (request, layout) = first_hull_layout(snapshot, civilization, role, advancement);
        if role == ShipRoleType::Armored {
            armored = Some(request);
        }
        clearances.push(layout.clearance.to_engine());
        stands.push(Stand {
            name: role.label().to_string(),
            position: Vec3::ZERO,
            design: layout.design,
            wreck: false,
        });
    }
    let armored = armored.expect("ShipRoleType::ALL carries Armored");
    let wreck = generate_wreck(snapshot, armored)
        .unwrap_or_else(|failure| panic!("faction_palettes: wreck: {failure}"));
    clearances.push(wreck.clearance.to_engine());
    stands.push(Stand {
        name: WRECK_NAME.to_string(),
        position: Vec3::ZERO,
        design: wreck.design,
        wreck: true,
    });

    let pitch = clearances.iter().copied().fold(0.0, f32::max) * 2.0 + STAND_GAP;
    let middle = (stands.len() as f32 - 1.0) * 0.5;
    for (index, stand) in stands.iter_mut().enumerate() {
        stand.position = Vec3::NEG_X * ((index as f32 - middle) * pitch);
    }
    let reach = stands
        .iter()
        .zip(&clearances)
        .map(|(stand, clearance)| stand.position.length() + clearance)
        .fold(1.0, f32::max);
    Stage {
        stands,
        light_scale: clearances.iter().copied().fold(1.0, f32::max) * 0.5,
        // Off the bows: -Z is ahead of an unturned hull.
        camera_position: Meters3::from_engine(Vec3::new(0.0, reach * 0.5, -reach * 1.3)),
    }
}

/// The highest advancement at which the snapshot serves `role` for
/// `civilization`, on [`ADVANCEMENT_STEPS`].
fn highest_eligible_advancement(
    snapshot: &ShipPartSnapshot,
    civilization: CivilizationId,
    role: ShipRoleType,
) -> f32 {
    (0..=ADVANCEMENT_STEPS)
        .rev()
        .map(|step| step as f32 / ADVANCEMENT_STEPS as f32)
        .find(|advancement| {
            snapshot
                .eligible_roles(civilization, *advancement)
                .contains(&role)
        })
        .unwrap_or_else(|| {
            panic!(
                "faction_palettes: the base snapshot serves no {} at any advancement",
                role.label()
            )
        })
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
        "faction_palettes: no {} hull seed lays out: {}",
        role.label(),
        last_failure.expect("the search range is non-empty"),
    );
}

/// `design` in `palette`, keeping the role finish the generator gave it.
fn worn(design: &ShipDesign, palette: &FactionPaletteDesignId) -> ShipDesign {
    let mut design = design.clone();
    let worn = design
        .presentation
        .palette
        .as_mut()
        .expect("the generator dresses every hull in a palette");
    worn.id = palette.clone();
    design
}

/// The ship config of `stand` in `palette`.
fn stand_config(stand: &Stand, palette: &FactionPaletteDesignId) -> SpaceshipConfig {
    SpaceshipConfig {
        controller: SpaceshipController::None,
        allegiance: Some(Allegiance::Neutral),
        lootable: stand.wreck,
        design: ShipDesignSource::Inline(worn(&stand.design, palette)),
        ..default()
    }
}

/// Load the row once the assets are in.
fn load_row(
    commands: Commands,
    stage: Res<Stage>,
    shown: Res<Shown>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    game_assets: Res<GameAssets>,
) {
    load_scenario(commands, &stage, &shown.0, &sections, &styles, &game_assets);
}

/// Load the row in `palette`, under the three-point rig. The wreck stands
/// when the scenario is loaded, as a derelict.
fn load_scenario(
    mut commands: Commands,
    stage: &Stage,
    palette: &FactionPaletteDesignId,
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
                kind: ScenarioObjectKind::Spaceship(stand_config(stand, palette)),
            })
        });
    let camera = EventActionConfig::SetCamera(SetCameraActionConfig {
        position: stage.camera_position,
        look_at: Meters3::ZERO,
        blend: None,
    });
    let actions = ships
        .chain(ThreePointRig::around("stand", Meters3::ZERO, stage.light_scale).actions())
        .chain(std::iter::once(camera))
        .collect();
    let scenario = ScenarioConfig {
        description: "Generated hulls of one civilization in one faction palette".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "faction_palettes".to_string(),
            "Faction Palettes".to_string(),
            game_assets.cubemap.clone().into(),
        )
    };
    let errors = lint_errors(&scenario, sections, &GameItems::default(), styles);
    assert!(
        errors.is_empty(),
        "faction_palettes: a stand is content the game would refuse:\n  {}",
        errors.join("\n  ")
    );
    commands.trigger(LoadScenario(scenario));
}

/// Stand the wreck as the open world spawns a derelict: with
/// [`DerelictShipMarker`] in the root's bundle. A scenario action has no
/// derelict field.
fn stand_the_wreck(
    _: On<ScenarioLoaded>,
    mut commands: Commands,
    stage: Res<Stage>,
    shown: Res<Shown>,
) {
    for stand in stage.stands.iter().filter(|stand| stand.wreck) {
        commands.spawn((
            base_scenario_object(&BaseScenarioObjectConfig {
                id: stand.name.clone(),
                name: stand.name.clone(),
                position: Meters3::from_engine(stand.position),
                rotation: Quat::from_rotation_y(SHIP_YAW),
            }),
            spaceship_scenario_object(stand_config(stand, &shown.0)),
            DerelictShipMarker,
        ));
    }
}

/// `]` shows the next palette of the catalog, `[` the previous one.
fn change_palette_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    styles: Res<GameStyles>,
    mut shown: ResMut<Shown>,
) {
    let step = if keys.just_pressed(KeyCode::BracketRight) {
        1
    } else if keys.just_pressed(KeyCode::BracketLeft) {
        styles.palettes.len() - 1
    } else {
        return;
    };
    let at = styles
        .palettes
        .iter()
        .position(|palette| palette.id == shown.0)
        .unwrap_or(0);
    shown.0 = styles.palettes[(at + step) % styles.palettes.len()]
        .id
        .clone();
}

/// Load the row again when the palette shown changes. The first run only
/// notes the palette the boot load used: change detection on a resource
/// inserted at build reads as changed on any first run.
fn reload_on_change(
    mut loaded: Local<Option<FactionPaletteDesignId>>,
    commands: Commands,
    stage: Res<Stage>,
    shown: Res<Shown>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    game_assets: Res<GameAssets>,
) {
    match loaded.as_ref() {
        None => *loaded = Some(shown.0.clone()),
        Some(palette) if *palette == shown.0 => {}
        Some(_) => {
            *loaded = Some(shown.0.clone());
            load_scenario(commands, &stage, &shown.0, &sections, &styles, &game_assets);
        }
    }
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

/// Name the palette shown in the top left corner.
fn label_the_palette(
    mut commands: Commands,
    shown: Res<Shown>,
    styles: Option<Res<GameStyles>>,
    mut q_label: Query<&mut Text, With<PaletteLabel>>,
) {
    let Some(styles) = styles else { return };
    let name = styles
        .get_palette(&shown.0)
        .map_or("unknown", |palette| palette.name.as_str());
    let text = format!("{name} ({})   [ ] palette", shown.0);
    if let Some(mut label) = q_label.iter_mut().next() {
        if label.0 != text {
            label.0 = text;
        }
        return;
    }
    commands.spawn((
        Name::new("Palette Label"),
        PaletteLabel,
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
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// The palettes the walk shows: four hues apart on the wheel.
    const WALK: [&str; 4] = ["ochre", "teal", "cobalt", "indigo"];
    /// Seconds a row gets to load, clad and dress on a software GPU.
    const SETUP_SECS: f32 = 300.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;

    /// Every mesh under `root`, with its material's base colour.
    fn mesh_colours(world: &World, root: Entity) -> Vec<(Entity, Color)> {
        let materials = world.resource::<Assets<StandardMaterial>>();
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            if let Some(material) = world.get::<MeshMaterial3d<StandardMaterial>>(entity) {
                if let Some(material) = materials.get(&material.0) {
                    out.push((entity, material.base_color));
                }
            }
            if let Some(children) = world.get::<Children>(entity) {
                stack.extend(children.iter());
            }
        }
        out
    }

    /// Whether `entity` has a mesh under it.
    fn has_mesh(world: &World, entity: Entity) -> bool {
        world.get::<Mesh3d>(entity).is_some()
            || world
                .get::<Children>(entity)
                .is_some_and(|children| children.iter().any(|child| has_mesh(world, child)))
    }

    /// Whether `color` is the greeble key magenta.
    fn is_key(color: Color) -> bool {
        let linear = color.to_linear();
        linear.red > 0.95 && linear.green < 0.05 && linear.blue > 0.95
    }

    /// The row stands in `palette`, and every greeble model is in.
    fn dressed(palette: &'static str) -> Arc<dyn Fn(&World) -> bool + Send + Sync> {
        Arc::new(move |world: &World| {
            let Some(mut roots) =
                world.try_query_filtered::<&ShipStyle, With<SpaceshipRootMarker>>()
            else {
                return false;
            };
            let worn: Vec<_> = roots.iter(world).collect();
            let stands = world.resource::<Stage>().stands.len();
            let Some(mut decor) = world.try_query_filtered::<Entity, With<ShipDecorMarker>>()
            else {
                return false;
            };
            worn.len() == stands
                && worn.iter().all(|style| {
                    style
                        .palette
                        .as_ref()
                        .is_some_and(|worn| worn.id.as_str() == palette)
                })
                && decor.iter(world).all(|piece| has_mesh(world, piece))
        })
    }

    /// Every hull wears `palette`, its plates carry the palette's top colour
    /// for its role, the civilian hull's keyed greebles took that colour, and
    /// no mesh is magenta. Logs the draw count of the row.
    fn assert_the_row(world: &mut World, palette: &str) {
        let catalog = world.resource::<GameStyles>().clone();
        let config = catalog
            .get_palette(&palette.into())
            .unwrap_or_else(|| panic!("faction_palettes: '{palette}' is not in the catalog"));
        let roots: Vec<(Entity, String, ShipStyle)> = world
            .query_filtered::<(Entity, &Name, &ShipStyle), With<SpaceshipRootMarker>>()
            .iter(world)
            .map(|(entity, name, style)| (entity, name.to_string(), style.clone()))
            .collect();
        let decor: Vec<Entity> = world
            .query_filtered::<Entity, With<ShipDecorMarker>>()
            .iter(world)
            .collect();
        let mut meshes = 0;
        let mut tinted_total = 0;
        for (root, name, style) in &roots {
            let worn = style
                .palette
                .as_ref()
                .unwrap_or_else(|| panic!("faction_palettes: {name} wears no palette"));
            assert_eq!(
                worn.id.as_str(),
                palette,
                "faction_palettes: {name} palette"
            );
            let top = config.roles.get(worn.role).top.color;
            let colours = mesh_colours(world, *root);
            meshes += colours.len();
            assert!(
                colours.iter().all(|(_, colour)| !is_key(*colour)),
                "faction_palettes: {name} shows magenta in {palette}"
            );
            assert!(
                colours.iter().any(|(_, colour)| *colour == top),
                "faction_palettes: {name} plates do not wear the {palette} top colour"
            );
            let tinted = decor
                .iter()
                .filter(|piece| is_under(world, **piece, *root))
                .flat_map(|piece| mesh_colours(world, *piece))
                .filter(|(_, colour)| *colour == top)
                .count();
            if name == ShipRoleType::Civilian.label() {
                assert!(
                    tinted > 0,
                    "faction_palettes: no keyed greeble on {name} took the {palette} tint"
                );
            }
            tinted_total += tinted;
        }
        info!(
            "faction_palettes: PASS {palette}: {} hulls, {} greebles, {tinted_total} tinted \
             greeble meshes, {meshes} meshes drawn",
            roots.len(),
            decor.len()
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

    /// The smoke walk: four palettes, each checked and shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let mut script = AutopilotPlugin::<GameStates>::new()
            .step("faction palettes: reach the row")
            .until(scenario_is_built())
            .deadline(SETUP_SECS)
            .add();
        for palette in WALK {
            let shot = format!("faction-palette-{palette}.png");
            script = script
                .step(format!("faction palettes: show {palette}"))
                .on_enter(move |world: &mut World| {
                    world.resource_mut::<Shown>().0 = palette.into();
                })
                .until(dressed(palette))
                .deadline(SETUP_SECS)
                .add()
                .step(format!("faction palettes: settle {palette}"))
                .until(frames(SETTLE_FRAMES))
                .add()
                .step(format!("faction palettes: check {palette}"))
                .on_enter(move |world: &mut World| assert_the_row(world, palette))
                .add()
                .step(format!("faction palettes: shoot {palette}"))
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
