# Pad map: flight gamepad layout, collisions, and layout options

Read-only map made 2026-11-12 for the open line "Pad players see pad buttons for every chip
shown". No code was run. Claims carry `path:line`; claims with no proof are marked UNVERIFIED.
Paths are under `crates/` unless they start with `web/`, `examples/` or `tasks/`.

Naming trap: bevy `LeftTrigger`/`RightTrigger` are the BUMPERS (LB/RB). `LeftTrigger2`/
`RightTrigger2` are the analog TRIGGERS (LT/RT). See `nova_hud/src/key_glyphs.rs:211-214`.

## 1. The pad in flight today

Input library: `bevy_enhanced_input` 0.27.2 (`Cargo.toml:1699`, `Cargo.lock:1107`), bevy 0.20.1.
Nova wraps it in `nova_input` (registry, contexts, dispatch).

| Button | Verb(s) | Context and gate | Source |
|---|---|---|---|
| A `South` | ORBIT | Flight; needs a dominant well; re-press ends it | `nova_ship/src/input/bindings.rs:48-50`, `player/flight_rig.rs:643-700` |
| B `East` | STOP; MATCH while L3 is held | Flight | `bindings.rs:40-42`, `flight_rig.rs:495-538` |
| Y `North` | GOTO | Flight; needs a travel lock | `bindings.rs:43-45` |
| X `West` | CANCEL | Flight; acts only while engaged | `bindings.rs:51-53`, `player/hints.rs:480-486` |
| RB `RightTrigger` | DRIVE; editor thruster default | Flight | `bindings.rs:30-32`, `nova_editor/src/placement.rs:229` |
| RT `RightTrigger2` | FIRE (turret section default) | section binding; press denied cold | `placement.rs:230-233`, `nova_scenario/src/objects/start_role.rs:110-113` |
| LB `LeftTrigger` | free look (hold) | Flight; dead while LT is held | `bindings.rs:137-139`, `nova_ship/src/camera/mode.rs:282-289` |
| LT `LeftTrigger2` | AIM `combat_stance`; editor torpedo default | Flight | `bindings.rs:140-142`, `placement.rs:234` |
| L3 `LeftThumb` | RCS modifier (hold); editor play-test "back to editor" raw read | Flight | `bindings.rs:95-97`, `nova_editor/src/lib.rs:1061-1076` |
| R3 `RightThumb` | open interface; editor railgun default | Always | `nova_interface/src/bindings.rs:50-53`, `placement.rs:235` |
| D-Pad Up | radar: hold sweeps, tap clears | Flight | `bindings.rs:71-83`, `flight_rig.rs:240-260` |
| D-Pad Left/Right | component lock prev/next; RCS down/up while RCS is active | Flight; needs a combat lock | `bindings.rs:84-89`, `input/targeting/component_lock.rs:233-263` |
| D-Pad Down | scenario Advance and Skip Scene | Flight | `nova_scenario/src/loader/lifecycle.rs:708-722` |
| View `Select` | HUD On/Cinematic | Always; system gated to Playing, not Interface/Commands | `nova_hud/src/lib.rs:432-454` |
| Menu `Start` | pause; interface back-out | fixed raw read, never capturable | `nova_menu/src/pause.rs:86-88`, `nova_interface/src/terminal/input.rs:128-130`, `nova_input/src/poll.rs:113-126` |
| Guide `Mode`, `C`, `Z` | unbound; no glyph art | - | `examples/screenshots/screenshot_key_hints.rs:72-75`, `key_glyphs.rs:561-565` |
| Left stick | RCS aim, only while `RcsActive` | Flight | `bindings.rs:104-106`, `flight_rig.rs:896-910` |
| Right stick | camera aim | Flight | `bindings.rs:134-136` |

No pad source at all: roll, DOCK, HELM (`bindings.rs:37-39,62,66`), zoom (wheel only,
`:111-122`), MINE, and in the open world TORPEDO and RAILGUN (`start_role.rs:115-116` binds
`R` and `F` only).

