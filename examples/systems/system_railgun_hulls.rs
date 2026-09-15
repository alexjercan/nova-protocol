//! system_railgun_hulls: one shipped lance, bolted off the axis of a light hull
//! and of a heavy one, and judged against what each hull actually weighs.
//!
//! Task 20260909-213623. `system_railgun_lance` already owns the weapon: the
//! commit, the charge bolt, the rake, the one-shell magazine, and the SIGN of
//! the recoil. What it cannot say is whether any of that scales, because it
//! flies one three-section rig. A recoil figure tuned on that rig is a figure
//! nobody has ever checked against a capital.
//!
//! So this range fires the SAME shipped `railgun_lance_section`, from the SAME
//! ship-local station, off two hulls an order of magnitude apart - `block_skiff`
//! and `block_carrier` - and reads every claim off numbers the run measured
//! rather than off numbers anybody typed.
//!
//! The station is a pylon under the keel. There is no cell that is free on both
//! hulls AND touching both, so each hull grows its own short column of
//! `reinforced_hull_section` down the `x=0, z=0` line from its own belly to the
//! shared mount cell. The LANCE sits at the same ship-local cell either way,
//! which is what makes the pair comparable; the pylon under it is the part that
//! differs, and its mass and its lever are measured live rather than assumed.
//!
//! Five claims:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: one lance sits at the same station on both hulls` | same prototype, same mount cell, a real off-bore lever on each, and two hulls far enough apart in mass for the rest to mean anything |
//! | 2 | `outcome: recoil moves each hull by its own measured mass` | each hull's velocity step is the authored impulse over the mass avian weighed it at, and the two steps stand in the inverse of the mass ratio |
//! | 3 | `outcome: the off-axis shot spins the hull inside its own limits` | each hull's rate step is the same impulse through its own lever and its own inertia tensor, and neither hull is left turning faster than its own structure allows |
//! | 4 | `outcome: the computer takes the recoil back out of the heading` | the kick really landed on both hulls, and on both the flight computer returns the hull to the heading it was holding |
//! | 5 | `outcome: the bore sight and the slug agree where the shot went` | the drawn sight leaves the muzzle the shot leaves, runs down the line the slug runs down, and every section it ringed is a section that dies |
//!
//! What the shipped pair reads (2026-09-14, `block_skiff` at 21 sections plus a
//! four-cell pylon, `block_carrier` at 2 081 plus a one-cell one):
//!
//! | | light (`block_skiff`) | heavy (`block_carrier`) |
//! |---|---|---|
//! | live sections | 26 | 2 083 |
//! | mass avian weighed | 33 kg | 2 364 kg |
//! | largest principal inertia | 1.95e2 | 2.20e5 |
//! | structural arm | 48.8 m | 194.1 m |
//! | lever off the bore | 4.16 u | 5.39 u |
//! | velocity step | 1.3717 u/s | 0.0190 u/s |
//! | rate step | 0.96022 rad/s | 0.00117 rad/s |
//! | its own sustained rate | 1.2685 rad/s | 0.6359 rad/s |
//! | thrown off heading | 0.305 rad | under the f32 floor |
//!
//! One 45-impulse shell therefore moves the skiff 72.07x further than the
//! carrier against a 72.07x mass split, and leaves the skiff turning at 76% of
//! the rate its own structure can hold - so a lance on a needle is one shot from
//! the limit, and the same lance is something a capital barely registers. The
//! heavy hull's heading excursion is below the f32 quaternion floor, which is
//! why claim 4's non-vacuity guard is stated on the PAIR and not per hull.
//!
//! Why the readings are taken on the FIXED clock: `charge_and_fire_railgun`
//! runs in `FixedUpdate` and `apply_linear_impulse_at_point` moves
//! `LinearVelocity` and `AngularVelocity` immediately, so the whole of the
//! recoil is inside one physics tick. The sampler here runs in `FixedPostUpdate`
//! after the solver, keeps the previous tick's pair, and differences them on the
//! tick the shot went out. Nothing in this range is read off a frame count and
//! nothing is compared against a clock.
//!
//! What contaminates that difference, and by how much: the flight computer is
//! live on both hulls and applies its own torque every tick. It is applied
//! against the heading ERROR, which is zero on the tick the gun fires - the hull
//! has been sitting on its command - so the angular reading carries at most one
//! tick of a loop that is asking for nothing. The tolerances below are sized for
//! that, and the range records the residual so a change in it is visible.
//!
//! Controls: none. The stance is held for the run, the shot is scripted; fly and
//! look around freely.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_railgun_hulls --features debug
//! # look for: `railgun hulls: 'light' ...`,
//! #           `railgun hulls: 'heavy' ...`,
//! #           `railgun hulls: the sight and the slug agree ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../screenshots/shared/kit.rs"]
mod kit;

use std::collections::BTreeMap;
#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_railgun_hulls")]
#[command(version = "1.0.0")]
#[command(
    about = "A test range for one spinal lance bolted off-axis on a light hull and a heavy hull. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// One hull's turn on the range.
#[derive(Clone, Copy, Debug)]
struct Round {
    /// The key this round's reading is filed under.
    #[cfg(feature = "debug")]
    key: &'static str,
    /// The shipped hull it flies.
    hull: &'static str,
    /// The scenario id it loads under - DISTINCT per round, so a beat can wait
    /// on the second scene arriving rather than on a frame count.
    scenario: &'static str,
    /// What the scene calls the ship.
    name: &'static str,
}

/// The light hull: the fleet's smallest armed-group hull, 21 sections.
const LIGHT: Round = Round {
    #[cfg(feature = "debug")]
    key: "light",
    hull: "block_skiff",
    scenario: "railgun_hulls_light",
    name: "Skiff Lance Rig",
};

/// The heavy hull: the largest hull the base game ships, 2 081 sections.
#[cfg(feature = "debug")]
const HEAVY: Round = Round {
    key: "heavy",
    hull: "block_carrier",
    scenario: "railgun_hulls_heavy",
    name: "Carrier Lance Rig",
};

/// The rounds, in the order they are flown.
#[cfg(feature = "debug")]
const ROUNDS: [Round; 2] = [LIGHT, HEAVY];

/// The shooting ship's scenario id. Shared by both rounds: one scene is up at a
/// time and the readings are filed under the ROUND, not the object.
const SHIP_ID: &str = "lance_rig";

/// The lance's ship-local section id.
const LANCE_SECTION: &str = "lance";

/// The target column's scenario id.
const TARGET_ID: &str = "target";

