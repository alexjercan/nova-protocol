//! The music director: it reads the game each frame, runs the context rule,
//! and tells the [`MusicDeck`] what to play.
//!
//! The decisions live in [`MusicDirector::tick`], which reads plain facts and
//! returns [`MusicCue`]s, so the tests drive it without a world or a sound
//! device. [`direct_music`] gathers the facts and hands the cues to the deck.

use std::{collections::HashMap, ops::RangeInclusive, time::Duration};

use bevy::{audio::AudioSource, ecs::system::SystemParam, prelude::*};
use bevy_rand::prelude::{ForkableRng, GlobalRng, WyRand};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_sound::prelude::*;
use rand::{seq::SliceRandom as _, Rng, RngExt as _};

use super::{
    crossfade_secs, music_context, GameMusicTracks, MusicContextType, MusicFacts,
    MusicInCombatType, MusicMood, MusicMoodType, MusicThenType, MusicTrackDesignId,
    AFTER_FIGHT_GAP_SECS,
};
use crate::loader::prelude::{CurrentScenario, ScenarioRole, UnloadScenario};

/// How far the music ducks while the pause menu, the command monitor or the
/// debug window is open, in dB. The song stays audible under the menu, so
/// the player knows the game kept its place. Tab (the interface screens)
/// does not duck: the player is still flying.
pub const PAUSE_DUCK_DB: f32 = -8.0;

/// How fast the pause duck falls, in seconds: quick, so the menu answers the
/// key.
pub const PAUSE_DUCK_FADE_SECS: f32 = 0.5;

/// How fast the pause duck lifts, in seconds: slower than it falls, so the
/// return to flight does not jump.
pub const PAUSE_LIFT_FADE_SECS: f32 = 1.0;

/// The system set of the director, in `Update`. It runs after the scenario's
/// actions, so a `PlayMusic` beat is heard in the frame it fires; the deck
/// applies what it asks in `PostUpdate`
/// ([`SoundSystems`](nova_sound::prelude::SoundSystems)).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MusicSystems;

/// Runs the music director: the context rule, the playlists and the pause
/// duck. Added by `NovaScenarioPlugin`.
pub struct MusicDirectorPlugin;

impl Plugin for MusicDirectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMusicTracks>()
            .init_resource::<ScenarioMusic>()
            .init_resource::<MusicDirector>()
            .init_resource::<MusicHits>()
            .add_observer(hear_music_hits)
            .add_observer(end_scenario_music)
            .add_systems(Update, direct_music.in_set(MusicSystems));
    }
}

/// A song the director plays: a catalog track, or the scenario's own song.
#[derive(Clone, Debug, PartialEq)]
pub enum MusicSong {
    /// A track of the music catalog.
    Track(MusicTrackDesignId),
    /// The song of the scenario's `PlayMusic` beat.
    Scenario(Handle<AudioSource>),
}

/// What the director asks of the deck.
#[derive(Clone, Debug, PartialEq)]
pub enum MusicCue {
    /// Crossfade to `song` over `fade`.
    Play {
        /// The song.
        song: MusicSong,
        /// The crossfade.
        fade: Duration,
    },
    /// Fade every song out over `fade`.
    Stop {
        /// The fade.
        fade: Duration,
    },
    /// Duck the music to `db` over `fade`.
    Duck {
        /// The attenuation, zero or below.
        db: f32,
        /// The ramp.
        fade: Duration,
    },
}

/// A scenario's own song, from its `PlayMusic` beat.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioSong {
    /// The song.
    pub sound: Handle<AudioSource>,
    /// Its fade in, or the crossfade from the song that plays.
    pub fade_in: Duration,
    /// What it does at its end.
    pub then: MusicThenType,
    /// What it does when a fight starts.
    pub in_combat: MusicInCombatType,
}

#[derive(Clone, Debug, PartialEq)]
enum ScenarioMusicRequest {
    Play(ScenarioSong),
    Stop(Duration),
}

