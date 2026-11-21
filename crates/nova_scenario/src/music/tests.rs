//! The context rule, the mood's hysteresis, the playlists and the director.

use std::time::Duration;

use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use nova_gameplay::prelude::*;
use nova_sound::prelude::MusicDeckStatusType;
use rand::SeedableRng as _;

use super::*;
use crate::loader::prelude::ScenarioRole;

/// A seeded stream, so a failing draw replays.
pub(crate) fn seeded() -> WyRand {
    WyRand::seed_from_u64(7)
}

/// One frame of calm flight in a chapter, with the deck playing.
pub(crate) fn flight_tick(tracks: &GameMusicTracks) -> MusicTick<'_> {
    MusicTick {
        facts: facts(
            GameStates::Playing,
            GameMode::NewGame,
            Some(ScenarioRole::Chapter),
        ),
        signal: false,
        hit: false,
        game_dt: 0.1,
        real_dt: 0.1,
        deck: MusicDeckStatusType::Playing,
        paused: false,
        tracks,
    }
}

fn facts(game: GameStates, mode: GameMode, scenario: Option<ScenarioRole>) -> MusicFacts {
    MusicFacts {
        game,
        mode,
        scenario,
        scenario_song: None,
        mood: MusicMoodType::Calm,
        docked_at_station: false,
    }
}

fn track(id: &str, contexts: &[MusicContextType]) -> MusicTrackDesign {
    MusicTrackDesign {
        id: MusicTrackDesignId::from(id),
        sound: AssetRef::from(format!("self://music/{id}.ogg")),
        contexts: contexts.to_vec(),
        title: id.to_string(),
        artist: "Artist".to_string(),
        licence: MusicLicenceType::Cc0,
        credit: format!("{id} by Artist, CC0 1.0, https://example.org/{id}"),
    }
}

fn catalog() -> GameMusicTracks {
    use MusicContextType::*;
    GameMusicTracks::new([
        track("theme", &[Menu]),
        track("cruise_a", &[Cruise, Docked]),
        track("cruise_b", &[Cruise]),
        track("cruise_c", &[Cruise, Docked]),
        track("threat_a", &[Threat]),
        track("threat_b", &[Threat]),
        track("combat_a", &[Combat]),
        track("combat_b", &[Combat]),
    ])
}

/// The mood after `seconds` of `signal` and no hit, from `mood`, in steps
/// of 0.1 s.
fn hold(mut mood: MusicMood, seconds: f32, signal: bool) -> MusicMood {
    for _ in 0..(seconds * 10.0).round() as usize {
        mood = mood.step(0.1, signal, false);
    }
    mood
}

// --- The context rule -------------------------------------------------------

