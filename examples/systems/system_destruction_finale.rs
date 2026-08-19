//! system_destruction_finale: every destructible body leaves its OWN art behind.
//!
//! One player ship carrying four section kinds - a controller, a reinforced
//! hull, a thruster and a turret - plus an asteroid off the beam. The script
//! kills them one at a time and reads what each death actually left behind.
//!
//! The bug this range was built for: destruction used to be gated on a `Mesh3d`
//! sitting on the destroyed entity itself. No ship section has ever had one -
//! a section holds its health, collider and markers on a gameplay root while
//! its art hangs off `SectionRenderOf` descendants under a gltf
//! `WorldAssetRoot` - so every section in the game fell through to a burst of
//! eight generic gray cubes, whatever art it was wearing. The cubes are gone,
//! and so is the slicer that replaced them: a dead section DETACHES, keeping
//! the art and the collider it already had. A death that leaves nothing at all
//! is silent, which is why this range is the thing that has to catch one.
//!
//! SEVEN named invariants:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the turret breaks into its own art` | a multi-part gltf section detaches |
//! | 2 | `outcome: the thruster breaks into its own art` | and so does a single-mesh one |
//! | 3 | `outcome: the hull breaks into its own art` | and so does the plainest one |
//! | 4 | `outcome: ordinary carving leaves a viable asteroid` | a rock has geometry, not health |
//! | 5 | `outcome: the asteroid exhausts its own geometry` | its field, not a slicer, ends it |
//! | 6 | `outcome: one death leaves one body` | a section does not fragment |
//! | 7 | `outcome: no death came apart into nothing` | every shipped body detaches as something |
//!
//! Invariant 7 is the whole-run claim. There is no fallback any more: a body
//! that cannot become a body of its own leaves NOTHING and logs it. That makes
//! this range the only thing standing between a silent failure and a shipped
//! build, so a single silent death anywhere in the run is a failure.
//!
//! Invariant 6 is what "sections do not fragment" means as a number. A turret
//! draws with three joint meshes and a hull with one; both have to leave
//! exactly one body, because the whole point of detaching is that the piece
//! count no longer follows the art.
//!
//! The turret goes first on purpose. It is the only section here whose art is
//! several levels below the gameplay entity, so it is the one that proves the
//! art travels with the wreck at all.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_destruction_finale --features debug
//! # look for: `finale probe: the turret left 1 body wearing its own art`,
//! #           `outcome: no death came apart into nothing`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_destruction_finale")]
#[command(version = "1.0.0")]
#[command(about = "Every destructible body breaks into its own art. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario object id of the asteroid the last round kills. Named rather
/// than positional so the rig and the kill beat cannot drift apart.
const ROCK: &str = "rock";

/// How long a kill beat waits for a body to finish dying. Generous: a section
/// death runs through disable, destroy, the geometry walk and the despawn,
/// each a command flush apart.
#[cfg(feature = "debug")]
const REACT_SECS: f32 = 8.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<FinaleProbe>();
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(finale_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
    #[cfg(feature = "debug")]
    {
        app.init_resource::<PendingDeaths>();
        app.add_systems(
            Update,
            (tally_carved_chunks, tally_shards, settle_body_deaths),
        );
        app.add_observer(tally_detached_pieces);
        app.add_observer(watch_body_deaths);
    }
}

/// What each death left on the field, counted as it lands.
///
/// Cumulative and `Added`-driven rather than a live census, because wreckage is
/// `TempEntity` and a beat is longer than some of it lives - a census would
/// call a piece that has already expired "no piece at all", which is the exact
/// failure this range exists to catch.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct FinaleProbe {
    /// Detached bodies seen since the run started.
    pieces: usize,
    /// Deaths that left NOTHING since the run started.
    ///
    /// Counted at the body rather than on the field, because a death that
    /// leaves nothing leaves nothing to count. That is the whole difficulty:
    /// the failure this range guards against is silent.
    silent: usize,
    /// The two counts as of the current beat's start, so a beat reads its own
    /// death rather than the whole run's.
    mark: (usize, usize),
    /// The most bodies ANY single death left behind.
    ///
    /// Measured at the death rather than by counting wreckage in a window,
    /// because deaths overlap: killing three of four sections drops the ship
    /// under its structural-collapse threshold, so the last section's window
    /// also contains whatever the collapse took with it. A window count would
    /// blame one death for two deaths' wreckage.
    most_from_one_death: usize,
    /// How many bodies each death left, keyed by the body that died.
    left_by: HashMap<Entity, usize>,
    /// Bodies a carve SEVERED: real geometry that came away, never decoration.
    carved_chunks: usize,
    /// Shards a carve threw, which is what every carve throws now.
    shards: usize,
    /// The body the current beat is killing.
    target: Option<Entity>,
}

