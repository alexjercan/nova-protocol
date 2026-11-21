# Music: code map and agent proposal

Agent findings, 2026-11-21, master at `0736b9bbe`. Map and proposal only. No
code changed. Licences and track choices are in `RESEARCH-music.md` (other
agent). Line numbers are for this commit.

## 1. nova_sound today

### Backend
- bevy_audio 0.20.1 over rodio 0.22.2 (`Cargo.lock:806`, `:6591`). No kira.
  `crates/nova_sound/Cargo.toml` adds `bevy/wav`; vorbis is a bevy default
  (lewton in the rodio deps, `Cargo.lock:6599`). OGG plays today with no
  feature change.
- One playback path. Only `voice.rs` builds an `AudioPlayer`
  (`crates/nova_sound/src/voice.rs:230-257`, `voice_player`). A test reads the
  whole workspace and fails on any other constructor (`voice.rs:1460`).
- Requests: `PlaySound` (one-shot) and `SoundLoop` (loop)
  (`play.rs:62`, `play.rs:97`), both turned into `SoundVoice` entities.

### Buses and gain
- `SoundBus { Effect, Ui }` (`category.rs:17-26`). Two sliders:
  `InterfaceVolume`, `WorldVolume` (`bus.rs:34-81`). `MasterVolume`
  (`master.rs:18-54`) goes to bevy `GlobalVolume` at sink creation
  (`master.rs:117-125`) and is folded in by hand on every per-frame write
  (`bus.rs:121-124`).
- Gain chain (`lib.rs:23-26`): clip loudness gain (to -23 LUFS) x category
  level x distance fade and pan x bus slider x master.
- Slider taper: `slider_gain`, 0 = silent, else -40 dB..0 dB linear in dB
  (`bus.rs:19-32`).
- `Mixer` system param (`bus.rs:89-125`): every resource is `Option`, so a rig
  without the plugin plays at full volume. `bus_gain` matches on `SoundBus`
  (`bus.rs:102-112`).
- `HarnessMute` masks the master only (`master.rs:47-53`, `:77`). Harness runs
  are muted unless `NOVA_MUTE=0` (`master.rs:63`, `:92-97`).

### Categories
- `SoundCategoryType`, 30 closed variants (`category.rs:31-97`), one balance row
  each (`category.rs:181-216`): bus, level dB, reach, max voices, repeat
  interval, cap priority, loop dB range. "This table is the ONLY balance knob"
  (`category.rs:5`). Content names a variant in a scenario `PlaySound`
  (`crates/nova_scenario/src/actions/audio.rs:39-48`); `fault()` refuses a
  bus/place mismatch and loop categories (`audio.rs:62-74`).

### Spatial vs flat
- `Effect` = spatial, faded, panned, frozen with the sim, retired when gameplay
  ends. `Ui` = flat, plays through the freeze (`voice.rs:269-279`
  `VoiceView::of`: `spatial = world = bus == Effect`).
- Freeze hook: `pause_world_voices` / `resume_world_voices` on every frozen
  `PauseStates` variant, `stop_world_voices` on `OnExit(Playing)`
  (`crates/nova_gameplay/src/plugin.rs:159-173`, `voice.rs:641-718`). Only
  Effect voices are touched.

### Looping, fades, end of clip
- Loop = `PlaybackMode::Loop`, one-shot = `PlaybackMode::Despawn`
  (`voice.rs:242-246`). No `Once` / `Remove` use.
- The only fade is the loop release: 0.12 s ramp to silence
  (`voice.rs:64`, `:83-107`). No crossfade, no fade-in.
- One-shot quirks that do NOT fit music:
  - an inaudible one-shot at start is despawned (`voice.rs:395-398`), so a
    track started with the slider at 0 would vanish;
  - a one-shot with no sink after 2 s is retired (`voice.rs:328`, `:418-431`),
    so with no audio device a track would "end" every 2 s;
  - the repeat interval and voice cap apply per root (`play.rs:171`).
- bevy `PlaybackMode::Once` keeps the sink; `AudioSinkPlayback::empty()` tells
  the end (`bevy_audio-0.20.1/src/audio.rs:11-26`,
  `audio_output.rs:284-323`).

