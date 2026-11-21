//! The context rule: which situation's songs play, from one frame of facts.
//! Pure, so the table and the mood's hysteresis are tested without a world.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use nova_gameplay::prelude::{GameMode, GameStates};

use crate::loader::prelude::ScenarioRole;

/// How long a hostile lock on the player, or a hostile AI engaging the
/// player, must hold before the mood turns to [`MusicMoodType::Threat`]. A
/// lock that only flickers past is not a contact worth a song.
pub const THREAT_HOLD_SECS: f32 = 2.0;

/// How long the mood stays [`MusicMoodType::Threat`] after the last hostile
/// lock or engage. Long enough that a lock that blinks on and off never
/// leaves the threat song.
pub const THREAT_RELEASE_SECS: f32 = 10.0;

/// How long the mood stays [`MusicMoodType::Combat`] after the last combat
/// hit: weapon fire between the player's ship and a hostile one. A lull
/// between passes keeps the combat song.
pub const COMBAT_RELEASE_SECS: f32 = 15.0;

/// What the music is for: which situation's songs play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MusicContextType {
    /// No music: the boot load, the editor and Mod Hub, and the load between
    /// the menu and the world. Not a playlist.
    Silent,
    /// The main menu, with its backdrop scenes.
    Menu,
    /// Flight with no fight and not docked at a station.
    Cruise,
    /// A hostile has the player in its lock or is engaging the player.
    Threat,
    /// Weapon fire lands between the player's ship and a hostile one.
    Combat,
    /// The player's ship is docked at a station.
    Docked,
    /// A scenario's own song from `PlayMusic`. Not a playlist.
    Scenario,
}

impl MusicContextType {
    /// Whether tracks may name this situation: every situation but `Silent`
    /// and `Scenario`, which play no catalog song.
    pub const fn has_playlist(self) -> bool {
        !matches!(self, Self::Silent | Self::Scenario)
    }

    /// The fade a song of this situation starts with, in seconds.
    pub const fn fade_in_secs(self) -> f32 {
        match self {
            // Into a fight the music must answer the first hit.
            Self::Combat => 1.5,
            Self::Menu | Self::Threat | Self::Docked => 3.0,
            Self::Cruise => 4.0,
            Self::Silent | Self::Scenario => 0.0,
        }
    }

    /// The fade a song of this situation leaves with, in seconds.
    pub const fn fade_out_secs(self) -> f32 {
        match self {
            // A fight winds down slower than it starts.
            Self::Combat => 5.0,
            Self::Docked => 4.0,
            Self::Threat => 3.0,
            // The menu theme leaves as the load starts, and a scenario's
            // song leaves as its scenario ends: both short, so the next
            // situation is not late.
            Self::Menu | Self::Cruise | Self::Scenario => 2.0,
            Self::Silent => 0.0,
        }
    }

    /// The silence between two songs of this situation, in seconds, or
    /// `None` when the next song follows at once.
    ///
    /// Calm situations breathe: about one minute in five of cruise is quiet.
    /// A fight and the menu play without a gap.
    pub const fn gap_secs(self) -> Option<RangeInclusive<f32>> {
        match self {
            Self::Cruise => Some(30.0..=120.0),
            Self::Docked => Some(10.0..=30.0),
            Self::Silent | Self::Menu | Self::Threat | Self::Combat | Self::Scenario => None,
        }
    }

    /// The silence before the first song when this situation starts from
    /// silence, in seconds, or `None` when its first song starts at once.
    ///
    /// Cruise waits after the world loads or the player undocks, so the
    /// first song does not land on the loading screen's heels.
    pub const fn entry_gap_secs(self) -> Option<RangeInclusive<f32>> {
        match self {
            Self::Cruise => Some(20.0..=40.0),
            Self::Docked => Some(10.0..=30.0),
            Self::Silent | Self::Menu | Self::Threat | Self::Combat | Self::Scenario => None,
        }
    }
}

/// The silence before the next cruise song after a fight, in seconds. The
/// song the fight cut does not resume.
pub const AFTER_FIGHT_GAP_SECS: RangeInclusive<f32> = 10.0..=20.0;

/// The fade of a crossfade from a song of `from` to one of `to`, in seconds.
///
/// The deck runs one fade for both songs, so it takes the longer of the
/// leaving song's fade-out and the entering song's fade-in. Into a fight it
/// is always the combat fade-in: the music must answer the first hit.
pub fn crossfade_secs(from: MusicContextType, to: MusicContextType) -> f32 {
    if to == MusicContextType::Combat {
        return to.fade_in_secs();
    }
    from.fade_out_secs().max(to.fade_in_secs())
}

/// How tense the player's situation is, from the mood's hysteresis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum MusicMoodType {
    /// No hostile in contact.
    #[default]
    Calm,
    /// A hostile holds its lock on the player or engages the player.
    Threat,
    /// Weapon fire lands between the player's ship and a hostile one.
    Combat,
}

/// The mood with its hysteresis: it moves up fast and comes down slowly, so
/// a lock that blinks or a lull in a fight never makes the music flap.
///
/// ```text
/// Calm   -> Threat  signal held THREAT_HOLD_SECS
/// any    -> Combat  a combat hit, at once
/// Combat -> Threat  COMBAT_RELEASE_SECS with no hit, signal seen within
///                   THREAT_RELEASE_SECS
/// Combat -> Calm    COMBAT_RELEASE_SECS with no hit, no signal for
///                   THREAT_RELEASE_SECS
/// Threat -> Calm    THREAT_RELEASE_SECS with no signal
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct MusicMood {
    mood: MusicMoodType,
    /// How long the signal has held without a break, in seconds.
    held: f32,
    /// Seconds since the signal was last on.
    since_signal: f32,
    /// Seconds since the last hit.
    since_hit: f32,
}

