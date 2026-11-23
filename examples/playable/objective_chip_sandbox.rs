//! objective_chip_sandbox: five marked Enemy raiders, and the gold objective
//! chip that leads to each one, then settles to a diamond.
//!
//! The player flies the bounty hunter's interceptor, spawned as New Game
//! spawns a start role. Five pirate raiders hold station with nobody at the
//! controls, each tagged `JOB-n Neutralize` as a held bounty job tags its
//! target: four ahead, from 270 m to 1.46 km, and one behind on the right at
//! about 1 km. Nothing ends the run.
//!
//! What to try:
//!
//! 1. watch the four raiders ahead: each wears the full chip for 4 s, then
//!    only the gold diamond above its red triangle;
//! 2. the raider behind keeps its full chip at the screen edge, and no red
//!    edge arrow beside it;
//! 3. turn until a raider leaves the view for a second or more, then turn
//!    back: its full chip shows again for 4 s. A shorter look away keeps the
//!    diamond;
//! 4. lock a raider (hold `Ctrl` on it): the diamond stays;
//! 5. press `1`: every label changes (a second job joins each raider), and
//!    every chip in view opens again; press `1` again to go back;
//! 6. press `2`: the tags leave; press `2` again: they land fresh and open.
//!
//! ```text
//! nix develop --command cargo run --example objective_chip_sandbox --features debug
//! ```
//!
//! The diagnostics (`F3`) list each raider's label and whether its chip is
//! full or settled, and put the same word over each raider.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the cast and the five
//! chips, holds the clock inside the dwell and checks four full chips in view
//! and the fifth pinned with no red arrow, lets the clock run until the four
//! settle and checks the diamonds, then changes the labels and checks that the
//! four open again. Last it brings raider 2 into the top-right corner and takes
//! a travel lock on it, so the target inset shows over its brackets, logs the
//! chips against the inset, drops the lock and logs them again. With `NOVA_CAPTURE=1` it shoots
//! `objective-chips-{dwell,settled,lock}-{phosphor,hardware}.png`. Unset, no
//! walk and no deadline run.

use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[derive(Parser)]
#[command(name = "objective_chip_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Five marked Enemy raiders: the gold objective chip leads to each, then settles \
             to a diamond",
    long_about = None
)]
struct Cli;

/// Scenario id of the player ship.
const PLAYER_ID: &str = "objective_chip_player";
/// The start role the player flies.
const PLAYER_ROLE: &str = "bounty_hunter";
/// The start role whose hull the raiders carry.
const RAIDER_ROLE: &str = "pirate";
/// Where each raider holds station, by job number: four ahead of the player
/// and one behind on the right.
const RAIDERS: [Meters3; 5] = [
    Meters3::new(-40.0, 10.0, -270.0),
    Meters3::new(330.0, 20.0, -560.0),
    Meters3::new(-560.0, 150.0, -850.0),
    Meters3::new(420.0, 160.0, -1_380.0),
    Meters3::new(450.0, 0.0, 885.0),
];
/// The raiders the walk expects in view at the start: all but the last.
const AHEAD: usize = 4;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(sandbox_plugin).build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "a marked raider's chip settles to a diamond; look away a second and back to see it again",
    ));

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn sandbox_plugin(app: &mut App) {
    app.init_resource::<Tags>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_sandbox);
    app.add_systems(
        Update,
        (read_the_keys, tag_the_raiders)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_chips
            .in_set(DiagnosticsSystems)
            .after(ObjectiveChipSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_sandbox(world: &mut World) {
    let scenario = sandbox(world);
    world.trigger(LoadScenario(scenario));
}

/// What the raiders' tags read.
#[derive(Resource, Debug)]
struct Tags {
    /// The tags are on the raiders.
    on: bool,
    /// A second job joins each raider's label.
    joined: bool,
}

impl Default for Tags {
    fn default() -> Self {
        Self {
            on: true,
            joined: false,
        }
    }
}

impl Tags {
    /// The tag label of raider `index`, as a held job writes it.
    fn label(&self, index: usize) -> String {
        let job = index + 1;
        if self.joined {
            format!("JOB-{job}, JOB-{} Neutralize", job + RAIDERS.len())
        } else {
            format!("JOB-{job} Neutralize")
        }
    }
}

/// Scenario id of raider `index`.
fn raider_id(index: usize) -> String {
    format!("objective_chip_raider_{}", index + 1)
}

/// A start role's ship, as `SpawnStartRole` builds it: the catalog hull, the
/// bindings a role hull spawns with, and the role's kit.
///
/// # Panics
///
/// On a role or a hull the loaded content does not hold.
fn role_ship(
    world: &World,
    role: &str,
    controller: impl FnOnce(&ResolvedShipDesign) -> SpaceshipController,
) -> (String, SpaceshipConfig) {
    let role = world
        .resource::<GameStartRoles>()
        .get_role(&role.into())
        .unwrap_or_else(|| panic!("objective_chip_sandbox: no start role '{role}'"))
        .clone();
    let designs = world.resource::<GameShipDesigns>();
    let sections = world.resource::<GameSections>();
    let name = designs
        .get_design(&role.hull)
        .unwrap_or_else(|| panic!("objective_chip_sandbox: no hull '{}'", role.hull))
        .name
        .clone();
    let design = ShipDesignSource::prototype(role.hull.clone());
    let (hull, _) = resolve_ship_design(&design, designs, sections);
    let ship = SpaceshipConfig {
        design,
        controller: controller(&hull),
        capabilities: default(),
        initial_velocity: MetersPerSecond3::ZERO,
        allegiance: None,
        inventory: role.kit.stock.clone(),
        lootable: false,
        credits: role.kit.credits,
    };
    (name, ship)
}

fn object(
    id: &str,
    name: String,
    position: Meters3,
    kind: ScenarioObjectKind,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name,
            position,
            rotation: Quat::IDENTITY,
        },
        kind,
    })
}

