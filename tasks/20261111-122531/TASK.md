# Contextual flight key row: show what applies, hide what does nothing

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.18.0,hud,input

Planned 2026-11-11 from the v0.17.0 post-release review. Owner direction: the
flight bottom row is not right; some keybinds are missing from it and some are
useless unless something happens (docking, orbit).

## User facts
- The row must show the keys a player needs now, and stop showing chips that
  do nothing in the current state.
- Keys named as missing: TAB, raise weapons, fire, torpedo, lock cycle,
  roll Q/E, main drive. Keys named as dead most of the time: DOCK, ORBIT, HELM.

## Agent findings
- Row builder `keybind_dock_hud` and `DOCK_VERBS` fixed slot order:
  `crates/nova_hud/src/keybind_dock.rs:97-99,260-359`; state rule `chip_state`
  `:593-617`; update `:688-781`. Verb availability
  `crates/nova_ship/src/input/player/hints.rs:137-346`.
- v0.16.0 set the shape: fixed slots (`d41369ba9`), dim instead of hide
  (`abf185570`), COMPONENT chip removed and HELM docked-only (`53f881a7a`).
  v0.17.0 `tasks/20261026-003807/LABELS.md:36` marked the row KEEP. This task
  replaces that decision.
- The interface footer already shows only hints that apply
  (`crates/nova_interface/src/pane.rs:1089-1112`, `HintWhenType`).
- RCS chip stays lit with an empty magazine (`hints.rs:323-328`;
  `tasks/20261026-003807/AUDIT.md:19`).
- Pad: DOCK and HELM have no pad button (`hints.rs:386-397`); torpedo default
  LT2 collides with raise weapons LT2 (`crates/nova_editor/src/placement.rs:234`,
  `crates/nova_ship/src/input/bindings.rs:140-142`); railgun default R3 likely
  collides with open-interface R3 (`crates/nova_interface/src/bindings.rs:50-53`),
  unconfirmed at runtime. `flight_rig_reserved_sources` reads flight bindings
  only (`bindings.rs:156-166`).
- Weapon keys are per-section bindings, not Settings rows (`ship_rebind`).

## Decisions (pending owner review of the proposal)
- Proposal: a core group that is always on while flying (drive, RCS, radar,
  TAB) and a situational group that appears on state change: DOCK only with a
  dockable lock or while docked, HELM only docked, ORBIT only in a well,
  CANCEL only while engaged, GOTO only with a travel lock, weapon chips (fire,
  torpedo, railgun, lock cycle) only while weapons are raised. Roll Q/E shows
  in the core group or in the first-flight lesson, owner call.
- A chip that enters or leaves must not shift the others (keep slot stability
  within a group). Device-follow rule stays.
- Pad gets DOCK and HELM buttons; the LT2 and R3 collisions are resolved in
  the same slice, and `flight_rig_reserved_sources` must refuse a section
  default that collides with a rig or interface binding (lint, then load).

## Delivery
- Owner: `nova_hud` keybind_dock; inputs from `nova_ship` hints and
  `nova_interface` bindings. Dies: the fixed eight-slot rule and the dim state
  for inapplicable chips. May break: `screenshot_key_hints` example, HUD
  probes that assert chip rects, wiki `web/src/wiki/hud.md` and
  `keybinds.md`.
- Sequence: owner approves the chip groups -> hints expose group + visibility
  -> row reconciler -> pad buttons and collision lint -> wiki and changelog.

## Verification
- Asserted example: with no lock the row shows the core group only; a
  dockable lock adds DOCK; raising weapons adds the weapon chips; docking
  swaps to HELM. Capture under NOVA_CAPTURE_UI_SCALE 100 and 150.
- Unit test: an empty RCS magazine dims the RCS chip.
- Lint test: a section default that collides with a rig binding fails.

## Done when
- The row shows no chip that does nothing in the current state.
- Drive, weapons, lock cycle and TAB are visible from the row when they apply.
- Pad players see pad buttons for every chip shown.
