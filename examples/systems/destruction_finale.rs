//! destruction_finale: every destructible body breaks into its OWN art.
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
//! eight generic gray cubes, whatever art it was wearing. The asteroid, whose
//! mesh happens to sit on the entity that dies, was the only body that ever
//! fragmented properly.
//!
//! SIX named invariants:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the turret breaks into its own art` | a multi-part gltf section fragments |
//! | 2 | `outcome: the thruster breaks into its own art` | and so does a single-mesh one |
//! | 3 | `outcome: the hull breaks into its own art` | and so does the plainest one |
//! | 4 | `outcome: the asteroid breaks into its own art` | the path that already worked is untouched |
//! | 5 | `outcome: no death spends more than its budget` | the budget is per BODY, not per mesh |
//! | 6 | `outcome: no death emitted both fragments and cubes` | one death, one finale |
//!
//! Invariant 6 is the whole-run claim and the reason the tally is cumulative:
//! the fallback and the fragments used to be chosen by two separate observers
//! that could each fire on the same death. Only one thing decides now, so a
//! single fallback cube anywhere in this run is a failure.
//!
//! The turret goes first on purpose. It is the only section here that draws
//! with SEVERAL meshes (yaw, pitch and barrel joints), so it is the one that
//! proves both that descendants are reached at all and that a body with many
//! parts does not out-spend a body with one.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example destruction_finale --features debug
//! # look for: `finale probe: the turret broke into 4 pieces of its own art`,
//! #           `outcome: no death emitted both fragments and cubes`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "destruction_finale")]
#[command(version = "1.0.0")]
#[command(about = "Every destructible body breaks into its own art", long_about = None)]
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
        app.add_systems(Update, tally_debris);
        app.add_observer(watch_body_fragments);
    }
}

/// What each death left on the field, counted as it lands.
///
/// Cumulative and `Added`-driven rather than a live census, because debris is
/// `TempEntity` and the fallback's two-second lifetime is shorter than the gap
/// between two beats - a census would call a burst that has already expired
/// "no burst at all", which is the exact failure this range exists to catch.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct FinaleProbe {
    /// Real mesh fragments seen since the run started.
    fragments: usize,
    /// Generic fallback cubes seen since the run started.
    fallback: usize,
    /// The two counts as of the current beat's start, so a beat reads its own
    /// death rather than the whole run's.
    mark: (usize, usize),
    /// The most fragments ANY single body produced.
    ///
    /// Measured at the body rather than by counting debris in a window,
    /// because deaths overlap: killing three of four sections drops the ship
    /// under its structural-collapse threshold, so the last section's window
    /// also contains whatever the collapse took with it. A window count would
    /// blame one body for two bodies' debris.
    most_from_one_body: usize,
    /// The body the current beat is killing.
    target: Option<Entity>,
}

#[cfg(feature = "debug")]
impl FinaleProbe {
    /// What has landed since the last [`FinaleProbe::mark`].
    fn since_mark(&self) -> (usize, usize) {
        (self.fragments - self.mark.0, self.fallback - self.mark.1)
    }
}

/// Sort every piece of debris the frame it appears into the two paths that can
/// produce one. The name is the tell: the fallback burst names its cubes, the
/// fragment path names its pieces after the body they came off.
#[cfg(feature = "debug")]
fn tally_debris(q_new: Query<&Name, Added<MeshFragmentMarker>>, mut probe: ResMut<FinaleProbe>) {
    for name in &q_new {
        if name.as_str().starts_with("Fallback") {
            probe.fallback += 1;
        } else {
            probe.fragments += 1;
        }
    }
}

/// Record what ONE body's geometry walk produced, at the body, before any of it
/// reaches the field. This is the honest measurement of the per-body budget.
#[cfg(feature = "debug")]
fn watch_body_fragments(
    add: On<Add, ExplodeFragments>,
    q_body: Query<&ExplodeFragments>,
    mut probe: ResMut<FinaleProbe>,
) {
    let Ok(fragments) = q_body.get(add.entity) else {
        return;
    };
    probe.most_from_one_body = probe.most_from_one_body.max(fragments.len());
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
        // The body that always worked, to prove nothing regressed under it.
        .step("kill the asteroid")
        .on_enter(kill_asteroid)
        .until(target_is_gone())
        .deadline(REACT_SECS)
        .add()
        .step("assert the asteroid still fragments")
        .on_enter(|world: &mut World| {
            assert_broke_into_art(
                world,
                "asteroid",
                "outcome: the asteroid breaks into its own art",
            );
        })
        .add()
        .step("assert one death, one finale")
        .on_enter(assert_one_finale_per_death)
        .add()
}

