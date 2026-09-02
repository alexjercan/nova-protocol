//! The scenario the editor hands off to on Play: an open range for the ship
//! you just built, and the DEFAULT WORLD a new document is seeded with.
//!
//! It is a SANDBOX, so it deliberately has no win: one standing objective that
//! never completes, no chapter to chain into, and the only outcome is your own
//! death - which offers a retry of this same range. F1 is the way out.
//!
//! ENGINE UNITS in this layout. The positions, radii and reaches below are
//! world units - one is 10 m - because they are checked against the editor
//! stage's own geometry and against avian-derived figures (an SOI radius, a
//! noise-displaced body radius). They cross into the authored register once,
//! where the scenario config is built, through `Meters::from_engine` /
//! `Meters3::from_engine`; nothing here does the arithmetic by hand.
//!
//! What is out there: two seeded rock belts, a corridor of inert target hulks
//! to shoot, three DORMANT pickets that only fight once you paint or crowd
//! them (the farthest of which mounts the spinal lance), two beacons that swap the sky, one planetoid parked far enough away to
//! be scenery rather than a wall (see [`PLANETOID_POSITION`]), and the light
//! rig that makes any of it visible.
//!
//! The split down the middle of this file is the LOWERING CONVENTION, and it
//! now has only one thing on the far side of it. The objects are LAYOUT and
//! become editor nodes ([`default_world_objects`] seeds them, [`lower_objects`]
//! reads them back); the belts, the trip spheres, the wake handlers and the
//! outcome are the SCRIPT and are seeded the same way ([`default_script`] seeds
//! them, [`crate::event`] reads them back). What stays derived is the one
//! handler that SPAWNS the world: it is the layout written as an action list,
//! and a copy of it in the tree could disagree with the tree.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::{Allegiance, AssetRef};
use nova_input::prelude::InputSource;
use nova_scenario::prelude::*;
use nova_ship::prelude::{
    BASIC_CONTROLLER_SECTION_ID, BASIC_THRUSTER_SECTION_ID, LIGHT_HULL_SECTION_ID,
    PDC_KINETIC_TURRET_SECTION_ID, RAILGUN_LANCE_SECTION_ID, REINFORCED_HULL_SECTION_ID,
};

use crate::{
    asset_index::prelude::AssetIndex,
    event::{named_ids, NamedIds, ScriptNodes},
    node::{
        objects_of, sections_of, EditContext, NodeId, ObjectNodes, ScenarioNode, SectionNodes,
        ShipDriver, ShipNode, ASTEROID_TEXTURE, DESTROY_SOUND, SHIP_COLLAPSE_SOUND,
    },
};

/// The sandbox's scenario id. Registered in [`GameScenarios`] on hand-off so
/// the DEFEAT overlay's Retry can reload it by id like any other scenario;
/// `hidden` keeps it out of the Scenarios picker, which lists shipped content.
pub(crate) const SANDBOX_ID: &str = "editor_sandbox";
/// The player ship's scenario id, referenced by every handler that scopes to
/// the player and by the editor's input mapping.
pub(crate) const PLAYER_ID: &str = "player_spaceship";

/// The planetoid: far enough that it reads as a destination.
///
/// At 314u from the spawn the player is INSIDE it in every sense that matters:
/// an asteroid's real surface is `radius *` 3.5-6.0 (the
/// noise mesh displaces outward - see `ASTEROID_GEOMETRIC_FACTOR_MAX`), so a
/// 55u planetoid was a ~250u ball of rock, and its well reached
/// `sqrt(mu / soi_cutoff_accel)` ~ 1095u. You spawned ~60u off its surface and
/// fell in. Both numbers are derived from authored data, so
/// `the_spawn_is_clear_of_the_planetoid` recomputes them rather than trusting
/// this comment.
const PLANETOID_POSITION: Vec3 = Vec3::new(-560.0, -110.0, -380.0);
/// Nominal planetoid radius; the drawn body is 3.5-6.0x this.
const PLANETOID_RADIUS: f32 = 24.0;
/// Planetoid mass parameter, authored by the REACH it buys:
/// `mu = soi_cutoff_accel * soi^2` at the shipped 0.25 cutoff gives a 400u
/// sphere of influence - a well the player can go looking for, and cannot fall
/// into by accident from the spawn.
const PLANETOID_MASS: f32 = 40_000.0;
/// Pinned silhouette. An unseeded rock redraws itself every load, and this
/// body is the one thing here whose real radius the layout is measured
/// against.
const PLANETOID_SEED: u32 = 20_260_815;

/// The inert hulks, port-forward of the spawn: a corridor of things to shoot
/// that shoot back at nobody. Clear of both belt boxes by construction.
const HULK_POSITIONS: [Vec3; 5] = [
    Vec3::new(-60.0, 20.0, -170.0),
    Vec3::new(-140.0, -25.0, -240.0),
    Vec3::new(-70.0, 50.0, -300.0),
    Vec3::new(-200.0, 15.0, -360.0),
    Vec3::new(-95.0, -55.0, -430.0),
];

/// One dormant picket: where it sits, how far its proximity trip reaches, and
/// the callsign it answers on when it wakes.
struct Picket {
    id: &'static str,
    name: &'static str,
    callsign: &'static str,
    position: Vec3,
    /// Radius of the sphere that wakes it when the player flies in.
    trip_radius: f32,
    /// Carries the spinal lance as well as the shared PDC.
    ///
    /// One picket, not three: a lance is a telegraphed 1.5 s charge followed
    /// by a shot that guts whatever the bore was on, and meeting three of
    /// them at once is a range that kills you for exploring it.
    spinal: bool,
}

/// The three pickets. They spawn NEUTRAL with a live AI pilot, which is what
/// makes them dormant rather than scripted-asleep: the AI's target acquisition
/// only considers `Relation::Hostile` contacts, and a neutral ship has none.
/// Flipping the allegiance is the whole wake-up.
const PICKETS: [Picket; 3] = [
    Picket {
        id: "picket_warden",
        name: "Picket Warden",
        callsign: "Warden",
        position: Vec3::new(520.0, -45.0, -290.0),
        trip_radius: 150.0,
        spinal: false,
    },
    Picket {
        id: "picket_sentinel",
        name: "Picket Sentinel",
        callsign: "Sentinel",
        position: Vec3::new(-300.0, 70.0, -680.0),
        trip_radius: 150.0,
        spinal: false,
    },
    Picket {
        id: "picket_lance",
        name: "Picket Lance",
        callsign: "Lance",
        position: Vec3::new(-20.0, 130.0, -740.0),
        trip_radius: 170.0,
        spinal: true,
    },
];

/// One rock belt: a seeded box of asteroids.
struct Belt {
    id_prefix: &'static str,
    name: &'static str,
    seed: u64,
    count: u32,
    min: Vec3,
    max: Vec3,
    radius: (f32, f32),
    /// Centre-to-centre clearance. Sized on the widest two rocks side by side
    /// (`radius.1 * ASTEROID_GEOMETRIC_FACTOR_MAX`, doubled): overlapping
    /// dynamic bodies are shoved apart on the first physics step hard enough
    /// to damage each other. `ScatterObjects` measures against every body
    /// scattered so far, this scenario's earlier belts included, so the two
    /// boxes may abut.
    separation: f32,
}

/// The belts, as boxes rather than rings so the hand-placed hulks, pickets and
/// beacons can be kept out of them by inspection - which
/// `hand_placed_bodies_stay_out_of_the_belts` then checks.
const BELTS: [Belt; 2] = [
    Belt {
        id_prefix: "shallow_rock_",
        name: "Shallow Rock",
        seed: 0x5A11_0B37,
        count: 30,
        min: Vec3::new(70.0, -50.0, -440.0),
        max: Vec3::new(430.0, 50.0, -40.0),
        radius: (1.0, 3.0),
        separation: 45.0,
    },
    Belt {
        id_prefix: "deep_rock_",
        name: "Deep Rock",
        seed: 0xDEE9_0C11,
        count: 34,
        min: Vec3::new(-280.0, -60.0, -840.0),
        max: Vec3::new(300.0, 60.0, -540.0),
        radius: (1.5, 4.0),
        separation: 55.0,
    },
];

/// One sky beacon: fly through it and the cubemap changes.
struct SkyBeacon {
    id: &'static str,
    name: &'static str,
    label: &'static str,
    position: Vec3,
    trip_radius: f32,
    color: Color,
    /// The cubemap entering it installs.
    cubemap: &'static str,
    /// The comms line it answers with.
    line: &'static str,
}

/// The two beacons, one per shipped cubemap. Both are DIRECT asset paths for
/// the same reason the sounds below are: they are built in code and always
/// name base-game art, so a scheme buys them nothing. A ref the BUILDER picks
/// does carry one, and [`AssetIndex::resolved`] resolves it on the way to
/// Play.
const SKY_BEACONS: [SkyBeacon; 2] = [
    SkyBeacon {
        id: "beacon_veil",
        name: "Veil Beacon",
        label: "VEIL",
        position: Vec3::new(-430.0, 140.0, -690.0),
        trip_radius: 130.0,
        color: Color::srgb(0.70, 0.35, 1.0),
        cubemap: "base/textures/cubemap_alt.png",
        line: "Veil relay - you are under the other sky now.",
    },
    SkyBeacon {
        id: "beacon_home",
        name: "Home Beacon",
        label: "HOME",
        position: Vec3::new(40.0, 0.0, 260.0),
        trip_radius: 70.0,
        color: Color::srgb(0.20, 0.90, 1.0),
        cubemap: DEFAULT_SKY,
        line: "Home relay - the old sky is back.",
    },
];