/// The scenario's music: the song its `PlayMusic` beat started, if it still
/// plays, and the beat the director has not read yet. Cleared when the
/// scenario unloads, so a scenario's song never outlives it.
#[derive(Resource, Clone, Debug, Default)]
pub struct ScenarioMusic {
    song: Option<ScenarioSong>,
    request: Option<ScenarioMusicRequest>,
}

impl ScenarioMusic {
    /// Play `song` in place of the context rule's songs.
    pub fn play(&mut self, song: ScenarioSong) {
        self.request = Some(ScenarioMusicRequest::Play(song));
    }

    /// End the scenario's song over `fade`. With no scenario song, the song
    /// that plays fades out and its situation waits its normal gap.
    pub fn stop(&mut self, fade: Duration) {
        self.request = Some(ScenarioMusicRequest::Stop(fade));
    }

    /// The scenario's song, while it plays.
    pub fn song(&self) -> Option<&ScenarioSong> {
        self.song.as_ref()
    }
}

/// Whether a combat hit landed since the director last looked: weapon fire
/// between the player's ship and a hostile ship. Set by [`hear_music_hits`].
#[derive(Resource, Debug, Default)]
pub struct MusicHits(pub bool);

/// One situation's playlist: its tracks in a shuffled order, drawn one at a
/// time and shuffled again when spent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MusicPlaylist {
    bag: Vec<MusicTrackDesignId>,
    last: Option<MusicTrackDesignId>,
}

impl MusicPlaylist {
    /// The next track of `tracks`, or `None` when it is empty.
    ///
    /// A new shuffle never opens with the track that just played, so the
    /// same song never plays twice in a row unless it is the only one. A
    /// track the catalog no longer holds is skipped.
    pub fn next(
        &mut self,
        tracks: &[MusicTrackDesignId],
        rng: &mut impl Rng,
    ) -> Option<MusicTrackDesignId> {
        self.bag.retain(|id| tracks.contains(id));
        if self.bag.is_empty() {
            self.bag = tracks.to_vec();
            self.bag.shuffle(rng);
            // The bag is drawn from its end.
            let end = self.bag.len().saturating_sub(1);
            if end > 0 && self.bag.last() == self.last.as_ref() {
                self.bag.swap(0, end);
            }
        }
        let next = self.bag.pop()?;
        self.last = Some(next.clone());
        Some(next)
    }
}

/// Whether `pause` ducks the music: the pause menu, the command monitor and
/// the debug window do. The interface screens (Tab) do not: the player is
/// still flying.
pub fn pause_ducks_music(pause: &PauseStates) -> bool {
    match pause {
        PauseStates::Paused | PauseStates::Commands | PauseStates::Debug => true,
        PauseStates::Unpaused | PauseStates::Interface => false,
    }
}

/// A silence of `range` seconds, drawn from `rng`.
pub fn draw_gap(range: RangeInclusive<f32>, rng: &mut impl Rng) -> f32 {
    rng.random_range(range)
}

/// One frame of what the director reads.
#[derive(Debug)]
pub struct MusicTick<'a> {
    /// The facts of the rule. The director writes the mood.
    pub facts: MusicFacts,
    /// Whether a hostile holds its lock on the player or engages the player.
    pub signal: bool,
    /// Whether a combat hit landed: weapon fire between the player's ship
    /// and a hostile one.
    pub hit: bool,
    /// Game seconds since the last frame: the mood's clock, which stops in
    /// a pause.
    pub game_dt: f32,
    /// Real seconds since the last frame: the gaps' clock, the deck's too.
    pub real_dt: f32,
    /// What the deck's current song is doing.
    pub deck: MusicDeckStatusType,
    /// Whether a menu that ducks the music is open.
    pub paused: bool,
    /// The catalog.
    pub tracks: &'a GameMusicTracks,
}

/// The song that just left, so a situation that comes back before it faded
/// out picks it up where it is.
#[derive(Clone, Debug, PartialEq)]
struct LeavingSong {
    context: MusicContextType,
    song: MusicSong,
    secs: f32,
}