### Streaming vs decoded, memory, decode cost
- Playback streams. `AudioSource` holds the compressed file bytes
  (`bevy_audio-0.20.1/src/audio_source.rs:8-21`); rodio decodes on the audio
  thread from a `Cursor` over those bytes (`audio_source.rs:93-101`). Resident
  memory per track = file size (about 3.5-5 MB for a 4 min OGG at q3-q5).
- Loudness measure does NOT stream. Every loaded clip is fully decoded on the
  main thread in `Update` (`loudness.rs:82-88` `measure_clip` collects all
  samples; `loudness.rs:310-356` `measure_loaded_sounds`). `clip_loudness`
  then allocates a per-frame f64 energy Vec and a prefix-sum Vec
  (`loudness.rs:108-126`).
  - 4 min, 44.1 kHz stereo: 21.2 M f32 samples (85 MB) + 10.6 M f64 energy
    (85 MB) + prefix (85 MB) + a copy of the file bytes. About 255 MB
    transient per track. 5 min: about 320 MB. (Arithmetic; RSS not measured.)
  - CPU: one full Vorbis decode plus K-weighting per track on the main
    thread. Unverified estimate 1-3 s stall per 4 min track. On wasm the
    stall is the same or worse (one thread).
  - A voice waits until its clip is measured (`voice.rs:386-394`), so a
    track cannot start before the measure ends.
  - This must change before music lands (see 5.4).
- `bevy` `Decodable::decoder` unwraps (`audio_source.rs:99`): a bad file panics
  at playback. The measure guards this today: an unmeasured clip never plays.
  Music must keep that guard.

### Publishing for tools
- `VoiceMix` per voice, every frame, pre-master, with or without a sink
  (`voice.rs:132-160`, `:546-561`). `nova_debug::sfx_sidecar` renders WebM
  loop audio offline from it (`crates/nova_debug/src/sfx_sidecar.rs:1-21`).
  A music voice that publishes `VoiceMix` would appear in devlog loops.

## 2. The removed Music bus (baac8df5a, in v0.18.0)

`git tag --contains baac8df5a` = v0.18.0 only. v0.17.0 shipped the reserved
Music slider and `music_volume`; v0.18.0 shipped without it.

Removed:
- `MusicVolume(pub f32)` resource, `factor()`, serde + Reflect, default 1.0
  (`bus.rs`, old). `AudioBus::Music`, `AudioRoute::Music` (legacy routing,
  since deleted entirely). `Mixer.music`. Prelude export (`lib.rs`).
- Menu: `VolumeChannel::Music`, label "Music", `AudioLevels.music`,
  `SettingsValues.music_volume`, the slider arm in `on_volume_slider_change`
  (now `crates/nova_menu/src/settings.rs:109-150`, `:891`, `:2011-2032`).
- Store: `PersistedSettings.music_volume` with
  `#[serde(default = "default_volume")]`, its load, live, change-detect and
  snapshot arms (now `crates/nova_menu/src/settings_store.rs:35-46`,
  `:469-471`, `:532-538`, `:566`, `:686`, `:734`).
- Console: `"music"` in `CHANNELS` and read/write arms
  (`crates/nova_console/src/settings.rs:78`, `:240-256`); catalog usage
  (`crates/nova_command/src/commands.rs:398-408`).
- Test: `the_mixer_bus_volumes_persist_and_an_older_store_still_loads` now
  pins that `music_volume` is IGNORED (`settings_store.rs:1095-1111`).
- Changelog v0.18.0 line (`CHANGELOG.md:177`; the task cites `:79`, stale).

Shape it can come back in: the old resource was a pure slider; no voice read
it. The new one must be a real `SoundBus::Music` arm in `Mixer::bus_gain`, so
the compiler finds every match.

### settings.ron format and migration
- File: `<config_dir>/nova-protocol/settings.ron` (native), localStorage key
  `nova_protocol.settings` (web) (`settings_store.rs:296-298`).
- Policy for this player-written file: every field has a serde default, and
  unknown fields are ignored (`settings_store.rs:7-10`, `:28-30`, `:210-212`).
  This is the shipped rule for settings; the explicit-field rule is for
  content. The only migration code is the keybind rename table
  (`settings_store.rs:512-526`).