#[test]
fn the_context_rule_table_picks_the_first_matching_situation() {
    use MusicContextType::*;
    let chapter = Some(ScenarioRole::Chapter);
    let lesson = Some(ScenarioRole::Lesson);
    let backdrop = Some(ScenarioRole::Backdrop);
    let with = |base: MusicFacts, edit: fn(&mut MusicFacts)| {
        let mut facts = base;
        edit(&mut facts);
        facts
    };
    let flight = facts(GameStates::Playing, GameMode::NewGame, chapter);
    let rows: Vec<(&str, MusicFacts, MusicContextType, MusicContextType)> = vec![
        (
            "boot load",
            facts(GameStates::Loading, GameMode::Sandbox, None),
            Menu,
            Silent,
        ),
        (
            "main menu",
            facts(GameStates::MainMenu, GameMode::Sandbox, backdrop),
            Silent,
            Menu,
        ),
        (
            "a backdrop's fight is still the menu",
            with(
                facts(GameStates::Playing, GameMode::NewGame, backdrop),
                |f| f.mood = MusicMoodType::Combat,
            ),
            Menu,
            Menu,
        ),
        (
            "editor and Mod Hub",
            facts(GameStates::Playing, GameMode::Sandbox, None),
            Cruise,
            Silent,
        ),
        (
            "the load after the menu ends the theme",
            facts(GameStates::Playing, GameMode::NewGame, None),
            Menu,
            Silent,
        ),
        (
            "the load between scenarios keeps the song",
            facts(GameStates::Playing, GameMode::NewGame, None),
            Cruise,
            Cruise,
        ),
        (
            "the load ends a scenario song",
            facts(GameStates::Playing, GameMode::NewGame, None),
            Scenario,
            Silent,
        ),
        (
            "a kept scenario song holds through a fight",
            with(flight.clone(), |f| {
                f.scenario_song = Some(MusicInCombatType::Keep);
                f.mood = MusicMoodType::Combat;
            }),
            Scenario,
            Scenario,
        ),
        (
            "a yielding scenario song holds through a threat",
            with(flight.clone(), |f| {
                f.scenario_song = Some(MusicInCombatType::Yield);
                f.mood = MusicMoodType::Threat;
            }),
            Scenario,
            Scenario,
        ),
        (
            "a yielding scenario song gives way to a fight",
            with(flight.clone(), |f| {
                f.scenario_song = Some(MusicInCombatType::Yield);
                f.mood = MusicMoodType::Combat;
            }),
            Scenario,
            Combat,
        ),
        (
            "combat beats docked",
            with(flight.clone(), |f| {
                f.mood = MusicMoodType::Combat;
                f.docked_at_station = true;
            }),
            Docked,
            Combat,
        ),
        (
            "threat",
            with(flight.clone(), |f| f.mood = MusicMoodType::Threat),
            Cruise,
            Threat,
        ),
        (
            "docked at a station",
            with(flight.clone(), |f| f.docked_at_station = true),
            Cruise,
            Docked,
        ),
        ("calm flight", flight, Silent, Cruise),
        (
            "a lesson uses the normal rule",
            facts(GameStates::Playing, GameMode::NewGame, lesson),
            Silent,
            Cruise,
        ),
        (
            "the editor's test flight is flight",
            facts(GameStates::Playing, GameMode::Sandbox, chapter),
            Silent,
            Cruise,
        ),
    ];
    for (row, facts, previous, expected) in rows {
        assert_eq!(music_context(&facts, previous), expected, "{row}");
    }
}

// --- The mood's hysteresis --------------------------------------------------

#[test]
fn a_blinking_threat_never_flaps_the_music() {
    let mut mood = hold(MusicMood::default(), 1.9, true);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Calm,
        "a lock under 2 s is no threat"
    );
    mood = hold(mood, 0.2, false);
    mood = hold(mood, 1.9, true);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Calm,
        "a break restarts the hold"
    );

    mood = hold(mood, 2.0, true);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Threat,
        "a lock held 2 s is a threat"
    );
    // The lock blinks: 3 s off, 1 s on, for a minute.
    for _ in 0..15 {
        mood = hold(mood, 3.0, false);
        assert_eq!(
            mood.mood(),
            MusicMoodType::Threat,
            "the threat holds while off"
        );
        mood = hold(mood, 1.0, true);
        assert_eq!(
            mood.mood(),
            MusicMoodType::Threat,
            "the threat holds while on"
        );
    }

    mood = hold(mood, 9.9, false);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Threat,
        "9.9 s after the last lock"
    );
    mood = hold(mood, 0.2, false);
    assert_eq!(mood.mood(), MusicMoodType::Calm, "10 s after the last lock");
}

#[test]
fn combat_starts_on_the_first_hit_and_ends_15_s_after_the_last() {
    let mut mood = MusicMood::default().step(0.1, false, true);
    assert_eq!(mood.mood(), MusicMoodType::Combat, "the first hit, at once");

    mood = hold(mood, 14.0, false);
    assert_eq!(mood.mood(), MusicMoodType::Combat, "a lull keeps the fight");
    mood = mood.step(0.1, false, true);
    mood = hold(mood, 14.8, false);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Combat,
        "the clock counts from the last hit"
    );
    mood = hold(mood, 0.3, false);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Calm,
        "15 s after the last hit, no lock"
    );

    let mut mood = MusicMood::default().step(0.1, true, true);
    mood = hold(mood, 10.0, true);
    mood = hold(mood, 5.2, false);
    assert_eq!(
        mood.mood(),
        MusicMoodType::Threat,
        "15 s after the last hit, a lock seen 5 s ago: back to the threat"
    );
}