fn sandbox(world: &World) -> ScenarioConfig {
    let (player_name, player) = role_ship(world, PLAYER_ROLE, |hull| {
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: start_role_bindings(hull),
        })
    });
    // Nobody flies the raiders, so they hold their marks for the frames.
    let (raider_name, raider) = role_ship(world, RAIDER_ROLE, |_| SpaceshipController::None);
    let raider = SpaceshipConfig {
        allegiance: Some(Allegiance::Enemy),
        ..raider
    };
    let raiders = RAIDERS.iter().enumerate().map(|(index, &at)| {
        object(
            &raider_id(index),
            format!("{raider_name} {}", index + 1),
            at,
            ScenarioObjectKind::Spaceship(raider.clone()),
        )
    });

    ScenarioConfig {
        description: "A start-role player and five marked Enemy raiders".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![object(
                    PLAYER_ID,
                    player_name,
                    Meters3::ZERO,
                    ScenarioObjectKind::Spaceship(player),
                )],
                raiders.collect(),
                ThreePointRig::around("objective chips", Meters3::ZERO, 20.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "objective_chip_sandbox".to_string(),
            "Objective Chip Sandbox".to_string(),
            world.resource::<GameAssets>().cubemap.clone().into(),
        )
    }
}

/// The staged root with scenario id `id`.
fn staged(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), Without<ChildOf>>()?
        .iter(world)
        .find_map(|(entity, entity_id)| (entity_id.0 == id).then_some(entity))
}

/// `1` joins a second job to every label and back; `2` takes the tags off
/// and puts them back.
fn read_the_keys(keys: Res<ButtonInput<KeyCode>>, mut tags: ResMut<Tags>) {
    if keys.just_pressed(KeyCode::Digit1) {
        tags.joined = !tags.joined;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        tags.on = !tags.on;
    }
}

/// Hold every raider's tag to [`Tags`]: put on, relabelled or taken off.
fn tag_the_raiders(
    mut commands: Commands,
    tags: Res<Tags>,
    q_raiders: Query<(Entity, &EntityId, Option<&ObjectiveMarkerTarget>), Without<ChildOf>>,
) {
    for (entity, id, tag) in &q_raiders {
        let Some(index) = (0..RAIDERS.len()).find(|&index| id.0 == raider_id(index)) else {
            continue;
        };
        match (tags.on, tag) {
            (true, Some(tag)) if tag.label == tags.label(index) => {}
            (true, _) => {
                commands
                    .entity(entity)
                    .insert(ObjectiveMarkerTarget::new(&tags.label(index)));
            }
            (false, Some(_)) => {
                commands.entity(entity).remove::<ObjectiveMarkerTarget>();
            }
            (false, None) => {}
        }
    }
}