- Re-adding `music_volume` under the SAME name, with the same serde default:
  - v0.18.0 file (no field) loads the default. No code.
  - v0.17.0 file still carrying `music_volume` loads that value. This is the
    player's own choice on the old reserved slider, so it is correct.
  - v0.17.0 file re-saved by v0.18.0 lost the field; it loads the default.
  - No migration code is needed. The test above flips from "ignored" to
    "loads"; that is the proof of the v0.17 round trip.

## 3. Game contexts the rule can read

| Signal | Where | Notes |
| --- | --- | --- |
| `GameStates { Loading, MainMenu, Playing }` | `crates/nova_gameplay/src/lib.rs:158-167` | top level |
| `GameAssetsStates { Boot, Loading, Processing, Loaded, Failed }` | `crates/nova_assets/src/plugin.rs:52-75` | boot load |
| `PauseStates { Unpaused, Paused, Interface, Commands, Debug }` | `nova_gameplay/src/lib.rs:184-203` | all four frozen ones stop the clocks; `Interface` = TAB ship computer (station trade, jobs) |
| `SoundFreeze(bool)` | `nova_sound/src/voice.rs:641-642` | the engine's own view of the freeze |
| `GameMode { Sandbox, NewGame }` | `nova_gameplay/src/lib.rs:285-291` | Sandbox = editor flow |
| `CurrentScenario(Option<ScenarioConfig>)` | `crates/nova_scenario/src/loader/mod.rs:739-741` | `scenario_is_live` at `:755` |
| `ScenarioConfig.role: ScenarioRole { Chapter, Backdrop, Lesson }` | `loader/mod.rs:214-240`, `:395` | backdrop = menu scenery; lesson = practice range |
| `ScenarioConfig.world: ScenarioWorldType { Void, Generated }` | `loader/mod.rs:258-266` | open world vs authored |
| `LoadScenario` / `UnloadScenario` / `ScenarioLoaded` | `loader/mod.rs:698-709` | triggers |
| Editor state `ExampleStates { Loading, Hub, Editor, Scenario }` | `crates/nova_editor/src/lib.rs:224-232` | `pub(crate)`; not readable outside |
| Menu backdrop rotation | `crates/nova_menu/src/ambience.rs:21`, `:164-239` | loads a `role: Backdrop` scenario, so `CurrentScenario` is `Some` on the menu and backdrops fight |
| Hostile lock on the player | `nova_ship/src/input/targeting/state.rs:106` `CombatLock`; predicate in `nova_ship/src/ship_audio/cues.rs:336-379` | same test as the lock alarm: a `Hostile` (`ship_relation`) ship whose `CombatLock == player` |
| AI engaging | `AITarget(Option<Entity>)` `nova_ship/src/input/ai/acquisition.rs:25`; `AIBehaviorState` `ai/behavior.rs:29-65`, `engages()` is `pub(crate)` (`:56`) | needs a pub accessor |
| AI under-fire memory | `AIThreat` `ai/threat.rs:65-75`, 3 s window `:24` | AI side only |
| Player damage | `HealthApplyDamage { entity, source, amount }` `nova_gameplay/src/integrity/health.rs:98-106`; attribution via `ProjectileOwner` (`ai/threat.rs:389-410`) | for "player under fire" |
| Player weapons | `WeaponsHot(bool)` `targeting/state.rs:191`, player `CombatLock` | player intent, weak signal |
| Player ship | `PlayerSpaceshipMarker` `nova_gameplay/src/markers.rs:68` | |
| Docked | `DockedShip` on the root `nova_ship/src/sections/docking_section/connection.rs:200`; `StationDock(station)` on the connection `:224` | docked at a station = player root `DockedShip.connection` has `StationDock` |

Derived facts that avoid new hooks:
- Editor and Mod Hub = `Playing` + `GameMode::Sandbox` + `CurrentScenario`
  is `None`. The Editor state never keeps a scenario live (pinned by
  `nova_editor/src/lib.rs:1507`). Sandbox test flight = `Playing` + Sandbox +
  scenario live.
- `Playing` + `NewGame` + `CurrentScenario None` = a load gap. Hold the
  current music.
- No "in combat" signal exists today. The lock alarm latch is a `Local`
  (`cues.rs:347`), not shared.

## 4. Assets, packaging, licences