// --- The playlists ----------------------------------------------------------

#[test]
fn a_playlist_never_plays_a_track_twice_in_a_row() {
    let ids: Vec<MusicTrackDesignId> = ["a", "b", "c"].map(MusicTrackDesignId::from).to_vec();
    let mut playlist = MusicPlaylist::default();
    let mut rng = seeded();
    let mut last = None;
    let mut counts = [0usize; 3];
    for _ in 0..300 {
        let next = playlist.next(&ids, &mut rng).expect("a track");
        assert_ne!(Some(&next), last.as_ref(), "no immediate repeat");
        counts[ids.iter().position(|id| *id == next).unwrap()] += 1;
        last = Some(next);
    }
    assert_eq!(
        counts,
        [100, 100, 100],
        "each shuffle plays every track once"
    );

    let one = [MusicTrackDesignId::from("theme")];
    let mut menu = MusicPlaylist::default();
    assert_eq!(menu.next(&one, &mut rng), Some(one[0].clone()));
    assert_eq!(
        menu.next(&one, &mut rng),
        Some(one[0].clone()),
        "a one-track list repeats"
    );
    assert_eq!(MusicPlaylist::default().next(&[], &mut rng), None);
}

#[test]
fn the_gaps_stay_in_their_bounds() {
    let mut rng = seeded();
    for (range, low, high) in [
        (MusicContextType::Cruise.gap_secs().unwrap(), 30.0, 120.0),
        (MusicContextType::Docked.gap_secs().unwrap(), 10.0, 30.0),
        (AFTER_FIGHT_GAP_SECS, 10.0, 20.0),
    ] {
        let draws: Vec<f32> = (0..1000)
            .map(|_| draw_gap(range.clone(), &mut rng))
            .collect();
        assert!(
            draws.iter().all(|gap| (low..=high).contains(gap)),
            "{range:?}"
        );
        let spread = draws.iter().copied().fold(f32::MIN, f32::max)
            - draws.iter().copied().fold(f32::MAX, f32::min);
        assert!(spread > (high - low) * 0.9, "the draws cover {range:?}");
    }
    for context in [
        MusicContextType::Menu,
        MusicContextType::Threat,
        MusicContextType::Combat,
    ] {
        assert_eq!(context.gap_secs(), None, "{context:?} plays without a gap");
    }
}

// --- The director -----------------------------------------------------------

struct Rig {
    tracks: GameMusicTracks,
    director: MusicDirector,
    scenario: ScenarioMusic,
    rng: WyRand,
}

impl Rig {
    fn new() -> Self {
        Self {
            tracks: catalog(),
            director: MusicDirector::default(),
            scenario: ScenarioMusic::default(),
            rng: seeded(),
        }
    }