/// The player ship root, if it exists.
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

/// Kill the asteroid's collider/health node, which is where a rock's health
/// lives - the root above it is a bare `RigidBody` husk.
#[cfg(feature = "debug")]
fn kill_asteroid(world: &mut World) {
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
        .expect("finale probe: a rock carries its health on a child node");
    let health = world
        .get::<Health>(node)
        .expect("finale probe: the rock's node has health")
        .current;

    mark_and_target(world, node);
    info!("finale probe: killing the asteroid ({node:?}, {health} hp)");
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: health,
    });
}

/// Open a beat: snapshot the debris counts and record what is about to die, so
/// the assertion a beat later reads THIS death and not the run so far.
#[cfg(feature = "debug")]
fn mark_and_target(world: &mut World, entity: Entity) {
    let mut probe = world.resource_mut::<FinaleProbe>();
    probe.mark = (probe.fragments, probe.fallback);
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

/// Invariants 1-4, one per body: the death produced real fragments of the
/// body's own art, and did NOT fall back to generic cubes.
///
/// `marker` is passed in as a literal from the call site because the
/// catalog-drift roster reads invariant names straight out of this source.
#[cfg(feature = "debug")]
fn assert_broke_into_art(world: &mut World, body: &str, marker: &'static str) {
    let (fragments, fallback) = world.resource::<FinaleProbe>().since_mark();

    assert!(
        fragments > 0,
        "finale probe: the {body} died without leaving any of its own art behind \
         ({fallback} fallback cubes) - the geometry walk found nothing"
    );
    assert_eq!(
        fallback, 0,
        "finale probe: the {body} emitted {fallback} fallback cubes as well as \
         {fragments} fragments - two finales ran for one death"
    );

    info!("finale probe: the {body} broke into {fragments} pieces of its own art");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        marker,
        serde_json::json!({ "t": elapsed, "body": body, "fragments": fragments }),
    );
}

/// Invariants 5 and 6, over the whole run: nothing anywhere fell back, and no
/// death out-spent the budget.
#[cfg(feature = "debug")]
fn assert_one_finale_per_death(world: &mut World) {
    let probe = world.resource::<FinaleProbe>();
    assert_eq!(
        probe.fallback, 0,
        "finale probe: {} fallback cubes across the run - a body with real art \
         took the last-resort path",
        probe.fallback
    );
    assert!(
        probe.fragments > 0,
        "delivery guard: the run must have produced debris at all"
    );
    // The budget is the BODY's: a turret drawing with three joint meshes must
    // not cost three times a hull drawing with one, or a capital collapse
    // multiplies the difference across every section dying at once.
    assert!(
        probe.most_from_one_body <= BODY_FRAGMENT_BUDGET,
        "finale probe: one body spent {} fragments against a budget of \
         {BODY_FRAGMENT_BUDGET} - the budget is being charged per mesh",
        probe.most_from_one_body
    );

    let (fragments, fallback, most) = (probe.fragments, probe.fallback, probe.most_from_one_body);
    info!(
        "finale probe: {fragments} fragments, {fallback} fallback cubes, \
         at most {most} from one body"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: no death spends more than its budget",
        serde_json::json!({ "t": elapsed, "most_from_one_body": most, "budget": BODY_FRAGMENT_BUDGET }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: no death emitted both fragments and cubes",
        serde_json::json!({ "t": elapsed, "fragments": fragments, "fallback": fallback }),
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
                at("turret", "light_turret_section", Vec3::new(1.0, 0.0, 0.0)),
            ],
            ..default()
        }),
        ..default()
    };

    ScenarioConfig {
        description: "Every destructible body breaking into its own art.".to_string(),
        events: vec![ScenarioEventConfig {
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
                            health: 100.0,
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
        }],
        ..ScenarioConfig::new(
            "destruction_finale".to_string(),
            "Destruction Finale Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