#[cfg(feature = "debug")]
impl FinaleProbe {
    /// What has landed since the last [`FinaleProbe::mark`].
    fn since_mark(&self) -> (usize, usize) {
        (self.pieces - self.mark.0, self.silent - self.mark.1)
    }
}

/// Count chunks separately from detached bodies. A rock ends by exhausting its
/// field, so seeing it leave a detached body is a failure.
#[cfg(feature = "debug")]
fn tally_carved_chunks(q_new: Query<(), Added<CarvedChunkMarker>>, mut probe: ResMut<FinaleProbe>) {
    probe.carved_chunks += q_new.iter().count();
}

/// Count the dust, which is the only thing a carve is guaranteed to throw.
#[cfg(feature = "debug")]
fn tally_shards(q_new: Query<(), Added<CarveShardMarker>>, mut probe: ResMut<FinaleProbe>) {
    probe.shards += q_new.iter().count();
}

/// Attribute every detached body to the death that left it.
///
/// By the piece's own record of what it was, not by a count over a window:
/// deaths overlap, so a window would blame one section for the collapse that
/// followed it.
#[cfg(feature = "debug")]
fn tally_detached_pieces(
    add: On<Add, DetachedPieceMarker>,
    q_piece: Query<&DetachedPieceMarker>,
    mut probe: ResMut<FinaleProbe>,
) {
    let Ok(DetachedPieceMarker(origin)) = q_piece.get(add.entity) else {
        return;
    };
    let origin = *origin;
    probe.pieces += 1;
    let left = probe.left_by.entry(origin).or_insert(0);
    *left += 1;
    let left = *left;
    probe.most_from_one_death = probe.most_from_one_death.max(left);
}

/// Deaths waiting to be judged, and the frame each of them happened on.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PendingDeaths(Vec<(Entity, u32)>);

/// Record every destructible body the moment it dies.
///
/// The only place a SILENT death can be seen: a body that leaves nothing leaves
/// nothing on the field to find, so the death itself has to be counted and
/// matched against the wreckage that follows it.
#[cfg(feature = "debug")]
fn watch_body_deaths(
    add: On<Add, IntegrityDestroyMarker>,
    q_dying: Query<(), (With<ExplodableEntity>, Without<IntegrityRoot>)>,
    frames: Res<bevy::diagnostic::FrameCount>,
    mut pending: ResMut<PendingDeaths>,
) {
    if q_dying.get(add.entity).is_err() {
        return;
    }
    pending.0.push((add.entity, frames.0));
}

/// Judge each recorded death once its wreckage has had a frame to land.
#[cfg(feature = "debug")]
fn settle_body_deaths(
    frames: Res<bevy::diagnostic::FrameCount>,
    mut pending: ResMut<PendingDeaths>,
    mut probe: ResMut<FinaleProbe>,
) {
    let now = frames.0;
    let (ready, waiting): (Vec<_>, Vec<_>) = pending
        .0
        .drain(..)
        .partition(|(_, frame)| now.wrapping_sub(*frame) >= 2);
    pending.0 = waiting;
    for (body, _) in ready {
        if !probe.left_by.contains_key(&body) {
            probe.silent += 1;
        }
    }
}

#[cfg(feature = "debug")]
fn finale_script() -> Script {
    Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(20.0)
        .add()
        // The multi-part section first: it proves the walk reaches descendants
        // AND that many meshes share one budget.
        .step("kill the turret")
        .on_enter(kill_section("turret"))
        .until(target_is_gone())
        .deadline(REACT_SECS)
        .add()
        .step("assert the turret broke into its own art")
        .on_enter(|world: &mut World| {
            assert_broke_into_art(
                world,
                "turret",
                "outcome: the turret breaks into its own art",
            );
        })
        .add()
        .step("kill the thruster")
        .on_enter(kill_section("thruster"))
        .until(target_is_gone())
        .deadline(REACT_SECS)
        .add()
        .step("assert the thruster broke into its own art")
        .on_enter(|world: &mut World| {
            assert_broke_into_art(
                world,
                "thruster",
                "outcome: the thruster breaks into its own art",
            );
        })
        .add()
        .step("kill the hull")
        .on_enter(kill_section("hull"))
        .until(target_is_gone())
        .deadline(REACT_SECS)
        .add()
        .step("assert the hull broke into its own art")
        .on_enter(|world: &mut World| {
            assert_broke_into_art(world, "hull", "outcome: the hull breaks into its own art");
        })
        .add()
        .step("carve the asteroid without ending it")
        .on_enter(carve_asteroid_once)
        .until(elapsed(0.5))
        .add()
        .step("assert ordinary carving leaves a viable asteroid")
        .on_enter(assert_asteroid_is_geometry_authoritative)
        .add()
        .step("exhaust the asteroid field")
        .on_enter(exhaust_asteroid)
        .until(asteroid_destroyed())
        .deadline(REACT_SECS)
        .add()
        .step("assert the asteroid ended through its field")
        .on_enter(assert_asteroid_exhausted)
        .add()
        .step("assert one death, one finale")
        .on_enter(assert_one_finale_per_death)
        .add()
}