/// Lock range a beacon needs to be designatable from the spawn: the radar
/// gives a signed body `signature_range_per_unit` (30) x this. 30 buys 900u,
/// which covers the deepest beacon.
const BEACON_LOCK_SIGNATURE: f32 = 30.0;

pub(crate) fn setup_scenario(
    mut commands: Commands,
    context: Res<EditContext>,
    nodes: SectionNodes,
    q_objects: ObjectNodes,
    q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>,
    q_settings: Query<&ScenarioNode>,
    script: ScriptNodes,
    assets: AssetIndex,
    // Optional: the registry is written by the bundle merge, and a rig that
    // never merged content must still be able to fly the sandbox - it just
    // does not get the retry.
    scenarios: Option<ResMut<GameScenarios>>,
) {
    let scenario = assets.resolved(sandbox_scenario(
        &world_settings(&context, &q_settings),
        world_objects(&context, &q_objects),
        &lower_fleet(&q_ships, &nodes),
        world_script(&context, &script),
    ));

    // Re-register with the ship the editor just built: the boot-time entry
    // (`register_sandbox_scenario`) carries the DEFAULT hull, and the DEFEAT
    // overlay's Retry resolves the queued `NextScenario` against this registry.
    if let Some(mut scenarios) = scenarios {
        scenarios.insert(scenario.id.clone(), scenario.clone());
    }

    commands.trigger(LoadScenario(scenario));
}

/// Whether the sandbox is absent from [`GameScenarios`] - the run condition of
/// the repair pass below.
pub(crate) fn sandbox_unregistered(scenarios: Option<Res<GameScenarios>>) -> bool {
    scenarios.is_some_and(|scenarios| !scenarios.contains_key(SANDBOX_ID))
}

/// Put the sandbox in [`GameScenarios`] with the DEFAULT hull, so its id exists
/// before anything asks for it by name.
///
/// The sandbox used to register only on the editor's Play hand-off, which made
/// it the one scenario no id-driven caller could reach: the game binary's
/// `--scenario` membership check, the picker's hidden launch and the probe's
/// scenario runner all resolve ids against this registry long before Play. It
/// is registered here for the same reason every shipped scenario is registered
/// at load - an id nothing can name is not content.
///
/// [`setup_scenario`] overwrites the entry with the built ship on hand-off.
pub(crate) fn register_sandbox_scenario(
    context: Res<EditContext>,
    nodes: SectionNodes,
    q_objects: ObjectNodes,
    q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>,
    q_settings: Query<&ScenarioNode>,
    script: ScriptNodes,
    assets: AssetIndex,
    mut scenarios: ResMut<GameScenarios>,
) {
    let scenario = assets.resolved(sandbox_scenario(
        &world_settings(&context, &q_settings),
        world_objects(&context, &q_objects),
        &lower_fleet(&q_ships, &nodes),
        world_script(&context, &script),
    ));
    scenarios.insert(scenario.id.clone(), scenario);
}

/// The world the sandbox spawns: the DOCUMENT's objects once a document exists,
/// and the stock range before one does.
///
/// Keyed on the document existing, never on it being empty. A builder who
/// deletes every rock gets an empty range, which is the whole point of the
/// world being editable; an id registered before the editor has ever opened
/// still has to name something, because an id nothing can fly is not content.
pub(crate) fn world_objects(
    context: &EditContext,
    q_objects: &ObjectNodes,
) -> Vec<ScenarioObjectConfig> {
    match context.scenario() {
        Some(scenario) => lower_objects(scenario, q_objects),
        None => default_world_objects(),
    }
}

/// What the sandbox is CALLED and comes up under: the DOCUMENT's settings once
/// a document exists, and the stock ones before one does.
///
/// The same rule [`world_objects`] follows. An id registered before the editor
/// has ever opened still has to name a range with a sky.
pub(crate) fn world_settings(
    context: &EditContext,
    q_settings: &Query<&ScenarioNode>,
) -> ScenarioNode {
    context
        .scenario()
        .and_then(|scenario| q_settings.get(scenario).ok())
        .cloned()
        .unwrap_or_default()
}

/// The script the sandbox runs: the DOCUMENT's handlers once a document
/// exists, and the stock range's own before one does.
///
/// The same rule [`world_objects`] follows, for the same reason: a document
/// whose script the builder emptied runs no script, and an id registered
/// before the editor has ever opened still has to name a range that works.
pub(crate) fn world_script(
    context: &EditContext,
    script: &ScriptNodes,
) -> Vec<ScenarioEventConfig> {
    match context.scenario() {
        Some(scenario) => script.lower(scenario),
        None => default_script(),
    }
}

/// Every object node of the document, flattened back into the config it was
/// lifted from.
///
/// The node's `NodeId` is the scenario object id and the node's `Transform` is
/// the pose - the two facts the editor moves - so this is the whole of the
/// lowering. In id order, because the output is a file.
fn lower_objects(scenario: Entity, q_objects: &ObjectNodes) -> Vec<ScenarioObjectConfig> {
    objects_of(scenario, q_objects)
        .into_iter()
        .map(|(_, id, object, transform)| ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.0.clone(),
                name: object.name.clone(),
                position: Meters3::from_engine(transform.translation),
                rotation: transform.rotation,
            },
            kind: object.kind.clone(),
        })
        .collect()
}

/// The stock sandbox world, as authored objects: the planetoid, the hulk
/// corridor, the pickets, the beacons and the light rig.
///
/// What a new document is SEEDED with (see `crate::node::ensure_document`), and
/// what the registry advertises before one exists. These were constants baked
/// into the hand-off; they are the default world now, and everything about them
/// is editable once the editor opens.
///
/// The rock belts are NOT here - they are seeded scatter actions (see
/// [`belt_scatter`]), so the field is the same field on every load and on every
/// retry, and one node per rock would be sixty-four rows in the tree.
pub(crate) fn default_world_objects() -> Vec<ScenarioObjectConfig> {
    let mut objects = vec![planetoid()];
    objects.extend(
        HULK_POSITIONS
            .iter()
            .enumerate()
            .map(|(index, position)| target_hulk(index, *position)),
    );
    objects.extend(PICKETS.iter().map(picket_ship));
    objects.extend(SKY_BEACONS.iter().map(sky_beacon));
    // The sandbox lights itself: the engine spawns no light, so a scenario that
    // authors none renders black.
    objects.extend(ThreePointRig::around("sandbox", Meters3::ZERO, 10.0).objects());
    objects
}

/// One ship, flattened out of its subtree.
fn lower_ship(
    entity: Entity,
    id: &NodeId,
    ship: &ShipNode,
    pose: Transform,
    nodes: &SectionNodes,
) -> LoweredShip {
    let placed = sections_of(entity, nodes);
    LoweredShip {
        id: id.0.clone(),
        name: ship.name.clone(),
        driver: ship.driver,
        allegiance: ship.allegiance,
        pilot: ship.pilot.clone(),
        sections: placed
            .iter()
            .map(|(_, id, section, transform)| SpaceshipSectionConfig {
                id: id.0.clone(),
                position: transform.translation,
                rotation: transform.rotation,
                source: section.source.clone(),
                modifications: section.modifications.clone(),
            })
            .collect(),
        inputs: placed
            .iter()
            .filter(|(_, _, section, _)| !section.binds.is_empty())
            .map(|(_, id, section, _)| (id.0.clone(), section.binds.clone()))
            .collect(),
        skin: ship.skin,
        style: ship.style.clone(),
        position: pose.translation,
        rotation: pose.rotation,
    }
}

/// EVERY ship of the document, lowered out of it.
///
/// A query WALK, not a resource read: a ship is a subtree, and this is the
/// step that flattens each into the shape the scenario loader consumes.
///
/// A ship's pose is its OWN. The stage is one space and every node stands in
/// it, so a ship flies from the point it was dragged to and turning or moving
/// one moves nothing else. An earlier rule made the player's ship an anchor
/// that pinned the whole fleet to the range origin, which meant dragging the
/// only ship in a document looked like it did nothing at all. A document with
/// no player ship still produces the same empty hull an untouched editor
/// always handed over.
pub(crate) fn lower_fleet(
    q_ships: &Query<(Entity, &NodeId, &ShipNode, &Transform)>,
    nodes: &SectionNodes,
) -> LoweredFleet {
    let mut fleet = LoweredFleet::default();
    let mut ships: Vec<_> = q_ships.iter().collect();
    ships.sort_unstable_by(|a, b| a.1.cmp(b.1));
    for (entity, id, ship, transform) in ships {
        let lowered = lower_ship(entity, id, ship, *transform, nodes);
        match ship.driver {
            ShipDriver::Player => fleet.player = lowered,
            ShipDriver::Ai | ShipDriver::Adrift => fleet.standing.push(lowered),
        }
    }
    fleet
}

/// One lowered ship as the scenario wants it: a flat section list, the input
/// mapping keyed by those sections' STABLE ids, the skin choice and where
/// on the range it spawns.
#[derive(Default)]
pub(crate) struct LoweredShip {
    /// The SHIP NODE's id, which is also the prototype id a save writes this
    /// design under. Empty only for the placeholder hull a document with no
    /// player ship hands over - there is no node behind that one to name.
    pub(crate) id: String,
    /// What the builder called it, or empty where nothing did. The flown
    /// scenario shows it, which is the whole point of naming a ship.
    pub(crate) name: String,
    /// Who is at the controls, which is what decides the spawn's controller.
    driver: ShipDriver,
    /// Which side it fights for, or `None` to take the driver's default.
    allegiance: Option<Allegiance>,
    /// The AI pilot's standing orders, used only by an AI-driven ship.
    pilot: AIControllerConfig,
    pub(crate) sections: Vec<SpaceshipSectionConfig>,
    inputs: Vec<(SectionId, Vec<InputSource>)>,
    pub(crate) skin: bool,
    pub(crate) style: Option<String>,
    position: Vec3,
    rotation: Quat,
}