- Layout: engine chrome sounds at `assets/sounds/` (636 KB, not moddable,
  `assets/sounds/README.md`); base-mod world sounds at `assets/base/sounds/`
  (declared in `assets/base/base.bundle.ron` resources, reachable as
  `dep://base/sounds/...`). No music folder. `assets/` = 45 MB.
- UI sounds load through a `GameAssets` `paths(...)` list that gates loading
  (`crates/nova_assets/src/collections.rs:227-240`) plus
  `SoundBank::load(UI_SFX_FILES)` (`collections.rs:449-451`). Folder
  collections are avoided because they fail on wasm (`collections.rs:233`).
- Packaging: no embedding. Release workflow copies `assets/` and `credits/`
  next to the binary for macOS, Linux and Windows
  (`.github/workflows/release.yaml:133-135`, `:194`, `:235-240`). Web: Trunk
  `copy-dir` of `assets` and `credits` (`index.html:8`;
  `assets/base/sounds/README.md:169-174`). itch: `scripts/prepare-itch-upload.sh`
  repacks the GitHub release artifacts; `.github/workflows/deploy-itch.yaml`.
  So every track ships in every native zip AND the web download.
- Repo size: no `.gitattributes`, no git LFS. Pack is 1.65 GiB. Five 4 min
  OGG tracks add about 17-25 MB to every release artifact and to git history
  for good; every re-encode adds again.
- Licence records today:
  - `credits/CREDITS.md`: single source of truth, ships in every build,
    `credits/licenses/*` for licence texts (OFL, CC0, MIT, Unlicense).
  - `assets/base/sounds/README.md:157-167`: per-file table for licensed
    recordings (file, source URL, author, licence, original + edits).
  - Dependencies: `cargo about` at release (`scripts/gen-licenses.sh`).
  - No in-game credits screen (the "Credits" in `nova_menu` is currency). No
    wiki credits page. The repo itself is MIT (`LICENSE`, `Cargo.toml:6`) and
    the raw asset files are in a public-facing git tree, so a track licence
    must allow redistribution of the file itself, not only inside a binary.
- Stale record (out of this lane, small): `credits/CREDITS.md:17-27` says the
  sounds use "no third-party recordings", but `assets/base/sounds/README.md:157-167`
  lists four CC0 recordings (Kenney, ReadeOnly, gracenew, JPolito). Fix in the
  music delivery, since it edits the same section.

## 5. Agent proposal

### 5.1 End state, what dies, what may break
- End state: music plays on a Music bus through the one engine path, picked
  by a pure context rule; a Music slider in Settings, `volume music` in the
  console, `music_volume` in settings.ron; each track has a licence row.
- Dies: the test assertion that `music_volume` is ignored. Nothing else.
- May break: every exhaustive `match SoundBus` (compiler finds them:
  `bus.rs:102`, `category.rs:372-384` test, `actions/audio.rs:68-72`,
  `lint/scenario.rs:1162`); the scenario `PlaySound` action must refuse the
  Music category; WebM loop captures will carry music unless excluded.
- Must fail loudly: an undecodable or silent track (lint error, load error as
  today); a context with no track when the catalog says it must have one
  (lint); an unknown track id in content (lint, then load).

### 5.2 Engine (nova_sound)

```rust
// category.rs
pub enum SoundBus { Effect, Ui, /// Flat, plays through the freeze, scaled by MusicVolume.
    Music }
// one new row, so the table stays the only balance knob:
Self::Music => row(Music, -12.0, 0.0, 0.0, 2, 0.0, 0, None),

// bus.rs - same shape as InterfaceVolume
pub struct MusicVolume(pub f32);          // Resource, Reflect, serde, default 1.0
// Mixer gains `music: Option<Res<MusicVolume>>`, bus_gain gets a Music arm.

// music.rs (new module) - the deck. Knows tracks and fades, not game states.
#[derive(Resource, Default)]
pub struct MusicDeck { /* requested, current, outgoing, duck */ }
impl MusicDeck {
    pub fn play(&mut self, track: Handle<AudioSource>, fade: MusicFade);
    pub fn stop(&mut self, fade_out: Duration);
    pub fn duck(&mut self, db: f32);       // 0.0 = none
    pub fn state(&self) -> MusicDeckState;  // Idle | Starting | Playing | Ended | NoOutput
}
pub struct MusicFade { pub out: Duration, pub gap: Duration, pub r#in: Duration }
/// Pure: the gain of a fade at `t` seconds. Unit-tested.
pub fn fade_gain(from: f32, to: f32, t: f32, length: f32) -> f32;

// voice.rs
#[derive(Component)]
pub struct MusicVoice { sound: Handle<AudioSource>, fade: f32, target: f32, rate: f32 }
fn drive_music_voices(...)   // in SoundSystems, after drive_sfx_voices
```