    fn tick(&mut self, edit: impl FnOnce(&mut MusicTick<'_>)) -> Vec<MusicCue> {
        let mut tick = flight_tick(&self.tracks);
        edit(&mut tick);
        self.director.tick(tick, &mut self.scenario, &mut self.rng)
    }

    /// Run `seconds` of frames, collecting every cue.
    fn run(&mut self, seconds: f32, edit: impl Fn(&mut MusicTick<'_>)) -> Vec<MusicCue> {
        let mut cues = Vec::new();
        for _ in 0..(seconds * 10.0).round() as usize {
            cues.extend(self.tick(&edit));
        }
        cues
    }
}

fn played(cues: &[MusicCue]) -> Vec<(MusicSong, Duration)> {
    cues.iter()
        .filter_map(|cue| match cue {
            MusicCue::Play { song, fade } => Some((song.clone(), *fade)),
            _ => None,
        })
        .collect()
}

fn track_song(id: &str) -> MusicSong {
    MusicSong::Track(MusicTrackDesignId::from(id))
}

fn is_cruise(song: &MusicSong) -> bool {
    matches!(song, MusicSong::Track(id) if id.as_str().starts_with("cruise"))
}

#[test]
fn cruise_waits_its_entry_gap_and_songs_end_into_a_gap() {
    let mut rig = Rig::new();
    let first = rig.run(19.9, |_| {});
    assert!(
        played(&first).is_empty(),
        "no song in the first 20 s: {first:?}"
    );
    let first = played(&rig.run(20.2, |_| {}));
    assert_eq!(first.len(), 1, "one song by 40 s");
    assert!(is_cruise(&first[0].0));
    assert_eq!(
        first[0].1,
        Duration::from_secs(4),
        "cruise fades in over 4 s"
    );

    let ended = rig.tick(|tick| tick.deck = MusicDeckStatusType::Ended);
    assert!(played(&ended).is_empty(), "a cruise song ends into a gap");
    let gap = rig.director.wait().expect("a gap");
    assert!((30.0..=120.0).contains(&gap), "{gap}");
}

#[test]
fn a_hit_cuts_to_combat_at_once_and_a_short_lull_keeps_its_song() {
    let mut rig = Rig::new();
    rig.run(41.0, |_| {});
    let hit = played(&rig.tick(|tick| tick.hit = true));
    assert_eq!(rig.director.context(), MusicContextType::Combat);
    assert_eq!(hit.len(), 1);
    assert!(matches!(&hit[0].0, MusicSong::Track(id) if id.as_str().starts_with("combat")));
    assert_eq!(
        hit[0].1,
        Duration::from_millis(1500),
        "into combat over 1.5 s"
    );

    let lull = rig.run(14.0, |_| {});
    assert!(lull.is_empty(), "a lull plays on: {lull:?}");

    let combat_song = hit[0].0.clone();
    let after = rig.run(2.0, |_| {});
    assert_eq!(rig.director.context(), MusicContextType::Cruise);
    assert!(
        after.is_empty(),
        "the combat song plays on into the after-fight gap: {after:?}"
    );
    assert_eq!(rig.director.current(), Some(&combat_song));
    // The fight ended about 1 s into the run, so up to 1 s of the gap is
    // spent.
    let pending = rig
        .director
        .pending()
        .expect("a held switch after the fight");
    assert_eq!(pending.held, MusicContextType::Combat);
    assert!((9.0..=20.0).contains(&pending.due_secs), "{pending:?}");
}

fn docked(tick: &mut MusicTick<'_>) {
    tick.facts.docked_at_station = true;
}

/// A rig in cruise with its first cruise song playing.
fn cruising() -> (Rig, MusicSong) {
    let mut rig = Rig::new();
    let first = played(&rig.run(41.0, |_| {}));
    assert_eq!(first.len(), 1, "one cruise song by 41 s");
    (rig, first[0].0.clone())
}

#[test]
fn a_context_change_with_a_gap_keeps_the_playing_song_until_the_gap_ends_then_crossfades() {
    let (mut rig, cruise_song) = cruising();
    let dock = rig.tick(docked);
    assert_eq!(rig.director.context(), MusicContextType::Docked);
    assert!(dock.is_empty(), "no fade-out at the change: {dock:?}");
    assert_eq!(rig.director.current(), Some(&cruise_song));
    let pending = rig.director.pending().expect("a held switch");
    assert_eq!(pending.held, MusicContextType::Cruise);
    assert!((10.0..=30.0).contains(&pending.due_secs), "{pending:?}");

    let held = rig.run(pending.due_secs - 0.2, docked);
    assert!(held.is_empty(), "the cruise song plays on: {held:?}");
    assert_eq!(rig.director.current(), Some(&cruise_song));

    let switch = rig.run(0.4, docked);
    assert_eq!(switch.len(), 1, "{switch:?}");
    let MusicCue::Play { song, fade } = &switch[0] else {
        panic!("a crossfade, not {switch:?}");
    };
    assert!(
        rig.tracks
            .playlist(MusicContextType::Docked)
            .any(|track| *song == MusicSong::Track(track.id.clone())),
        "a docked song: {song:?}"
    );
    assert_eq!(*fade, Duration::from_secs(3), "the docked fade-in");
    assert_eq!(rig.director.pending(), None);
}

#[test]
fn a_song_that_ends_during_the_held_gap_leaves_silence_until_the_gap_ends() {
    let (mut rig, _) = cruising();
    rig.tick(docked);
    let due = rig.director.pending().expect("a held switch").due_secs;
    rig.run(5.0, docked);

    let ended = rig.tick(|tick| {
        docked(tick);
        tick.deck = MusicDeckStatusType::Ended;
    });
    assert!(ended.is_empty(), "nothing starts early: {ended:?}");
    assert_eq!(rig.director.current(), None);
    assert_eq!(rig.director.pending(), None);
    let wait = rig.director.wait().expect("the rest of the gap");
    assert!(
        (wait - (due - 5.1)).abs() < 0.05,
        "the gap keeps its end: {wait} of {due}"
    );

    let silence = rig.run(wait - 0.2, |tick| {
        docked(tick);
        tick.deck = MusicDeckStatusType::Ended;
    });
    assert!(silence.is_empty(), "{silence:?}");
    let next = played(&rig.run(0.4, |tick| {
        docked(tick);
        tick.deck = MusicDeckStatusType::Ended;
    }));
    assert!(
        matches!(&next[..], [(MusicSong::Track(_), fade)] if *fade == Duration::from_secs(3)),
        "the docked song after the gap: {next:?}"
    );
}

#[test]
fn combat_still_cuts_in_at_once() {
    let (mut rig, _) = cruising();
    rig.tick(docked);
    assert!(rig.director.pending().is_some());
    let hit = played(&rig.tick(|tick| {
        docked(tick);
        tick.hit = true;
    }));
    assert_eq!(rig.director.context(), MusicContextType::Combat);
    assert!(
        matches!(&hit[..], [(MusicSong::Track(id), fade)]
            if id.as_str().starts_with("combat") && *fade == Duration::from_millis(1500)),
        "into combat over 1.5 s during the held gap: {hit:?}"
    );
    assert_eq!(rig.director.pending(), None);
}

#[test]
fn changing_back_during_the_gap_keeps_the_song() {
    let (mut rig, cruise_song) = cruising();
    rig.run(5.0, docked);
    assert!(rig.director.pending().is_some());

    let undock = rig.tick(|_| {});
    assert!(undock.is_empty(), "{undock:?}");
    assert_eq!(rig.director.context(), MusicContextType::Cruise);
    assert_eq!(rig.director.pending(), None);
    assert_eq!(rig.director.wait(), None);
    assert_eq!(rig.director.current(), Some(&cruise_song));

    let later = rig.run(60.0, |_| {});
    assert!(later.is_empty(), "the cruise song plays on: {later:?}");
    let ended = rig.tick(|tick| tick.deck = MusicDeckStatusType::Ended);
    assert!(played(&ended).is_empty(), "it ends into the cruise gap");
    assert!(rig.director.wait().is_some());
}

#[test]
fn a_fight_that_returns_before_its_song_fades_picks_it_up() {
    let mut rig = Rig::new();
    let hit = played(&rig.tick(|tick| {
        tick.hit = true;
        tick.signal = true;
    }));
    let combat_song = hit[0].0.clone();
    // 15 s on, with the lock held: combat steps down to threat.
    let down = played(&rig.run(15.1, |tick| tick.signal = true));
    assert_eq!(rig.director.context(), MusicContextType::Threat);
    assert_eq!(down.len(), 1);
    let back = played(&rig.tick(|tick| tick.hit = true));
    assert_eq!(back, vec![(combat_song, Duration::from_millis(1500))]);
}

#[test]
fn the_pause_menu_ducks_the_music_and_tab_does_not() {
    assert!(pause_ducks_music(&PauseStates::Paused));
    assert!(pause_ducks_music(&PauseStates::Commands));
    assert!(
        !pause_ducks_music(&PauseStates::Interface),
        "Tab does not duck"
    );
    assert!(!pause_ducks_music(&PauseStates::Unpaused));

    let mut rig = Rig::new();
    let paused = rig.tick(|tick| tick.paused = true);
    assert!(paused.contains(&MusicCue::Duck {
        db: PAUSE_DUCK_DB,
        fade: Duration::from_millis(500)
    }));
    assert!(rig.tick(|tick| tick.paused = true).is_empty(), "one duck");
    let lifted = rig.tick(|_| {});
    assert!(lifted.contains(&MusicCue::Duck {
        db: 0.0,
        fade: Duration::from_secs(1)
    }));
}

#[test]
fn the_menu_theme_starts_at_once_and_ends_with_the_load() {
    let mut rig = Rig::new();
    let menu = played(&rig.tick(|tick| tick.facts.game = GameStates::MainMenu));
    assert_eq!(menu, vec![(track_song("theme"), Duration::from_secs(3))]);
    let again = played(&rig.tick(|tick| {
        tick.facts.game = GameStates::MainMenu;
        tick.deck = MusicDeckStatusType::Ended;
    }));
    assert_eq!(
        again,
        vec![(track_song("theme"), Duration::ZERO)],
        "the theme loops"
    );

    let load = rig.tick(|tick| tick.facts.scenario = None);
    assert_eq!(rig.director.context(), MusicContextType::Silent);
    assert_eq!(
        load,
        vec![MusicCue::Stop {
            fade: Duration::from_secs(2)
        }]
    );
}

fn scenario_song(then: MusicThenType, in_combat: MusicInCombatType) -> ScenarioSong {
    ScenarioSong {
        sound: Handle::default(),
        fade_in: Duration::from_secs(2),
        then,
        in_combat,
    }
}

#[test]
fn a_looping_scenario_song_replays_and_a_resuming_one_hands_back() {
    let mut rig = Rig::new();
    rig.scenario
        .play(scenario_song(MusicThenType::Loop, MusicInCombatType::Keep));
    let start = played(&rig.tick(|_| {}));
    assert_eq!(
        start,
        vec![(
            MusicSong::Scenario(Handle::default()),
            Duration::from_secs(2)
        )]
    );
    let looped = played(&rig.tick(|tick| tick.deck = MusicDeckStatusType::Ended));
    assert_eq!(
        looped,
        vec![(MusicSong::Scenario(Handle::default()), Duration::ZERO)]
    );
    assert_eq!(rig.director.context(), MusicContextType::Scenario);

    rig.scenario.play(scenario_song(
        MusicThenType::Resume,
        MusicInCombatType::Keep,
    ));
    rig.tick(|_| {});
    rig.tick(|tick| tick.deck = MusicDeckStatusType::Ended);
    assert!(
        rig.scenario.song().is_none(),
        "Resume ends the scenario song"
    );
    rig.tick(|_| {});
    assert_eq!(rig.director.context(), MusicContextType::Cruise);
    let gap = rig.director.wait().expect("the normal gap");
    assert!((30.0..=120.0).contains(&gap), "{gap}");
}

#[test]
fn a_kept_scenario_song_holds_a_fight_and_a_yielding_one_gives_way() {
    let mut rig = Rig::new();
    rig.scenario
        .play(scenario_song(MusicThenType::Loop, MusicInCombatType::Keep));
    rig.tick(|_| {});
    let fight = rig.tick(|tick| tick.hit = true);
    assert!(played(&fight).is_empty(), "Keep plays through: {fight:?}");
    assert_eq!(rig.director.context(), MusicContextType::Scenario);

    let mut rig = Rig::new();
    rig.scenario
        .play(scenario_song(MusicThenType::Loop, MusicInCombatType::Yield));
    rig.tick(|_| {});
    let fight = played(&rig.tick(|tick| tick.hit = true));
    assert_eq!(rig.director.context(), MusicContextType::Combat);
    assert!(
        matches!(&fight[..], [(MusicSong::Track(_), fade)] if *fade == Duration::from_millis(1500))
    );
    assert!(
        rig.scenario.song().is_none(),
        "the yielded song does not come back"
    );
}

#[test]
fn stop_music_ends_the_scenario_song_with_its_fade() {
    let mut rig = Rig::new();
    rig.scenario
        .play(scenario_song(MusicThenType::Loop, MusicInCombatType::Keep));
    rig.tick(|_| {});
    rig.scenario.stop(Duration::from_secs(6));
    let stop = rig.tick(|_| {});
    assert_eq!(
        stop,
        vec![MusicCue::Stop {
            fade: Duration::from_secs(6)
        }]
    );
    assert_eq!(rig.director.context(), MusicContextType::Cruise);
}

// --- The catalog lint -------------------------------------------------------

#[test]
fn a_track_that_is_not_ogg_names_the_ffmpeg_fix() {
    let mut wav = track("song", &[MusicContextType::Cruise]);
    wav.sound = AssetRef::from("self://music/song.wav".to_string());
    let faults = music_track_faults(&wav);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(
        faults[0].contains("ffmpeg -i song.wav -c:a libvorbis -q:a 4 song.ogg"),
        "{faults:?}"
    );
    assert!(music_track_faults(&track("song", &[MusicContextType::Cruise])).is_empty());
}

#[test]
fn a_track_with_no_licence_record_is_refused() {
    let mut bare = track("song", &[MusicContextType::Cruise]);
    bare.credit = String::new();
    let faults = music_track_faults(&bare);
    assert!(
        faults
            .iter()
            .any(|fault| fault.starts_with("music_licence")),
        "{faults:?}"
    );

    let mut unsourced = track("song", &[MusicContextType::Cruise]);
    unsourced.licence = MusicLicenceType::CcBy4;
    unsourced.credit = "Song by Artist".to_string();
    assert!(
        music_track_faults(&unsourced)
            .iter()
            .any(|fault| fault.contains("must link the source")),
        "a CC BY credit links its source"
    );
}

#[test]
fn a_track_names_only_situations_with_a_playlist() {
    let faults = music_track_faults(&track(
        "song",
        &[
            MusicContextType::Silent,
            MusicContextType::Cruise,
            MusicContextType::Cruise,
        ],
    ));
    assert_eq!(faults.len(), 2, "{faults:?}");
    assert!(
        !music_track_faults(&track("song", &[])).is_empty(),
        "no situation"
    );
}

// --- The hits that start a fight --------------------------------------------

/// A rig with only [`hear_music_hits`], and the player's ship with one
/// section.
struct HitRig {
    app: App,
    player: Entity,
    player_section: Entity,
}

impl HitRig {
    fn new() -> Self {
        let mut app = App::new();
        app.init_resource::<MusicHits>()
            .add_observer(hear_music_hits);
        let player = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, RetaliationTarget(None)))
            .id();
        let player_section = app.world_mut().spawn(ChildOf(player)).id();
        Self {
            app,
            player,
            player_section,
        }
    }