/// The whole document, lowered: the player's ship and every hull standing
/// beside it - escorts, seeded pickets, derelict hulks.
#[derive(Default)]
pub(crate) struct LoweredFleet {
    player: LoweredShip,
    standing: Vec<LoweredShip>,
}

impl LoweredFleet {
    /// Every design in the document that a node stands behind, in id order -
    /// what a save writes one ship prototype per.
    ///
    /// The placeholder player hull is skipped: it names no node, so there is
    /// nothing for an instance to reference and nothing to write.
    pub(crate) fn designs(&self) -> impl Iterator<Item = &LoweredShip> {
        std::iter::once(&self.player)
            .chain(self.standing.iter())
            .filter(|ship| !ship.id.is_empty())
    }
}

/// How a lowered ship names its hull in the config it goes into.
///
/// The same document lowers both ways: Play hands the runtime a hull it can
/// spawn with no catalog behind it, and a SAVE writes a reference to the ship
/// prototype the same file carries - so editing a design changes every
/// instance of it and the file never holds two copies of one hull.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HullForm {
    /// The whole section list, written out where the ship is spawned.
    Inline,
    /// A reference to the ship prototype of the same id.
    Prototype,
}

/// The hull a lowered ship spawns, in the form asked for.
///
/// A design with no node behind it falls back to `Inline` whatever was asked:
/// a prototype reference to nothing spawns nothing, and the empty hull an
/// untouched editor hands over has to stay an empty hull.
fn hull_of(ship: &LoweredShip, form: HullForm) -> ShipSource {
    match form {
        HullForm::Prototype if !ship.id.is_empty() => ShipSource::Prototype(ship.id.clone()),
        _ => ShipSource::Inline(ship_hull(ship)),
    }
}

/// The design itself: what is intrinsic to the hull, apart from any one spawn.
pub(crate) fn ship_hull(ship: &LoweredShip) -> ShipHull {
    ShipHull {
        sections: ship.sections.clone(),
        skin: ship.skin,
        style: ship.style.clone(),
        collapse_sound: Some(AssetRef::from(SHIP_COLLAPSE_SOUND)),
        ..default()
    }
}

/// Build the range: the world's objects, two rock belts, the wake script, and
/// the ships the editor just built.
///
/// One body for both readers of the document. `id` and `form` are what they
/// disagree about: the Play hand-off builds the hidden sandbox with hulls
/// written out inline, and a SAVE builds a scenario of its own id whose ships
/// reference the prototypes the same file carries.
///
/// `settings` is what the BUILDER said and `range` is what the build target
/// decides, which is why the sky arrives with the first and the id with the
/// second.
pub(crate) fn range_scenario(
    settings: &ScenarioNode,
    range: Range<'_>,
    world: Vec<ScenarioObjectConfig>,
    fleet: &LoweredFleet,
    script: Vec<ScenarioEventConfig>,
) -> ScenarioConfig {
    ScenarioConfig {
        description: settings.description.clone(),
        skybox_brightness: settings.skybox_brightness,
        hidden: range.hidden,
        events: range_events(
            range.id,
            sandbox_objects(world, fleet, range.form, range.flight),
            script,
        ),
        ..ScenarioConfig::new(
            range.id.to_string(),
            settings.name.clone(),
            settings.cubemap.clone(),
        )
    }
}

/// The sky a range comes up under, as a PATH.
///
/// A save cannot write a resolved handle: a handle has no authorable form, so
/// a file written from one refuses to serialize. The path is the same sky the
/// home beacon swaps back to.
pub(crate) const DEFAULT_SKY: &str = "base/textures/cubemap.png";

/// What a new document calls itself until the builder renames it.
pub(crate) const DEFAULT_SCENARIO_NAME: &str = "Saved Range";

/// What a new document says about itself, which is what the stock world is.
pub(crate) const DEFAULT_SCENARIO_DESCRIPTION: &str =
    "A free-flight range: rocks, target hulks, dormant pickets and a planetoid.";

/// Which range a lowering is building: its id, whether the Scenarios picker
/// lists it, how its ships name their hulls, and whether it may stand in for
/// what the document has not built yet.
///
/// The name, description and sky are NOT here: those the builder authors on
/// the [`ScenarioNode`], and both targets write what they said.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Range<'a> {
    pub(crate) id: &'a str,
    /// Kept out of the Scenarios picker, which lists shipped content.
    pub(crate) hidden: bool,
    pub(crate) form: HullForm,
    /// Whether the range may compose a FLIGHT rather than the document.
    ///
    /// Play tolerates an unfinished document: one with no player ship still
    /// gets a hull to sit in, and a ship node with no sections spawns nothing
    /// to fly into. A save must not. A file holding more or less than the
    /// document is not the document, and opening it hands the builder back
    /// something they did not build.
    pub(crate) flight: bool,
}

/// The range Play hands off to: registered so the DEFEAT overlay's Retry can
/// find it by id, hidden so it never stands next to shipped content, and
/// carrying its hulls inline because nothing has registered them as prototypes.
pub(crate) const SANDBOX: Range<'static> = Range {
    id: SANDBOX_ID,
    hidden: true,
    form: HullForm::Inline,
    flight: true,
};

/// Build the sandbox the editor plays: the range above, with the document in it.
///
/// The refs it comes out with are the ones the DOCUMENT holds. A caller flying
/// it has to put them through [`AssetIndex::resolved`] first, because nothing
/// merges this range.
pub(crate) fn sandbox_scenario(
    settings: &ScenarioNode,
    world: Vec<ScenarioObjectConfig>,
    fleet: &LoweredFleet,
    script: Vec<ScenarioEventConfig>,
) -> ScenarioConfig {
    range_scenario(settings, SANDBOX, world, fleet, script)
}

/// Everything the range spawns on start: the world's own objects, then the
/// builder's whole fleet.
///
/// The fleet is NOT in the world list because it is not one: a ship node is
/// designed section by section and lowers through [`lower_fleet`], while an
/// object node is placed and lowers verbatim. Both end up in the same OnStart
/// handler, which is what makes them one saved scenario.
///
/// `flight` is what a save and a Play disagree about. A save spawns the
/// document and nothing else, so that opening the file gives the same nodes
/// back. Play adds what it needs to be flyable at all.
fn sandbox_objects(
    mut objects: Vec<ScenarioObjectConfig>,
    fleet: &LoweredFleet,
    form: HullForm,
    flight: bool,
) -> Vec<ScenarioObjectConfig> {
    // A document with no player ship still has to be flyable, so Play hands
    // the runtime a bare hull to sit in. Saving one would write a ship node
    // the builder never added.
    if flight || !fleet.player.id.is_empty() {
        objects.push(player_ship(&fleet.player, form));
    }
    // A blank Add Ship is a decision not yet made rather than a zero-section
    // spaceship, so Play skips it. A save keeps it: the node is the builder's,
    // and dropping it loses their place.
    objects.extend(
        fleet
            .standing
            .iter()
            .filter(|ship| !flight || !ship.sections.is_empty())
            .map(|ship| standing_ship(ship, form)),
    );

    objects
}

/// The planetoid: a large, invulnerable, PINNED rock with an explicit mass, so
/// it reads as a proper well and as scenery rather than a target.
fn planetoid() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "planetoid".to_string(),
            name: "Planetoid".to_string(),
            position: Meters3::from_engine(PLANETOID_POSITION),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            // DIRECT paths, not dep://: the editor's world is built at runtime
            // outside the mod merge, so scheme refs would never rewrite.
            material: None,
            destroy_sound: Some(AssetRef::from(DESTROY_SOUND)),
            radius: Meters::from_engine(PLANETOID_RADIUS),
            texture: AssetRef::from(ASTEROID_TEXTURE),
            mass: Some(PLANETOID_MASS),
            invulnerable: true,
            seed: Some(PLANETOID_SEED),
            lock_signature: None,
        }),
    }
}

/// One inert target: a ship-shaped cross of bare hull, no controller, no
/// pilot, no allegiance. It has never carried a weapon section, so the
/// integrity layer never NEUTRALIZES it either - it is a silhouette that takes
/// damage and comes apart, and nothing else.
fn target_hulk(index: usize, position: Vec3) -> ScenarioObjectConfig {
    let hull = |id: &str, offset: Vec3, prototype: &str| SpaceshipSectionConfig {
        id: id.to_string(),
        position: offset,
        rotation: Quat::IDENTITY,
        source: SectionSource::Prototype(prototype.to_string()),
        modifications: vec![],
    };

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("hulk_{index}"),
            name: format!("Derelict Hulk {index}"),
            position: Meters3::from_engine(position),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(ShipHull {
                sections: vec![
                    hull("spine", Vec3::ZERO, REINFORCED_HULL_SECTION_ID),
                    hull("bow", Vec3::new(0.0, 0.0, -1.0), LIGHT_HULL_SECTION_ID),
                    hull("stern", Vec3::new(0.0, 0.0, 1.0), LIGHT_HULL_SECTION_ID),
                    hull("port", Vec3::new(-1.0, 0.0, 0.0), LIGHT_HULL_SECTION_ID),
                    hull("starboard", Vec3::new(1.0, 0.0, 0.0), LIGHT_HULL_SECTION_ID),
                ],
                ..default()
            }),
            ..default()
        }),
    }
}

