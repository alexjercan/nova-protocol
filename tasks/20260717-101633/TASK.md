# Controller section sounds: lock, radar deny/retarget and weapons-safety cues as authorable AssetRefs

- STATUS: OPEN
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