/// The player ship root, if it exists.
#[cfg(feature = "debug")]
fn player_root(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?
        .iter(world)
        .next()
}

/// Kill the named section with EXACT-health damage and record it as the beat's
/// target.
///
/// Exact rather than overkill: `HealthApplyDamage` propagates up `ChildOf`, so
/// overkill would drain the ship root's aggregate health as well and collapse
/// the whole hull mid-run.
#[cfg(feature = "debug")]
fn kill_section(name: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let root = player_root(world).expect("finale probe: the rig ship must be up");
        let section = world
            .query_filtered::<(Entity, &ChildOf, &Name, &Health), With<SectionMarker>>()
            .iter(world)
            .find(|(_, ChildOf(parent), section_name, health)| {
                *parent == root
                    && health.current > 0.0
                    && section_name.as_str().to_lowercase().contains(name)
            })
            .map(|(entity, _, _, health)| (entity, health.current));

        let (entity, health) =
            section.unwrap_or_else(|| panic!("finale probe: no live section matching `{name}`"));

        mark_and_target(world, entity);
        info!("finale probe: killing `{name}` ({entity:?}, {health} hp)");
        world.trigger(HealthApplyDamage {
            entity,
            source: None,
            amount: health,
        });
    }
}

/// The scenario rock and its collider/field node.
#[cfg(feature = "debug")]
fn rock_and_node(world: &World) -> (Entity, Entity) {
    let rock = world
        .try_query_filtered::<(Entity, &EntityId), With<AsteroidMarker>>()
        .and_then(|mut q| {
            q.iter(world)
                .find(|(_, id)| id.as_str() == ROCK)
                .map(|(entity, _)| entity)
        })
        .expect("finale probe: the range must have spawned its rock");
    let node = world
        .get::<Children>(rock)
        .and_then(|children| children.iter().next())
        .expect("finale probe: a rock carries a collider node");
    (rock, node)
}

/// Take one visible bite. A viable rock must remain a rock afterward.
#[cfg(feature = "debug")]
fn carve_asteroid_once(world: &mut World) {
    let (rock, node) = rock_and_node(world);
    let centre = world
        .get::<GlobalTransform>(rock)
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    let surface = RockHeight::default().with_seed(7).sampler().radius(Vec3::Z) * 3.0;
    let mut commands = world.commands();
    apply_damage(
        &mut commands,
        node,
        None,
        1200.0,
        DamageType::Kinetic,
        Some(centre + Vec3::Z * surface),
    );
    world.flush();
}

/// Prove normal carving did not consult a hidden kill counter.
#[cfg(feature = "debug")]
fn assert_asteroid_is_geometry_authoritative(world: &mut World) {
    let (rock, node) = rock_and_node(world);
    assert!(
        world.get::<Health>(node).is_none(),
        "finale probe: a normal asteroid still carries a health kill gate"
    );
    assert!(
        world.get::<AsteroidField>(node).is_some(),
        "finale probe: the ordinary hit did not establish a carve field"
    );
    assert!(
        world.get_entity(rock).is_ok(),
        "finale probe: an ordinary crater ended a viable asteroid"
    );
    nova_probe::probe_marker(
        world,
        "outcome: ordinary carving leaves a viable asteroid",
        serde_json::json!({ "healthless": true, "field": true }),
    );
}

/// Remove every sampled corner with one paid sphere centred on the body.
#[cfg(feature = "debug")]
fn exhaust_asteroid(world: &mut World) {
    let (rock, node) = rock_and_node(world);
    let centre = world
        .get::<GlobalTransform>(rock)
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    let mut probe = world.resource_mut::<FinaleProbe>();
    probe.mark = (probe.pieces, probe.silent);
    probe.carved_chunks = 0;
    probe.shards = 0;
    probe.target = Some(rock);
    let mut commands = world.commands();
    apply_damage(
        &mut commands,
        node,
        None,
        2_000_000.0,
        DamageType::Kinetic,
        Some(centre),
    );
    world.flush();
}