    /// Another ship root with one section: the root and the section.
    fn ship(&mut self, allegiance: Allegiance) -> (Entity, Entity) {
        let root = self
            .app
            .world_mut()
            .spawn((SpaceshipRootMarker, allegiance, RetaliationTarget(None)))
            .id();
        let section = self.app.world_mut().spawn(ChildOf(root)).id();
        (root, section)
    }

    /// A station root with one section: the root and the section.
    fn station(&mut self) -> (Entity, Entity) {
        let (root, section) = self.ship(Allegiance::Neutral);
        self.app.world_mut().entity_mut(root).insert(StationMarker);
        (root, section)
    }

    /// A round `owner` fired, with the side it launched under.
    fn round(&mut self, owner: Entity) -> Entity {
        let allegiance = *self
            .app
            .world()
            .get::<Allegiance>(owner)
            .expect("the shooter has a side");
        self.app
            .world_mut()
            .spawn((ProjectileOwner(owner), allegiance))
            .id()
    }

    /// Whether a hit on `target` from `source` starts the combat song.
    fn hit(&mut self, target: Entity, source: Entity) -> bool {
        self.app.world_mut().resource_mut::<MusicHits>().0 = false;
        self.app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: Some(source),
            amount: 5.0,
        });
        self.app.world().resource::<MusicHits>().0
    }
}

