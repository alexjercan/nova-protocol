# Editor: full-spaceship copy-paste palette

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,editor,ui

Goal: full-spaceship palette in the editor - browse complete ship prototypes
with 3D preview and stamp ("copy paste") a whole ship into the scene;
duplicate an existing in-scene ship. Owner direction 2026-08-12: "full
spaceships to copy paste".

Context:
- Same gallery widget as the section picker task (build there, reuse here).
- parts_viewer's Tab ship view (assembled/exploded) is the preview
  reference.

Scope:
- Ship gallery tab: complete ship prototypes, assembled preview, name +
  section-count/mass readout.
- Stamp placement: paste a ship prototype at cursor/anchor; sensible
  id/name handling for duplicates; repeated stamping.
- In-scene duplicate: select an existing ship, copy, paste (deep copy of
  section list with fresh entity identity).
- Out of scope: saving a custom-built ship back AS a prototype (that is a
  content/modding feature; note it as follow-up if cheap hooks appear).

DoD:
- UI harness walk: open palette -> preview -> stamp two copies -> duplicate
  an in-scene ship -> scenario saves and reloads with all copies intact.
- probe green.

## Backlogged 2026-08-18

Moves with the editor epic `20260812-131912`. Unchanged, rescheduled.