/// One dormant picket: a real armed corvette under AI, spawned NEUTRAL.
///
/// `spinal` pickets carry the railgun lance on top of the shared PDC. That is
/// the sandbox's whole railgun beat: the only shipped craft that fires one,
/// parked at the far end of the range so a builder meets the charge cue before
/// they meet the slug.
///
/// Neutral is the dormancy. The AI runs its passive routine (here: station
/// keeping, no patrol) and never acquires, because acquisition only looks at
/// hostile contacts. `wake_picket` flips the allegiance and the same pilot
/// starts fighting - no controller swap, no second spawn.
fn picket_ship(picket: &Picket) -> ScenarioObjectConfig {
    let section =
        |id: &str, offset: Vec3, rotation: Quat, prototype: &str| SpaceshipSectionConfig {
            id: id.to_string(),
            position: offset,
            rotation,
            source: SectionSource::Prototype(prototype.to_string()),
            modifications: vec![],
        };

    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: picket.id.to_string(),
            name: picket.name.to_string(),
            position: Meters3::from_engine(picket.position),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Neutral),
            controller: SpaceshipController::AI(AIControllerConfig {
                // Territorial: a woken picket fights over its own patch and
                // gives up if the player runs. Otherwise waking one drags a
                // pursuer across the whole range.
                leash: Some(Meters::from_engine(400.0)),
                // It wakes hot - the wake IS the warning, and the trip sphere
                // gives the player a beat before the guns bear.
                ..default()
            }),
            hull: ShipSource::Inline(ShipHull {
                sections: vec![
                    section(
                        "controller",
                        Vec3::ZERO,
                        Quat::IDENTITY,
                        BASIC_CONTROLLER_SECTION_ID,
                    ),
                    section(
                        "hull_front",
                        Vec3::new(0.0, 0.0, 1.0),
                        Quat::IDENTITY,
                        REINFORCED_HULL_SECTION_ID,
                    ),
                    section(
                        "hull_back",
                        Vec3::new(0.0, 0.0, -1.0),
                        Quat::IDENTITY,
                        REINFORCED_HULL_SECTION_ID,
                    ),
                    section(
                        "thruster",
                        Vec3::new(0.0, 0.0, 2.0),
                        Quat::IDENTITY,
                        BASIC_THRUSTER_SECTION_ID,
                    ),
                    if picket.spinal {
                        // The bore owns the nose face, so the PDC moves to the
                        // roof of the same plate rather than standing in front
                        // of the muzzle.
                        section(
                            "turret",
                            Vec3::new(0.0, 0.75, -1.0),
                            Quat::IDENTITY,
                            PDC_KINETIC_TURRET_SECTION_ID,
                        )
                    } else {
                        // Seated on the rear hull's -Z face: the shared PDC
                        // bolts down by its base plate a quarter-cell in.
                        section(
                            "turret",
                            Vec3::new(0.0, 0.0, -1.75),
                            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                            PDC_KINETIC_TURRET_SECTION_ID,
                        )
                    },
                ]
                .into_iter()
                .chain(picket.spinal.then(|| {
                    // Three cells long, breech on the nose plate's -Z face, so
                    // the muzzle stands clear at the front of the hull. The
                    // picket keeps every plate it had - the lance is bolted
                    // ON, which is what makes it the heavier ship.
                    section(
                        "lance",
                        Vec3::new(0.0, 0.0, -3.0),
                        Quat::IDENTITY,
                        RAILGUN_LANCE_SECTION_ID,
                    )
                }))
                .collect(),
                ..default()
            }),
            ..default()
        }),
    }
}

/// One sky beacon: a lockable nav orb that is its own trigger sphere.
fn sky_beacon(beacon: &SkyBeacon) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: beacon.id.to_string(),
            name: beacon.name.to_string(),
            position: Meters3::from_engine(beacon.position),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: beacon.label.to_string(),
            radius: Meters::from_engine(3.0),
            color: beacon.color,
            area_radius: Some(Meters::from_engine(beacon.trip_radius)),
            lock_signature: Some(Meters::from_engine(BEACON_LOCK_SIGNATURE)),
        }),
    }
}

/// What the range calls a lowered ship: the name the builder gave it, or the
/// one the sandbox used before ships could be named.
fn ship_name(ship: &LoweredShip, fallback: &str) -> String {
    if ship.name.is_empty() {
        fallback.to_string()
    } else {
        ship.name.clone()
    }
}

/// The ship the editor just built, with the keybinds it was built with.
fn player_ship(player: &LoweredShip, form: HullForm) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: ship_name(player, "Player's Spaceship"),
            position: Meters3::from_engine(player.position),
            rotation: player.rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: player.allegiance,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: player.inputs.iter().cloned().collect(),

                speed_cap: None,
                // The editor sandbox keeps normal finite magazines. Safe even
                // on a range built for shooting: weapons auto-reload, so a dry
                // gun is a cadence beat rather than a permanent disarm.
            }),
            // What the builder saw is what they fly. The editor shows the same
            // derived skin over the same structure, so the flown ship must not
            // come up bare (or skinned) against it.
            hull: hull_of(player, form),
            ..default()
        }),
    }
}

/// One ship of the document that the player does not fly, standing where it
/// was dragged to.
///
/// Every fact about it comes off its own node: an escort the builder added is
/// a NEUTRAL AI ship by default, a picket is the same with the leash it was
/// seeded with, and a target hulk has nobody at the controls at all. It flies
/// (or sits) with exactly the sections it was built from - what you laid out
/// on the stage is what the range holds.
fn standing_ship(ship: &LoweredShip, form: HullForm) -> ScenarioObjectConfig {
    let controller = match ship.driver {
        // A ship the document lowers here is not the flown one, whatever its
        // node says: `lower_fleet` routes the player's elsewhere.
        ShipDriver::Player | ShipDriver::Ai => SpaceshipController::AI(ship.pilot.clone()),
        ShipDriver::Adrift => SpaceshipController::None,
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: ship.id.clone(),
            name: ship_name(ship, &format!("Sandbox Ship {}", ship.id)),
            position: Meters3::from_engine(ship.position),
            rotation: ship.rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: ship.allegiance,
            controller,
            hull: hull_of(ship, form),
            ..default()
        }),
    }
}

/// One belt as a seeded scatter action.
fn belt_scatter(belt: &Belt) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: belt.id_prefix.to_string(),
        count: belt.count,
        seed: belt.seed,
        region: ScatterRegion::Box {
            min: Meters3::from_engine(belt.min),
            max: Meters3::from_engine(belt.max),
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: belt.id_prefix.to_string(),
                name: belt.name.to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                // DIRECT paths, not dep:// - see `planetoid`.
                material: None,
                destroy_sound: Some(AssetRef::from(DESTROY_SOUND)),
                radius: Meters::from_engine(belt.radius.0),
                texture: AssetRef::from(ASTEROID_TEXTURE),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some((
            Meters::from_engine(belt.radius.0),
            Meters::from_engine(belt.radius.1),
        )),
        min_separation: Some(Meters::from_engine(belt.separation)),
    })
}

/// A boolean expression node, for the once-only wake guards.
fn boolean(value: bool) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Boolean(value)),
    ))
}

/// A named-variable expression node.
fn variable(name: &str) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(VariableFactorNode::new_name(
        name.to_string(),
    )))
}

/// The scenario variable that remembers a picket has already woken, so the
/// comms line fires once rather than on every re-entry of its trip sphere.
fn woke_key(picket: &Picket) -> String {
    format!("{}_awake", picket.id)
}

/// The two ways a picket wakes: the player PAINTS it (combat lock), or the
/// player CROWDS it (flies into its trip sphere). Same actions either way.
fn wake_picket(picket: &Picket) -> Vec<ScenarioEventConfig> {
    let key = woke_key(picket);
    let still_asleep = EventFilterConfig::Expression(ExpressionFilterConfig(
        VariableConditionNode::new_equals(variable(&key), boolean(false)),
    ));
    let wake = vec![
        EventActionConfig::VariableSet(VariableSetActionConfig {
            key: key.clone(),
            expression: boolean(true),
        }),
        EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
            id: picket.id.to_string(),
            allegiance: Allegiance::Enemy,
        }),
        EventActionConfig::StoryMessage(StoryMessageActionConfig {
            speaker: picket.callsign.to_string(),
            text: "Contact acknowledged. Weapons free.".to_string(),
            dwell: None,
            icon: None,
        }),
    ];

    vec![
        // Painted: the lock event's primary entity is the TARGET, the other is
        // the ship that locked it.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnCombatLockStart,
            once: false,
            filters: vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(picket.id.to_string()),
                    other_id: Some(PLAYER_ID.to_string()),
                    ..default()
                }),
                still_asleep.clone(),
            ],
            actions: wake.clone(),
        },
        // Crowded: the trip sphere's own id is the primary entity.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: false,
            filters: vec![
                EventFilterConfig::Entity(EntityFilterConfig {
                    id: Some(trip_area_id(picket)),
                    other_id: Some(PLAYER_ID.to_string()),
                    ..default()
                }),
                still_asleep,
            ],
            actions: wake,
        },
    ]
}

/// The scenario area id of a picket's proximity trip.
fn trip_area_id(picket: &Picket) -> String {
    format!("{}_trip", picket.id)
}

/// The proximity trip sphere around one picket.
fn trip_area(picket: &Picket) -> EventActionConfig {
    EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: trip_area_id(picket),
        name: format!("{} Trip", picket.name),
        position: Meters3::from_engine(picket.position),
        rotation: Quat::IDENTITY,
        radius: Meters::from_engine(picket.trip_radius),
    })
}

/// Flying through a beacon swaps the sky and says so. Deliberately UNGUARDED,
/// unlike the picket wakes: re-entering a beacon is meant to swap the sky
/// again, which is the whole point of having two.
fn beacon_swaps_the_sky(beacon: &SkyBeacon) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnEnter,
        once: false,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(beacon.id.to_string()),
            other_id: Some(PLAYER_ID.to_string()),
            ..default()
        })],
        actions: vec![
            EventActionConfig::SetSkybox(SetSkyboxActionConfig::new(beacon.cubemap)),
            EventActionConfig::StoryMessage(StoryMessageActionConfig {
                speaker: beacon.label.to_string(),
                text: beacon.line.to_string(),
                dwell: None,
                icon: None,
            }),
        ],
    }
}

