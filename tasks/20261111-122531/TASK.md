# Contextual flight key row: show what applies, hide what does nothing

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.18.0, hud, input

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

## Decisions (owner said start 2026-11-11; the calls below are the agent's, stated, not asked)
- `VerbHint` gains `shown: bool`: the verb applies to the situation, so the
  dock draws its chip. `available` keeps its meaning (the press does
  something). Shown and unavailable draws Dim; not shown leaves the layout.
- Groups, in `DOCK_VERBS` order. Core, always shown while the flight rig
  exists: INTERFACE (`interface_toggle`), DRIVE (`main_drive`), STOP, RCS,
  RADAR. Situational: GOTO (travel lock, not the docked partner), ORBIT (a
  dominant well, or orbiting), CANCEL (engaged), DOCK (docked, or the travel
  lock carries a docking port; dim until the gates pass or while refused),
  HELM (docked), MINE (a bound mining section). Weapons: AIM
  (`combat_stance`, shown while the ship carries a turret, torpedo bay or
  railgun; hot while `WeaponsHot`), FIRE / TORPEDO / RAILGUN (shown while
  weapons are hot and a section of that kind holds a binding; key from the
  section's own `Spaceship*InputBinding` on the device pressed last; FIRE
  hot while firing). DRIVE is hot while the main drive is held.
- Roll Q/E stays out of the row: a two-key verb has no chip shape, and roll
  is a camera-class control like free look and zoom. The first-flight lesson
  owns it.
- The lock-cycle chip stays out, as decided in `53f881a7a`.
- Layout: the core group is right-aligned to the screen centre line, the
  other groups grow rightward from it in order. A core chip never moves; a
  situational chip entering only pushes the chips to its right. HELM is an
  ordinary situational chip, no longer absolute.
- An emphasized verb (`HintEmphasis`) is always shown, dim when unavailable,
  so a tutorial can still point at a key before it applies. The verb names
  content uses today (STOP, GOTO, ORBIT, RADAR, RCS, DOCK) are unchanged.
- RCS is dim while the magazine is empty, matching the MATCH gate.
- Pad DOCK and HELM buttons and the LT2 / R3 section-default collisions are
  NOT in this slice: every pad button is taken, so they need the pad layout
  decision in `tasks/20260714-001140`. Recorded there as the next slice.
- Row-level wiki text (`web/src/wiki/hud.md` keybind dock, `glossary.md`),
  the lessons text and the changelog ship with the code.

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

## Delivery record (2026-11-11)
- Landed `822e330e1` on master: `VerbHint.shown`, the new hint fields, the
  two-half centre-line dock, the emphasis-keeps-shown rule, the RCS
  empty-magazine fix, wiki, lesson text and changelog.
- Proof run: `nova_ship --lib input::player::hints` 15 pass; `nova_hud --lib
  keybind_dock` 26 pass; `cargo check --examples --keep-going` clean;
  `screenshot_key_hints` smoke under Xvfb + lavapipe passed all 22 steps
  without capture.
- Frames: `row-stop.png` and `row-match-chord.png` in this folder, from the
  same example with `NOVA_CAPTURE=1` under lavapipe. They show the core
  group ending at the centre line, GOTO / ORBIT / DOCK (dim: the lock
  carries a port, the gates fail) / AIM to its right, and the MATCH word
  with RCS inverted while Shift is held. The capture run stalled at `let the
  modifier up` because the MATCH completed under the slow frames before the
  step's predicate ran (`maneuver complete, disengaging` in the log); the
  same step passes without capture, so it is a lavapipe timing limit, not a
  row defect.
- Still OPEN: pad players see keyboard caps on DOCK, HELM and MINE (no pad
  button exists). That is the pad layout slice in `tasks/20260714-001140`.
  The `Done when` pad line is not met until it lands.