/// A switch the director holds back for a gap: the song of `held` plays on
/// with no fade, and when the gap ends the next song of the director's
/// situation crossfades in over that situation's fade-in.
///
/// The owner asked for this (2026-11-21): where a change of situation would
/// leave a pause with no music, the song that plays fills the pause.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingSwitch {
    /// The situation whose song plays on through the gap.
    pub held: MusicContextType,
    /// The real seconds left until the switch.
    pub due_secs: f32,
}

/// The gap a change from `from` to `to` waits before `to`'s first song, in
/// seconds, or `None` when `to` starts its song at once or plays none.
///
/// After a fight, cruise waits [`AFTER_FIGHT_GAP_SECS`]. After a scenario
/// song, cruise waits its normal gap between songs. Otherwise cruise and
/// docked wait their entry gap.
pub fn switch_gap(from: MusicContextType, to: MusicContextType) -> Option<RangeInclusive<f32>> {
    match (from, to) {
        (MusicContextType::Threat | MusicContextType::Combat, MusicContextType::Cruise) => {
            Some(AFTER_FIGHT_GAP_SECS)
        }
        (MusicContextType::Scenario, MusicContextType::Cruise) => to.gap_secs(),
        (_, MusicContextType::Cruise | MusicContextType::Docked) => to.entry_gap_secs(),
        _ => None,
    }
}

/// Whether a song of `from` plays on through the gap of a change, in place
/// of a fade-out. A flight song does. The menu theme and a scenario's song
/// end with what they belong to, so they fade out at once.
pub const fn song_plays_on_through_a_gap(from: MusicContextType) -> bool {
    matches!(
        from,
        MusicContextType::Cruise
            | MusicContextType::Threat
            | MusicContextType::Combat
            | MusicContextType::Docked
    )
}

/// The director's state: the situation, the mood, what plays, the silence it
/// waits out or the switch it holds back, and one playlist per situation.
#[derive(Resource, Debug)]
pub struct MusicDirector {
    context: MusicContextType,
    mood: MusicMood,
    current: Option<MusicSong>,
    leaving: Option<LeavingSong>,
    wait: Option<f32>,
    pending: Option<PendingSwitch>,
    playlists: HashMap<MusicContextType, MusicPlaylist>,
    ducked: bool,
    stop_fade: Option<Duration>,
    rng: Option<WyRand>,
}

impl Default for MusicDirector {
    fn default() -> Self {
        Self {
            context: MusicContextType::Silent,
            mood: MusicMood::default(),
            current: None,
            leaving: None,
            wait: None,
            pending: None,
            playlists: HashMap::new(),
            ducked: false,
            stop_fade: None,
            rng: None,
        }
    }
}

fn secs(seconds: f32) -> Duration {
    Duration::from_secs_f32(seconds.max(0.0))
}

impl MusicDirector {
    /// The situation whose songs play.
    pub fn context(&self) -> MusicContextType {
        self.context
    }

    /// The mood.
    pub fn mood(&self) -> MusicMoodType {
        self.mood.mood()
    }

    /// The song the director has the deck play, if any. While a switch is
    /// pending, the song of the situation it left.
    pub fn current(&self) -> Option<&MusicSong> {
        self.current.as_ref()
    }

    /// The silence left before the next song, in real seconds, if the
    /// director waits with no song.
    pub fn wait(&self) -> Option<f32> {
        self.wait
    }

    /// The switch the director holds back while the last situation's song
    /// plays on, if any.
    pub fn pending(&self) -> Option<PendingSwitch> {
        self.pending
    }