impl Default for MusicMood {
    fn default() -> Self {
        Self {
            mood: MusicMoodType::Calm,
            held: 0.0,
            since_signal: f32::INFINITY,
            since_hit: f32::INFINITY,
        }
    }
}

impl MusicMood {
    /// The mood after `dt` seconds in which the threat `signal` (a hostile
    /// lock on the player or a hostile AI engaging the player) was on or
    /// off, and a combat `hit` (weapon fire between the player's ship and a
    /// hostile one) landed or not.
    #[must_use]
    pub fn step(self, dt: f32, signal: bool, hit: bool) -> Self {
        let (held, since_signal) = if signal {
            (self.held + dt, 0.0)
        } else {
            (0.0, self.since_signal + dt)
        };
        let since_hit = if hit { 0.0 } else { self.since_hit + dt };
        let threat_lingers = since_signal < THREAT_RELEASE_SECS;
        let mood = match self.mood {
            _ if hit => MusicMoodType::Combat,
            MusicMoodType::Combat if since_hit < COMBAT_RELEASE_SECS => MusicMoodType::Combat,
            MusicMoodType::Combat | MusicMoodType::Threat if threat_lingers => {
                MusicMoodType::Threat
            }
            MusicMoodType::Combat | MusicMoodType::Threat => MusicMoodType::Calm,
            MusicMoodType::Calm if held >= THREAT_HOLD_SECS => MusicMoodType::Threat,
            MusicMoodType::Calm => MusicMoodType::Calm,
        };
        Self {
            mood,
            held,
            since_signal,
            since_hit,
        }
    }

    /// The mood now.
    pub fn mood(self) -> MusicMoodType {
        self.mood
    }
}

/// What a running scenario song does when a fight starts: the `PlayMusic`
/// action's `in_combat` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MusicInCombatType {
    /// A fight takes over with the combat songs, and the scenario song does
    /// not come back.
    Yield,
    /// The scenario song plays through fights: a boss theme.
    Keep,
}

/// What a scenario song does when it plays to its end: the `PlayMusic`
/// action's `then` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MusicThenType {
    /// Hand back to the context rule, which plays its next song after the
    /// situation's normal gap.
    Resume,
    /// Play the song again, until `StopMusic` or the scenario ends.
    Loop,
}

/// One frame of facts the rule reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicFacts {
    /// The top-level game state.
    pub game: GameStates,
    /// Editor flow or New Game.
    pub mode: GameMode,
    /// The live scenario's role, or `None` when no scenario is live.
    pub scenario: Option<ScenarioRole>,
    /// The running scenario song's combat rule, or `None` when no scenario
    /// song plays.
    pub scenario_song: Option<MusicInCombatType>,
    /// The mood.
    pub mood: MusicMoodType,
    /// Whether the player's ship is docked at a station.
    pub docked_at_station: bool,
}

/// The situation whose songs play, first match wins. `previous` is the
/// situation of the last frame, kept through the load gap.
///
/// 1. The boot load is silent.
/// 2. The main menu, and a menu backdrop scene wherever it runs, is Menu:
///    a backdrop's fights do not count.
/// 3. The editor and Mod Hub (Sandbox, no scenario) are silent.
/// 4. The load gap (New Game, no scenario yet) keeps the song, except the
///    menu theme and a scenario song, which end with what they belong to.
/// 5. A scenario song plays, unless it yields and the mood is Combat.
/// 6. Combat, then Threat, from the mood.
/// 7. Docked at a station.
/// 8. Anything else in flight is Cruise: the open world, chapters, lessons
///    and the editor's test flight.
pub fn music_context(facts: &MusicFacts, previous: MusicContextType) -> MusicContextType {
    match facts.game {
        GameStates::Loading => return MusicContextType::Silent,
        GameStates::MainMenu => return MusicContextType::Menu,
        GameStates::Playing => {}
    }
    match (facts.scenario, facts.mode) {
        (Some(ScenarioRole::Backdrop), _) => return MusicContextType::Menu,
        (None, GameMode::Sandbox) => return MusicContextType::Silent,
        (None, GameMode::NewGame) => {
            return match previous {
                MusicContextType::Menu | MusicContextType::Scenario => MusicContextType::Silent,
                kept => kept,
            };
        }
        (Some(ScenarioRole::Chapter | ScenarioRole::Lesson), _) => {}
    }
    match (facts.scenario_song, facts.mood) {
        (Some(MusicInCombatType::Keep), _) => return MusicContextType::Scenario,
        (Some(MusicInCombatType::Yield), mood) if mood != MusicMoodType::Combat => {
            return MusicContextType::Scenario;
        }
        _ => {}
    }
    match facts.mood {
        MusicMoodType::Combat => MusicContextType::Combat,
        MusicMoodType::Threat => MusicContextType::Threat,
        MusicMoodType::Calm if facts.docked_at_station => MusicContextType::Docked,
        MusicMoodType::Calm => MusicContextType::Cruise,
    }
}
