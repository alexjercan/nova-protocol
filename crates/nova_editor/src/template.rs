//! What a NEW document starts from.
//!
//! Three templates rather than one hard-coded world: the free-flight RANGE the
//! editor has always opened on, the duelling ARENA the generator's bench fights
//! in, and an EMPTY scenario for a builder who wants neither. File > New
//! Scenario picks between them.
//!
//! A template is a SEED and nothing else. Everything it puts in the document is
//! an ordinary node the moment it lands - movable, editable, deletable - and
//! nothing afterwards remembers which template a document came from.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::*;
use nova_ship::prelude::DEFAULT_SKYBOX_BRIGHTNESS;

use crate::{
    node::{ScenarioNode, ASTEROID_TEXTURE, DESTROY_SOUND},
    scenario::{
        default_script, default_world_objects, DEFAULT_SCENARIO_DESCRIPTION, DEFAULT_SCENARIO_NAME,
        DEFAULT_SKY,
    },
};

/// The world a new document is seeded with.
///
/// A `Component` as well as a value, because the picker's rows ARE the choice:
/// a row carries the template it starts, exactly as an object palette row
/// carries the kind it places.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ScenarioTemplate {
    /// The stock range: rocks, target hulks, dormant pickets, a planetoid and
    /// the script that wakes them.
    #[default]
    Range,
    /// A duelling ring: an empty middle to fight over, two rings of rock for
    /// depth and one pinned landmark.
    Arena,
    /// Nothing at all. Not even a light - a scenario that authors none renders
    /// black, which is what the note under the row says.
    Empty,
}

impl ScenarioTemplate {
    /// Every template, in the order the picker lists them. The default leads.
    pub(crate) const ALL: [Self; 3] = [Self::Range, Self::Arena, Self::Empty];

    /// The name on the row.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Range => "Free-Flight Range",
            Self::Arena => "Duelling Arena",
            Self::Empty => "Empty Scenario",
        }
    }

    /// What the row starts, in one line under it.
    pub(crate) fn hint(self) -> &'static str {
        match self {
            Self::Range => "Rocks, target hulks, dormant pickets and a planetoid, with the script that wakes them.",
            Self::Arena => "An empty middle to fight over, two rings of rock and one pinned landmark.",
            Self::Empty => "Nothing at all - not even a light, so the stage starts black.",
        }
    }

    /// The scenario node the document is founded on.
    pub(crate) fn settings(self) -> ScenarioNode {
        let (name, description) = match self {
            Self::Range => (DEFAULT_SCENARIO_NAME, DEFAULT_SCENARIO_DESCRIPTION),
            Self::Arena => (ARENA_NAME, ARENA_DESCRIPTION),
            Self::Empty => (EMPTY_NAME, EMPTY_DESCRIPTION),
        };
        ScenarioNode {
            name: name.to_string(),
            description: description.to_string(),
            cubemap: AssetRef::from(DEFAULT_SKY),
            skybox_brightness: DEFAULT_SKYBOX_BRIGHTNESS,
        }
    }

    /// The objects the document is seeded with, in the form a SAVE writes -
    /// so a template goes in through the same lift a loaded file does.
    pub(crate) fn objects(self) -> Vec<ScenarioObjectConfig> {
        match self {
            Self::Range => default_world_objects(),
            Self::Arena => arena_objects(),
            Self::Empty => vec![],
        }
    }

    /// The script the document is seeded with.
    pub(crate) fn script(self) -> Vec<ScenarioEventConfig> {
        match self {
            Self::Range => default_script(),
            Self::Arena => arena_script(),
            Self::Empty => vec![],
        }
    }
}

/// What the arena is called, and what the picker's details pane reads.
const ARENA_NAME: &str = "Arena";
const ARENA_DESCRIPTION: &str =
    "A duelling ring: an empty middle, two rings of rock and a pinned planetoid.";
/// And the empty one's, which still has to say something: a scenario with a
/// blank name is one the picker cannot list.
const EMPTY_NAME: &str = "New Scenario";
const EMPTY_DESCRIPTION: &str = "An empty scenario.";