    /// Run one frame: read `scenario`'s beats, step the mood, run the rule,
    /// and return what the deck must do.
    pub fn tick(
        &mut self,
        mut tick: MusicTick<'_>,
        scenario: &mut ScenarioMusic,
        rng: &mut impl Rng,
    ) -> Vec<MusicCue> {
        let mut cues = Vec::new();

        let in_flight = tick.facts.game == GameStates::Playing
            && matches!(
                tick.facts.scenario,
                Some(ScenarioRole::Chapter | ScenarioRole::Lesson)
            );
        self.mood = if in_flight {
            self.mood.step(tick.game_dt, tick.signal, tick.hit)
        } else {
            MusicMood::default()
        };
        tick.facts.mood = self.mood.mood();

        let mut restart = false;
        match scenario.request.take() {
            Some(ScenarioMusicRequest::Play(song)) if in_flight => {
                scenario.song = Some(song);
                restart = true;
            }
            Some(ScenarioMusicRequest::Stop(fade)) if in_flight => {
                if scenario.song.take().is_some() {
                    self.stop_fade = Some(fade);
                } else if self.current.take().is_some() {
                    self.pending = None;
                    cues.push(MusicCue::Stop { fade });
                    self.wait = self.context.gap_secs().map(|range| draw_gap(range, rng));
                }
            }
            _ => {}
        }
        if !in_flight {
            scenario.song = None;
        }
        tick.facts.scenario_song = scenario.song.as_ref().map(|song| song.in_combat);

        if let Some(leaving) = &mut self.leaving {
            leaving.secs -= tick.real_dt;
            if leaving.secs <= 0.0 {
                self.leaving = None;
            }
        }

        let context = music_context(&tick.facts, self.context);
        if context != self.context {
            info!(
                "nova music: context {:?} -> {context:?} (mood {:?})",
                self.context, tick.facts.mood
            );
            self.enter(self.context, context, &tick, scenario, rng, &mut cues);
            self.context = context;
        } else if restart && context == MusicContextType::Scenario {
            if let Some(song) = scenario.song.clone() {
                self.play(MusicSong::Scenario(song.sound), song.fade_in, &mut cues);
            }
        } else {
            self.carry_on(&tick, scenario, rng, &mut cues);
        }

        if tick.paused != self.ducked {
            self.ducked = tick.paused;
            cues.push(if tick.paused {
                MusicCue::Duck {
                    db: PAUSE_DUCK_DB,
                    fade: secs(PAUSE_DUCK_FADE_SECS),
                }
            } else {
                MusicCue::Duck {
                    db: 0.0,
                    fade: secs(PAUSE_LIFT_FADE_SECS),
                }
            });
        }
        cues
    }

    /// Change from the songs of `from` to those of `to`.
    fn enter(
        &mut self,
        from: MusicContextType,
        to: MusicContextType,
        tick: &MusicTick<'_>,
        scenario: &mut ScenarioMusic,
        rng: &mut impl Rng,
        cues: &mut Vec<MusicCue>,
    ) {
        let stop_fade = self.stop_fade.take();
        // A song held through a gap is a song of the situation it came from:
        // its fades, and the next gap, follow that situation.
        let from = self.pending.take().map_or(from, |pending| pending.held);
        if to == from {
            info!("nova music: back to {to:?} before the gap ended; its song plays on");
            return;
        }
        if stop_fade.is_none() && self.current.is_some() {
            if let Some(gap) = switch_gap(from, to).filter(|_| song_plays_on_through_a_gap(from)) {
                let due_secs = draw_gap(gap, rng);
                info!("nova music: the {from:?} song plays on {due_secs:.1} s into {to:?}");
                self.wait = None;
                self.pending = Some(PendingSwitch {
                    held: from,
                    due_secs,
                });
                return;
            }
        }
        let fade_out = stop_fade.unwrap_or_else(|| secs(from.fade_out_secs()));
        let crossfade = stop_fade.unwrap_or_else(|| secs(crossfade_secs(from, to)));
        self.wait = None;
        let leaving = self.current.take();
        let returning = self
            .leaving
            .take()
            .filter(|leaving| leaving.context == to)
            .map(|leaving| leaving.song);
        if let Some(song) = &leaving {
            self.leaving = Some(LeavingSong {
                context: from,
                song: song.clone(),
                secs: fade_out.as_secs_f32(),
            });
        }

        match to {
            MusicContextType::Silent => {
                if leaving.is_some() {
                    cues.push(MusicCue::Stop { fade: fade_out });
                }
            }
            MusicContextType::Scenario => {
                if let Some(song) = scenario.song.clone() {
                    self.play(MusicSong::Scenario(song.sound), song.fade_in, cues);
                }
            }
            MusicContextType::Menu | MusicContextType::Threat | MusicContextType::Combat => {
                if to == MusicContextType::Combat
                    && from == MusicContextType::Scenario
                    && scenario
                        .song
                        .as_ref()
                        .is_some_and(|song| song.in_combat == MusicInCombatType::Yield)
                {
                    scenario.song = None;
                }
                match returning {
                    Some(song) => self.play(song, crossfade, cues),
                    None => self.start(to, crossfade, tick, rng, cues),
                }
            }
            MusicContextType::Docked => match (returning, &leaving) {
                (Some(song), _) => self.play(song, crossfade, cues),
                (None, Some(_)) => self.start(to, crossfade, tick, rng, cues),
                (None, None) => self.wait = switch_gap(from, to).map(|r| draw_gap(r, rng)),
            },
            MusicContextType::Cruise => {
                if leaving.is_some() {
                    cues.push(MusicCue::Stop { fade: fade_out });
                }
                self.wait = switch_gap(from, to).map(|range| draw_gap(range, rng));
            }
        }
    }

