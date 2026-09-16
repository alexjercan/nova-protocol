//! The hit voice: what a round SOUNDS like against what it hit.
//!
//! Both halves are CLOSED. The round is a [`DamageType`], which the engine owns
//! because damage resolution is engine work. The target is an
//! [`ImpactSurface`], which the engine owns for the same reason: the game has
//! exactly two substances a round can bite into - ship plate and stone - and
//! adding a third is engine work (a new component to tag it with, a new sample
//! to record), never a line of content.
//!
//! So the resolution is a `match`, not a lookup. Every pair has a voice, the
//! compiler proves it, and there is no sparse table to fall through, no id to
//! resolve and no silent pair. A mod that wants a different bang replaces the
//! sound FILE; it cannot invent an impact category, because a category nothing
//! can be made of would never play.

use bevy::prelude::*;

use crate::damage::prelude::DamageType;

/// The surface tag and the engine's four-sample bank.
pub mod prelude {
    pub use super::{ImpactSounds, ImpactSurface};
}

/// The bank's four samples, as committed base-mod assets.
const KINETIC_HULL_SOUND: &str = "base/sounds/impact.wav";
const KINETIC_ROCK_SOUND: &str = "base/sounds/impact_rock.wav";
const PIERCE_SOUND: &str = "base/sounds/impact_pierce.wav";
const EXPLOSIVE_SOUND: &str = "base/sounds/impact_explosive.wav";

/// What a damage target is MADE of.
///
/// Carried by every destructible body: sections are always [`Hull`], asteroids
/// and planets always [`Rock`]. A body without it takes [`Hull`], which is what
/// a nameless collider in this game is.
///
/// The audio observer walks UP `ChildOf` to find it, because an asteroid keeps
/// its health on a child node while the tag sits on the rock's root bundle.
///
/// [`Hull`]: ImpactSurface::Hull
/// [`Rock`]: ImpactSurface::Rock
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum ImpactSurface {
    /// Ship plate: every section in the catalog.
    #[default]
    Hull,
    /// Asteroid and planet stone.
    Rock,
}

/// The engine's impact samples, loaded once at startup.
///
/// A fixed four rather than a registry, because the `(DamageType,
/// ImpactSurface)` product is fixed: two of the three rounds sound the same
/// whatever they hit (a penetrator's voice is the PENETRATION, a blast's is the
/// blast), and only the kinetic slug cares about the substance under it.
#[derive(Resource, Clone, Debug)]
pub struct ImpactSounds {
    /// A slug into ship plate.
    pub kinetic_hull: Handle<AudioSource>,
    /// A slug into stone.
    pub kinetic_rock: Handle<AudioSource>,
    /// A penetrator, into anything.
    pub pierce: Handle<AudioSource>,
    /// A blast, against anything.
    pub explosive: Handle<AudioSource>,
}

impl ImpactSounds {
    /// The voice for a hit. Total: every pair resolves, and the compiler says
    /// so, which is the whole reason both axes are closed.
    pub fn sound(&self, damage: DamageType, surface: ImpactSurface) -> &Handle<AudioSource> {
        match (damage, surface) {
            (DamageType::Kinetic, ImpactSurface::Rock) => &self.kinetic_rock,
            (DamageType::Kinetic, ImpactSurface::Hull) => &self.kinetic_hull,
            (DamageType::Pierce, _) => &self.pierce,
            (DamageType::Explosive, _) => &self.explosive,
        }
    }
}

impl FromWorld for ImpactSounds {
    fn from_world(world: &mut World) -> Self {
        // A rig with no asset server (a headless unit test) gets empty
        // handles rather than a panic, exactly as the old empty registry did:
        // a hit resolves a voice that plays nothing.
        let Some(assets) = world.get_resource::<AssetServer>() else {
            return Self {
                kinetic_hull: Handle::default(),
                kinetic_rock: Handle::default(),
                pierce: Handle::default(),
                explosive: Handle::default(),
            };
        };
        Self {
            kinetic_hull: assets.load(KINETIC_HULL_SOUND),
            kinetic_rock: assets.load(KINETIC_ROCK_SOUND),
            pierce: assets.load(PIERCE_SOUND),
            explosive: assets.load(EXPLOSIVE_SOUND),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bank as the engine plugin builds it: four paths loaded through a
    /// real asset server, so the handles differ exactly as the files do.
    fn bank() -> ImpactSounds {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<ImpactSounds>();
        app.world().resource::<ImpactSounds>().clone()
    }

    /// Four samples and six pairs, so which pairs SHARE a sample is a design
    /// decision worth pinning: the slug is the only round that hears what it
    /// hit, and it must never take stone's voice against plate or the reverse.
    #[test]
    fn every_damage_and_surface_pair_resolves_to_its_own_voice() {
        let bank = bank();
        let voices = [
            &bank.kinetic_hull,
            &bank.kinetic_rock,
            &bank.pierce,
            &bank.explosive,
        ];
        for (index, voice) in voices.iter().enumerate() {
            assert!(
                !voices[index + 1..].contains(voice),
                "two of the four samples loaded as one handle"
            );
        }
        for surface in [ImpactSurface::Hull, ImpactSurface::Rock] {
            assert_eq!(*bank.sound(DamageType::Pierce, surface), bank.pierce);
            assert_eq!(*bank.sound(DamageType::Explosive, surface), bank.explosive);
        }
        assert_eq!(
            *bank.sound(DamageType::Kinetic, ImpactSurface::Hull),
            bank.kinetic_hull
        );
        assert_eq!(
            *bank.sound(DamageType::Kinetic, ImpactSurface::Rock),
            bank.kinetic_rock,
            "a slug into stone must not ring like a bulkhead"
        );
    }

    /// The committed files. The bank names four paths and nothing checks them
    /// at compile time, so a renamed sample would go silent in the shipped
    /// game and pass every headless test.
    #[test]
    fn the_four_samples_are_files_the_repo_ships() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/");
        for path in [
            KINETIC_HULL_SOUND,
            KINETIC_ROCK_SOUND,
            PIERCE_SOUND,
            EXPLOSIVE_SOUND,
        ] {
            let full = format!("{root}{path}");
            assert!(
                std::path::Path::new(&full).is_file(),
                "the impact bank names {path}, which the repo does not ship"
            );
        }
    }
}
