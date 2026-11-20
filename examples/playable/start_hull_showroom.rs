//! start_hull_showroom: the six start hulls, one at a time, up close.
//!
//! Each start hull wears its own style: a palette and a greeble kit. This
//! example stands one hull alone under the photo rig, on the game's own
//! skybox, clad and dressed by the game's own spawn path, so what you see is
//! what the hull looks like in a scene.
//!
//! What to try:
//!
//! 1. `1` to `6` stand a start hull, `N` stands the next one;
//! 2. fly round the hull (WASD, mouse) and look at it from bow, stern, flank
//!    and above;
//! 3. step through all six and compare the hull colours and the pieces.
//!
//! ```text
//! nix develop --command cargo run --example start_hull_showroom --features debug
//! nix develop --command cargo run --example start_hull_showroom --features debug -- --ship raider
//! ```
//!
//! The diagnostics (`F3`) name the hull on stage, the style it wears and the
//! count of greeble pieces on it; `1-6` and `N` are hinted.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk stands each hull in turn, checks
//! that it carries greeble pieces and that the six hulls wear six styles, and
//! shoots `start-hull-<id>.png` with `NOVA_CAPTURE=1`. Unset, no walk and no
//! deadline run.

#[path = "../screenshots/shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[derive(Parser)]
#[command(name = "start_hull_showroom")]
#[command(version = "1.0.0")]
#[command(about = "The six start hulls, one at a time, up close", long_about = None)]
struct Cli {
    /// The start hull to stand first.
    #[arg(long, default_value = "mining_tug")]
    ship: String,
}

/// The start hulls, in the order the keys `1` to `6` pick them, each with
/// the scale of its eye offset.
const HULLS: [(&str, f32); 6] = [
    ("mining_tug", 1.0),
    ("salvage_hauler", 1.0),
    ("light_freighter", 1.0),
    ("interceptor", 0.9),
    ("raider", 0.9),
    ("patrol_picket", 0.8),
];

/// The keys that pick a hull, in [`HULLS`] order.
const PICK_KEYS: [KeyCode; 6] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
];

/// The eye's offset from the hull: above the port bow, as in
/// `screenshot_base_fleet`.
const EYE: Meters3 = Meters3::new(-110.0, 55.0, -95.0);

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let start = HULLS
        .iter()
        .position(|(id, _)| *id == cli.ship)
        .unwrap_or_else(|| panic!("start_hull_showroom: '{}' is not a start hull", cli.ship));

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(Focus(start));
            app.init_resource::<Staged>();
            app.add_systems(OnEnter(GameAssetsStates::Loaded), load_hull);
            app.add_systems(
                Update,
                (
                    pick_hull_on_key.run_if(in_state(GameStates::Playing)),
                    frame_new_camera,
                    read_the_stage,
                ),
            );
        })
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "the six start hulls, one at a time",
    ));
    app.add_systems(
        PostUpdate,
        fill_the_panel
            .in_set(DiagnosticsSystems)
            .run_if(in_state(GameStates::Playing)),
    );
    // Nothing flies the hull; a still hull stays in the frame.
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

/// The index into [`HULLS`] of the hull on stage.
#[derive(Resource, Clone, Copy)]
struct Focus(usize);

/// What stands on the stage this frame.
#[derive(Resource, Default)]
struct Staged {
    /// The index into [`HULLS`] of the hull whose root is up.
    hull: Option<usize>,
    /// The id of the style the hull wears.
    style: String,
    /// How many greeble pieces are spawned.
    pieces: usize,
}

/// The scenario object id of hull `id`.
fn stage_id(id: &str) -> String {
    format!("hull_{id}")
}

/// Stand the focused hull once the assets are in.
fn load_hull(
    mut commands: Commands,
    focus: Res<Focus>,
    game_assets: Res<GameAssets>,
    ships: Res<GameShipDesigns>,
) {
    let (id, _) = HULLS[focus.0];
    let hull = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: stage_id(id),
            name: id.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            design: ShipDesignSource::Inline(kit::catalog_ship(&ships, id)),
            ..default()
        }),
    });
    let rig = ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions();
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "One start hull, alone, under the photo rig.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: std::iter::once(hull).chain(rig).collect(),
        }],
        ..ScenarioConfig::new(
            "start_hull_showroom".to_string(),
            "Start Hull Showroom".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// `1` to `6` stand that hull; `N` stands the next one.
fn pick_hull_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<Focus>,
    mut commands: Commands,
) {
    let picked = PICK_KEYS
        .iter()
        .position(|key| keys.just_pressed(*key))
        .or_else(|| {
            keys.just_pressed(KeyCode::KeyN)
                .then_some((focus.0 + 1) % HULLS.len())
        });
    let Some(picked) = picked.filter(|picked| *picked != focus.0) else {
        return;
    };
    focus.0 = picked;
    commands.run_system_cached(load_hull);
}