- `voice_player` takes a `PlaybackMode` instead of `looping: bool`, so the
  music voice uses `PlaybackMode::Once` and the one constructor stays.
- Music voices are NOT `SoundVoice`: none of the one-shot rules (despawn when
  inaudible, 2 s sink grace, repeat interval) apply. They never despawn at
  slider 0; they ramp on `Time<Real>`; at most two live (outgoing, incoming).
- End of track: `sink.empty()` -> `MusicDeckState::Ended`. No sink after a
  grace -> `NoOutput`, logged once; the deck holds and does not advance.
- They publish `VoiceMix` like every voice (tools and tests read it).
- Freeze: not touched by `pause_world_voices` (not Effect). Ducking is the
  director's call, through `MusicDeck::duck`.

### 5.3 Context rule (nova_scenario::music, new module)

nova_scenario already sees `GameStates`, `PauseStates`, `GameMode`
(nova_gameplay), `CurrentScenario` and roles (own), `CombatLock`, `AITarget`,
`DockedShip`, `StationDock` (nova_ship), and nova_sound. No new crate edges.
nova_editor is not needed (editor is derived, see 3).

```rust
/// What the music is for. Closed.
pub enum MusicContextType { Silent, Menu, Flight, Docked, Combat, Editor }

/// One frame of facts, read by a thin system; the rule is pure.
pub struct MusicSample {
    pub game: GameStates,
    pub mode: GameMode,
    pub scenario: Option<ScenarioRole>,   // None = no scenario live
    pub docked_at_station: bool,
    pub threat: bool,                     // see below
}

/// Combat with hysteresis. Pure step, unit-tested.
pub struct CombatMood { engaged: bool, held_s: f32 }
impl CombatMood {
    pub fn step(self, threat: bool, hit: bool, dt: f32) -> Self;
    pub fn engaged(self) -> bool;
}

/// The rule. `previous` is returned for a load gap.
pub fn music_context(s: &MusicSample, combat: CombatMood,
                     previous: MusicContextType) -> MusicContextType;
```

Rule, first match wins:
1. `Loading` -> `Silent`.
2. `MainMenu` -> `Menu` (backdrop fights are ignored).
3. `Playing`, Sandbox, no scenario -> `Editor`.
4. `Playing`, NewGame, no scenario -> `previous` (load gap).
5. combat engaged -> `Combat`.
6. docked at a station -> `Docked`.
7. else -> `Flight` (chapters, open world, lessons, test flight).

Threat = a Hostile ship has `CombatLock == player` (same predicate as the
lock alarm, `cues.rs:356-366`; extract it to one pub fn and use it in both),
OR a Hostile `AITarget == player` with `engages()` (make a pub accessor).
Hit = `HealthApplyDamage` on a player section from a Hostile source this
frame.

Hysteresis (`CombatMood::step`): enter at once on a hit, or after threat holds
2 s; leave after 15 s with no threat and no hit. A threat that blinks every
second never flaps the music. Numbers are proposals.

Director system (`Update`, after the ship's targeting sets, before
`SoundSystems`): build the sample, step the mood, run the rule; on a context
change pick a track and call `MusicDeck::play`; on `Ended` wait the gap and
play the next; duck on frozen `PauseStates`.

### 5.4 Loudness measure for long clips (required)
- Make `clip_loudness` streaming: run the K-weighting per sample and keep only
  100 ms sub-block energies (2400 f64 for 4 min). Buffer samples only while
  the clip is shorter than `LOOP_SECONDS` (`loudness.rs:36`), where the loop
  rule needs repeats. Memory drops from about 255 MB to a few KB per track.
  Every clip benefits; the result must stay identical for short clips (the
  existing loudness tests pin it).