/// The range's whole script: the derived spawn handler, then the document's
/// own handlers.
///
/// The SPAWN handler is derived and stays derived. It is the layout written as
/// an action list, and an authored copy of it would be a second answer to
/// "what stands on this range" - one the tree could disagree with the moment a
/// rock was dragged. Everything else - the belts, the trip spheres, the
/// wake-ups, the briefing, the outcome - is the document's, held as event
/// nodes and lowered through [`world_script`].
fn range_events(
    id: &str,
    objects: Vec<ScenarioObjectConfig>,
    script: Vec<ScenarioEventConfig>,
) -> Vec<ScenarioEventConfig> {
    let spawned: HashSet<String> = objects
        .iter()
        .map(|object| object.base.id.clone())
        .collect();
    let mut events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        // The world's own lights come through `objects` - the engine spawns
        // none, so a scenario that authors none renders black. (The editor
        // VIEW has its own light - `ui/mod.rs` - which is a different
        // surface.)
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect(),
    }];
    events.extend(following_the_objects(script, spawned, id));
    events
}

/// The script, minus the handlers that name something the range never spawns.
///
/// A handler naming an id nothing spawns is a lint ERROR and the loader
/// refuses the scenario over it, so a document that lost its pickets, its
/// beacons or its player ship has to lose their handlers too. Deleting the
/// object is the gesture; the panel paints the reference as a fault before the
/// drop, so the handler does not vanish unannounced.
///
/// Declarations are collected from the WHOLE script before anything is
/// dropped, so a handler is never judged against a spawn a later one makes.
fn following_the_objects(
    script: Vec<ScenarioEventConfig>,
    mut spawned: HashSet<String>,
    range: &str,
) -> Vec<ScenarioEventConfig> {
    let named: Vec<NamedIds> = script.iter().map(named_ids).collect();
    for ids in &named {
        spawned.extend(ids.declared.iter().cloned());
    }
    let prefixes: Vec<String> = named
        .iter()
        .flat_map(|ids| ids.prefixes.iter().cloned())
        .filter(|prefix| !prefix.is_empty())
        .collect();
    script
        .into_iter()
        .zip(named)
        .filter(|(_, ids)| {
            ids.referenced.iter().all(|id| {
                spawned.contains(id) || prefixes.iter().any(|prefix| id.starts_with(prefix))
            })
        })
        .map(|(mut event, _)| {
            retarget_retries(&mut event.actions, SANDBOX_ID, range);
            event
        })
        .collect()
}

/// Rewrite a retry that names `from` so it names `to` instead.
///
/// A `NextScenario` naming the document's own range is the document's name for
/// ITSELF - the seeded death handler offers this range again - and which id
/// that range answers to is decided by the lowering: `editor_sandbox` on Play,
/// `editor_save` in a file. Left alone, a saved range's retry would reload a
/// hidden scenario the saved mod does not contain.
///
/// In memory the document always names itself [`SANDBOX_ID`], so this runs
/// BOTH ways: the lowering points a retry at the range being written, and
/// [`crate::bundle::lift_content`] points it back at the sandbox when a saved
/// range is opened. Without the return leg a document that had been through a
/// file would keep the FILE's id, Play would lower it into `editor_sandbox`
/// beside a retry naming `editor_save`, and the range would refuse to start on
/// a dangling reference.
pub(crate) fn retarget_retries(actions: &mut [EventActionConfig], from: &str, to: &str) {
    for action in actions {
        match action {
            EventActionConfig::NextScenario(next) if next.scenario_id == from => {
                next.scenario_id = to.to_string();
            }
            EventActionConfig::Sequence(sequence) => {
                for step in &mut sequence.steps {
                    retarget_retries(&mut step.actions, from, to);
                }
            }
            _ => {}
        }
    }
}

/// The script a new document opens with: everything the range does that is not
/// putting its objects on the board.
///
/// These were constants written into the hand-off until the editor could hold
/// them. They are seeded into the document instead, as event nodes, so the
/// first thing a builder sees in the EVENTS tab is a working scenario they can
/// read - a briefing, an objective, two ways to wake a picket, a sky that
/// swaps, and a death that offers a retry.
pub(crate) fn default_script() -> Vec<ScenarioEventConfig> {
    let mut events = vec![
        // The range stands ITSELF up: the two rock belts, the proximity trips
        // the wake-ups fire on, and the guard variable each wake reads. The
        // variables have to exist before the first filter reads one.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: BELTS
                .iter()
                .map(belt_scatter)
                .chain(PICKETS.iter().map(trip_area))
                .chain(PICKETS.iter().map(|picket| {
                    EventActionConfig::VariableSet(VariableSetActionConfig {
                        key: woke_key(picket),
                        expression: boolean(false),
                    })
                }))
                .collect(),
        },
        // The standing objective. Nothing completes it - there is nothing to
        // complete - so it stays on the HUD as the sandbox's only instruction.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    "sandbox_free_flight",
                    "Free flight: press F1 to return to the editor",
                )),
                EventActionConfig::StoryMessage(StoryMessageActionConfig {
                    speaker: "Range Control".to_string(),
                    text: "Range is yours. Hulks to port, live pickets deeper in - they wake if \
                           you paint them or crowd them. F1 puts you back on the build deck."
                        .to_string(),
                    dwell: None,
                    icon: None,
                }),
            ],
        },
        // Death is the only outcome, and it offers this same range again.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: false,
            filters: player(PLAYER_ID),
            actions: [EventActionConfig::DebugMessage(DebugMessageActionConfig {
                message: "The player's spaceship was destroyed!".to_string(),
            })]
            .into_iter()
            .chain(retry(
                "Range is quiet. Your ship is part of the scenery now.",
            ))
            .collect(),
        },
        // Shot to pieces without dying: the same offer. A ship that never
        // carried a weapon cannot reach this, so an unarmed build is not
        // instantly "neutralized" off the line.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: false,
            filters: player(PLAYER_ID),
            actions: retry("Nothing left to fight with - you drift the range dead."),
        },
    ];
    events.extend(PICKETS.iter().flat_map(wake_picket));
    events.extend(SKY_BEACONS.iter().map(beacon_swaps_the_sky));
    events
}

/// The outcome a death declares, and the retry it queues.
///
/// `linger` holds the switch until the overlay's Retry (or Enter) releases it.
/// The scenario named is [`SANDBOX_ID`], which [`retarget_retries`] rewrites
/// to whichever range the document was lowered into.
fn retry(message: &str) -> Vec<EventActionConfig> {
    vec![
        EventActionConfig::Outcome(OutcomeActionConfig::new(
            ScenarioOutcomeKind::Defeat,
            message,
        )),
        EventActionConfig::NextScenario(NextScenarioActionConfig {
            scenario_id: SANDBOX_ID.to_string(),
            linger: true,
            delay: None,
        }),
    ]
}

/// The filter that scopes a handler to one ship by id.
fn player(id: &str) -> Vec<EventFilterConfig> {
    vec![EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.to_string()),
        type_name: None,
        ..default()
    })]
}

#[cfg(test)]
mod tests {
    /// The point the range is laid out around. Ships no longer spawn here by
    /// rule - each stands where the builder put it - but the belts, planetoid
    /// and pickets are still authored about this point, and the layout tests
    /// measure from it.
    const RANGE_ORIGIN: Vec3 = Vec3::ZERO;

    use bevy::ecs::system::RunSystemOnce;
    use nova_gameplay::prelude::{GravitySettings, GravityWell};

    use super::*;
    use crate::{
        config::SelectedNode,
        node::{ensure_document, ObjectNode},
    };

    /// The camera's far plane (bevy's `PerspectiveProjection` default, which
    /// nothing in the game overrides). A body authored past this is simply not
    /// drawn until the player closes on it, so the whole range stays inside it.
    const CAMERA_FAR: f32 = 1000.0;

    fn objects() -> Vec<ScenarioObjectConfig> {
        sandbox_objects(
            default_world_objects(),
            &LoweredFleet::default(),
            HullForm::Inline,
            true,
        )
    }

    fn events() -> Vec<ScenarioEventConfig> {
        range_events(SANDBOX_ID, objects(), default_script())
    }

    /// Every id the SCRIPT names: the entities its filters match on, and the
    /// ships its actions reach for. Not the spawns - those are where ids come
    /// from rather than uses of one.
    fn referenced_ids(events: &[ScenarioEventConfig]) -> HashSet<String> {
        let mut ids = HashSet::new();
        for event in events {
            for filter in &event.filters {
                if let EventFilterConfig::Entity(entity) = filter {
                    ids.extend(entity.id.clone());
                    ids.extend(entity.other_id.clone());
                }
            }
            for action in &event.actions {
                if let EventActionConfig::SetAllegiance(set) = action {
                    ids.insert(set.id.clone());
                }
            }
        }
        ids
    }