/// Where the lance is bolted, in ship-local BUILD CELLS.
///
/// Under the keel on the `x=0, z=0` line, one cell clear of the deepest hull
/// either ship carries there (`block_carrier` bottoms out at `y=-3`,
/// `block_skiff` at `y=0`). Two facts make this the station:
///
/// - It is FREE on both hulls, so neither ship has to have a cell carved out of
///   it to carry the gun.
/// - It is OFF the bore axis through either centre of mass, which is the whole
///   subject: `apply_linear_impulse_at_point` derives the torque from the lever,
///   so a lance on the spine would spin nothing and prove nothing.
const MOUNT: Vec3 = Vec3::new(0.0, -5.0, 0.0);

/// Where the target column stands, in meters.
///
/// On the bore line - `MOUNT.y` is -5 cells, and a cell is 10 m - and a
/// kilometre downrange: far enough that the slug is in free flight when it
/// arrives, close enough that it arrives inside a handful of fixed ticks.
const TARGET_AT: Meters3 = Meters3::new(0.0, -50.0, -1_000.0);

/// How many `reinforced_hull_section` layers the target column stands.
const TARGET_LAYERS: i32 = 4;

/// The mass ratio the pair's premise needs the two hulls to actually have.
///
/// A DELIVERY GUARD against the catalog, not an authored number: "the recoil
/// follows the mass" is only worth measuring while the two hulls really are this
/// far apart. Re-tune the fleet so the carrier is four times the skiff and this
/// fails and says so.
#[cfg(feature = "debug")]
const MASS_RATIO_FLOOR: f32 = 20.0;

/// The shortest off-bore lever, in world units, the station must give either
/// hull.
///
/// The claim is about an OFF-AXIS mount. A lever inside a cell of the bore line
/// would make the rotational reading noise, so the range refuses to call such a
/// mount off-axis.
#[cfg(feature = "debug")]
const LEVER_FLOOR: f32 = 1.0;

/// Fractional slack on the translational recoil reading.
///
/// The impulse is applied to `LinearVelocity` directly and nothing else on the
/// hull pushes it, so this is float accumulation over a 2 081-section mass solve
/// and nothing more.
#[cfg(feature = "debug")]
const RECOIL_TOLERANCE: f32 = 0.02;

/// Fractional slack on the rotational recoil reading.
///
/// Wider than the translational one for the reason the module doc gives: the
/// flight computer's own torque rides in the same tick. Sized off that and not
/// off the reading - a loop saturated at the light hull's own 1.27 rad/s2
/// ceiling would move it 0.02 rad/s in one fixed tick, which is 2% of the
/// 0.96 rad/s the shell puts in - so this is that bound with room over it. The
/// loop asks for nothing at the moment of the shot and the measured error is
/// zero on both hulls, which is what the run records.
#[cfg(feature = "debug")]
const SPIN_TOLERANCE: f32 = 0.05;

/// How much of the peak rate the computer must have taken back out before the
/// hull counts as recovered. The VERDICT, read after the hull has settled.
#[cfg(feature = "debug")]
const RESIDUAL_FRACTION: f32 = 0.05;

/// How far below its own peak the hull's rate has to fall before the range
/// stops waiting for it.
///
/// A twenty-fifth of [`RESIDUAL_FRACTION`], and deliberately not the same
/// number: a beat that waited on the ASSERT's threshold would leave the reading
/// sitting exactly on it and make the assert unfailable.
/// `system_attitude_hold` states the same rule for its tracking error - the
/// beats wait on the world, the asserts decide. A hull that never settles hits
/// the beat's window instead and fails the verdict, which is the case this
/// separation exists for.
#[cfg(feature = "debug")]
const SETTLED_FRACTION: f32 = 0.002;

/// The rate under which a hull counts as stopped whatever its peak was, rad/s.
///
/// A capital barely notices one lance: [`SETTLED_FRACTION`] of the heavy hull's
/// peak is under the solver's own residual, and a beat waiting for it would sit
/// out its whole window on a hull that had already finished.
#[cfg(feature = "debug")]
const SETTLED_FLOOR: f32 = 1.0e-5;

/// How close to its held heading a hull has to come back, in radians.
///
/// Well above the f32 quaternion noise floor (~1e-3 rad), which is the same
/// bound `system_attitude_hold` states for the same reading.
#[cfg(feature = "debug")]
const RECONVERGE_TOLERANCE_RAD: f32 = 0.02;

/// How far over that tolerance at least one hull's heading must actually have
/// been thrown.
///
/// The non-vacuity half of claim 4, and deliberately not a per-hull rule: a
/// carrier is barely moved by one lance, and demanding a visible heading
/// excursion from it would be demanding the game be wrong. What the pair must
/// show is that the reading CAN see a disturbance - and the light hull is where
/// it is seen.
#[cfg(feature = "debug")]
const HEADING_GUARD_RATIO: f32 = 10.0;

/// How much of the modelled rate step must actually show up in the peak, before
/// the recovery reading means anything.
#[cfg(feature = "debug")]
const KICK_GUARD_FRACTION: f32 = 0.5;

/// How far apart, in radians, the drawn sight and the slug's own line may sit.
///
/// The sight is traced from the EASED render pose and the shot from the raw
/// physics one. On a hull at rest they are the same pose, so this is quaternion
/// noise on a direction and not a pose difference.
#[cfg(feature = "debug")]
const SIGHT_ANGLE_TOLERANCE_RAD: f32 = 5.0e-3;

/// How far apart, in world units, the sight's origin and the shot's muzzle may
/// sit. Same argument; a cell is 1.0.
#[cfg(feature = "debug")]
const SIGHT_ORIGIN_TOLERANCE: f32 = 0.05;

/// In-step seconds a beat gets to reach its world condition.
///
/// Well over the fleet's usual, for the reason `system_hull_scaling` gives: CI
/// runs the correctness pass on a software rasterizer, where one frame of 2 081
/// sections costs seconds. A backstop that NAMES a hung beat, not a budget the
/// range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 240.0;

/// In-step seconds each hull gets to link its colliders, settle its mass solve
/// and sit down on its command before the gun is armed.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 4.0;

/// In-step seconds the shot gets to finish taking the target apart.
#[cfg(feature = "debug")]
const IMPACT_SECS: f32 = 3.0;

