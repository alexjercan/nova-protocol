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

## Open decisions (owner)
- Music source and licence: commissioned, self-made, generated, or CC.
- Contexts: menu, flight, combat, station; adaptive or plain playlist.

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

## Verification
- Unit test for the context rule; a live run that plays music on the
  menu and in flight, with the slider muting it.

## Done when
- Music plays in the chosen contexts, the slider controls it, and every
  track has a recorded licence.
