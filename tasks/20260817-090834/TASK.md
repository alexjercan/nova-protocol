# Promote the thruster shells: check the candidates, ship the picks

- STATUS: OPEN
- PRIORITY: 47
- TAGS: v0.11.0,art,ship,content

## Goal

Promote the thruster shell candidates from art to the game: the seven
bell/vector-language shells (task 20260817-013639, landed as
art/part-candidates/shells/) become real thruster looks, and each candidate
is CHECKED before promotion.

## The checks (per candidate, before any promotion)

- exhaust geometry agrees with the thrust convention: thrust is -Z, the
  bell opens +Z (clearance.rs exit_normal), and the exit_pocket / exhaust
  lane clearance fits the mesh silhouette
- triangle budget and material sanity at ship render distance (the gallery
  render is the judging view; owner picks WHICH candidates promote)
- recipe determinism stays under gen-thruster-shells.py --check

## Promotion (1x1 only)

- the chosen candidate(s) replace or join the basic_thruster render mesh as
  authored prototype models (nova_authoring section builders + content gen;
  never hand-edit the generated RON)
- wfc_ships / wfc_arena inherit the new look automatically through the
  prototype; verify with a bench or arena render
- the large formats (shell_bank 3x3x1, shell_capital 5x5x3) stay ART until
  the multi-cell section question (THRUSTERS.md follow-up 3) is decided -
  this task does NOT open that

## Done when

- owner has picked from the gallery; picked shells fly on real ships in a
  render; checks recorded per candidate; large formats explicitly deferred