/// In-step seconds the flight computer gets to put the heading back.
#[cfg(feature = "debug")]
const RECOVERY_SECS: f32 = 25.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<Live>();
        app.init_resource::<Sight>();
        app.init_resource::<Tick>();
        app.init_resource::<Readings>();
        app.add_observer(record_the_shot);
        // After the solver, so the pair this differences is the pair the tick
        // ended with - the recoil impulse included.
        app.add_systems(FixedPostUpdate, sample_the_hull.after(PhysicsSystems::Last));
        // The sight is drawn in `Update`; read it there, every frame, so the
        // snapshot the shot freezes is the last one a player could have seen.
        app.add_systems(Update, read_the_sight.run_if(in_state(GameStates::Playing)));
        // No frame-time pass: a 2 081-section player hull is not a frame budget
        // anyone should read, and this range claims nothing about one.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(assert_scenario_loaded(LIGHT.scenario));
        app.add_plugins(nova_screenshot(hulls_script()));
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    // The safety is not something a range gets to write around: `WeaponsHot` is
    // DERIVED every frame from the held combat stance, so a range that pokes the
    // flag has it stomped back before the lance - or the sight - ever reads it.
    // Held in `PreUpdate` after the frame's input is collected, so the whole
    // safety chain runs the way it does in flight.
    app.add_systems(
        PreUpdate,
        hold_combat_stance
            .after(bevy::input::InputSystems)
            .run_if(in_state(GameStates::Playing)),
    );
}

/// Hold the combat stance for the frame, which is what raises the weapons and
/// puts the bore sight on the HUD.
///
/// It also FREEZES the commanded heading: in the stance the mouse drives the
/// turret rig and the normal rig stops moving, so the hull's attitude command
/// stays where the ship spawned. That is exactly the reading claim 4 wants - a
/// computer holding a fixed heading through a kick.
fn hold_combat_stance(mut mouse: ResMut<ButtonInput<MouseButton>>) {
    mouse.press(MouseButton::Right);
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShipDesigns>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(hull_range(
        LIGHT,
        &game_assets,
        &ships,
        &sections,
    )));
}

/// The shipped hull, plus the pylon that carries the lance to [`MOUNT`].
///
/// Taken from the catalog rather than copied: a hand-typed mount a tenth of a
/// cell off its seat joins no component, `derive_link_point_graph` rejects the
/// WHOLE ship as `Disconnected`, and section integrity falls back to empty
/// adjacency. The pylon runs straight down the mount's own column from the
/// deepest cell the hull already has there, so every new cell mates the one
/// above it and the lance's `positive_y_mid` socket mates the last of them.
fn hull_with_lance(ships: &GameShipDesigns, hull: &str) -> ShipDesign {
    let mut built = kit::catalog_ship(ships, hull);
    let footing = built
        .sections
        .iter()
        .filter(|section| {
            (section.position.x - MOUNT.x).abs() < 1.0e-3
                && (section.position.z - MOUNT.z).abs() < 1.0e-3
        })
        .map(|section| section.position.y)
        .fold(f32::INFINITY, f32::min);
    assert!(
        footing.is_finite(),
        "railgun hulls: '{hull}' carries no cell on the mount column x={} z={}, so the pylon has \
         nothing to hang from",
        MOUNT.x,
        MOUNT.z,
    );
    assert!(
        footing > MOUNT.y,
        "railgun hulls: '{hull}' already fills the mount cell {MOUNT:?} - the station has to be \
         clear on BOTH hulls or the pair is not comparable",
    );

    let mut cell = footing - 1.0;
    while cell > MOUNT.y + 0.5 {
        built.sections.push(SpaceshipSectionConfig {
            id: format!("pylon_{}", (-cell) as i32),
            position: Vec3::new(MOUNT.x, cell, MOUNT.z),
            rotation: Quat::IDENTITY,
            source: SectionSource::prototype(REINFORCED_HULL_SECTION_ID),
        });
        cell -= 1.0;
    }
    built.sections.push(SpaceshipSectionConfig {
        id: LANCE_SECTION.to_string(),
        position: MOUNT,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(RAILGUN_LANCE_SECTION_ID),
    });
    built
}

/// One round's scene: the hull under test, flown by the player, and a column of
/// hull blocks standing on its bore a kilometre out.
fn hull_range(
    round: Round,
    game_assets: &GameAssets,
    ships: &GameShipDesigns,
    sections: &GameSections,
) -> ScenarioConfig {
    // Player-controlled with an EMPTY input mapping, exactly as
    // `system_railgun_lance`'s rig is: the bore sight is gated on
    // `PlayerSpaceshipMarker`, so the hull under test has to BE the player - and
    // nothing bound to a key can move it while the recoil is being read.
    let shooter = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            // None, not a cap: a capped hull would have its recoil clipped,
            // which is the one number claim 2 is reading.
            speed_cap: None,
        }),
        design: ShipDesignSource::Inline(hull_with_lance(ships, round.hull)),
        ..default()
    };

    let mut column: Vec<SectionSpec> = (0..TARGET_LAYERS)
        .map(|layer| {
            SectionSpec::new(
                format!("layer_{layer}"),
                REINFORCED_HULL_SECTION_ID,
                // Stacked toward the shooter: the slug arrives from +Z, so
                // layer 0 is the face it meets first.
                Vec3::new(0.0, 0.0, (TARGET_LAYERS - 1 - layer) as f32),
            )
        })
        .collect();
    column.push(SectionSpec::new(
        "core",
        BASIC_CONTROLLER_SECTION_ID,
        Vec3::new(0.0, 0.0, -1.0),
    ));
    let target = fixtures::ship(sections, SpaceshipController::None, &column);

    let spawn = |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        })
    };

    ScenarioConfig {
        description: "A shipped spinal lance bolted off the keel of one hull, and a column of \
                      blocks on its bore."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    spawn(SHIP_ID, round.name, Meters3::ZERO, shooter),
                    spawn(TARGET_ID, "Block Column", TARGET_AT, target),
                ],
                // The rig lights itself: the engine spawns no light, so a
                // scenario that authors none renders black.
                ThreePointRig::around("lance", Meters3::ZERO, 60.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            round.scenario.to_string(),
            "Railgun Hull Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

// --- What the range records --------------------------------------------------

/// Which round is on the range.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct Live(Round);

#[cfg(feature = "debug")]
impl Default for Live {
    fn default() -> Self {
        Self(LIGHT)
    }
}

/// The last look the HUD's own pass got at the bore sight.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Clone, Debug)]
struct Sight {
    /// Where the drawn line starts.
    origin: Vec3,
    /// The unit direction it runs in.
    bore: Vec3,
    /// How long it is, world units.
    length: f32,
    /// Every collider the sight says this shot would DESTROY.
    marks: Vec<Entity>,
}

/// The velocity pair the previous fixed tick ended on, and whether the shot went
/// out during this one.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Tick {
    previous: Option<(Vec3, Vec3)>,
    fired: bool,
}