/// The arena's landmark: far enough out to be scenery, massive enough to read
/// as a well, and PINNED so a fight cannot move it. Ported from the numbers
/// `wfc_arena` composes its backdrop from.
const ARENA_PLANETOID_POSITION: Meters3 = Meters3::new(-6_200.0, -1_400.0, -4_200.0);
const ARENA_PLANETOID_RADIUS: Meters = Meters(240.0);
const ARENA_PLANETOID_MASS: f32 = 40_000.0;
/// A fixed shape, so two documents from this template look alike.
const ARENA_PLANETOID_SEED: u32 = 20_260_816;
/// How far the arena's three-point rig stands off the origin, as a multiple of
/// the subject size the rig is composed around.
const ARENA_RIG_DISTANCE: f32 = 8.0;

/// One ring of dressing rock: a seeded scatter, kept off the fight plane so
/// cover never decides the fight.
struct Ring {
    id_prefix: &'static str,
    name: &'static str,
    seed: u64,
    count: u32,
    inner: Meters,
    outer: Meters,
    y: (Meters, Meters),
    radius: (Meters, Meters),
    separation: Meters,
}

/// Depth parallax UNDER the fight plane, and a sparser, wider ring past it so
/// the void has a middle distance.
const ARENA_RINGS: [Ring; 2] = [
    Ring {
        id_prefix: "arena_rock_low_",
        name: "Arena Rock",
        seed: 0x0A11_5A11,
        count: 14,
        inner: Meters(1_600.0),
        outer: Meters(2_400.0),
        y: (Meters(-700.0), Meters(-350.0)),
        radius: (Meters(15.0), Meters(40.0)),
        separation: Meters(500.0),
    },
    Ring {
        id_prefix: "arena_rock_far_",
        name: "Arena Rock",
        seed: 0x0FA2_1D77,
        count: 10,
        inner: Meters(3_200.0),
        outer: Meters(4_000.0),
        y: (Meters(-200.0), Meters(800.0)),
        radius: (Meters(20.0), Meters(50.0)),
        separation: Meters(600.0),
    },
];

/// The arena's hand-placed half: the landmark and the lights.
///
/// The lights are NODES rather than script actions, like the range's, so a
/// builder can see the rig in the tree and move it.
fn arena_objects() -> Vec<ScenarioObjectConfig> {
    let mut objects = vec![arena_planetoid()];
    objects.extend(ThreePointRig::around("arena", Meters3::ZERO, ARENA_RIG_DISTANCE).objects());
    objects
}

/// The arena's scattered half: two rings of rock, laid down on start.
///
/// A SCATTER rather than two dozen authored rocks, for the reason the range's
/// belts are one: a seeded region is one node to re-tune, and two dozen rocks
/// in the tree is a tree nobody can find a ship in.
fn arena_script() -> Vec<ScenarioEventConfig> {
    vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: ARENA_RINGS.iter().map(ring_scatter).collect(),
    }]
}

/// The landmark, as an authored object.
fn arena_planetoid() -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "arena_planetoid".to_string(),
            name: "Arena Planetoid".to_string(),
            position: ARENA_PLANETOID_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            // DIRECT paths, not `dep://` - see `crate::node::ASTEROID_TEXTURE`.
            material: KIND_ROCK.to_string(),
            destroy_sound: Some(AssetRef::from(DESTROY_SOUND)),
            radius: ARENA_PLANETOID_RADIUS,
            texture: AssetRef::from(ASTEROID_TEXTURE),
            mass: Some(ARENA_PLANETOID_MASS),
            invulnerable: true,
            seed: Some(ARENA_PLANETOID_SEED),
            lock_signature: None,
        }),
    }
}

/// One ring as a seeded scatter action.
fn ring_scatter(ring: &Ring) -> EventActionConfig {
    EventActionConfig::ScatterObjects(ScatterObjectsConfig {
        id_prefix: ring.id_prefix.to_string(),
        count: ring.count,
        seed: ring.seed,
        region: ScatterRegion::Ring {
            center: Meters3::ZERO,
            inner: ring.inner,
            outer: ring.outer,
            y_min: ring.y.0,
            y_max: ring.y.1,
        },
        template: ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: ring.id_prefix.to_string(),
                name: ring.name.to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                material: KIND_ROCK.to_string(),
                destroy_sound: Some(AssetRef::from(DESTROY_SOUND)),
                radius: ring.radius.0,
                texture: AssetRef::from(ASTEROID_TEXTURE),
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        },
        asteroid_radius: Some(ring.radius),
        // One kind, unlike the range's mixed belts: this is a backdrop, and a
        // ring of four materials at four kilometers reads as noise.
        asteroid_kinds: vec![(KIND_ROCK.to_string(), 1)],
        min_separation: Some(ring.separation),
    })
}

#[cfg(test)]
mod tests;