Free buttons:
- Free in every flight state, with art: none.
- Free but unusable: `Mode`, `C`, `Z` have no glyph. The whole-table test refuses a shipped
  source with no art (`nova_menu/src/tests/settings.rs:1181-1193`).
- Free axis: the left stick while L3 is up (`flight_rig.rs:908-910`). Dead while LT is held:
  LB (`mode.rs:282-289`).
- Dead only while L3 is held: Y and A. RCS start removes the autopilot
  (`flight_rig.rs:841-845`), so GOTO and ORBIT oppose the mode.
- No-op without a combat lock: D-Pad Left/Right. No-op while not engaged: X.
- Low-value holders: View (HUD cinematic) and LB (free look). Neither has a chip on the key
  row (`nova_hud/src/keybind_dock.rs:103-119`). Docked frees nothing: without the helm the
  presses are refused, not reassigned (`hints.rs:387-390`).

Registry rule that blocks plain sharing: two actions in overlapping contexts may not hold one
source unless one `follows` the other (`nova_input/src/registry.rs:747-762,855-857`). All
flight, camera and scenario actions are `Flight` (`bindings.rs:124-126,144-146`). Tests
enforce it: `bindings.rs:242-259`, `nova_menu/src/tests/settings.rs:1149-1171`.

What the library supports, and what the code uses:
- Hold versus tap: used. `Hold::new` and `Tap::new` on one source, paired by `follows`
  (`flight_rig.rs:240-260`, `bindings.rs:71-83`).
- Chords: the library has `Chord` and `BlockBy` (crate `src/condition/chord.rs`,
  `block_by.rs`). The code rejects them (`flight_rig.rs:123-127`). A chord is one action whose
  observer reads a modifier's `TriggerState`: `action_held(&q_modifier)` in STOP
  (`flight_rig.rs:495`, rule at `:456-462`). The keyboard MATCH chord is L3+B on pad: same two
  actions, pad columns (`bindings.rs:42,97`). The row swaps the chip word; it draws no second
  cap (`keybind_dock.rs:831-836`).
- Context layers: `ContextActivity`/`ContextPriority` exist in the library and are unused.
  Nova uses `ActionContext` + `ActiveContexts` (`nova_input/src/context.rs:22-51`), raised by
  the owner (`nova_ship/src/input/player/mod.rs:87-100`), plus observer self-guards on the
  pause. Reuse example: the Viewer verbs reuse flight buttons (`nova_interface/src/bindings.rs:32-38`).
- Every rig and section action sets `consume_input: false` (`flight_rig.rs:137-140`,
  `player/weapons.rs:81,208,305,402,512`). No binding "wins"; every listener fires.

## 2. DOCK, HELM, MINE on keyboard, and how a chip gets its cap

- DOCK: `D`, action `dock`, Flight (`bindings.rs:62`). Docked: undocks, ahead of the capability
  gate. Else needs `dock_enabled`, a travel lock, admission, a port pair (`flight_rig.rs:745-792`).
- HELM: `H`, action `dock_helm`, Flight (`bindings.rs:66`). No-op unless docked
  (`flight_rig.rs:798-816`).
- MINE: no registry action. Each mining section holds its own key (`V`, then
  `ADDED_MINING_KEYS`; `start_role.rs:117-127`; editor `placement.rs:183,225-228`). Hold to run
  the beam out. Not gated by the weapons safety (`weapons.rs:521-541`). Two emitters may not
  share a source (`nova_scenario/src/lint/scenario.rs:1931-1934`, `objects/spaceship.rs:1846-1849`).
- Correction to the brief: DOCK and HELM DO apply together. Docked shows both chips
  (`hints.rs:500,520`), because DOCK is also the undock.
- `VerbHint` has no `pad` field. It has one `key: String` (`hints.rs:27-42`). A rig verb
  resolves `hint_source(device)`: first source on the device pressed last, else the first on
  the other device, then `glyph_label()` (`hints.rs:302-312`, `nova_input/src/device.rs:36-42`).
  A pad label reads `Pad <name>` (`nova_input/src/source.rs:49-54`). A section chip uses
  `section_key` with the same fallback (`hints.rs:213-222`).