## Pad slice decisions (owner said continue 2026-11-12; agent calls, stated)
Map with path:line evidence in `PAD-MAP.md`. No pad button with glyph art is
free in every flight state, so every new pad verb reuses a button in a state
where its v0.17.0 verb is dead. No v0.17.0 pad button moves and no stored
settings row changes.
- DOCK = L3 + Y and HELM = L3 + A, on the MATCH (L3 + B) pattern: GOTO and
  ORBIT oppose the RCS mode, so their buttons are dead while L3 is held. The
  chip draws both caps (modifier cap + button) on pad.
- MINE: the first mining emitter also binds RT. FIRE is refused while
  weapons are cold and mining is not weapons gated.
- TORPEDO = LB and RAILGUN = View as section defaults on pad. Both launch
  only while weapons are hot; pad free look and the pad HUD toggle are dead
  while weapons are hot (free look is already dead while LT is held).
- The LT torpedo default and the R3 railgun default leave the editor table:
  both collisions are confirmed (LT raises the stance and launches in one
  pull; R3 commits a charge under the interface).
- One default table for section bindings in `nova_ship`, used by the editor
  and by start roles. Test: every default source is free in Flight, or is a
  named shared pair with its guard. The authored-content finding for a
  player binding on a reserved source becomes an error.
- Limit: a ship frozen in a v0.17.0 save keeps its section bindings, so its
  TORPEDO, RAILGUN and MINE chips keep keys on pad until rebound.
- Proof without hardware: a headless walk drives a synthesized pad through
  `press_source`; frames of the row on pad. No hardware playthrough; that
  stays in `tasks/20260714-001140`.

## Pad slice delivery record (2026-11-12, master, not pushed)
- `520af03d6`: `section_default_bindings` in `nova_ship` is the one default
  table for the editor and start roles. Torpedo LB and railgun View on pad,
  first mining emitter on RT, LT and R3 defaults removed. Pad free look and
  the pad HUD toggle are dead while weapons are hot (read from the pressed
  source, so Alt and the grave key still work). The content finding for a
  player binding on a reserved source is an error; the Ledger webmod moved
  its torpedo pods from LT to LB.
- `b3e9c89e0`: L3 + Y docks and undocks, L3 + A takes and leaves the helm,
  through `DockPress` and `HelmPress` (one gate per verb). `VerbHint.modifier`
  and a second cap on the chip. The chord is device-agnostic like MATCH, so
  Shift+G and Shift+O do the same on keyboard.
- `d67463de4`: `screenshot_key_hints` walks the pad row with a synthesized
  pad: L3 + Y docks and flies no GOTO, L3 + A takes the helm, L3 + Y undocks.
  Wiki and changelog.
- `285842c0d`: the row moves left by the overflow when its right half is
  wider than half the screen (13 pad chips at 150 % cut RAILGUN before).
- Proof: `nova_ship` input, camera and docking 439 pass; `nova_hud`
  keybind_dock 28; `nova_menu` settings 54 (the shared-pair rule test sees
  all five binding owners); content lint 0 errors; the walk passes 32 steps
  at 100 % and 150 %. Frames `pad-weapons-100.png`, `pad-weapons-150.png`,
  `pad-docked-100.png` in this folder; every cap is glyph art.
- Done when: all three lines are met for a new game.

Limits:
- A ship frozen in a v0.17.0 save keeps its section bindings; its TORPEDO,
  RAILGUN and MINE chips keep keys on pad until rebound.
- No hardware pad playthrough (`tasks/20260714-001140`).
- The lint reserved list sees the flight and camera rigs only; the HUD View
  and interface R3 owners are covered by the `nova_menu` test, not by
  content lint.
- Seen in the frames, not changed: lit chips over a bright hull (aim
  camera) lose contrast; the docking sight brackets draw over the row and
  the DOCKED notice; the training screen draws keyboard caps for every
  lesson action on pad (`nova_menu/src/training.rs:1277`), so DOCK and HELM
  read D and H there.