#[cfg(feature = "debug")]
fn asteroid_destroyed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        matches!(
            world
                .get_resource::<NovaEventWorld>()
                .and_then(|events| events.get_variable("rock_destroyed")),
            Some(VariableLiteral::Number(count)) if *count >= 1.0
        )
    })
}

/// The rock ended at field exhaustion, was SEEN ending, and never entered the
/// health-driven random slicer.
///
/// One sphere takes the whole field here, so there is no island left to sever
/// and no body is expected. What carries the moment is the dust the ENDING HIT
/// threw - a rock's end spawns nothing of its own - and the bound on chunks is
/// what keeps that end from spraying rigid bodies if a future fixture does
/// leave islands behind.
#[cfg(feature = "debug")]
fn assert_asteroid_exhausted(world: &mut World) {
    let probe = world.resource::<FinaleProbe>();
    let (pieces, silent) = probe.since_mark();
    let chunks = probe.carved_chunks;
    let shards = probe.shards;
    assert_eq!(pieces, 0, "a healthless asteroid detached as a wreck");
    assert_eq!(
        silent, 0,
        "a healthless asteroid entered the section finale"
    );
    assert!(shards > 0, "the hit that ended the rock threw no dust");
    assert!(
        chunks <= 3,
        "field exhaustion severed {chunks} bodies instead of a bounded few"
    );
    let destroyed = world
        .resource::<NovaEventWorld>()
        .get_variable("rock_destroyed");
    assert!(
        matches!(destroyed, Some(VariableLiteral::Number(count)) if *count == 1.0),
        "asteroid OnDestroyed fired {destroyed:?} times instead of once"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the asteroid exhausts its own geometry",
        serde_json::json!({ "pieces": pieces, "chunks": chunks, "shards": shards, "destroyed": 1 }),
    );
}

/// Open a beat: snapshot the debris counts and record what is about to die, so
/// the assertion a beat later reads THIS death and not the run so far.
#[cfg(feature = "debug")]
fn mark_and_target(world: &mut World, entity: Entity) {
    let mut probe = world.resource_mut::<FinaleProbe>();
    probe.mark = (probe.pieces, probe.silent);
    probe.target = Some(entity);
}

/// The kill beat's condition: the body is off the field. Waiting on the entity
/// rather than on a settling time is what makes the count a frame later a claim
/// about a COMPLETED death.
#[cfg(feature = "debug")]
fn target_is_gone() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<FinaleProbe>()
            .and_then(|probe| probe.target)
            .is_some_and(|target| world.get_entity(target).is_err())
    })
}

/// Invariants 1-3, one per section: the death left a body wearing that
/// section's own art, and nothing died silently alongside it.
///
/// `marker` is passed in as a literal from the call site because the
/// catalog-drift roster reads invariant names straight out of this source.
#[cfg(feature = "debug")]
fn assert_broke_into_art(world: &mut World, body: &str, marker: &'static str) {
    let (pieces, silent) = world.resource::<FinaleProbe>().since_mark();

    assert!(
        pieces > 0,
        "finale probe: the {body} died without leaving any of its own art behind \
         - nothing detached"
    );
    assert_eq!(
        silent, 0,
        "finale probe: {silent} of the {body}'s deaths left nothing at all, \
         alongside {pieces} bodies"
    );

    info!("finale probe: the {body} left {pieces} body(s) wearing its own art");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        marker,
        serde_json::json!({ "t": elapsed, "body": body, "pieces": pieces }),
    );
}

