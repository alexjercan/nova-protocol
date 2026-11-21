//! Background music: the track catalog, the context rule that picks which
//! situation's songs play, and the director that drives the
//! [`MusicDeck`](nova_sound::prelude::MusicDeck).
//!
//! ```text
//! game state, scenario, mood, dock --> music_context --> MusicDirector
//!   locks, AI engages, hits --> MusicMood --^             |
//!   PlayMusic / StopMusic --> ScenarioMusic --^           v
//!                                               MusicDeck (nova_sound)
//! ```
//!
//! The catalog is content: base and mods add `MusicTrack` rows, merged into
//! [`GameMusicTracks`].

mod catalog;
mod director;
mod rule;

pub use catalog::*;
pub use director::*;
pub use rule::*;

/// Glob-import surface: the music vocabulary.
pub mod prelude {
    pub use super::{catalog::*, director::*, rule::*};
}

#[cfg(test)]
pub(crate) mod tests;