    /// Stay in the situation: play out a held switch, wait out a silence,
    /// or follow a song that ended.
    fn carry_on(
        &mut self,
        tick: &MusicTick<'_>,
        scenario: &mut ScenarioMusic,
        rng: &mut impl Rng,
        cues: &mut Vec<MusicCue>,
    ) {
        let context = self.context;
        if let Some(pending) = &mut self.pending {
            if tick.deck == MusicDeckStatusType::Ended {
                // The held song ended by itself: the rest of the gap is
                // silent, and the next song does not start early.
                self.wait = Some(pending.due_secs);
                self.pending = None;
                self.current = None;
            } else {
                pending.due_secs -= tick.real_dt;
                if pending.due_secs <= 0.0 {
                    self.pending = None;
                    self.start(context, secs(context.fade_in_secs()), tick, rng, cues);
                }
                return;
            }
        }
        if let Some(wait) = &mut self.wait {
            *wait -= tick.real_dt;
            if *wait <= 0.0 {
                self.wait = None;
                self.start(context, secs(context.fade_in_secs()), tick, rng, cues);
            }
            return;
        }
        if self.current.is_none() || tick.deck != MusicDeckStatusType::Ended {
            return;
        }
        self.current = None;
        if context == MusicContextType::Scenario {
            let Some(song) = scenario.song.clone() else {
                return;
            };
            match song.then {
                MusicThenType::Loop => {
                    self.play(MusicSong::Scenario(song.sound), Duration::ZERO, cues);
                }
                // The rule takes over next frame, and waits the normal gap.
                MusicThenType::Resume => scenario.song = None,
            }
            return;
        }
        match context.gap_secs() {
            Some(range) => self.wait = Some(draw_gap(range, rng)),
            // The song ended at its own end: the next one starts at once.
            None => self.start(context, Duration::ZERO, tick, rng, cues),
        }
    }

    /// Play the next track of `context`'s playlist over `fade`.
    fn start(
        &mut self,
        context: MusicContextType,
        fade: Duration,
        tick: &MusicTick<'_>,
        rng: &mut impl Rng,
        cues: &mut Vec<MusicCue>,
    ) {
        let tracks: Vec<MusicTrackDesignId> = tick
            .tracks
            .playlist(context)
            .map(|track| track.id.clone())
            .collect();
        let playlist = self.playlists.entry(context).or_default();
        let Some(id) = playlist.next(&tracks, rng) else {
            info!("nova music: {context:?} has no tracks; it stays silent");
            if self.current.take().is_some() {
                cues.push(MusicCue::Stop { fade });
            }
            return;
        };
        if let Some(track) = tick.tracks.get(&id) {
            info!(
                "nova music: {context:?} plays {id} \"{}\" by {} (fade {:.1} s)",
                track.title,
                track.artist,
                fade.as_secs_f32()
            );
        }
        self.play(MusicSong::Track(id), fade, cues);
    }