- So "no pad button" is never `None`: the chip keeps the keyboard cap. An empty `key` means no
  rig, and the chip leaves the row (`keybind_dock.rs:784`). The cap is `glyphs.get(&hint.key)`;
  a label with no art draws as text (`keybind_dock.rs:667-675,790-799`).
- Pinned today: `hints.rs:584-607` asserts "DOCK is the verb with no pad button";
  `hints.rs:866` asserts the railgun keeps `R` on pad. The no-pad reason is at `bindings.rs:54-66`.

## 3. The two collisions

Scope first. Both come only from the editor table `default_binds` (`placement.rs:216-237`),
used by placement (`:241-259`) and Generate (`nova_editor/src/generate.rs:190-205`). Shipped
content holds neither: `assets/` has 8 `Gamepad(RightTrigger2)` and no other pad source.
Open-world hulls bind torpedo `F` and railgun `R` only (`start_role.rs:105-131`).

Torpedo LT versus AIM LT: CONFIRMED, state dependent.
- Both are live in Flight, and nothing consumes the press. Observers trigger in `PreUpdate`;
  `WeaponsRaised` is derived later in `Update` (`player/wheel.rs:9-15`). `WeaponsHot` is
  raised OR combat lock (`input/targeting/safety.rs:26-33`).
- No combat lock: the torpedo `Start` sees a cold ship and returns (`weapons.rs:329-336`). The
  stance rises a moment later. The press never latches. LT cannot launch.
- Combat lock held: the ship is hot before the press. Pulling LT to aim launches a torpedo.
  A torpedo needs that lock to launch (`web/src/wiki/keybinds.md:329`), so on pad every launch
  is tied to AIM.

Railgun R3 versus open-interface R3: CONFIRMED while hot.
- `interface_toggle` is `Always` and polled in `Update` (`nova_interface/src/pane.rs:132-152`).
  It sets `NextState`, so the pause is not frozen on the press frame.
- The railgun `Start` runs first, in `PreUpdate`, with the pause still `Unpaused`
  (`weapons.rs:414-441`). Hot: the charge commits and cannot be aborted (`:411-413`), and the
  interface opens over it. Cold: only the interface opens. The close press is safe: the pause
  is frozen and the observer returns.

Who checks a section binding against reserved sources:
- `flight_rig_reserved_sources` (`bindings.rs:156-166`): `flight_bindings()` only. It misses
  camera (LT, LB), interface (R3), HUD (View) and scenario (D-Pad Down). A drift guard pins it
  to the flight rig "no more, no less" (`player/intent.rs:801-845`).
- Content lint: `scenario_input_overlaps` (`nova_authoring/src/lint_walk.rs:877-904`) walks
  player `input_mapping` against that list. Severity Warn (`lint_walk.rs:1164-1183`); warnings
  never gate (`nova_authoring/src/cli.rs:146-154`).
- `nova_scenario/src/lint/ship.rs` has no input check. The scenario lint has only the
  mining-key rule (`lint/scenario.rs:1931-1980`).
- Ship pane rebind: `holder_in(ActionContext::Flight, source)` on the LIVE table, policy Refuse
  (`nova_interface/src/ship/rebind.rs:15-19`, `registry.rs:788-792,215-231`). `Flight` overlaps
  `Always` (`context.rs:44-50`), so it already refuses LT and R3. Editor rebind: same lookup,
  policy Warn (`nova_editor/src/keybind.rs:338-342`).
- `default_binds` passes through neither. Its own test compares kinds to each other only
  (`placement.rs:2577-2643`). The arena test checks weapons against
  `flight_rig_reserved_sources` only (`examples/playable/wfc_arena.rs:3264-3289`).
- Reverse gap: the registry cannot see section triggers. Settings accepts a rig verb on RT
  (`nova_menu/src/tests/settings.rs:660-676` lists it as free).