/// Everything one hull's turn on the range produced.
#[cfg(feature = "debug")]
#[derive(Clone, Debug, Default)]
struct Reading {
    /// The shot left.
    fired: bool,
    /// Live sections on the hull when it fired.
    sections: usize,
    /// What avian weighed the hull at, kg.
    mass: f32,
    /// The largest principal moment of inertia - the axis the structural ceiling
    /// budgets against.
    inertia: f32,
    /// Centre of mass to the furthest live face, world units.
    arm: f32,
    /// Summed authored `max_torque` over the live flight computers.
    torque: f32,
    /// World centre of mass on the tick the shot went out.
    com: Vec3,
    /// Where the shot left from.
    muzzle: Vec3,
    /// The line the slug actually took, from its own spawn velocity.
    bore: Vec3,
    /// The lance's authored recoil impulse, read off the live gun.
    recoil_impulse: f32,
    /// The heading the computer was holding when the gun fired.
    held: Quat,
    /// Velocity step across the firing tick.
    delta_v: Vec3,
    /// Rate step across the same tick.
    delta_spin: Vec3,
    /// What the impulse over the measured mass says the velocity step should be.
    modelled_delta_v: Vec3,
    /// What the same impulse through the measured lever and inertia tensor says
    /// the rate step should be.
    modelled_delta_spin: Vec3,
    /// The angular impulse the lever turned the recoil into.
    torque_impulse: Vec3,
    /// Where the sight started the frame before the shot.
    sight_origin: Vec3,
    /// The direction it ran in.
    sight_bore: Vec3,
    /// Every collider it ringed.
    sight_marks: Vec<Entity>,
    /// How many of those were gone by the time the shot resolved.
    marks_destroyed: usize,
    /// Fastest the hull turned after the shot, rad/s.
    peak_spin: f32,
    /// Furthest the hull got from its held heading, rad.
    peak_heading: f32,
    /// How fast it was still turning when the recovery window closed.
    final_spin: f32,
    /// How far off it still was.
    final_heading: f32,
}

#[cfg(feature = "debug")]
impl Reading {
    /// The part of the lever that is actually off the bore - the part that turns
    /// a push into a spin.
    fn lever(&self) -> Vec3 {
        let arm = self.muzzle - self.com;
        arm - self.bore * arm.dot(self.bore)
    }

    /// The hull's own two attitude ceilings, from the numbers this run measured.
    fn envelope(&self) -> AttitudeEnvelope {
        AttitudeEnvelope::new(self.torque, self.inertia, Meters::from_engine(self.arm))
    }

    /// How far the measured velocity step sits from the modelled one, u/s.
    fn recoil_error(&self) -> f32 {
        (self.delta_v - self.modelled_delta_v).length()
    }