    fn play(&mut self, song: MusicSong, fade: Duration, cues: &mut Vec<MusicCue>) {
        self.current = Some(song.clone());
        cues.push(MusicCue::Play { song, fade });
    }
}

/// What the director reads of the player's ship: who locks or hunts it, and
/// where it is docked.
#[derive(SystemParam)]
pub struct MusicShipFacts<'w, 's> {
    q_player: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static Allegiance>,
            Option<&'static DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_lockers: Query<
        'w,
        's,
        (
            Entity,
            &'static CombatLock,
            Option<&'static Allegiance>,
            Option<&'static RetaliationTarget>,
        ),
    >,
    q_hunters: Query<
        'w,
        's,
        (
            Entity,
            &'static AITarget,
            &'static AIBehaviorState,
            Option<&'static Allegiance>,
            Option<&'static RetaliationTarget>,
        ),
    >,
    q_station_docks: Query<'w, 's, (), With<StationDock>>,
}

impl MusicShipFacts<'_, '_> {
    /// Whether a hostile holds its lock on the player's ship or engages it,
    /// and whether the player's ship is docked at a station.
    fn sample(&self) -> (bool, bool) {
        let Some((player, mine, docked)) = self.q_player.iter().next() else {
            return (false, false);
        };
        let us = RelationParty {
            entity: player,
            allegiance: mine,
            retaliation: None,
        };
        let locked = self
            .q_lockers
            .iter()
            .any(|(entity, lock, theirs, answering)| {
                let them = RelationParty {
                    entity,
                    allegiance: theirs,
                    retaliation: answering,
                };
                lock.is_hostile_on(them, us)
            });
        let hunted = self
            .q_hunters
            .iter()
            .any(|(entity, target, state, theirs, answering)| {
                let them = RelationParty {
                    entity,
                    allegiance: theirs,
                    retaliation: answering,
                };
                target.0 == Some(player)
                    && state.engages()
                    && ship_relation(them, us) == Relation::Hostile
            });
        let at_station =
            docked.is_some_and(|docked| self.q_station_docks.contains(docked.connection));
        (locked || hunted, at_station)
    }
}

/// What the director reads of the game's state.
#[derive(SystemParam)]
pub struct MusicGameFacts<'w> {
    game: Option<Res<'w, State<GameStates>>>,
    pause: Option<Res<'w, State<PauseStates>>>,
    mode: Option<Res<'w, GameMode>>,
    scenario: Option<Res<'w, CurrentScenario>>,
    game_time: Res<'w, Time<Virtual>>,
    real_time: Res<'w, Time<Real>>,
}