/// Each raider's label and the look of its chip, in the panel and over the
/// raider.
fn draw_the_chips(
    mut panel: ResMut<DiagnosticsPanel>,
    tags: Res<Tags>,
    q_chips: Query<(&AnchoredChipTarget, &ObjectiveChipType), With<ObjectiveMarkerChipHudMarker>>,
    q_raiders: Query<(&ObjectiveMarkerTarget, &GlobalTransform)>,
) {
    panel.hint(
        "1",
        if tags.joined {
            "split the jobs"
        } else {
            "join a second job"
        },
    );
    panel.hint(
        "2",
        if tags.on {
            "take the tags off"
        } else {
            "put the tags on"
        },
    );
    for (target, shown) in &q_chips {
        let Ok((tag, transform)) = q_raiders.get(**target) else {
            continue;
        };
        let look = match shown {
            ObjectiveChipType::Full => "full chip",
            ObjectiveChipType::Mark => "diamond",
        };
        panel.line(format!("{}: {look}", tag.label));
        panel.label_tinted(transform.translation(), look, palette::TARGET);
    }
}

mod smoke {
    use std::sync::Arc;

    use bevy::prelude::*;
    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate},
        prelude::*,
    };
    use nova_ui::theme::{ActiveUiTheme, SelectedUiTheme};

    use super::{raider_id, staged, Tags, AHEAD, PLAYER_ID, RAIDERS};

    /// How far out raider 2 stands for the lock beat.
    const LOCK_RANGE: Meters = Meters(900.0);
    /// Where on screen raider 2 stands for the lock beat (UI px from the top
    /// right corner): inside the corner the target inset takes, so the inset
    /// shows over its brackets and its chip reaches the inset.
    const LOCK_SPOT_FROM_CORNER: Vec2 = Vec2::new(150.0, 170.0);

    /// Bring raider 2 to [`LOCK_SPOT_FROM_CORNER`] at [`LOCK_RANGE`] and take a
    /// travel lock on it.
    fn lock_raider_two(world: &mut World) {
        let raider = staged(world, &raider_id(1)).expect("objective_chip_sandbox: no raider 2");
        let player = staged(world, PLAYER_ID).expect("objective_chip_sandbox: no player");
        let ui_scale = world.resource::<UiScale>().0;
        let at = world
            .query_filtered::<(&Camera, &GlobalTransform), With<ScreenIndicatorCamera>>()
            .iter(world)
            .find_map(|(camera, transform)| {
                let view = camera.logical_viewport_size()?;
                let spot = Vec2::new(
                    view.x - LOCK_SPOT_FROM_CORNER.x * ui_scale,
                    LOCK_SPOT_FROM_CORNER.y * ui_scale,
                );
                let ray = camera.viewport_to_world(transform, spot).ok()?;
                Some(ray.get_point(LOCK_RANGE.to_engine()))
            })
            .expect("objective_chip_sandbox: no screen indicator camera");
        if let Some(mut position) = world.get_mut::<avian3d::prelude::Position>(raider) {
            position.0 = at;
        }
        if let Some(mut transform) = world.get_mut::<Transform>(raider) {
            transform.translation = at;
        }
        world.entity_mut(player).insert(TravelLock(Some(raider)));
    }

    /// Drop the travel lock, so the inset closes.
    fn release_the_lock(world: &mut World) {
        let player = staged(world, PLAYER_ID).expect("objective_chip_sandbox: no player");
        world.entity_mut(player).insert(TravelLock(None));
    }

    /// Log the job chips' rects against the target inset's, and how much of
    /// each chip the inset covers.
    fn log_the_inset(world: &mut World) {
        let inset = world
            .query_filtered::<(&ComputedNode, &UiGlobalTransform, &Visibility), With<TargetInsetHudMarker>>()
            .iter(world)
            .find(|(.., visibility)| **visibility != Visibility::Hidden)
            .map(|(computed, transform, _)| {
                let scale = computed.inverse_scale_factor();
                Rect::from_center_size(transform.translation * scale, computed.size() * scale)
            });
        info!("objective chip sandbox: lock: inset {inset:?}");
        let chips: Vec<Rect> = world
            .query_filtered::<(&ComputedNode, &UiGlobalTransform, &Visibility), With<AnchoredChipNodeMarker>>()
            .iter(world)
            .filter(|(.., visibility)| **visibility != Visibility::Hidden)
            .map(|(computed, transform, _)| {
                let scale = computed.inverse_scale_factor();
                Rect::from_center_size(transform.translation * scale, computed.size() * scale)
            })
            .collect();
        for chip in chips {
            let covered = inset.map(|inset| chip.intersect(inset)).unwrap_or_default();
            let covered = if covered.is_empty() {
                0.0
            } else {
                covered.width() * covered.height()
            };
            info!("objective chip sandbox: lock: chip {chip:?} under the inset {covered:.0} px2");
        }
    }

    /// Real seconds the load may take on lavapipe.
    const LOAD_DEADLINE: f32 = 600.0;
    /// Real seconds a beat or a shot may take on lavapipe.
    const BEAT_DEADLINE: f32 = 300.0;
    /// Frames a theme switch gets to re-tint.
    const THEME_FRAMES: u32 = 4;
    /// The themes every frame is shot on, by id and file tag.
    const THEMES: [(&str, &str); 2] = [
        (nova_ui::theme::PHOSPHOR_THEME_ID, "phosphor"),
        (nova_ui::theme::HARDWARE_THEME_ID, "hardware"),
    ];

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn cast_present(world: &World) -> bool {
        staged(world, PLAYER_ID).is_some()
            && (0..RAIDERS.len()).all(|index| staged(world, &raider_id(index)).is_some())
    }

    /// What each raider's chip draws, by job number; `None` before it has a
    /// chip.
    fn looks(world: &World) -> Vec<Option<ObjectiveChipType>> {
        let chips: Vec<(Entity, ObjectiveChipType)> = world
            .try_query_filtered::<(&AnchoredChipTarget, &ObjectiveChipType), With<ObjectiveMarkerChipHudMarker>>()
            .map(|mut query| {
                query
                    .iter(world)
                    .map(|(target, shown)| (**target, *shown))
                    .collect()
            })
            .unwrap_or_default();
        (0..RAIDERS.len())
            .map(|index| {
                let raider = staged(world, &raider_id(index))?;
                chips
                    .iter()
                    .find(|(target, _)| *target == raider)
                    .map(|(_, shown)| *shown)
            })
            .collect()
    }

    /// The raiders ahead all draw `ahead`, and the one behind its full chip.
    fn the_chips_read(ahead: ObjectiveChipType) -> impl Fn(&World) -> bool {
        move |world| {
            let looks = looks(world);
            looks[..AHEAD].iter().all(|shown| *shown == Some(ahead))
                && looks[AHEAD] == Some(ObjectiveChipType::Full)
        }
    }

    /// The raider behind: its red edge indicator exists and its arrow is
    /// hidden, so its gold chip is the one edge cue.
    fn assert_one_cue_behind(world: &mut World) {
        let behind = staged(world, &raider_id(AHEAD)).expect("objective_chip_sandbox: no raider 5");
        let indicator = world
            .query_filtered::<(Entity, &EdgeIndicatorTarget), With<EdgeIndicatorMarker>>()
            .iter(world)
            .find(|(_, target)| ***target == behind)
            .map(|(indicator, _)| indicator)
            .expect("objective_chip_sandbox: raider 5 has no red edge indicator to stand down");
        let children: Vec<Entity> = world
            .get::<Children>(indicator)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        let arrow_shows = children.iter().any(|&child| {
            world.get::<ScreenIndicatorArrowMarker>(child).is_some()
                && world.get::<Visibility>(child) != Some(&Visibility::Hidden)
        });
        assert!(
            !arrow_shows,
            "objective_chip_sandbox: raider 5 shows a red edge arrow beside its gold chip"
        );
    }

    fn assert_the_dwell(world: &mut World) {
        let looks = looks(world);
        assert!(
            looks
                .iter()
                .all(|shown| *shown == Some(ObjectiveChipType::Full)),
            "objective_chip_sandbox: every chip is full inside the dwell: {looks:?}"
        );
        assert_one_cue_behind(world);
        nova_probe::probe_marker(
            world,
            "outcome: five full chips inside the dwell, one edge cue behind",
            serde_json::json!({ "looks": format!("{looks:?}") }),
        );
        info!("objective chip sandbox: PASS five full chips inside the dwell, one cue behind");
    }

    fn assert_the_settle(world: &mut World) {
        let looks = looks(world);
        assert!(
            the_chips_read(ObjectiveChipType::Mark)(world),
            "objective_chip_sandbox: four diamonds ahead and the full chip behind: {looks:?}"
        );
        assert_one_cue_behind(world);
        nova_probe::probe_marker(
            world,
            "outcome: the chips in view settle to diamonds, the edge chip stays full",
            serde_json::json!({ "looks": format!("{looks:?}") }),
        );
        info!("objective chip sandbox: PASS four diamonds ahead, the full chip behind");
    }

    fn assert_the_reopen(world: &mut World) {
        let looks = looks(world);
        assert!(
            the_chips_read(ObjectiveChipType::Full)(world),
            "objective_chip_sandbox: a label change opens every chip again: {looks:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a label change opens the settled chips again",
            serde_json::json!({ "looks": format!("{looks:?}") }),
        );
        info!("objective chip sandbox: PASS a label change opens the four chips again");
    }

    fn set_virtual_paused(paused: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let mut time = world.resource_mut::<Time<Virtual>>();
            if paused {
                time.pause();
            } else {
                time.unpause();
            }
        }
    }

    /// Shoot `what` on each theme, then go back to the default theme.
    fn shoot_both_themes(
        mut script: AutopilotPlugin<GameStates>,
        what: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        for (theme, tag) in THEMES {
            let shot = format!("objective-chips-{what}-{tag}.png");
            let written = shot.clone();
            script = script
                .step(format!(
                    "objective chip sandbox: switch to {tag} for {what}"
                ))
                .on_enter(move |world: &mut World| {
                    world.resource_mut::<SelectedUiTheme>().0 = theme.to_string();
                })
                .until(when(move |world| {
                    world.resource::<ActiveUiTheme>().id() == theme
                }))
                .deadline(BEAT_DEADLINE)
                .add()
                .step(format!("objective chip sandbox: settle {tag} for {what}"))
                .until(frames(THEME_FRAMES))
                .add()
                .step(format!("objective chip sandbox: shoot {shot}"))
                .on_enter(move |world: &mut World| shoot(world, &shot))
                .until(shot_written(written))
                .deadline(BEAT_DEADLINE)
                .add();
        }
        script
            .step(format!(
                "objective chip sandbox: back to the default theme after {what}"
            ))
            .on_enter(|world: &mut World| {
                *world.resource_mut::<SelectedUiTheme>() = SelectedUiTheme::default();
            })
            .until(frames(THEME_FRAMES))
            .add()
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("objective chip sandbox: load the cast")
            .enter(GameStates::Loading)
            .until(and(
                and(state_is(GameStates::Playing), player_ship_present()),
                Arc::new(cast_present),
            ))
            .deadline(LOAD_DEADLINE)
            .add()
            // The clock holds from the frame the five chips stand, so the
            // shots fall inside the dwell however long a frame takes.
            .step("objective chip sandbox: the five chips land")
            .until(when(|world| looks(world).iter().all(Option::is_some)))
            .deadline(BEAT_DEADLINE)
            .add()
            .step("objective chip sandbox: hold the clock inside the dwell")
            .on_enter(set_virtual_paused(true))
            .until(frames(4))
            .add()
            .step("objective chip sandbox: assert the dwell")
            .on_enter(assert_the_dwell)
            .add();
        let script = shoot_both_themes(script, "dwell")
            .step("objective chip sandbox: run the clock until the chips in view settle")
            .on_enter(set_virtual_paused(false))
            .until(when(the_chips_read(ObjectiveChipType::Mark)))
            .deadline(BEAT_DEADLINE)
            .add()
            .step("objective chip sandbox: hold the clock on the diamonds")
            .on_enter(set_virtual_paused(true))
            .until(frames(4))
            .add()
            .step("objective chip sandbox: assert the settle")
            .on_enter(assert_the_settle)
            .add();
        let script = shoot_both_themes(script, "settled")
            .step("objective chip sandbox: change every label")
            .on_enter(set_virtual_paused(false))
            .on_enter(|world: &mut World| world.resource_mut::<Tags>().joined = true)
            .until(when(the_chips_read(ObjectiveChipType::Full)))
            .deadline(BEAT_DEADLINE)
            .add()
            .step("objective chip sandbox: assert the reopen")
            .on_enter(assert_the_reopen)
            .add()
            .step("objective chip sandbox: lock raider 2 near the inset")
            .on_enter(set_virtual_paused(true))
            .on_enter(lock_raider_two)
            .until(frames(8))
            .add()
            .step("objective chip sandbox: log the chips against the inset")
            .on_enter(log_the_inset)
            .add();
        shoot_both_themes(script, "lock")
            .step("objective chip sandbox: release the lock")
            .on_enter(release_the_lock)
            .until(frames(8))
            .add()
            .step("objective chip sandbox: log the chips with no inset")
            .on_enter(log_the_inset)
            .add()
    }
}
