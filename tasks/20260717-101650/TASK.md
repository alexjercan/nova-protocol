# Thruster loop as a section sound: per-handle loop entities replace the single global hum

- STATUS: CLOSED
- PRIORITY: 26
- TAGS: spike, v0.7.0, audio, modding, feature

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

## Plan (2026-07-17, grounded)

Verified: `thruster_section(config)` bundle (thruster_section.rs:49) is the
snapshot point; the hum pipeline is ensure (one persistent loop entity) ->
compute (per-source avg throttle x attenuation, loudest wins, smoothed) ->
apply (single AudioSink write), plus pause/resume on state transitions.

### Steps

- [x] `loop_sound: Option<AssetRef<AudioSource>>` on ThrusterSectionConfig
      (serde attrs like render_mesh); bundle snapshots
      `ThrusterSectionLoopSound` (pub(crate)). Sweep other config literals.
- [x] Rework the hum pipeline per RESOLVED HANDLE (authored-or-silent):
      compute resolves each authored thruster's ref (idempotent), groups
      per-source-per-handle, target[handle] = max over sources of
      engine_volume(avg) x attenuation; smoothing per handle; unauthored
      thrusters contribute nothing. ensure spawns one loop entity per handle
      seen (`ThrusterLoopSfx(Handle)`), persistent like today's single one;
      apply writes each sink from its handle's smoothed level; pause/resume
      iterate all loop entities unchanged.
- [x] Delete WorldSfx::ThrusterLoop (bank 2 -> 1 key); guard test row.
- [x] gen_content: SectionMeshRefs += thruster_loop_sound
      (self://sounds/thruster_loop.wav); wire catalog thruster(s); regen;
      parity + lint green.
- [x] Tests: hum rig thrusters author their loop; existing per-ship
      attenuation/max/smoothing tests keep semantics (updated resource shape);
      NEW: two distinct handles hum independently; unauthored thruster is
      silent (delivery-guarded); menu-ambience path = base content authored.
- [x] Docs: wiki thruster field; sounds README (thruster_loop -> authored
      table, bank -> 1); CHANGELOG; spike fix record; prose-grep "bank" across
      assets/ web/ docs/.
- [x] Verify: fmt; workspace all-targets check; nova_gameplay lib + gates
      (read outputs).