    /// How far the measured rate step sits from the modelled one, rad/s.
    fn spin_error(&self) -> f32 {
        (self.delta_spin - self.modelled_delta_spin).length()
    }
}

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Readings(BTreeMap<&'static str, Reading>);

/// Freeze what the sight was showing, and what the gun was authored with, on the
/// tick the shell actually leaves.
#[cfg(feature = "debug")]
fn record_the_shot(
    fired: On<RailgunFired>,
    live: Res<Live>,
    sight: Res<Sight>,
    mut tick: ResMut<Tick>,
    mut readings: ResMut<Readings>,
    q_lance: Query<&RailgunSectionConfigHelper>,
) {
    tick.fired = true;
    let entry = readings.0.entry(live.0.key).or_default();
    entry.fired = true;
    entry.muzzle = fired.muzzle;
    if let Ok(config) = q_lance.get(fired.entity) {
        entry.recoil_impulse = config.recoil_impulse;
    }
    entry.sight_origin = sight.origin;
    entry.sight_bore = sight.bore;
    entry.sight_marks.clone_from(&sight.marks);
}

/// Difference the hull's own velocities across the firing tick, and watch the
/// computer put the heading back afterwards.
#[cfg(feature = "debug")]
fn sample_the_hull(
    live: Res<Live>,
    mut tick: ResMut<Tick>,
    mut readings: ResMut<Readings>,
    q_hull: Query<
        (
            &LinearVelocity,
            &AngularVelocity,
            &Rotation,
            &Position,
            &ComputedMass,
            &ComputedAngularInertia,
            &ComputedCenterOfMass,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_slug: Query<&RoundVelocity, With<RailgunSlugProjectileMarker>>,
) {
    let Some((linear, angular, rotation, position, mass, inertia, centre)) = q_hull.iter().next()
    else {
        tick.previous = None;
        tick.fired = false;
        return;
    };
    let now = (linear.0, angular.0);

    if tick.fired {
        let previous = tick.previous.unwrap_or((Vec3::ZERO, Vec3::ZERO));
        // The slug's OWN velocity is the line the shot took. It is spawned
        // before the recoil is applied and the hull is at rest, so it carries no
        // muzzle motion and is the bore exactly.
        let bore = q_slug
            .iter()
            .next()
            .map(|velocity| velocity.0.normalize_or_zero())
            .unwrap_or_default();
        let entry = readings.0.entry(live.0.key).or_default();
        entry.bore = bore;
        entry.mass = mass.value();
        entry.inertia = inertia
            .principal_angular_inertia_with_local_frame()
            .0
            .max_element();
        entry.com = position.0 + rotation.0 * **centre;
        entry.delta_v = now.0 - previous.0;
        entry.delta_spin = now.1 - previous.1;

        // The model, in the same two steps production applies: the push over the
        // mass, and the lever's cross product through the world-space inverse
        // inertia tensor.
        let impulse = -bore * entry.recoil_impulse;
        entry.modelled_delta_v = impulse / mass.value().max(f32::EPSILON);
        entry.torque_impulse = (entry.muzzle - entry.com).cross(impulse);
        entry.modelled_delta_spin = inertia.rotated(rotation.0).inverse() * entry.torque_impulse;
    }
    tick.fired = false;
    tick.previous = Some(now);

    if let Some(entry) = readings.0.get_mut(live.0.key) {
        if entry.fired {
            entry.final_spin = now.1.length();
            entry.peak_spin = entry.peak_spin.max(entry.final_spin);
            entry.final_heading = rotation.0.angle_between(entry.held);
            entry.peak_heading = entry.peak_heading.max(entry.final_heading);
        }
    }
}

/// Read the drawn sight: where its line starts, where it runs, and every section
/// it has ringed as one this shot would take off.
#[cfg(feature = "debug")]
fn read_the_sight(
    mut sight: ResMut<Sight>,
    q_segment: Query<(&BoreSightSegment, &Transform)>,
    q_mark: Query<&BoreSightMark>,
) {
    // `segment_transform` builds the line as a unit cylinder along +Y: the
    // translation is its MIDPOINT, the rotation maps +Y onto the bore, and
    // `scale.y` is its length. (`scale.x`/`scale.z` carry the charge thickening
    // and say nothing about where the line goes.)
    let Some((_, transform)) = q_segment.iter().next() else {
        return;
    };
    let bore = transform.rotation * Vec3::Y;
    sight.length = transform.scale.y;
    sight.bore = bore;
    sight.origin = transform.translation - bore * (sight.length * 0.5);
    sight.marks = q_mark.iter().map(|mark| mark.target).collect();
}

// --- The scripted run --------------------------------------------------------

/// Both rounds, then the pair.
#[cfg(feature = "debug")]
fn hulls_script() -> Script {
    let mut script = Script::new();
    for (index, round) in ROUNDS.into_iter().enumerate() {
        script = round_beats(script, round, index == 0);
    }
    script.step("read the pair").on_enter(assert_the_pair).add()
}

/// One hull's turn: load it, let it settle, fire it, let the shot land, let the
/// computer put the heading back, and read it.
#[cfg(feature = "debug")]
fn round_beats(script: Script, round: Round, first: bool) -> Script {
    let key = round.key;
    let script = if first {
        script
            .step(format!("load the {key} hull"))
            .enter(GameStates::Loading)
            .until(round_ready(round))
            .deadline(STEP_DEADLINE_SECS)
            .add()
    } else {
        script
            // Waited on a DISTINCT scenario id, not on the old ship going away:
            // `on_load_scenario` queues the scoped despawns and fires
            // `OnStartEvent` on the same `Commands`, so the teardown and the
            // respawn land in one flush and a gone-then-present pair never
            // happens.
            .step(format!("load the {key} hull"))
            .on_enter(move |world: &mut World| load_round(world, round))
            .until(round_ready(round))
            .deadline(STEP_DEADLINE_SECS)
            .add()
    };
    script
        .step(format!("let the {key} hull settle"))
        .until(elapsed(SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("fire the {key} hull's lance"))
        .on_enter(move |world: &mut World| arm_the_lance(world, round))
        .until(the_shot_left(round))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("let the {key} hull's shot land"))
        // Waited on the TARGET dying, not on the rings the sight drew: the
        // rings are what claim 5 judges, and a beat holding until they were all
        // gone would make that judgement unfailable. The column dies whole or
        // the beat spends its window and the claim says which ring was a
        // promise the shot did not keep.
        .until(or(the_target_is_gone(), elapsed(IMPACT_SECS)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("read what the {key} hull's shot took"))
        .on_enter(move |world: &mut World| read_the_marks(world, round))
        .add()
        .step(format!("let the {key} hull's computer recover"))
        .until(or(the_hull_is_settled(round), elapsed(RECOVERY_SECS)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step(format!("read the {key} hull"))
        .on_enter(move |world: &mut World| log_round(world, round))
        .add()
}

/// Swap the scene for the next hull.
#[cfg(feature = "debug")]
fn load_round(world: &mut World, round: Round) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let ships = world.resource::<GameShipDesigns>();
        let sections = world.resource::<GameSections>();
        hull_range(round, game_assets, ships, sections)
    };
    world.insert_resource(Live(round));
    *world.resource_mut::<Sight>() = Sight::default();
    *world.resource_mut::<Tick>() = Tick::default();
    info!(
        "railgun hulls: loading the {} hull ({})",
        round.key, round.hull
    );
    world.trigger(LoadScenario(config));
}

/// Commit the shot, and record the heading the computer is holding as it does.
#[cfg(feature = "debug")]
fn arm_the_lance(world: &mut World, round: Round) {
    world.insert_resource(Live(round));
    let held = ship_rotation(world).unwrap_or(Quat::IDENTITY);
    world
        .resource_mut::<Readings>()
        .0
        .entry(round.key)
        .or_default()
        .held = held;
    let lance = lance_section(world).unwrap_or_else(|| {
        panic!(
            "railgun hulls: the {} hull carries no live lance at {MOUNT:?}",
            round.key
        )
    });
    // The scripted order, not a synthesized key: it holds the trigger until the
    // shell actually leaves and retires itself on the shot, so every gate the
    // gun has - the charge, the magazine, the live safety - stays in force.
    world.entity_mut(lance).insert(ScriptedRailgunOrder);
}

/// Count how many of the sections the sight ringed are actually gone.
#[cfg(feature = "debug")]
fn read_the_marks(world: &mut World, round: Round) {
    let marks = world
        .resource::<Readings>()
        .0
        .get(round.key)
        .map(|reading| reading.sight_marks.clone())
        .unwrap_or_default();
    let destroyed = marks
        .iter()
        .filter(|&&collider| destroyed(world, collider))
        .count();
    if let Some(reading) = world.resource_mut::<Readings>().0.get_mut(round.key) {
        reading.marks_destroyed = destroyed;
    }
    info!(
        "railgun hulls: the {} hull's sight ringed {} sections and {destroyed} of them died",
        round.key,
        marks.len(),
    );
}

/// Whether the collider carrying a health pool has stopped being a live one.
#[cfg(feature = "debug")]
fn destroyed(world: &World, collider: Entity) -> bool {
    world
        .get::<Health>(collider)
        .is_none_or(|health| health.current <= 0.0)
}

/// Close out one hull: fill in the structural numbers and say what it read.
#[cfg(feature = "debug")]
fn log_round(world: &mut World, round: Round) {
    let (sections, torque, arm) = hull_structure(world);
    {
        let mut readings = world.resource_mut::<Readings>();
        let reading = readings.0.entry(round.key).or_default();
        reading.sections = sections;
        reading.torque = torque;
        reading.arm = arm;
    }
    let reading = reading_of(world, round.key);
    let envelope = reading.envelope();
    info!(
        "railgun hulls: '{}' ({}): sections={} mass={:.0} kg inertia={:.4e} arm={:.1} m \
         lever={:.2} u impulse={:.1} dv={:.4} u/s (modelled {:.4}, off by {:.2e}) \
         dspin={:.5} rad/s (modelled {:.5}, off by {:.2e}) peak_spin={:.5} rad/s \
         sustained={:.4} rad/s peak_heading={:.5} rad final_spin={:.6} rad/s \
         final_heading={:.6} rad marks={}/{}",
        round.key,
        round.hull,
        reading.sections,
        reading.mass,
        reading.inertia,
        Meters::from_engine(reading.arm).get(),
        reading.lever().length(),
        reading.recoil_impulse,
        reading.delta_v.length(),
        reading.modelled_delta_v.length(),
        reading.recoil_error(),
        reading.delta_spin.length(),
        reading.modelled_delta_spin.length(),
        reading.spin_error(),
        reading.peak_spin,
        envelope.sustained_turn_rate(),
        reading.peak_heading,
        reading.final_spin,
        reading.final_heading,
        reading.marks_destroyed,
        reading.sight_marks.len(),
    );
}

// --- The claims --------------------------------------------------------------

/// Every claim, read off the pair the run measured.
#[cfg(feature = "debug")]
fn assert_the_pair(world: &mut World) {
    let light = reading_of(world, LIGHT.key);
    let heavy = reading_of(world, HEAVY.key);

    assert_the_station(world, &light, &heavy);
    assert_the_recoil(world, &light, &heavy);
    assert_the_spin(world, &light, &heavy);
    assert_the_recovery(world, &light, &heavy);
    assert_the_sight(world, &light, &heavy);
}

/// Claim 1: one gun, one station, two hulls that are actually a pair.
#[cfg(feature = "debug")]
fn assert_the_station(world: &mut World, light: &Reading, heavy: &Reading) {
    for (round, reading) in [(LIGHT, light), (HEAVY, heavy)] {
        assert!(
            reading.fired,
            "railgun hulls: the {} hull never fired, so nothing below is a reading",
            round.key,
        );
        assert!(
            reading.recoil_impulse > 0.0,
            "railgun hulls: the {} hull's lance reported no recoil impulse",
            round.key,
        );
        let lever = reading.lever().length();
        assert!(
            lever >= LEVER_FLOOR,
            "railgun hulls: the {} hull's muzzle sits {lever:.3} u off its own bore through the \
             centre of mass, under the {LEVER_FLOOR} u this range calls off-axis - a mount on the \
             spine spins nothing and proves nothing",
            round.key,
        );
    }
    assert!(
        (light.recoil_impulse - heavy.recoil_impulse).abs() < f32::EPSILON,
        "railgun hulls: the two hulls fired guns with different recoil ({} and {}) - the pair is \
         only comparable while it is the same prototype",
        light.recoil_impulse,
        heavy.recoil_impulse,
    );
    let ratio = heavy.mass / light.mass.max(f32::EPSILON);
    assert!(
        ratio >= MASS_RATIO_FLOOR,
        "railgun hulls: the heavy hull is only {ratio:.1}x the light one ({:.0} kg against {:.0} \
         kg) - under {MASS_RATIO_FLOOR}x the pair is not a scale test",
        heavy.mass,
        light.mass,
    );
    info!(
        "railgun hulls: one {:.1}-impulse lance at {MOUNT:?} on {:.0} kg and {:.0} kg, {ratio:.1}x \
         apart, levered {:.2} u and {:.2} u off the bore",
        light.recoil_impulse,
        light.mass,
        heavy.mass,
        light.lever().length(),
        heavy.lever().length(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: one lance sits at the same station on both hulls",
        serde_json::json!({
            "mount_cell": [MOUNT.x, MOUNT.y, MOUNT.z],
            "recoil_impulse": light.recoil_impulse,
            "light_mass_kg": light.mass,
            "heavy_mass_kg": heavy.mass,
            "mass_ratio": ratio,
            "light_lever_u": light.lever().length(),
            "heavy_lever_u": heavy.lever().length(),
            "light_sections": light.sections,
            "heavy_sections": heavy.sections,
        }),
    );
}

/// Claim 2: the push is the impulse over the mass, on each hull and between
/// them.
#[cfg(feature = "debug")]
fn assert_the_recoil(world: &mut World, light: &Reading, heavy: &Reading) {
    for (round, reading) in [(LIGHT, light), (HEAVY, heavy)] {
        let modelled = reading.modelled_delta_v;
        let error = reading.recoil_error();
        let slack = modelled.length() * RECOIL_TOLERANCE;
        assert!(
            error <= slack,
            "railgun hulls: the {} hull stepped {:?} u/s on the shot, and {:.1} of impulse over \
             the {:.0} kg avian weighed it at says {modelled:?} - {error:.5} u/s apart, over the \
             {slack:.5} allowed",
            round.key,
            reading.delta_v,
            reading.recoil_impulse,
            reading.mass,
        );
        // The push is BACK along the bore, not merely large.
        let along = reading.delta_v.dot(-reading.bore);
        assert!(
            along > 0.0,
            "railgun hulls: the {} hull's velocity step has {along:.5} u/s back along its own \
             bore - the recoil pushed it the wrong way",
            round.key,
        );
    }
    let step_ratio = light.delta_v.length() / heavy.delta_v.length().max(f32::EPSILON);
    let mass_ratio = heavy.mass / light.mass.max(f32::EPSILON);
    let drift = (step_ratio - mass_ratio).abs() / mass_ratio;
    assert!(
        drift <= RECOIL_TOLERANCE,
        "railgun hulls: the light hull's velocity step is {step_ratio:.2}x the heavy one's and the \
         heavy hull is {mass_ratio:.2}x the mass - {:.1}% apart, over the {:.1}% allowed. One \
         impulse over two masses has to be the inverse of the masses",
        drift * 100.0,
        RECOIL_TOLERANCE * 100.0,
    );
    info!(
        "railgun hulls: one {:.1}-impulse shot steps the light hull {:.4} u/s and the heavy one \
         {:.5} u/s, a {step_ratio:.2}x split against a {mass_ratio:.2}x mass split",
        light.recoil_impulse,
        light.delta_v.length(),
        heavy.delta_v.length(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: recoil moves each hull by its own measured mass",
        serde_json::json!({
            "light_delta_v_u_per_s": light.delta_v.length(),
            "light_modelled_u_per_s": light.modelled_delta_v.length(),
            "heavy_delta_v_u_per_s": heavy.delta_v.length(),
            "heavy_modelled_u_per_s": heavy.modelled_delta_v.length(),
            "light_error_u_per_s": light.recoil_error(),
            "heavy_error_u_per_s": heavy.recoil_error(),
            "step_ratio": step_ratio,
            "mass_ratio": mass_ratio,
            "drift": drift,
        }),
    );
}

/// Claim 3: the lever turns the same push into the hull's own spin, and leaves
/// it inside the hull's own structural ceiling.
#[cfg(feature = "debug")]
fn assert_the_spin(world: &mut World, light: &Reading, heavy: &Reading) {
    for (round, reading) in [(LIGHT, light), (HEAVY, heavy)] {
        let modelled = reading.modelled_delta_spin;
        assert!(
            modelled.length() > 0.0,
            "railgun hulls: the {} hull's lever produced no modelled spin at all - the reading \
             below would be vacuous",
            round.key,
        );
        let error = reading.spin_error();
        let slack = modelled.length() * SPIN_TOLERANCE;
        assert!(
            error <= slack,
            "railgun hulls: the {} hull stepped {:?} rad/s on the shot, and its own lever and \
             inertia tensor say {modelled:?} - {error:.6} rad/s apart, over the {slack:.6} allowed",
            round.key,
            reading.delta_spin,
        );
        let envelope = reading.envelope();
        let sustained = envelope.sustained_turn_rate();
        assert!(
            reading.peak_spin <= sustained,
            "railgun hulls: the {} hull's own shot left it turning at {:.4} rad/s, past the \
             {sustained:.4} rad/s its {:.1} m arm can hold - one shell should not tear the ship \
             that fired it",
            round.key,
            reading.peak_spin,
            Meters::from_engine(reading.arm).get(),
        );
    }
    info!(
        "railgun hulls: the lever spins the light hull {:.5} rad/s against a {:.4} rad/s ceiling, \
         and the heavy one {:.6} rad/s against {:.4}",
        light.peak_spin,
        light.envelope().sustained_turn_rate(),
        heavy.peak_spin,
        heavy.envelope().sustained_turn_rate(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the off-axis shot spins the hull inside its own limits",
        serde_json::json!({
            "light_delta_spin_rad_per_s": light.delta_spin.length(),
            "light_modelled_rad_per_s": light.modelled_delta_spin.length(),
            "light_error_rad_per_s": light.spin_error(),
            "light_peak_rad_per_s": light.peak_spin,
            "light_sustained_rad_per_s": light.envelope().sustained_turn_rate(),
            "light_inertia": light.inertia,
            "heavy_delta_spin_rad_per_s": heavy.delta_spin.length(),
            "heavy_modelled_rad_per_s": heavy.modelled_delta_spin.length(),
            "heavy_error_rad_per_s": heavy.spin_error(),
            "heavy_peak_rad_per_s": heavy.peak_spin,
            "heavy_sustained_rad_per_s": heavy.envelope().sustained_turn_rate(),
            "heavy_inertia": heavy.inertia,
        }),
    );
}

/// Claim 4: the kick landed, and the computer took it back out.
#[cfg(feature = "debug")]
fn assert_the_recovery(world: &mut World, light: &Reading, heavy: &Reading) {
    for (round, reading) in [(LIGHT, light), (HEAVY, heavy)] {
        let kick = reading.modelled_delta_spin.length() * KICK_GUARD_FRACTION;
        assert!(
            reading.peak_spin >= kick,
            "railgun hulls: the {} hull only ever reached {:.6} rad/s after the shot, under the \
             {kick:.6} its own lever asked for - nothing was there to recover from",
            round.key,
            reading.peak_spin,
        );
        let residual = reading.peak_spin * RESIDUAL_FRACTION;
        assert!(
            reading.final_spin <= residual,
            "railgun hulls: the {} hull is still turning at {:.6} rad/s after the recovery window, \
             over the {residual:.6} that is {:.0}% of the {:.6} the shot left it at",
            round.key,
            reading.final_spin,
            RESIDUAL_FRACTION * 100.0,
            reading.peak_spin,
        );
        assert!(
            reading.final_heading <= RECONVERGE_TOLERANCE_RAD,
            "railgun hulls: the {} hull is still {:.5} rad off the heading it was holding, over \
             the {RECONVERGE_TOLERANCE_RAD} allowed",
            round.key,
            reading.final_heading,
        );
    }
    // Non-vacuity, stated on the PAIR: a capital is barely moved by one lance,
    // so demanding a visible heading excursion from it would be demanding the
    // game be wrong. What must be true is that the reading can see one at all.
    let thrown = light.peak_heading.max(heavy.peak_heading);
    let guard = RECONVERGE_TOLERANCE_RAD * HEADING_GUARD_RATIO;
    assert!(
        thrown >= guard,
        "railgun hulls: the furthest either hull was thrown off its heading was {thrown:.5} rad, \
         under the {guard:.5} this reading needs to be about anything",
    );
    info!(
        "railgun hulls: the computer took {:.5} rad off the light hull and {:.6} off the heavy \
         one, leaving {:.6} and {:.6} rad",
        light.peak_heading, heavy.peak_heading, light.final_heading, heavy.final_heading,
    );
    nova_probe::probe_marker(
        world,
        "outcome: the computer takes the recoil back out of the heading",
        serde_json::json!({
            "light_peak_heading_rad": light.peak_heading,
            "light_final_heading_rad": light.final_heading,
            "light_peak_spin_rad_per_s": light.peak_spin,
            "light_final_spin_rad_per_s": light.final_spin,
            "heavy_peak_heading_rad": heavy.peak_heading,
            "heavy_final_heading_rad": heavy.final_heading,
            "heavy_peak_spin_rad_per_s": heavy.peak_spin,
            "heavy_final_spin_rad_per_s": heavy.final_spin,
            "tolerance_rad": RECONVERGE_TOLERANCE_RAD,
        }),
    );
}

/// Claim 5: the instrument and the shell agree.
#[cfg(feature = "debug")]
fn assert_the_sight(world: &mut World, light: &Reading, heavy: &Reading) {
    for (round, reading) in [(LIGHT, light), (HEAVY, heavy)] {
        assert!(
            reading.sight_bore.length() > 0.0,
            "railgun hulls: the {} hull drew no bore sight at all - the HUD is gated on the live \
             weapons safety, so a cold ship reads as agreement with nothing",
            round.key,
        );
        let origin_gap = (reading.sight_origin - reading.muzzle).length();
        assert!(
            origin_gap <= SIGHT_ORIGIN_TOLERANCE,
            "railgun hulls: the {} hull's sight starts {origin_gap:.4} u from the muzzle the shot \
             actually left, over the {SIGHT_ORIGIN_TOLERANCE} allowed",
            round.key,
        );
        let angle = reading.sight_bore.angle_between(reading.bore);
        assert!(
            angle <= SIGHT_ANGLE_TOLERANCE_RAD,
            "railgun hulls: the {} hull's sight runs {angle:.5} rad off the line the slug took, \
             over the {SIGHT_ANGLE_TOLERANCE_RAD} allowed",
            round.key,
        );
        assert!(
            !reading.sight_marks.is_empty(),
            "railgun hulls: the {} hull's sight ringed nothing, so 'what it ringed died' is \
             vacuous",
            round.key,
        );
        assert_eq!(
            reading.marks_destroyed,
            reading.sight_marks.len(),
            "railgun hulls: the {} hull's sight ringed {} sections and only {} of them died - the \
             sight promised a kill the shot did not deliver",
            round.key,
            reading.sight_marks.len(),
            reading.marks_destroyed,
        );
    }
    info!(
        "railgun hulls: the sight and the slug agree to {:.6} rad on the light hull and {:.6} on \
         the heavy one, over {} and {} ringed sections",
        light.sight_bore.angle_between(light.bore),
        heavy.sight_bore.angle_between(heavy.bore),
        light.sight_marks.len(),
        heavy.sight_marks.len(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the bore sight and the slug agree where the shot went",
        serde_json::json!({
            "light_origin_gap_u": (light.sight_origin - light.muzzle).length(),
            "light_angle_rad": light.sight_bore.angle_between(light.bore),
            "light_marks": light.sight_marks.len(),
            "light_marks_destroyed": light.marks_destroyed,
            "heavy_origin_gap_u": (heavy.sight_origin - heavy.muzzle).length(),
            "heavy_angle_rad": heavy.sight_bore.angle_between(heavy.bore),
            "heavy_marks": heavy.sight_marks.len(),
            "heavy_marks_destroyed": heavy.marks_destroyed,
        }),
    );
}

// --- Lookups and beats -------------------------------------------------------

/// One round's reading, cloned out: every caller here goes on to take
/// `&mut World` for a probe marker.
#[cfg(feature = "debug")]
fn reading_of(world: &mut World, key: &'static str) -> Reading {
    world
        .resource::<Readings>()
        .0
        .get(key)
        .cloned()
        .unwrap_or_else(|| panic!("railgun hulls: the {key} hull left no reading"))
}

/// The live player hull's section census, summed computer torque and structural
/// arm.
#[cfg(feature = "debug")]
fn hull_structure(world: &mut World) -> (usize, f32, f32) {
    let Some(root) = player_hull(world) else {
        return (0, 0.0, 0.0);
    };
    let sections = world
        .query_filtered::<&ChildOf, (With<SectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();
    let torque: f32 = world
        .query_filtered::<(&ControllerSectionTuning, &ChildOf), Without<SectionInactiveMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == root)
        .map(|(tuning, _)| tuning.max_torque.max(0.0))
        .sum();
    let arm = world.get::<HullRadius>(root).map_or(0.0, |radius| **radius);
    (sections, torque, arm)
}

/// The player hull, off a borrowed world.
#[cfg(feature = "debug")]
fn player_hull(world: &World) -> Option<Entity> {
    let mut query = world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?;
    query.iter(world).next()
}

/// The player hull's live attitude.
#[cfg(feature = "debug")]
fn ship_rotation(world: &World) -> Option<Quat> {
    let root = player_hull(world)?;
    world.get::<Rotation>(root).map(|rotation| rotation.0)
}

/// The lance bolted to the player hull.
#[cfg(feature = "debug")]
fn lance_section(world: &mut World) -> Option<Entity> {
    let root = player_hull(world)?;
    let mut query = world.try_query_filtered::<(Entity, &ChildOf), (
        With<RailgunSectionMarker>,
        Without<SectionInactiveMarker>,
    )>()?;
    query
        .iter(world)
        .find(|(_, parent)| parent.parent() == root)
        .map(|(entity, _)| entity)
}

/// The round's scene is up AND weighed AND hot: a root avian has not measured
/// yet publishes no inertia, a cold ship draws no sight, and a hull with no
/// target downrange has nothing for the sight to ring.
#[cfg(feature = "debug")]
fn round_ready(round: Round) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        let scenario_named = world
            .get_resource::<CurrentScenario>()
            .and_then(|current| current.0.as_ref())
            .is_some_and(|config| config.id == round.scenario);
        if !scenario_named {
            return false;
        }
        let Some(root) = player_hull(world) else {
            return false;
        };
        let weighed = world
            .get::<ComputedAngularInertia>(root)
            .is_some_and(|inertia| {
                inertia
                    .principal_angular_inertia_with_local_frame()
                    .0
                    .max_element()
                    > 0.0
            })
            && world.get::<HullRadius>(root).is_some();
        let hot = world.get::<WeaponsHot>(root).is_some_and(|hot| hot.0);
        let lance = world
            .try_query_filtered::<&ChildOf, (
                With<RailgunSectionMarker>,
                Without<SectionInactiveMarker>,
            )>()
            .is_some_and(|mut query| {
                query.iter(world).any(|parent| parent.parent() == root)
            });
        let target = world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == TARGET_ID));
        weighed && hot && lance && target
    })
}

/// The round's shell has left.
#[cfg(feature = "debug")]
fn the_shot_left(round: Round) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Readings>(move |readings| {
        readings.0.get(round.key).is_some_and(|reading| {
            reading.fired && reading.bore.length() > 0.0 && reading.delta_v.length() > 0.0
        })
    })
}

/// The column the shot was fired at has come apart.
#[cfg(feature = "debug")]
fn the_target_is_gone() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| !query.iter(world).any(|id| id.0 == TARGET_ID))
    })
}

/// The hull has stopped turning - the settling condition, well inside the
/// verdict claim 4 goes on to read.
#[cfg(feature = "debug")]
fn the_hull_is_settled(round: Round) -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Readings>(move |readings| {
        readings.0.get(round.key).is_some_and(|reading| {
            reading.fired
                && reading.peak_spin > 0.0
                && reading.final_spin <= (reading.peak_spin * SETTLED_FRACTION).max(SETTLED_FLOOR)
        })
    })
}
