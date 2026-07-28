# HUD restyle + on-screen text reduction

- STATUS: OPEN
- PRIORITY: 36
- TAGS: v0.9.0,ui,hud

## Story

The flight HUD chrome (status bar, objective hint, comms panel, keybind hint
cluster, marker chips) still speaks the old flat language and carries the bulk
of the on-screen text. Restyle it to the accepted direction and cut text per
the spike decisions: icon-style keybind chips (folds backlog 20260710-231927),
slimmer objective/beacon chips, comms density, with detail delegated to the
NOVA OS computer.

## Steps (direction-level - refined at spike close)

- [ ] DIRECTION: restyle status bar, marker chips, comms panel, keybind hints
      per demo 2.
- [ ] DIRECTION: apply the text-reduction list per demo 2 (what goes, what
      shrinks, what moves into NOVA OS commands).
- [ ] Refine into real Steps/DoD from the accepted SPIKE.md before any
      implementation.

## Definition of Done (direction-level - refined at spike close)

1. Refined at spike close. Must include at minimum: harness coverage per
   changed element, updated screenshots, and a manual owner playtest verdict
   on text density.
