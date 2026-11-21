# Background music: bus, slider, first licensed tracks

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.19.0, audio

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- "I guess adding some kind of background music could work".

## Agent findings
- No music assets. Buses are Ui and Effect only
  (`crates/nova_sound/src/lib.rs:28-35`). All audio goes through nova_sound
  (`lib.rs:4-11`).
- The unused Music bus and slider were removed in v0.18.0 (`baac8df5a`,
  `CHANGELOG.md:79`) because nothing played on them.
- Prior direction: music ships as OGG (`tasks/20260824-125955/TASK.md:69-70`).
- The v0.14 trailer score came from a seeded generator outside the repo and
  "carries no licence" (`tasks/20260909-212954/TASK.md:122-123`).

## Owner picks (2026-11-21)
- Theme: "Exploration Theme" (Cleyton Kauffman, CC0), "maybe in main
  menu". Gameplay: Type 42 tracks (CC BY 4.0), "it has a lot of songs so
  it could add some variety".
- Situations get their own songs: "if you are in a combat or within
  enemies we have a different song then when you are simply cruising".
- "the music is more as background so it shouldn't really bother us".
- Mods: an event like "play song" with a path to an audio file, so a mod
  can play its own music.
- Wants a design HTML page with examples of when and how each song plays
  (DESIGN-music.html in this folder) before the build.

## Decisions (owner approved DESIGN-music.html recommendations, 2026-11-21)
- All 10 songs ship: Exploration Theme (menu); Stellar Cruise, Little
  Blue Speck, Uncharted (cruise); Unseen, Unknown (enemies near); Intense
  Electronic Action Soundtrack, The Hunter (combat); Ambient Soundtrack,
  Hidden Nebula (docked). OGG Vorbis q4, about 34 MB.
- Enemies near: a hostile lock on you held 2 s, or a hostile AI engaging
  you; ends after 10 s with neither. Combat: at once on a hit on or by
  your ship; ends after 15 s with no hit.
- Background: Music category -16 dB; 30-120 s gaps between cruise songs,
  10-30 s between docked songs; fades 3-4 s, 1.5 s into combat; pause
  ducks 8 dB; Tab does not duck; editor and Mod Hub are silent.
- Mods: `PlayMusic` (sound, fade_in, then: Resume|Loop, in_combat:
  Yield|Keep, all required) and `StopMusic` (fade_out); mods may add
  tracks to a situation playlist. Mod music is OGG only; other formats
  are a lint error that shows the ffmpeg command.
- Track registry is content, generated from a nova_authoring builder;
  every track carries a required licence record.
- Devlog loop capture leaves music out.
- 2026-11-21, owner on switching: "I guess the only feedback would be to not
  switch away from a song instantly when a state changes, but like in the
  sense of -> if there would be a pause with no music that's when you let me
  previous song keep playing and when that pause would stop you switch to the
  new one". Rule: a change to Cruise or Docked does not fade the playing
  flight song out. It plays on for the gap (cruise entry 20-40 s, docked
  entry 10-30 s, after a fight 10-20 s), then crossfades with the new
  situation's fade-in. A song that ends in the gap leaves the rest silent.
  A change during the gap restarts the gap for the newest situation; a change
  back to the song's situation cancels the switch. Threat and Combat still
  switch at once. Silent, the menu theme, scenario songs, `StopMusic` and the
  pause duck keep their behaviour.

## Delivery
1. Music bus, volume slider and settings field.
2. First tracks with a recorded licence, and a context rule.
3. Wiki and changelog.

## Progress
- 2026-11-21, engine groundwork (owner choices not needed): streaming
  loudness measure off the main thread, `SoundBus::Music` with its
  `SoundCategoryType::Music` row, `MusicVolume`, `music_volume` in
  settings.ron, and `MusicDeck` (`play`, `stop`, `duck`, `status`) in
  `nova_sound`.
- Next, in the same change as the first tracks: the Music row in the audio
  settings menu (`VolumeChannel::Music`), the console `volume music` channel
  and its catalog usage, the wiki `settings.md` and `commands.md` rows, and
  the changelog entry. They stay out until a track plays, so no player sees a
  slider that controls nothing.
- 2026-11-21, music delivered: 10 OGG q4 tracks in `assets/base/music/`
  with their licence records (`credits/CREDITS.md`, `credits/licenses/`,
  the folder README); the `MusicTrack` content kind built by
  `nova_authoring` and refused at lint for an unknown situation, a missing
  file, a non-OGG file (with the `ffmpeg` command) or no licence record;
  `nova_scenario::music` (context rule, mood hysteresis, director with
  shuffled playlists, gaps on a forked seeded stream, fades and the pause
  duck); `PlayMusic` and `StopMusic`; the Music slider, `volume music`,
  the wiki and creator docs, the changelog, and `music_sandbox`. The WebM
  sidecar leaves music voices out.
- 2026-11-21, live walk (`music_sandbox`, Xvfb + lavapipe, no audio
  device): contexts Silent, Menu, Cruise, Docked, Cruise, Threat, Combat,
  Cruise, Docked; `volume music 0` held the voice at gain 0; the deck held
  `NoOutput`. Not yet heard on a real audio device.

## Verification
- Unit test for the context rule; a live run that plays music on the
  menu and in flight, with the slider muting it.

## Done when
- Music plays in the chosen contexts, the slider controls it, and every
  track has a recorded licence.