- Run the measure of a clip off the main thread (`AsyncComputeTaskPool`) on
  native. On wasm it stays on the main thread; load music after the menu is
  up so the stall does not land on the loading screen. Unverified cost; see 6.

### 5.5 Track registry

| Option | Shape | Mods | Cost |
| --- | --- | --- | --- |
| R1 Rust table | `MusicTrackType` + `MUSIC_TRACK_FILES: [(MusicTrackType, &str, &[MusicContextType])]` beside `UI_SFX_FILES` (`lib.rs:215`), files in root `assets/music/` (engine chrome) | No | Small |
| R2 Content catalog | `music/base.content.ron` with `MusicTrackDesignId`, `sound: AssetRef<AudioSource>`, `contexts: Vec<MusicContextType>`; built by a Rust builder in `crates/nova_authoring/src/base_content/`, generated by `content -- gen`, never hand-edited; files in `assets/base/music/`, listed in the bundle `resources` | Yes: a mod adds tracks with `self://`, base tracks via `dep://base/music/...` | Medium: registry, merge, lint (ids, contexts, loudness through the existing sound measure) |

Recommendation: R2. The base game is a mod; world sounds and UI themes are
content; the lint already measures sound files. A playlist per context is the
union of enabled mods' tracks. Lint fails a context in the catalog with no
track only if the owner wants every context filled (else: `Silent` there).

Playlist: shuffle per context, never the same track twice in a row (reuse the
backdrop rotation pattern, `ambience.rs:88`), seeded from the `GlobalRng`
WyRand like the backdrop draw.

### 5.6 Behaviour per event (proposal)

| Event | Music |
| --- | --- |
| Context change | crossfade 3 s; into Combat 1.5 s |
| Track ends, same context | silence gap 20-60 s (seeded), then next track; Combat: no gap |
| Pause, Commands, Debug | keep playing, duck -8 dB |
| TAB interface (`PauseStates::Interface`) | keep playing, no duck (station trade lives here) |
| Combat start / stop | hysteresis in 5.3; stop fades 4 s back to Flight |
| Scenario end (`OnExit(Playing)`) | not stopped by `stop_world_voices`; the rule moves to Menu and crossfades |
| Editor and Mod Hub | `Editor` context: one quiet track or silence (owner) |
| Test flight | normal Flight/Combat rules |
| Lessons | Flight |
| Menu backdrop | Menu; backdrop combat is ignored |
| Slider at 0 | voices keep running at gain 0; the playlist goes on |
| No audio device | deck reports `NoOutput`, logs once, holds |

### 5.7 Settings, menu, console, docs
- `PersistedSettings.music_volume: f32`, `#[serde(default = "default_volume")]`,
  placed after `world_volume` (`settings_store.rs:44-46`); load, live,
  change-detect and snapshot arms; plugin `init_resource` (`:469-471`).
- `VolumeChannel::Music`, label "Music", last row (`settings.rs:109-150`).
- Console `CHANNELS` + arms (`nova_console/src/settings.rs:78`, `:240-256`);
  catalog usage `volume [master|effects|interface|music [0..1]]`
  (`nova_command/src/commands.rs:398-408`).
- Docs: `web/src/wiki/settings.md:18-30` (three -> four sliders, a Music row),
  `web/src/wiki/commands.md:151`, `crates/nova_sound/src/lib.rs:28-35` bus
  doc, an `assets/base/music/README.md` licence table, `credits/CREDITS.md`.
- Changelog under `### Audio & Visuals`; not breaking (settings stay
  compatible).
- Playable example per feature (owner rule): `examples/playable/music_contexts.rs`
  that boots the full app and lets a player walk menu -> flight -> combat.

### 5.8 Graphs

Before:
```text
PlaySound / SoundLoop -> SoundVoice -> start_sfx_voices -> voice_player
                                    -> drive_sfx_voices (Mixer: Ui|Effect x master)
Settings slider (Master|Interface|Effects) -> *Volume -> Mixer
PauseStates frozen -> pause_world_voices (Effect only)
```