/// Stand every new scenario camera off the hull's port bow.
///
/// The free-fly rig reads its pose from the `Transform` when its component is
/// inserted and writes the `Transform` from then on, so the rig component is
/// inserted again after the pose is set.
fn frame_new_camera(
    mut commands: Commands,
    focus: Res<Focus>,
    mut q_camera: Query<
        (Entity, &mut Transform, &WASDCamera),
        (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>),
    >,
) {
    let (_, scale) = HULLS[focus.0];
    for (camera, mut transform, rig) in &mut q_camera {
        *transform =
            Transform::from_translation((EYE * scale).to_engine()).looking_at(Vec3::ZERO, Vec3::Y);
        commands.entity(camera).insert(*rig);
    }
}

/// Read which hull is up, the style it wears and its piece count.
fn read_the_stage(
    mut staged: ResMut<Staged>,
    styles: Option<Res<GameStyles>>,
    q_root: Query<(&EntityId, &ShipStyle), With<SpaceshipRootMarker>>,
    q_pieces: Query<(), With<ShipDecorMarker>>,
) {
    let Some(styles) = styles else { return };
    let up = q_root.iter().find_map(|(id, worn)| {
        let hull = HULLS.iter().position(|(hull, _)| stage_id(hull) == id.0)?;
        Some((hull, worn.resolve(&styles)?.id.clone()))
    });
    *staged = match up {
        Some((hull, style)) => Staged {
            hull: Some(hull),
            style,
            pieces: q_pieces.iter().count(),
        },
        None => Staged::default(),
    };
}

/// Name the hull, its style and its piece count on the panel.
fn fill_the_panel(focus: Res<Focus>, staged: Res<Staged>, mut panel: ResMut<DiagnosticsPanel>) {
    panel.hint("1-6", "pick a hull");
    panel.hint("N", "next hull");
    let (id, _) = HULLS[focus.0];
    if staged.hull == Some(focus.0) {
        panel.line(format!(
            "hull {id}   style {}   {} pieces",
            staged.style, staged.pieces
        ));
    } else {
        panel.line(format!("hull {id}: waiting for it"));
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// Seconds a hull gets to load and clad on a software GPU.
    const SETUP_SECS: f32 = 300.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the view gets before the check and the shot: the pieces spawn
    /// some frames after the root.
    const SETTLE_FRAMES: u32 = 20;
    /// The name of the panel a scenario load raises while the game plays.
    const LOAD_PANEL: &str = "Scenario Loading Screen";

    /// The style each hull wore when the walk checked it, in [`HULLS`] order.
    #[derive(Resource, Default)]
    struct Worn(Vec<String>);

    fn stand(hull: usize) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            if world.resource::<Focus>().0 == hull {
                return;
            }
            world.resource_mut::<Focus>().0 = hull;
            world
                .run_system_cached(load_hull)
                .expect("the hull loader runs");
        }
    }

    /// The hull is up with its pieces, and the panel a load raises over the
    /// scene is down again.
    fn staged(hull: usize) -> Arc<dyn Fn(&World) -> bool + Send + Sync> {
        Arc::new(move |world: &World| {
            world
                .get_resource::<Staged>()
                .is_some_and(|staged| staged.hull == Some(hull) && staged.pieces > 0)
                && !ui_node_present(LOAD_PANEL)(world)
        })
    }

    fn check(hull: usize) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let (id, _) = HULLS[hull];
            let staged = world.resource::<Staged>();
            assert_eq!(
                staged.hull,
                Some(hull),
                "start hull showroom: {id} is on stage"
            );
            assert!(
                staged.pieces > 0,
                "start hull showroom: {id} carries greeble pieces"
            );
            let style = staged.style.clone();
            info!(
                "start hull showroom: {id} wears {style}, {} pieces",
                staged.pieces
            );
            world.get_resource_or_init::<Worn>().0.push(style);
            shoot(world, &format!("start-hull-{id}.png"));
        }
    }

    /// Six hulls, six styles.
    fn assert_six_styles(world: &mut World) {
        let worn = &world.resource::<Worn>().0;
        let mut distinct = worn.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(
            (worn.len(), distinct.len()),
            (HULLS.len(), HULLS.len()),
            "start hull showroom: each start hull wears its own style, got {worn:?}"
        );
        info!("start hull showroom: PASS six hulls, six styles: {worn:?}");
    }

    /// The smoke walk: each hull stands, carries pieces and is shot; the six
    /// wear six styles.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let mut script = AutopilotPlugin::<GameStates>::new();
        for (hull, (id, _)) in HULLS.into_iter().enumerate() {
            let shot = format!("start-hull-{id}.png");
            script = script
                .step(format!("start hull showroom: stand {id}"))
                .on_enter(stand(hull))
                .until(staged(hull))
                .deadline(SETUP_SECS)
                .add()
                .step(format!("start hull showroom: settle {id}"))
                .until(frames(SETTLE_FRAMES))
                .add()
                .step(format!("start hull showroom: check and shoot {id}"))
                .on_enter(check(hull))
                .until(shot_written(&shot))
                .deadline(SHOT_SECS)
                .add();
        }
        script
            .step("start hull showroom: six styles")
            .on_enter(assert_six_styles)
            .add()
    }
}