#[test]
fn a_collision_with_a_station_does_not_start_the_combat_song() {
    let mut rig = HitRig::new();
    let (_, dock) = rig.station();
    let player_section = rig.player_section;
    assert!(
        !rig.hit(player_section, dock),
        "the station scrapes the player"
    );
    assert!(
        !rig.hit(dock, player_section),
        "the player scrapes the station"
    );
}

#[test]
fn a_collision_with_another_ship_does_not_start_the_combat_song() {
    let mut rig = HitRig::new();
    let player_section = rig.player_section;
    for allegiance in [Allegiance::Enemy, Allegiance::Neutral, Allegiance::Player] {
        let (_, hull) = rig.ship(allegiance);
        assert!(
            !rig.hit(player_section, hull),
            "a {allegiance:?} hull rams the player"
        );
        assert!(
            !rig.hit(hull, player_section),
            "the player rams a {allegiance:?} hull"
        );
    }
}

#[test]
fn a_hit_between_the_player_and_a_neutral_ship_does_not_start_the_combat_song() {
    let mut rig = HitRig::new();
    let (player, player_section) = (rig.player, rig.player_section);
    let (hauler, hauler_section) = rig.ship(Allegiance::Neutral);
    let stray = rig.round(hauler);
    assert!(!rig.hit(player_section, stray), "a neutral's stray round");
    let shot = rig.round(player);
    assert!(
        !rig.hit(hauler_section, shot),
        "the player's round on a neutral"
    );
}

