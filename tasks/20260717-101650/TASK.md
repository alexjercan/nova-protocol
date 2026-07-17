# Thruster loop as a section sound: per-handle loop entities replace the single global hum

- STATUS: OPEN
- PRIORITY: 26
- TAGS: spike,v0.7.0,audio,modding,feature


## Goal

The engine hum becomes a section sound: `loop_sound` on
`ThrusterSectionConfig`. The single global loop entity becomes one loop entity
per DISTINCT resolved handle (normally one), with the existing per-ship
avg-throttle x distance-attenuation, loudest-wins math computed per handle
group. Pause/resume behavior and the menu-ambience backdrop hum (a live
scenario) must keep working. gen_content authors the base default; delete the
thruster `WorldSfx` key.

## Notes

- Spike: tasks/20260717-101524/SPIKE.md. Depends on 20260717-101615 (bank split).
- The risky one of the family: a continuous cue with volume smoothing, pause
  gating and per-ship attribution tests (audio.rs hum test rig) that all need
  to survive the regrouping.
- Stepless direction-level task: run /plan before /work.
