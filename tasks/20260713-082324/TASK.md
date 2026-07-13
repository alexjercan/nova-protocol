# Look-ray + camera-mode infrastructure: live aim in every view, robust mode transitions

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.5.0, input, camera, targeting, spike

## Goal

The deliberate-radar design (spike 20260713-082207) needs the LIVE look ray in
every view: hold-CTRL radar retargets to "what you look at" in Normal, FreeLook
and Turret views alike. Today the aim ray only integrates input on the rig
holding `SpaceshipRotationInputActiveMarker` (Turret mode), so outside combat
view it is frozen at the last raise - and the Normal/FreeLook/Turret mode
handling corrupts under nested holds (Alt + RMB overlap, last-writer-wins
observers).

Fix both, plus derive a public weapon-RAISED flag from `CombatInput` held state
(gameplay routes off the flag, never the camera enum).

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md.
- REINCARNATES closed task 20260712-231141 (wontdo with its family, but its
  body is design-agnostic infrastructure and was adversarially reviewed): at
  plan time, lift its Steps essentially verbatim - single mode derivation
  (Turret > FreeLook > Normal, memoryless), transition seeding from the
  OUTGOING rig, the active-look-ray accessor (with the press-frame property),
  re-point acquisition at the accessor, pause-latch verification, and the
  faithful split-rig test matrix. Replace its spike references with this
  family's.
- Pure infrastructure: no targeting-behavior change lands here; the radar task
  (20260713-082330) builds on it.
- No dependencies; first in the family.