/// Gather the frame's facts, run the director, and hand its cues to the
/// deck. Without a deck (a rig with no sound plugin) it does nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "one system: the deck, the catalog, the director, the scenario song, the hits, the facts, the asset server and the seed source"
)]
pub fn direct_music(
    deck: Option<ResMut<MusicDeck>>,
    tracks: Res<GameMusicTracks>,
    mut director: ResMut<MusicDirector>,
    mut scenario_music: ResMut<ScenarioMusic>,
    mut hits: ResMut<MusicHits>,
    ships: MusicShipFacts,
    game: MusicGameFacts,
    asset_server: Res<AssetServer>,
    mut global: Option<Single<&mut WyRand, With<GlobalRng>>>,
) {
    let Some(mut deck) = deck else {
        return;
    };
    let Some(state) = game.game.as_deref() else {
        return;
    };
    // The director draws its gaps and shuffles at moments that follow the
    // wall clock. It forks its own stream from the seeded global one on its
    // first frame, so its draws never move the gameplay stream.
    let mut rng = match director.rng.take() {
        Some(rng) => rng,
        None => match global.as_deref_mut() {
            Some(global) => global.fork_rng(),
            None => {
                error!("nova music: no single seeded GlobalRng; the music director cannot run");
                return;
            }
        },
    };
    let (signal, docked_at_station) = ships.sample();
    let paused = game
        .pause
        .as_deref()
        .is_some_and(|pause| pause_ducks_music(pause.get()));
    let tick = MusicTick {
        facts: MusicFacts {
            game: state.get().clone(),
            mode: game.mode.as_deref().copied().unwrap_or_default(),
            scenario: game
                .scenario
                .as_deref()
                .and_then(|current| current.0.as_ref())
                .map(|config| config.role),
            scenario_song: None,
            mood: MusicMoodType::Calm,
            docked_at_station,
        },
        signal,
        hit: std::mem::take(&mut hits.0),
        game_dt: game.game_time.delta_secs(),
        real_dt: game.real_time.delta_secs(),
        deck: deck.status(),
        paused,
        tracks: &tracks,
    };
    let cues = director.tick(tick, &mut scenario_music, &mut rng);
    director.rng = Some(rng);
    for cue in cues {
        match cue {
            MusicCue::Play { song, fade } => {
                let handle = match song {
                    MusicSong::Scenario(handle) => handle,
                    MusicSong::Track(id) => match tracks.get(&id) {
                        Some(track) => track.sound.resolve(&asset_server),
                        None => continue,
                    },
                };
                deck.play(handle, fade);
            }
            MusicCue::Stop { fade } => deck.stop(fade),
            MusicCue::Duck { db, fade } => deck.duck(db, fade),
        }
    }
}

/// Note a combat hit: weapon fire between the player's ship and a ship
/// hostile to it, either way. Hostile is [`ship_relation`], the rule the AI's
/// under-fire memory and the threat signal read, so a civilization the
/// player turned Enemy or a Neutral answering the player counts, and a
/// Neutral, a friend or the player's own fleet does not. A ram or a scrape
/// against a hull or a station is not fire, and a rock blames nobody. Only
/// the first hop of the damage event counts.
#[expect(
    clippy::too_many_arguments,
    reason = "one observer: the attacker walk's four lookups, the victim's relation, the player, and the flag"
)]
pub fn hear_music_hits(
    damage: On<HealthApplyDamage>,
    q_owner: Query<&ProjectileOwner>,
    q_allegiance: Query<&Allegiance>,
    q_parent: Query<&ChildOf>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    q_retaliation: Query<&RetaliationTarget>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut hits: ResMut<MusicHits>,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Some(source) = damage.source else {
        return;
    };
    let mut victim = damage.entity;
    while !q_ship_root.contains(victim) {
        let Ok(&ChildOf(parent)) = q_parent.get(victim) else {
            return;
        };
        victim = parent;
    }
    let attacker =
        resolve_damage_attacker(source, &q_owner, &q_allegiance, &q_parent, &q_ship_root);
    let Some(shooter) = attacker.ship.filter(|shooter| *shooter != victim) else {
        return;
    };
    if !attacker.weapon || !(q_player.contains(victim) || q_player.contains(shooter)) {
        return;
    }
    let them = RelationParty {
        entity: shooter,
        allegiance: attacker.allegiance.as_ref(),
        retaliation: q_retaliation.get(shooter).ok(),
    };
    let us = RelationParty {
        entity: victim,
        allegiance: q_allegiance.get(victim).ok(),
        retaliation: q_retaliation.get(victim).ok(),
    };
    if ship_relation(them, us) == Relation::Hostile {
        hits.0 = true;
    }
}

/// A scenario's song ends with it.
fn end_scenario_music(_: On<UnloadScenario>, mut scenario_music: ResMut<ScenarioMusic>) {
    *scenario_music = ScenarioMusic::default();
}
