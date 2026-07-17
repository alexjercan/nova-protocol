# Controller section sounds: lock, radar deny/retarget and weapons-safety cues as authorable AssetRefs

- STATUS: IN_PROGRESS
- PRIORITY: 30
- TAGS: spike,v0.7.0,audio,modding,feature


## Goal

The radar/lock/safety cue family becomes authorable on the CONTROLLER section
(the ship's computer - Lock is a computer-gated capability, targeting.rs
`ship_grants_verb`): `lock_on_sound`, `lock_off_sound`, `radar_deny_sound`,
`radar_retarget_sound`, `safety_on_sound` on `ControllerSectionConfig`. The
messages are player-scoped (no entity payload), so cues read the player ship's
`ControllerSectionMarker` child. gen_content authors base defaults; delete the
family's `WorldSfx` keys.

## Notes

- Spike: tasks/20260717-101524/SPIKE.md. Depends on 20260717-101615 (bank split).
- Open question from the spike: confirm safety_on belongs here vs UI (recommend
  controller).
- Stepless direction-level task: run /plan before /work.

## Plan (2026-07-17)

Verified: ControllerSectionConfig exists with render_mesh AssetRef
(controller_section.rs:20-33); the section is built by the BUNDLE fn
`controller_section(config)` (not an observer) - the sound snapshot joins the
bundle. The radar/lock messages carry no entity (player-scoped), so cues find
the player ship's controller child.

### Steps

- [x] Five `Option<AssetRef<AudioSource>>` fields on `ControllerSectionConfig`
      (lock_on_sound, lock_off_sound, radar_deny_sound, radar_retarget_sound,
      safety_on_sound; serde attrs like render_mesh; None defaults). Snapshot
      into ONE `ControllerSectionSounds` component (five Options) added by the
      `controller_section` bundle (and `preview_controller_section` if it
      carries render state - verify).
- [x] Cues: `play_lock_cues` + `play_safety_engaged_cue` drop the bank; query
      the PLAYER's controller (controller with `ChildOf` == entity carrying
      `PlayerSpaceshipMarker`), resolve the authored refs (Res<AssetServer>),
      authored-or-silent. Messages still DRAIN when no controller/player exists
      (the old no-bank drain guard, same reason).
- [x] WorldSfx shrinks 9 -> 4 (delete LockOn, LockOff, SafetyOn, RadarDeny,
      RadarRetarget keys + FILES rows + guard rows). Wavs stay shipped + in
      resources.
- [x] gen_content: SectionMeshRefs += the five self://sounds/... refs; wire the
      base controller section(s) in build_sections; regenerate; parity + lint
      green.
- [x] Tests: authored controller plays each cue's own handle / unauthored (or
      no controller) is silent with delivery guards; message-drain behavior
      preserved; snapshot test for the bundle.
- [x] Docs: wiki guide-author-section controller fields; base sounds README;
      CHANGELOG (fold into the weapon-sounds modding bullet); spike fix record.
      Prose-grep for the old model's words after the flip (LESSONS 101624).
- [x] Verify: fmt; nova_gameplay lib; content gates; workspace all-targets
      check (read output, not pipe exit).