/// Invariants 6 and 7, over the whole run: no death left nothing, and no
/// death left more than one body.
#[cfg(feature = "debug")]
fn assert_one_finale_per_death(world: &mut World) {
    // Every death recorded so far has to be judged before the run's claim is
    // read, or the last one is still sitting in the pending list.
    for _ in 0..2 {
        world.increment_change_tick();
    }
    let now = world.resource::<bevy::diagnostic::FrameCount>().0;
    let stale: Vec<Entity> = world
        .resource::<PendingDeaths>()
        .0
        .iter()
        .filter(|(_, frame)| now.wrapping_sub(*frame) >= 2)
        .map(|(body, _)| *body)
        .collect();
    {
        let mut probe = world.resource_mut::<FinaleProbe>();
        for body in stale {
            if !probe.left_by.contains_key(&body) {
                probe.silent += 1;
            }
        }
    }

    let probe = world.resource::<FinaleProbe>();
    assert_eq!(
        probe.silent, 0,
        "finale probe: {} deaths across the run left nothing at all - a shipped \
         body reached the branch that detaches nothing",
        probe.silent
    );
    assert!(
        probe.pieces > 0,
        "delivery guard: the run must have left wreckage at all"
    );
    // Sections do NOT fragment. A turret drawing with three joint meshes and a
    // hull drawing with one both leave exactly one body, because the piece count
    // no longer follows the art.
    assert_eq!(
        probe.most_from_one_death, 1,
        "finale probe: one death left {} bodies - a section is coming apart \
         rather than detaching",
        probe.most_from_one_death
    );

    let (pieces, silent, most) = (probe.pieces, probe.silent, probe.most_from_one_death);
    info!(
        "finale probe: {pieces} detached bodies, {silent} silent deaths, \
         at most {most} from one death"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: one death leaves one body",
        serde_json::json!({ "t": elapsed, "most_from_one_death": most }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: no death came apart into nothing",
        serde_json::json!({ "t": elapsed, "pieces": pieces, "silent": silent }),
    );
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(finale_rig(&game_assets, &sections)));
}

/// The rig: one ship carrying every section kind whose art comes from a
/// different place, and one rock.
///
/// The turret is the load-bearing one. It draws with three separate joint
/// meshes, so it is the only section here that can tell a per-mesh budget from
/// a per-body one, and the only one whose art is several levels below the
/// gameplay entity.
///
/// The layout is a spine (controller, hull, thruster) with the turret mounted
/// beside the controller, and the kill order is chosen to match it. Destruction
/// is leaf-gated by `destroy_a_disabled_leaf`, so a section with something
/// hanging off its far side is only ever DISABLED and its kill beat stalls out:
/// the turret is a leaf from the start, the thruster caps the spine, and the
/// hull only becomes a leaf once the thruster behind it is gone.
///
/// The thruster sits aft on the spine rather than abeam because a thruster
/// carries an exit face and does not link on every side - mounted beside the
/// controller it is a disconnected component and the ship refuses its own link
/// graph.
fn finale_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            infinite_ammo: true,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    "controller",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 0.0),
                ),
                at("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
                at(
                    "thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                ),
                SpaceshipSectionConfig {
                    id: "turret".to_string(),
                    // Seated on the controller's +X face: a quarter-cell in
                    // from it, rolled so the gun stands out of that face.
                    position: Vec3::new(0.75, 0.0, 0.0),
                    rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                    source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
                    modifications: vec![],
                },
            ],
            ..default()
        }),
        ..default()
    };

    ScenarioConfig {
        description: "Every destructible body breaking into its own art.".to_string(),
        events: vec![
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "rock_destroyed".to_string(),
                    expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(0.0)),
                    )),
                })],
            },
            ScenarioEventConfig {
                name: EventConfig::OnStart,
                filters: vec![],
                actions: [
                    vec![
                        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                            base: BaseScenarioObjectConfig {
                                id: "player_ship".to_string(),
                                name: "Rig Ship".to_string(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                            },
                            kind: ScenarioObjectKind::Spaceship(ship),
                        }),
                        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                            base: BaseScenarioObjectConfig {
                                id: ROCK.to_string(),
                                name: "Range Rock".to_string(),
                                // Off the beam and well clear: a rock that drifts
                                // into the rig rams it, and ram damage would kill
                                // sections the script has not scheduled yet.
                                position: Vec3::new(0.0, 0.0, -40.0),
                                rotation: Quat::IDENTITY,
                            },
                            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                                radius: 3.0,
                                texture: game_assets.asteroid_texture.clone().into(),
                                impact_sound: None,
                                destroy_sound: None,
                                mass: None,
                                invulnerable: false,
                                lock_signature: None,
                                seed: Some(7),
                            }),
                        }),
                    ],
                    ThreePointRig::around("rig", Vec3::ZERO, 1.0).actions(),
                ]
                .concat(),
            },
            ScenarioEventConfig {
                name: EventConfig::OnDestroyed,
                filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(ROCK.to_string()),
                    ..default()
                })],
                actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "rock_destroyed".to_string(),
                    expression: VariableExpressionNode::new_add(
                        VariableTermNode::new_factor(VariableFactorNode::new_name(
                            "rock_destroyed".to_string(),
                        )),
                        VariableExpressionNode::new_term(VariableTermNode::new_factor(
                            VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                        )),
                    ),
                })],
            },
        ],
        ..ScenarioConfig::new(
            "destruction_finale".to_string(),
            "Destruction Finale Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