    /// Every id the range PUTS on the board: the objects it spawns and the
    /// areas it creates.
    fn defined_ids(events: &[ScenarioEventConfig]) -> HashSet<String> {
        events
            .iter()
            .flat_map(|event| &event.actions)
            .filter_map(|action| match action {
                EventActionConfig::SpawnScenarioObject(object) => Some(object.base.id.clone()),
                EventActionConfig::CreateScenarioArea(area) => Some(area.id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The script follows the objects, whatever the builder deleted.
    ///
    /// A handler naming an id nothing spawns is a lint ERROR and the loader
    /// refuses the scenario over it, so a document that lost its pickets, its
    /// beacons or its player ship has to lose their handlers too.
    #[test]
    fn the_script_only_names_what_the_range_spawns() {
        let flown = LoweredFleet {
            player: LoweredShip {
                id: "ship_1".to_string(),
                ..default()
            },
            standing: vec![],
        };
        let stripped: Vec<ScenarioObjectConfig> = default_world_objects()
            .into_iter()
            .filter(|object| {
                !object.base.id.starts_with("picket_") && !object.base.id.starts_with("beacon_")
            })
            .collect();
        let ranges = [
            ("the whole range, flown", objects()),
            (
                "a save with no ship to fly it",
                sandbox_objects(
                    default_world_objects(),
                    &LoweredFleet::default(),
                    HullForm::Inline,
                    false,
                ),
            ),
            (
                "a save whose pickets and beacons were deleted",
                sandbox_objects(stripped, &flown, HullForm::Inline, false),
            ),
        ];

        for (what, objects) in ranges {
            let events = range_events(SANDBOX_ID, objects, default_script());
            let defined = defined_ids(&events);
            let mut dangling: Vec<String> = referenced_ids(&events)
                .difference(&defined)
                .cloned()
                .collect();
            dangling.sort();
            assert!(
                dangling.is_empty(),
                "{what}: the script names {dangling:?}, which the range never spawns"
            );
        }
    }

    fn find(objects: &[ScenarioObjectConfig], id: &str) -> ScenarioObjectConfig {
        objects
            .iter()
            .find(|object| object.base.id == id)
            .unwrap_or_else(|| panic!("the sandbox spawns '{id}'"))
            .clone()
    }

    /// Lower whatever world stands in `world`, through the same path the
    /// hand-off takes.
    fn lower(world: &mut World) -> Vec<ScenarioObjectConfig> {
        world
            .run_system_once(|context: Res<EditContext>, q_objects: ObjectNodes| {
                world_objects(&context, &q_objects)
            })
            .expect("the lowering runs")
    }

    /// Everything the hand-off spawns from the document: the world's objects,
    /// and the fleet standing among them.
    fn lower_range(world: &mut World) -> Vec<ScenarioObjectConfig> {
        world
            .run_system_once(
                |context: Res<EditContext>,
                 q_objects: ObjectNodes,
                 q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>,
                 nodes: SectionNodes| {
                    sandbox_objects(
                        world_objects(&context, &q_objects),
                        &lower_fleet(&q_ships, &nodes),
                        HullForm::Inline,
                        true,
                    )
                },
            )
            .expect("the lowering runs")
    }

    /// Stand the default range up in `world` as document nodes.
    fn stand_up_document(world: &mut World) {
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        world
            .run_system_once(ensure_document)
            .expect("the document is created");
    }

    /// The hand-off reads the DOCUMENT, and the document says the same thing
    /// the constants used to. This is what makes the editor's tree the source
    /// of the range rather than a view onto a table beside it.
    #[test]
    fn the_seeded_document_lowers_to_the_range_it_stood_up() {
        let mut world = World::new();
        stand_up_document(&mut world);

        let lowered = lower_range(&mut world);

        // The hulls go round through the ship half and the rest through the
        // object half, and the range they land in is the one that was seeded.
        let mut got: Vec<String> = lowered
            .iter()
            .map(|object| object.base.id.clone())
            .filter(|id| id != PLAYER_ID)
            .collect();
        got.sort();
        let mut want: Vec<String> = default_world_objects()
            .iter()
            .map(|object| object.base.id.clone())
            .collect();
        want.sort();
        assert_eq!(got, want);

        // The pose comes off the node's transform, so dragging a rock moves
        // the rock the scenario spawns.
        let planetoid = find(&lowered, "planetoid");
        assert_eq!(
            planetoid.base.position,
            Meters3::from_engine(PLANETOID_POSITION)
        );

        // And what makes a picket a picket rides along with it: the wake
        // handler flips a NEUTRAL ship to hostile, and the leash is what
        // stops the woken one chasing across the whole range.
        let ScenarioObjectKind::Spaceship(picket) = &find(&lowered, "picket_warden").kind else {
            panic!("'picket_warden' should be a spaceship");
        };
        assert_eq!(picket.allegiance, Some(Allegiance::Neutral));
        let SpaceshipController::AI(pilot) = &picket.controller else {
            panic!("a picket has a live AI pilot");
        };
        assert_eq!(pilot.leash, Some(Meters::from_engine(400.0)));

        let ScenarioObjectKind::Spaceship(hulk) = &find(&lowered, "hulk_0").kind else {
            panic!("'hulk_0' should be a spaceship");
        };
        assert!(
            matches!(hulk.controller, SpaceshipController::None),
            "and a target hulk still has nobody at the controls"
        );
    }

    /// An emptied range is an EMPTY range. The lowering keys on the document
    /// existing, never on it holding anything: "empty means refill" would
    /// resurrect every rock the player deleted the moment they pressed Play.
    #[test]
    fn a_world_the_player_emptied_stays_empty() {
        let mut world = World::new();
        stand_up_document(&mut world);
        let standing: Vec<Entity> = world
            .query_filtered::<Entity, Or<(With<ObjectNode>, With<ShipNode>)>>()
            .iter(&world)
            .collect();
        for node in standing {
            world.despawn(node);
        }

        assert!(
            lower(&mut world).is_empty(),
            "the document is the range, including when it is bare"
        );
        assert_eq!(
            lower_range(&mut world)
                .iter()
                .map(|object| object.base.id.as_str())
                .collect::<Vec<_>>(),
            vec![PLAYER_ID],
            "and all that is left to spawn is the hull the player arrives in"
        );
    }

    /// Before there is a document there is still a scenario to register: the
    /// hand-off is wired at asset-load, long before the editor is entered, and
    /// the Scenario path can be taken without opening the editor at all.
    #[test]
    fn without_a_document_the_stock_range_stands_in() {
        let mut world = World::new();
        world.init_resource::<EditContext>();

        let lowered = lower(&mut world);

        assert_eq!(lowered.len(), default_world_objects().len());
    }

    /// The planetoid's WORST-CASE drawn radius and its well, recomputed from
    /// the authored numbers exactly as the engine derives them.
    fn planetoid_reach() -> (f32, GravityWell) {
        let settings = GravitySettings::default();
        let body_radius = PLANETOID_RADIUS * ASTEROID_GEOMETRIC_FACTOR_MAX;
        (
            body_radius,
            GravityWell::from_mass(PLANETOID_MASS, body_radius, &settings),
        )
    }

    /// The bug this layout exists to fix: the player used to spawn inside the
    /// planetoid's sphere of influence AND within ~60u of its real surface,
    /// and simply fell into it. Both bounds are derived here rather than
    /// asserted as distances, so retuning the mass or the radius re-checks the
    /// spawn instead of silently re-breaking it.
    #[test]
    fn the_spawn_is_clear_of_the_planetoid() {
        let (body_radius, well) = planetoid_reach();
        let distance = RANGE_ORIGIN.distance(PLANETOID_POSITION);

        assert!(
            distance > well.soi_radius,
            "the spawn ({distance:.0}u out) must sit outside the planetoid's \
             {:.0}u sphere of influence - inside it the ship falls in on its own",
            well.soi_radius
        );
        // And with room to spare, not one unit outside: the SOI edge is where
        // the pull starts, not where the trouble starts.
        assert!(
            distance > well.soi_radius + 200.0,
            "the spawn wants clear air past the SOI edge, not a hairline"
        );
        assert!(
            distance > body_radius * 2.0,
            "the drawn planetoid reaches {body_radius:.0}u; the spawn must not \
             be parked against it"
        );
    }

    /// Nothing is authored past the camera's far plane, measured from the
    /// spawn: a body beyond it is simply not drawn, so a "landmark" out there
    /// is an invisible one.
    #[test]
    fn the_whole_range_is_inside_the_camera_far_plane() {
        for object in objects() {
            let distance = RANGE_ORIGIN.distance(object.base.position.to_engine());
            assert!(
                distance < CAMERA_FAR,
                "'{}' is {distance:.0}u from the spawn, past the {CAMERA_FAR:.0}u far plane",
                object.base.id
            );
        }
        // The belts are scattered, so check their corners instead.
        for belt in &BELTS {
            for corner in [belt.min, belt.max] {
                let distance = RANGE_ORIGIN.distance(corner);
                assert!(
                    distance < CAMERA_FAR,
                    "belt '{}' reaches {distance:.0}u, past the far plane",
                    belt.id_prefix
                );
            }
        }
    }

    /// Hand-placed bodies stay OUT of the scatter boxes. `ScatterObjects`
    /// separates its rocks from each other and from earlier belts, but it
    /// knows nothing about the hulks, pickets and beacons - and two
    /// overlapping dynamic bodies are shoved apart on the first physics step
    /// hard enough to damage each other, so a hulk dropped in a belt would
    /// come apart as the scenario loads.
    #[test]
    fn hand_placed_bodies_stay_out_of_the_belts() {
        for object in objects() {
            // The player is at the origin, clear of both boxes; the planetoid
            // is checked by size below.
            for belt in &BELTS {
                let position = object.base.position.to_engine();
                let inside = (belt.min.cmple(position) & belt.max.cmpge(position)).all();
                assert!(
                    !inside,
                    "'{}' sits inside belt '{}' and would be shoved apart at spawn",
                    object.base.id, belt.id_prefix
                );
            }
        }

        // The planetoid is not a point: its drawn body must clear the boxes
        // too, or rocks spawn inside the rock.
        let (body_radius, _) = planetoid_reach();
        for belt in &BELTS {
            let nearest = PLANETOID_POSITION.clamp(belt.min, belt.max);
            let gap = PLANETOID_POSITION.distance(nearest);
            assert!(
                gap > body_radius,
                "belt '{}' comes within {gap:.0}u of the planetoid's centre, \
                 inside its {body_radius:.0}u body",
                belt.id_prefix
            );
        }
    }

    /// Every rock belt keeps its copies apart, sized on the WIDEST pair of
    /// rocks it can draw. Without this a belt explodes as it spawns.
    #[test]
    fn every_belt_separates_its_widest_rocks() {
        for belt in &BELTS {
            let widest = belt.radius.1 * ASTEROID_GEOMETRIC_FACTOR_MAX;
            assert!(
                belt.separation >= widest * 2.0,
                "belt '{}' separates by {}u but can draw two {widest:.0}u rocks",
                belt.id_prefix,
                belt.separation
            );
        }
    }

    /// The targets are inert: bare hull, nobody driving, no side. A weapon
    /// section on one of these would make it a combatant that the integrity
    /// layer can neutralize, and a controller would make it fly away.
    #[test]
    fn the_target_hulks_are_unarmed_and_unpiloted() {
        let objects = objects();
        let hulks: Vec<&ScenarioObjectConfig> = objects
            .iter()
            .filter(|object| object.base.id.starts_with("hulk_"))
            .collect();
        assert_eq!(hulks.len(), HULK_POSITIONS.len(), "every hulk spawns");

        for hulk in hulks {
            let ScenarioObjectKind::Spaceship(ship) = &hulk.kind else {
                panic!("'{}' should be a spaceship", hulk.base.id);
            };
            assert!(
                matches!(ship.controller, SpaceshipController::None),
                "'{}' must have nobody driving it",
                hulk.base.id
            );
            assert_eq!(ship.allegiance, None, "'{}' takes no side", hulk.base.id);
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("'{}' authors its hull inline", hulk.base.id);
            };
            assert!(
                hull.sections.iter().all(|section| matches!(
                    &section.source,
                    SectionSource::Prototype(id) if id.contains("hull")
                )),
                "'{}' is hull only",
                hulk.base.id
            );
        }
    }

    /// A picket is dormant because it is NEUTRAL, not because it is asleep: it
    /// spawns with a live AI pilot, and the AI finds nothing to shoot while it
    /// takes no side. Both wake routes must then flip it to Enemy - one for
    /// the lock, one for the trip sphere - or half the promise is missing.
    #[test]
    fn every_picket_spawns_neutral_and_wakes_two_ways() {
        let objects = objects();
        let events = events();

        for picket in &PICKETS {
            let ScenarioObjectKind::Spaceship(ship) = &find(&objects, picket.id).kind else {
                panic!("'{}' should be a spaceship", picket.id);
            };
            assert_eq!(
                ship.allegiance,
                Some(Allegiance::Neutral),
                "'{}' must spawn neutral - that IS its dormancy",
                picket.id
            );
            assert!(
                matches!(ship.controller, SpaceshipController::AI(_)),
                "'{}' carries a live AI pilot while dormant",
                picket.id
            );
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("'{}' authors its hull inline", picket.id);
            };
            assert!(
                hull.sections.iter().any(|section| matches!(
                    &section.source,
                    SectionSource::Prototype(id) if id.contains("turret")
                )),
                "'{}' has something to fight with once woken",
                picket.id
            );

            let wakes: Vec<&ScenarioEventConfig> = events
                .iter()
                .filter(|event| {
                    event.actions.iter().any(|action| {
                        matches!(
                            action,
                            EventActionConfig::SetAllegiance(set)
                                if set.id == picket.id && set.allegiance == Allegiance::Enemy
                        )
                    })
                })
                .collect();
            // EventConfig has no PartialEq, so match the two shapes by hand.
            let names: Vec<&EventConfig> = wakes.iter().map(|event| &event.name).collect();
            assert!(
                names
                    .iter()
                    .any(|name| matches!(name, EventConfig::OnCombatLockStart)),
                "'{}' wakes when the player paints it: {names:?}",
                picket.id
            );
            assert!(
                names
                    .iter()
                    .any(|name| matches!(name, EventConfig::OnEnter)),
                "'{}' wakes when the player crowds it: {names:?}",
                picket.id
            );
        }

        // Each trip sphere is actually created, or the OnEnter half never fires.
        let areas: Vec<String> = events
            .iter()
            .flat_map(|event| &event.actions)
            .filter_map(|action| match action {
                EventActionConfig::CreateScenarioArea(area) => Some(area.id.clone()),
                _ => None,
            })
            .collect();
        for picket in &PICKETS {
            assert!(
                areas.contains(&trip_area_id(picket)),
                "'{}' has no trip sphere: {areas:?}",
                picket.id
            );
        }
    }

    /// Exactly one picket carries the lance, and it keeps its PDC.
    ///
    /// Both halves matter. One, because a range that opens three spinal
    /// charges on you is a range you stop exploring; and the PDC, because a
    /// lance-only picket is harmless the moment you are inside its cone,
    /// which teaches the wrong lesson about closing with one.
    #[test]
    fn one_picket_mounts_the_lance_and_still_carries_its_pdc() {
        let objects = default_world_objects();
        let mut armed = Vec::new();

        for picket in &PICKETS {
            let ScenarioObjectKind::Spaceship(ship) = &find(&objects, picket.id).kind else {
                panic!("'{}' should be a spaceship", picket.id);
            };
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("'{}' authors its hull inline", picket.id);
            };
            let mounts = |prototype: &str| {
                hull.sections.iter().any(|section| {
                    matches!(&section.source, SectionSource::Prototype(id) if id == prototype)
                })
            };

            assert!(
                mounts(PDC_KINETIC_TURRET_SECTION_ID),
                "'{}' keeps its PDC whatever else it carries",
                picket.id
            );
            assert_eq!(
                mounts(RAILGUN_LANCE_SECTION_ID),
                picket.spinal,
                "'{}' carries a lance exactly when it is spinal",
                picket.id
            );
            if picket.spinal {
                armed.push(picket.id);
            }
        }

        assert_eq!(
            armed,
            ["picket_lance"],
            "one lance in the sandbox, not three"
        );
    }

    /// Every beacon swaps the sky when the player flies through it, and the
    /// two of them cover both shipped cubemaps - one out, one back.
    #[test]
    fn the_beacons_swap_the_sky_both_ways() {
        let events = events();
        let swaps: Vec<String> = events
            .iter()
            .filter(|event| matches!(event.name, EventConfig::OnEnter))
            .flat_map(|event| &event.actions)
            .filter_map(|action| match action {
                EventActionConfig::SetSkybox(skybox) => skybox.cubemap.path().map(str::to_string),
                _ => None,
            })
            .collect();

        for beacon in &SKY_BEACONS {
            assert!(
                swaps.iter().any(|path| path == beacon.cubemap),
                "'{}' installs {}: {swaps:?}",
                beacon.id,
                beacon.cubemap
            );
        }
        assert_eq!(
            swaps.len(),
            SKY_BEACONS.len(),
            "one swap per beacon, no strays"
        );
    }

    /// The sandbox never ends. Nothing completes an objective, nothing
    /// declares a Victory, and the only scenario it ever chains to is itself -
    /// the retry the DEFEAT overlay offers.
    #[test]
    fn the_sandbox_has_no_ending_but_death_offers_a_retry() {
        let events = events();
        let actions: Vec<&EventActionConfig> =
            events.iter().flat_map(|event| &event.actions).collect();

        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, EventActionConfig::ObjectiveComplete(_))),
            "nothing in a sandbox is ever complete"
        );
        assert!(
            actions.iter().any(|action| matches!(
                action,
                EventActionConfig::Objective(objective) if objective.message.contains("F1")
            )),
            "the standing objective names the way out"
        );

        let outcomes: Vec<ScenarioOutcomeKind> = actions
            .iter()
            .filter_map(|action| match action {
                EventActionConfig::Outcome(outcome) => Some(outcome.outcome),
                _ => None,
            })
            .collect();
        assert!(
            !outcomes.is_empty() && outcomes.iter().all(|k| *k == ScenarioOutcomeKind::Defeat),
            "death is the only outcome the range declares: {outcomes:?}"
        );

        let chains: Vec<&NextScenarioActionConfig> = actions
            .iter()
            .filter_map(|action| match action {
                EventActionConfig::NextScenario(next) => Some(next),
                _ => None,
            })
            .collect();
        assert!(!chains.is_empty(), "death queues a retry");
        for next in chains {
            assert_eq!(
                next.scenario_id, SANDBOX_ID,
                "the range only ever chains back to itself"
            );
            assert!(
                next.linger,
                "the retry waits for the player's Retry, it does not cut"
            );
        }
    }

    /// The retry is only reachable because the sandbox registers itself: the
    /// switch resolves the queued id against `GameScenarios`, and this
    /// scenario is built at runtime rather than merged from content. It is
    /// hidden so that registration cannot leak it into the picker.
    #[test]
    fn the_sandbox_registers_itself_hidden_so_the_retry_resolves() {
        let scenario = ScenarioConfig {
            hidden: true,
            events: vec![],
            ..ScenarioConfig::new(
                SANDBOX_ID.to_string(),
                "Editor Sandbox".to_string(),
                AssetRef::from("base/textures/cubemap.png"),
            )
        };
        let mut scenarios = GameScenarios::default();
        scenarios.insert(scenario.id.clone(), scenario);

        let registered = scenarios
            .get(SANDBOX_ID)
            .expect("the retry's id resolves against the registry");
        assert!(
            registered.hidden,
            "a runtime scenario in the registry must stay out of the picker"
        );
        assert!(
            !registered.menu_backdrop,
            "and out of the menu's backdrop rotation"
        );
    }

    /// The Play hand-off is where a PICKED ref stops being a ref. The picker
    /// writes what a save needs - `dep://<bundle>/<file>` - and the sandbox is
    /// merged with nothing, so an unresolved ref reaches the asset server as an
    /// unknown source and the range comes up with no sky at all. The regression
    /// is the resolver going uncalled, not the resolver being wrong, so this
    /// runs the real system.
    #[test]
    fn play_hands_off_a_picked_sky_the_asset_server_can_actually_load() {
        use nova_assets::prelude::EnabledMods;
        use nova_modding::prelude::{
            BundleAsset, CatalogEntry, InstalledCatalog, ModEntry, ModMeta,
        };

        let mut world = World::new();
        let mut bundles = Assets::<BundleAsset>::default();
        let bundle = bundles.add(BundleAsset {
            content: vec![],
            meta: ModMeta::default(),
            new_game_scenario: None,
            resources: vec!["textures/cubemap_alt.png".to_string()],
            resource_base: "base".to_string(),
        });
        let mut catalogs = Assets::<InstalledCatalog>::default();
        catalogs.add(InstalledCatalog {
            entries: vec![CatalogEntry {
                decl: ModEntry {
                    id: "base".to_string(),
                    bundle: "base/base.bundle.ron".to_string(),
                    base: true,
                    hidden: false,
                },
                bundle,
            }],
        });
        world.insert_resource(bundles);
        world.insert_resource(catalogs);
        world.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
        world.insert_resource(GameScenarios::default());
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        world.run_system_once(ensure_document).expect("a document");

        let scenario = world.resource::<EditContext>().path[0];
        world.entity_mut(scenario).insert(ScenarioNode {
            cubemap: AssetRef::from("dep://base/textures/cubemap_alt.png"),
            ..default()
        });
        world
            .run_system_once(setup_scenario)
            .expect("Play hands off");

        assert_eq!(
            world.resource::<GameScenarios>()[SANDBOX_ID].cubemap.path(),
            Some("base/textures/cubemap_alt.png"),
            "the sky the builder picked is handed off as a path that loads"
        );
    }

    /// The repair trigger. `register_bundles` rebuilds `GameScenarios` from
    /// content files every time the installed set changes, and this scenario
    /// has no content file to be rebuilt from - so the id silently leaves the
    /// registry unless something notices it is gone.
    #[test]
    fn a_registry_rebuilt_from_content_reads_as_missing_the_sandbox() {
        let mut world = World::new();
        // What a merge produces: every shipped id, and not this one.
        let mut merged = GameScenarios::default();
        merged.insert(
            "some_shipped_scenario".to_string(),
            ScenarioConfig::new(
                "some_shipped_scenario".to_string(),
                "Shipped".to_string(),
                AssetRef::from("base/textures/cubemap.png"),
            ),
        );
        world.insert_resource(merged);
        assert!(
            world
                .run_system_once(sandbox_unregistered)
                .expect("the condition runs"),
            "a merged registry never carries the sandbox"
        );

        world.resource_mut::<GameScenarios>().insert(
            SANDBOX_ID.to_string(),
            ScenarioConfig::new(
                SANDBOX_ID.to_string(),
                "Editor Sandbox".to_string(),
                AssetRef::from("base/textures/cubemap.png"),
            ),
        );
        assert!(
            !world
                .run_system_once(sandbox_unregistered)
                .expect("the condition runs"),
            "and the repair stops once the id is back"
        );
    }

    /// Every non-empty standing design in the document flies beside the
    /// player, at the stage offset the builder dragged it to, carrying the
    /// side and the pilot its node holds. Before this only the player's ship
    /// was lowered, so a second ship simply never spawned - and the side was
    /// hard-coded here, so a picket lost its leash on the way through.
    #[test]
    fn standing_ships_spawn_at_their_stage_offset_with_their_own_orders() {
        let section = SpaceshipSectionConfig {
            id: "hull_1".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype(LIGHT_HULL_SECTION_ID.to_string()),
            modifications: vec![],
        };
        let fleet = LoweredFleet {
            player: LoweredShip::default(),
            standing: vec![
                LoweredShip {
                    id: "ship_2".to_string(),
                    driver: ShipDriver::Ai,
                    allegiance: Some(Allegiance::Neutral),
                    pilot: AIControllerConfig {
                        leash: Some(Meters::from_engine(400.0)),
                        ..default()
                    },
                    sections: vec![section.clone()],
                    position: Vec3::new(24.0, 0.0, 0.0),
                    ..default()
                },
                LoweredShip {
                    id: "hulk_0".to_string(),
                    driver: ShipDriver::Adrift,
                    sections: vec![section],
                    ..default()
                },
                // A blank Add Ship left in the document: a decision not yet
                // made, not a zero-section spaceship to load.
                LoweredShip {
                    id: "ship_3".to_string(),
                    ..default()
                },
            ],
        };

        let objects = sandbox_objects(default_world_objects(), &fleet, HullForm::Inline, true);
        let escort = find(&objects, "ship_2");
        assert_eq!(
            escort.base.position,
            Meters3::from_engine(Vec3::new(24.0, 0.0, 0.0))
        );
        let ScenarioObjectKind::Spaceship(ship) = &escort.kind else {
            panic!("'ship_2' should be a spaceship");
        };
        assert_eq!(
            ship.allegiance,
            Some(Allegiance::Neutral),
            "the side the node holds is the side it spawns on"
        );
        let SpaceshipController::AI(pilot) = &ship.controller else {
            panic!("an AI-driver design is driven by the AI, as its node says");
        };
        assert_eq!(
            pilot.leash,
            Some(Meters::from_engine(400.0)),
            "and the pilot's standing orders survive the trip through the document"
        );

        let ScenarioObjectKind::Spaceship(hulk) = &find(&objects, "hulk_0").kind else {
            panic!("'hulk_0' should be a spaceship");
        };
        assert!(
            matches!(hulk.controller, SpaceshipController::None),
            "a hull with nobody at the controls spawns with nobody at the controls"
        );

        assert!(
            !objects.iter().any(|object| object.base.id == "ship_3"),
            "an empty design spawns nothing"
        );
    }

    /// A ship turned on the stage launches turned. The lowering used to hand
    /// every ship `Quat::IDENTITY`, so the turn rings moved the hull in the
    /// editor and nothing at all in flight.
    #[test]
    fn a_ships_heading_survives_the_hand_off() {
        let section = SpaceshipSectionConfig {
            id: "hull_1".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype(LIGHT_HULL_SECTION_ID.to_string()),
            modifications: vec![],
        };
        let heading = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let listing = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let fleet = LoweredFleet {
            player: LoweredShip {
                sections: vec![section.clone()],
                rotation: heading,
                ..default()
            },
            standing: vec![LoweredShip {
                id: "ship_2".to_string(),
                sections: vec![section],
                position: Vec3::new(24.0, 0.0, 0.0),
                rotation: listing,
                ..default()
            }],
        };

        let objects = sandbox_objects(default_world_objects(), &fleet, HullForm::Inline, true);
        assert_eq!(
            find(&objects, PLAYER_ID).base.rotation,
            heading,
            "the player's own heading is the player's own"
        );
        let escort = find(&objects, "ship_2");
        assert_eq!(escort.base.rotation, listing);
        assert_eq!(
            escort.base.position,
            Meters3::from_engine(Vec3::new(24.0, 0.0, 0.0)),
            "and turning a ship does not move it"
        );
    }

    /// A ship's pose is its own. The player's ship used to ANCHOR the fleet -
    /// its own translation was subtracted from every ship's - so dragging the
    /// only ship in a document moved nothing, and dragging the player moved
    /// every other ship instead of itself.
    #[test]
    fn each_ship_flies_from_where_it_was_dragged() {
        let mut world = World::new();
        let player_at = Vec3::new(60.0, 10.0, -20.0);
        let escort_at = Vec3::new(84.0, 10.0, -20.0);
        world.spawn((
            NodeId("ship_1".to_string()),
            ShipNode {
                driver: ShipDriver::Player,
                ..default()
            },
            Transform::from_translation(player_at),
        ));
        world.spawn((
            NodeId("ship_2".to_string()),
            ShipNode {
                driver: ShipDriver::Ai,
                ..default()
            },
            Transform::from_translation(escort_at),
        ));

        let fleet = world
            .run_system_once(
                |q_ships: Query<(Entity, &NodeId, &ShipNode, &Transform)>, nodes: SectionNodes| {
                    let fleet = lower_fleet(&q_ships, &nodes);
                    (
                        fleet.player.position,
                        fleet
                            .standing
                            .iter()
                            .map(|ship| ship.position)
                            .collect::<Vec<_>>(),
                    )
                },
            )
            .expect("the lowering runs");

        assert_eq!(
            fleet.0, player_at,
            "the player's ship stands where it was left, not at the range origin"
        );
        assert_eq!(
            fleet.1,
            vec![escort_at],
            "and an escort's pose is its own, not an offset from the player's"
        );
    }

    /// What you see in the editor is what you fly: the skin toggle rides
    /// the hand-off, so a ship built with a skin does not come up bare.
    #[test]
    fn the_skin_toggle_reaches_the_flown_ship() {
        for skinned in [false, true] {
            let lowered = LoweredFleet {
                player: LoweredShip {
                    skin: skinned,
                    ..default()
                },
                ..default()
            };
            let player = find(
                &sandbox_objects(default_world_objects(), &lowered, HullForm::Inline, true),
                PLAYER_ID,
            );
            let ScenarioObjectKind::Spaceship(ship) = player.kind else {
                panic!("the player object is a spaceship");
            };
            let ShipSource::Inline(hull) = &ship.hull else {
                panic!("the editor hands off an inline hull");
            };
            assert_eq!(
                hull.skin, skinned,
                "the editor's toggle decides whether the flown ship wears a skin"
            );
        }
    }
}