After:
```text
nova_scenario::music director (Update)
  GameStates, GameMode, CurrentScenario.role, PauseStates
  hostile CombatLock/AITarget, HealthApplyDamage, DockedShip+StationDock
    -> MusicSample -> CombatMood::step -> music_context()   (pure)
    -> MusicCatalog (R2 content) playlist -> MusicDeck::play/duck
nova_sound (PostUpdate, SoundSystems)
  MusicDeck -> MusicVoice x<=2 -> voice_player(Once)
            -> drive_music_voices (fade x clip loudness x Music row
               x MusicVolume x master) -> VoiceMix
Settings slider Music -> MusicVolume -> Mixer::bus_gain(Music)
settings.ron music_volume <-> MusicVolume
```

### 5.9 Owner decisions still open

1. Source and licence. Recommend CC0 first (no attribution duty, matches the
   CC0 sound precedent), CC-BY accepted with a CREDITS.md row. Refuse NC, ND
   and any licence that forbids putting the raw file in a public repo. See
   RESEARCH-music.md for the candidates.
2. Contexts. Recommend Menu, Flight, Combat, Docked, plus Editor as one quiet
   track or silence. Lessons use Flight.
3. Adaptive or playlist. Recommend a playlist per context with crossfade on
   change and the combat hysteresis. No stems or layers in this task.
4. Registry. Recommend R2 (content, moddable) over R1 (Rust table).
5. Pause. Recommend duck -8 dB on Paused/Commands/Debug, no duck on TAB.
6. Editor. Recommend one low ambient track; silence is the cheap fallback.
7. Default slider. Recommend 1.0 like the other sliders, with the Music row
   level set so music sits under effects.
8. Format and budget. Recommend OGG Vorbis q4, 44.1 kHz stereo, at most about
   25 MB for the first set, no LFS for now.
9. Devlog loops. Recommend excluding Music voices from the WebM sidecar by
   default, so a loop's audio stays the game's sound.

## 6. Verification plan

- Unit (pure, permanent: named stable behaviour):
  - `music_context` table: menu with a backdrop fight is Menu; Sandbox with no
    scenario is Editor; NewGame load gap keeps the previous context; docked
    is Docked; combat wins over docked.
  - `CombatMood::step`: a hit engages at once; a threat that blinks every 1 s
    for 60 s never disengages; 15 s calm disengages.
  - `fade_gain` endpoints and monotonic ramp.
  - Streaming `clip_loudness` equals the current result on the existing
    short-clip fixtures.
  - Settings: `music_volume` round-trips; a v0.17-style file with
    `music_volume: 0.15` loads 0.15; a v0.18 file without it loads the
    default (rewrite of `settings_store.rs:1095-1111`).
- ECS asserted example: MinimalPlugins + NovaSoundPlugin, a measured fake
  clip, `MusicDeck::play` twice: two `MusicVoice` entities, outgoing
  `VoiceMix.gain` falls to 0 and despawns, incoming rises; `MusicVolume(0.0)`
  gives `VoiceMix.gain == 0` and the voice stays alive.
- Live run (owner box, real device): menu, New Game, a fight, dock.
  - `NOVA_MUTE=0` is required; a harness run is muted at the master
    (`master.rs:92-97`). `VoiceMix` is pre-master, so a muted harness run
    still proves the slider math, not that sound came out.
  - Judge info log lines from the director (`nova music: context Menu ->
    Flight, track <id>, fade 3.0 s`) and the deck state, plus the published
    `VoiceMix` of music voices. Slider proof: `volume music 0` in the console
    mid-flight, then `VoiceMix.gain == 0`, then `volume music 1`.
  - Audible proof needs ears or a recorded device; headless output cannot
    prove that sound came out.
- `ALSA_CONFIG_PATH=/dev/null` (CI-like, no device): bevy logs "No audio
  device found." and `AudioOutput.stream` is `None`
  (`bevy_audio-0.20.1/src/audio_output.rs:20-31`, `:103-106`), so no sink is
  ever built. Then: no `sink.empty()`, so no track end; the deck must report
  `NoOutput` and hold, not cycle. Context changes, log lines and `VoiceMix`
  still work, so the context rule and slider are provable headless.
- Loudness cost: measure one 4 min OGG before and after the streaming change,
  peak RSS and wall time, matched repeat sets against the current measure as
  the named reference. No timing assertions.