#[test]
fn a_hit_between_the_player_and_a_friendly_ship_does_not_start_the_combat_song() {
    let mut rig = HitRig::new();
    let (player, player_section) = (rig.player, rig.player_section);
    let (escort, escort_section) = rig.ship(Allegiance::Player);
    let stray = rig.round(escort);
    assert!(!rig.hit(player_section, stray), "the escort's stray round");
    let shot = rig.round(player);
    assert!(
        !rig.hit(escort_section, shot),
        "the player's round on the escort"
    );
}

#[test]
fn a_hostiles_round_on_the_player_starts_the_combat_song() {
    let mut rig = HitRig::new();
    let player_section = rig.player_section;
    let (raider, _) = rig.ship(Allegiance::Enemy);
    let round = rig.round(raider);
    assert!(rig.hit(player_section, round));
}

#[test]
fn the_players_round_on_a_hostile_starts_the_combat_song() {
    let mut rig = HitRig::new();
    let player = rig.player;
    let (_, raider_section) = rig.ship(Allegiance::Enemy);
    let round = rig.round(player);
    assert!(rig.hit(raider_section, round));
}

#[test]
fn a_neutral_that_answers_the_player_fights_with_the_combat_song() {
    let mut rig = HitRig::new();
    let (player, player_section) = (rig.player, rig.player_section);
    let (hauler, hauler_section) = rig.ship(Allegiance::Neutral);
    rig.app
        .world_mut()
        .entity_mut(hauler)
        .insert(RetaliationTarget(Some(player)));
    let answer = rig.round(hauler);
    assert!(rig.hit(player_section, answer), "its round on the player");
    let shot = rig.round(player);
    assert!(rig.hit(hauler_section, shot), "the player's round on it");
}