Where the owed lint test can live:
- Crate direction: `nova_scenario`, `nova_authoring` and `nova_editor` do NOT depend on
  `nova_interface` (`nova_authoring/Cargo.toml:22-31`). They see flight, camera, HUD and
  scenario bindings, never R3. `nova_menu` sees all five owners (`tests/settings.rs:1196-1203`)
  but not `nova_editor`. Only `nova_core` sees everything (`nova_core/Cargo.toml:17-27`).
- Cheapest sound shape: move the two default tables (`default_binds`, `start_role_bindings`;
  they disagree today) into one pure function in `nova_ship/src/input/bindings.rs`. Test it in
  `nova_menu/src/tests/settings.rs` beside the conflict test, with `holder_in(Flight, source)`
  over the whole table.
- For authored content: widen `flight_rig_reserved_sources` with `camera_bindings()` (same
  crate) and widen the drift guard. `nova_authoring` can add `hud_bindings()` and
  `scenario_bindings()`. R3 needs a `nova_interface` dependency or `interface_toggle` declared
  lower.
- The rule needs one exemption: the thruster default `Space` + RB is the main drive's own pair
  (`placement.rs:229`, `bindings.rs:30-32`). "Fails" means Error; the finding is Warn today.

## 4. Rebind and settings

- Registry verbs: Settings > Controls, a Desk chip and a Pad chip per row
  (`nova_menu/src/tests/settings.rs:650-652,682-687`). Pad capture reads the `Gamepad`
  component, never `Start` (`poll.rs:113-126`). `commit_rebind` refuses a source held in an
  overlapping context (`registry.rs:554-565`).
- A column that ships empty cannot be bound (`registry.rs:615-621`). A pad player cannot give
  DOCK, HELM or roll a button in Settings today.
- Stored: `PersistedSettings.keybinds`, only rows that differ from the default
  (`nova_menu/src/settings_store.rs:120-124,770`, `registry.rs:639-646`). Load renames old ids,
  applies, then resets stored rows that collide (`settings_store.rs:519-568`,
  `registry.rs:671-696`). All fields are `serde(default)`; a v0.17.0 file loads.
- A NEW action, or a new pad default on a row the player never moved, reaches an old file with
  no migration. A row the player DID move stores both columns (`registry.rs:109-116`): a player
  who moved `dock` to another key keeps `gamepad: []` until Reset Defaults.
- REMOVING a pad default is the hazard. A stored pad source on a row that now ships none is
  refused whole, keyboard half included, behind a `warn!` (`registry.rs:536-544,619-621`). An
  eviction needs a load step that strips the column.
- Section keys: Ship pane `ship_rebind` (`B`, no pad arm button; `nova_interface/src/bindings.rs:129-131`).
  The binding lives on the section and is frozen into the world save
  (`nova_scenario/src/objects/spaceship.rs:949-955`). A new section default reaches new spawns only.
- Tables a default change must update: `bindings.rs:187-228,265-311`;
  `nova_menu/src/tests/settings.rs:629-656`; `hints.rs:584-607,794-870`; `intent.rs:801-845`;
  `placement.rs:2577-2643`; `start_role.rs:340-362`.

## 5. Docs and proofs

- `web/src/wiki/keybinds.md`: DOCK/HELM rows with a dash (`:128-137`), roll (`:144`), mining
  (`:146,322-324`), torpedo LT and railgun R3 (`:300-317`; false for open-world hulls today),
  prose (`:329`).
- `web/src/wiki/hud.md:65` ("A verb with no gamepad binding keeps its key"), `:44`, `:178`;
  `interface.md:37-39`; `settings.md:93-124`. Changelog: one `[Unreleased]` entry. Lessons
  name one pad button (`nova_authoring/src/base_content/lessons.rs:1324`).
- `examples/screenshots/screenshot_key_hints.rs`: spawns a `Gamepad`, presses `Mode`, waits for
  `InputDeviceType::Pad`, shoots `key-hints-pad.png` (`:72-75,323-338`). It checks STOP only.
- A headless walk CAN drive a pad verb. `dispatch::press_source(world,
  InputSource::Gamepad(..), phase)` writes both halves of a synthesized pad
  (`nova_input/src/dispatch.rs:117-136,211-246`). A real rig reads it (`dispatch.rs:503`). The
  device tracker counts a scripted pad press (`nova_input/src/device.rs:11-12,68-88`).
- `press_action` presses the DESK source when one exists (`dispatch.rs:250-260`,
  `nova_debug/src/harness.rs:590-599`) and hands the hints back to the desk
  (`screenshot_key_hints.rs:346-356`). A pad walk resolves `action.gamepad[0]` and calls
  `press_source`. `nova_autopilot` has no pad helpers (`nova_autopilot/src/input.rs:27-29`).
  Unit tests insert `InputDeviceType::Pad` directly (`hints.rs:599,859`).

## 6. Layouts

Arithmetic: 5 verbs have no clean button (DOCK, HELM, MINE, TORPEDO, RAILGUN). Plain eviction
frees 2 at most (View, LB). So every layout below drops the LT and R3 section defaults and
leaves TORPEDO and RAILGUN on `F` and `R`, as open-world hulls are today. Their two chips
still draw keys on pad. That needs a weapon-select design; no layout here solves it.

Common to A, B and C:
- `default_binds`: torpedo `[F]`, railgun `[R]`. Both collisions end. Old editor ships keep
  their saved binds; the widened lint flags them.
- MINE: the first emitter adds RT (`start_role.rs:117-127`, `placement.rs:225-228`). RT is
  free of the registry in Flight. Reuse by context: FIRE is denied cold; mining is not weapons
  gated. Hot, on a hull with both, RT does both and both chips draw RT. Later emitters keep keys.

A. L3 layer. Nothing moves.
- DOCK = L3+Y, HELM = L3+A. Same pattern as MATCH = L3+B: the GOTO and ORBIT observers read
  the RCS modifier (`flight_rig.rs:495`). Hands: in a docking approach L3 is already held and
  the right thumb is idle (camera frozen under RCS, `mode.rs:176-190`).
- Cost: the chip needs a modifier cap (`VerbHint` gains a modifier label, the chip a second
  cap), or pad DOCK/HELM show only while L3 is held, as MATCH does. The keyboard modifier +G
  and +O also become DOCK and HELM, unless the observer checks the device. Pressing L3 drops
  an engaged autopilot. Settings shows Dock and Helm as Unbound on pad.
- Relearn: nothing. Settings migration: none. Files: `flight_rig.rs`, `hints.rs`,
  `keybind_dock.rs`, `placement.rs`, `start_role.rs`, tests, wiki.

B. Two plain buttons. Data only.
- DOCK = LB, HELM = View. Free look and HUD cinematic lose their pad button.
- Cost: no pad free look in manual flight. Stored `free_look` and `hud_cinematic` pad rows
  need the strip step (section 4).
- Relearn: LB now docks and undocks. A v0.17.0 habit press near a port docks. Files:
  `bindings.rs`, `nova_hud/src/lib.rs`, `settings_store.rs`, the common set.

C. One button, hold and tap.
- View: hold = DOCK/undock, tap = HELM. Only HUD cinematic leaves the pad.
- Two pad-only rig actions with `Hold` and `Tap`, paired by `follows`, as radar is.
  Binding-level conditions on `dock`'s pad column would be smaller but are UNVERIFIED beside a
  plain keyboard binding.
- Cost: two chips share one cap, so DOCK needs a HOLD mark. View is a poor reach while L3 is
  held. Stored `hud_cinematic` pad rows need the strip step.
- Relearn: View no longer toggles the HUD. Files: `flight_rig.rs`, `bindings.rs`, `hints.rs`,
  `keybind_dock.rs`, `nova_hud/src/lib.rs`, `settings_store.rs`, the common set.

Recommendation: A, with the modifier cap on the chip. It extends a pattern the row already
has, moves no v0.17.0 button, needs no settings migration, and fits the hands at the moment
DOCK is pressed. The modifier cap also keeps an emphasized DOCK chip honest on pad.

Open decisions for the owner: A, B or C; under A, modifier cap or MATCH-style show-on-hold;
whether the Done-when line may close with TORPEDO and RAILGUN on keys.
