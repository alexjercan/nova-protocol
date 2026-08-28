# The game as a process: named inputs, an input channel, and state on stdout

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.12.0, tooling, input, autopilot

Make the game drivable and observable as a PROCESS: named input actions, a
command channel that carries both inputs and scenario actions, world state on
stdout, and a step mode that lets a slow driver keep up.

Designed with the owner 2026-08-20. `--norender` already landed (`a47c6247`),
which is the precondition: no device, no window, no winit, main schedule still
ticking.

## Three independent payoffs, and the first one ships to players

1. **Rebinding.** `crates/nova_ship/src/input/reference.rs` is a hand-authored
   DISPLAY MIRROR of the flight rig, kept in parity by a test, carrying
   `TODO(20260710-231927)` for real remapping. Naming the rig's actions deletes
   that workaround: the settings menu reads actions by name instead of a copy.
   **This is a shipping feature, not test scaffolding.**
2. **Testing and debugging without a rebuild.** Today "what if I hold radar
   here" means editing Rust and waiting on a Bevy compile.
3. **An external agent can play.** The owner's case: an agent reads world state
   as JSONL and writes input JSONL back.

Any one of the three justifies the work. Do not let the third one inflate the
scope of the first two.

## What already exists -- most of the output half is BUILT

- `crates/nova_probe/src/capabilities/snapshot.rs` serialises the world to one
  JSON object: ships (identity, transform, velocity, health, mass, defeat and
  neutralize flags, weapon locks), every section (id, prototype, class, pose,
  health, alive, weapon state, fixtures), and all ordnance in flight (owner,
  position, velocity, damage, remaining lifetime, target). **`capture_snapshot`
  does no IO** - hand it `&mut World`, get the object. Its own module doc
  already says: "A future headless JSON mode that reads actions on stdin and
  emits state on stdout is the same call with a different sink."
- `EventActionConfig` (`crates/nova_scenario/src/actions/mod.rs`) already
  serialises for scenario RON, 25 actions.
- `nova_autopilot::input::press_key` / `press_mouse` already synthesise real
  input events.
- `AppBuilder::headless()` already exists.

## Settled design

**Named input actions.** Every player action gets a stable string -
`main_drive`, `radar_hold`, `fire_primary` - sourced from the input RIG
(`nova_ship/src/input/player/flight_rig.rs`), NOT from `reference.rs`, which is
a display mirror. Deriving from the mirror would create a third copy to desync.

**The dispatcher is a lookup table and an injector, not an interpreter.** One
function: `apply(name, phase)` resolves the name and injects into the input
layer. serde does the parsing; there is no language. Console, stdin and
autopilot all resolve to this one call, and `hold_radar` in
`examples/screenshots/screenshot_combat_lock.rs` stops hardcoding a `KeyCode`.

**One transport, two vocabularies**: `input <name>` and `action <name>`.

The asymmetry is deliberate and self-documenting: **a scenario RON holds actions
only; the channel holds both.** An input goes through the game's rules and can
legitimately fail - radar destroyed, magazine empty. An action bypasses them.
Merging the two namespaces would lose the guarantee that makes an input test
prove "a player could do this".

**Failure semantics differ by namespace.** An input that fails is
gameplay-truthful: report it in the SNAPSHOT output so a driver observes it did
not work, and continue. An action that fails is a script error: log it loudly.

**Actions reuse `EventActionConfig`.** The channel parses the same enum the RON
does, so every existing action is available over the channel with zero
per-action work and the two can never drift. This is the difference between
"add 25 commands" and "add none".

**Cheat actions go in the catalog.** Owner's call, 2026-08-20, with Wesnoth as
precedent - they are useful for scripting cutscenes. Consequences to accept:
they land in `/create/`, must be documented exactly, and become a contract mods
depend on. `kill_all` should COMPOSE WITH THE EXISTING FILTER SYSTEM rather than
be a blunt verb, so "kill all raiders" and "kill everything in this area" come
free and it matches how every other action already works.

**Step mode, and the agent case does not work without it.** An LLM thinks for
seconds; the game runs 60 ticks a second. Free-running, the world has moved
hundreds of ticks before a reply arrives and the observation describes a game
that no longer exists. So agent mode replaces the keyboard AND THE CLOCK:
advance N ticks -> emit snapshot -> BLOCK until input arrives -> repeat. It also
makes the loop deterministic, and it is cheap in `--norender` because no display
is being starved. `ScheduleRunnerPlugin` drives the headless loop, so that is
where the hook goes.

**Binding: scripted input MUST carry a tick.** Interactive input applies "now".
Anything replayable carries a tick number, or the same script diverges at
different frame rates - the reproducibility failure epic `20260818-220812` spent
a week fighting. Composes with the seeded-`bevy_rand` rule.

**The console is a front-end, not a second transport.** NOVA OS cannot be it:
opening NOVA OS FREEZES the game deliberately, so it can never drive a live one.
Share the dispatcher, not the surface. Console is optional and goes LAST.

## Open

- The line schema.
- Which crate owns the stdin reader.
- Whether step mode is a runner mode or a resource.
- Which cheats are catalog actions and which stay debug-only. The test is
  whether a SCENARIO would genuinely want it: refill-ammo in a tutorial, yes;
  `god`, no.

## Phases, each independently valuable

1. Named input actions + dispatcher. Unblocks rebinding on its own.
2. Snapshot to stdout + step mode. The agent loop's clock.
3. stdin transport carrying `input` and `action`.
4. Console UI + cheat actions.

## Traps

- More runtime strings. `CONVENTIONS.md` Nova rule 5: ids are runtime strings,
  nothing type-checks them, a rename compiles clean and fails at load. Whatever
  ships here MUST be reachable by `content lint` or a probe run, because that is
  the only detection this project has for that failure.
- Do not design it as a network protocol. It is shaped like a multiplayer
  server's input path, and that is a reason to keep the simulation clean, NOT a
  reason to add framing, versioning or auth now. Nothing on the board points at
  multiplayer.
- `SyncWorldPlugin`'s queue leaks under `--norender` (~24 bytes per synced spawn
  and component removal, linear in run length; `tasks/20260819-173219/notes-render-off.md`).
  Fine for probe-length runs, NOT for an indefinite driven session. Step mode
  makes long sessions likely, so this becomes load-bearing.

## Done when

- The settings menu reads keybinds by action name, not from a mirror.
- A range drives the game through the dispatcher instead of a raw `KeyCode`.
- A headless run emits world state and accepts input on a channel, and the same
  input sequence replays to the same result.

## Round 4 findings (2026-08-24) - what the audit changed

Scheduled into v0.12.0. Full audit:
`tasks/20260815-231945/INPUT-AND-PROCESS.md`; step-mode and snapshot detail
in `SCENARIO-PIPELINE.md` section 5.

**Phase 1 is a REGISTRY, not just names.** Naming rig actions does not
delete `reference.rs` by itself: the settings panel renders in the main menu
and no rig exists there (the rig spawns with the player ship). Build a
persistent bindings registry - per action: name, keyboard binds, gamepad
binds - that the flight rig is BUILT FROM. That one structure serves the
settings menu (`20260824-120527` depends on it), rebinding persistence, and
this task's dispatcher. Two more hand-kept mirrors die with it:
`flight_rig_reserved_sources()` (hints.rs:164-195, conflict checking) and
the `key_glyphs` coverage - both must become registry-derived or they go
stale on the first remap.

Phase 1 name scope: the fixed rigs only - flight + targeting (11 actions,
nova_ship/src/input/player/flight_rig.rs:97-260; every action already
carries a display `Name`), camera (3, camera/rig.rs:175-208), scenario
advance (1). Per-section weapon actions are dynamic (derived names later)
and the raw system chords (pause, HUD, NOVA OS, comms) stay fixed rows.

**Dispatcher route (proven path).** Inject where the autopilot already does:
`PreUpdate` after `InputSystems` (nova_autopilot/src/autopilot.rs:390);
`apply(name, phase)` resolves the registry to a live `Binding` and calls the
existing `press_key`/`press_mouse` helpers (input.rs:74-131). Gamepad
synthesis is deliberately absent there; for wheel/motion/gamepad sources,
spike the `bevy_enhanced_input` 0.26 mock API (mock.rs:173-200) - whether it
composes with `Hold`/`Tap` (the radar gesture) is UNVERIFIED. Proof-of-done
for phase 1: port `hollow::hold_radar`/`release_radar`
(examples/screenshots/shared/hollow.rs:509-523) and the 38 hardcoded
press_key/press_mouse call sites across 7 example files.

**Step mode (settled direction).** Replace `ScheduleRunnerPlugin` at
nova_core/src/lib.rs:211 with a custom runner: tick N with
`TimeUpdateStrategy::ManualDuration` (first manual update is dt 0 - warm
up), `capture_snapshot(&mut World)` (confirmed pure, snapshot.rs:364), write
the line, block on the channel.

**The SyncWorldPlugin leak is confirmed and worse than noted**: ~24 bytes
per synced spawn AND per component removal, ~2.4 MB per 100k
(nova_core/src/lib.rs:212-229), and `PendingSyncEntity` is `pub(crate)`
upstream so it CANNOT be drained from outside. Resolve the strategy
(upstream fix or bounded sessions) in this task before advertising
indefinite driven sessions.

## Scope narrowed (2026-08-27)

This task is `input` only: press named inputs by name, read world state back.

The `action` vocabulary moved to `20260827-120347`, to be designed together with
the in-game console rather than bolted onto stdin first. Reason: `action` is a
command layer that stdin happens to reach when there is no window, not a stdin
feature. Building the stdin half first means designing arming, classification
and eligibility twice.

The line schema still parses all three lane keys. `action` and `command` are
reserved and refused with a clear error, so adding them later is additive.

Checked before agreeing: the two-crate split survives the cut. `nova_channel`
still sits above `nova_scenario`, now because `capture_snapshot` reads
`CurrentScenario` and `NovaEventWorld` (`nova_probe/src/capabilities/snapshot.rs:107`)
rather than because the action vocabulary is `EventActionConfig`. The
observation half alone forces it.

Design record: `tasks/20260820-174148/design.html`. Its "Deferred" section is
the reasoning `20260827-120347` starts from.

## Progress

### Step 1 - the registry (`ee2edcaf`)

`nova_input` is a leaf crate holding `ActionBinding`, `BindingSpec` and the
`InputBindings` resource. The 14 shipped defaults are pure data in
`nova_ship/src/input/bindings.rs`; `flight_rig_reserved_sources` derives from
them, so the content lint's overlap check cannot go stale. The 15-row
`nova_ship/src/input/reference.rs` mirror is deleted.

### Step 2 - settings reads the registry (`82c6785e`, `9592ec83`)

The Controls list loops `bindings.groups()`; four rows stay declared fixed, not
five - `camera_rotate` can declare `mouse_motion` + `stick(Right)`, so "Aim"
renders from the registry after all. The list gained RCS Fine Adjust, RCS Aim,
the radar hold/tap split and Advance. Rebinds persist through
`PersistedSettings::keybinds`, and the settings card scrolls so a longer list
cannot push Back off the bottom.

Display strings needed three additions the design did not name: a second
`readout_label`/`key_symbol` labeller (`keyboard_label` stays raw, because it
keys `nova_hud`'s glyph files), `modifier_pair` collapsing Left+Right into
`Ctrl`/`Shift`, and machine-readable `ActionAxes` so `/ Scroll Up` is derived
rather than typed.

### Step 3 - the dispatcher

`nova_input::dispatch` - `apply(world, name, phase)`, `apply_axis(world, name,
delta)` and `primary_source(world, name)`. Source route, not `ActionMock`: the
mock REPLACES conditions and modifiers, so a mocked `radar_hold` proves the
state and never the gesture.

Three source kinds beyond the button: mouse motion and the wheel write
`AccumulatedMouseMotion` / `AccumulatedMouseScroll` (which is what
`bevy_enhanced_input` reads - `input_reader.rs:28`, and its own tests write the
same resources); gamepad is refused by name as `DispatchError::NoButton` rather
than silently doing nothing.

**Where the wrapper went.** The design names no crate for it - its only
mention of the autopilot is the TIMING claim, "it runs where the autopilot
already injects", which still holds. Putting the wrapper in `nova_autopilot`
was a working assumption, and it does not survive that crate's own contract:
bevy and nothing else, with Nova-typed helpers built on it in
`nova_debug::harness` (`nova_autopilot/src/lib.rs:1`). `press_action` needs
`InputBindings`, a `nova_input` type. So `press_action` / `release_action` /
`drive_action` live in `nova_debug::harness`, which every example already globs
through `nova_core::prelude`.

`drive_action` writes the button state alone, not the pointer helpers'
`WindowEvent`: no registry action is a UI click, every one is read through
`bevy_enhanced_input`, and the five per-frame `hold_inputs` ranges would
otherwise feed `bevy_picking` a synthetic press every frame.

Ported: 26 call sites across 10 example files, including all three separate
copies of `hold_radar` (`hollow.rs`, `screenshot_radar_lock.rs`,
`system_hud_indicators.rs`) and both copies of `raise_stance`.

**Axis timing, for step 5.** A button press survives across frames; an axis
write does not. Bevy REPLACES the accumulators every frame, so a caller driving
an axis must run `after(InputSystems).before(EnhancedInputSystems::Prepare)`.
The autopilot driver only guarantees the first half, which is why no example
drives an axis by name yet and why `nova_channel`'s system must state both.

**Proof.** `nova_ship` 683/683, `nova_input` 16/16, `nova_debug` 16/16;
`cargo check --workspace --all-targets` and `--features debug --examples` clean.
New test `a_named_radar_hold_is_a_real_hold_not_a_mocked_state` drives the real
rig by name and asserts the tap window, the threshold latch and the sticking
release - the three things a mock would have skipped.

Live on Xvfb :97, all exit 0: `system_player_path` (stance, two radar sweeps,
GOTO - full chain), `system_hud_indicators`, `system_turret_gunnery` (rounds
fired through the per-frame stance hold), `screenshot_radar_lock`,
`screenshot_combat_lock`, `loop_player_flight`.

**Pre-existing failure, not caused by this change.** `carve_asteroids` stalls at
`hold actual PDC fire on one point` (`pdc_rounds_landed()` never satisfied,
15 s deadline). Reproduced with the hunk reverted to the raw `ButtonInput`
press: identical stall. Untouched here.

### Step 3.5 - the whole player vocabulary (`3b273b85`, `15430962`, `4e0cf075`, `ca3340ef`)

Owner's widening, 2026-08-27: every key a player can press is a registry action,
so a rebind reaches all of them and stdin can name all of them. 15 actions ->
33.

**Retired, not ported.** The comms stack's V and B are deleted. Dismiss-oldest
and skip-backlog were near-duplicates of each other, and the paced window
already drains on its own. The panel no longer reads the keyboard at all. Its
dismiss test was replaced by one that proves the real behaviour: a burst larger
than `COMMS_VISIBLE_CAP` shows the cap's worth and lets the backlog through as
cards expire.

**Promoted.** `novaos_toggle` (Tab / right-stick click), `hud_cinematic`
(backquote / Select), and the 16 NOVA OS viewer actions - orbit, pan, reframe,
next/prev, GOTO, mates, reload, repair, rebind. The viewer actions carry no
gamepad source: they never had one, and inventing pad buttons would reserve
them against the flight rig.

**Fixed by owner's call.** Escape stays hardcoded. It is the universal back-out
at every rung, and a rebind could strand a player inside a mode. `FIXED_ROWS` in
the settings screen is down to one row.

**Why a polling seam and not a rig.** A `bevy_enhanced_input` rig spawns with
the player ship; NOVA OS must open with no ship on the field. So
`nova_input::poll::InputSources` resolves an action name against the live table
from an ordinary system. It splits `just_pressed_desk` from `just_pressed_pad`
because `toggle_nova_os` needs them apart - Tab opens and Escape closes, while
the pad button must do both.

**The footer would have lied after a rebind.** `NovaOsAppRuntime::hints`
returned fixed strings. It now takes `&InputBindings`, each surface builds its
row from the live table, and `rebuild_nova_os_footer_hints` gained
`bindings.is_changed()` as a third trigger so it repaints on a rebind and not
only on a mode change.

**Two shipped defects found and fixed.**

The gamepad shortcuts outside the flight rig were dead in v0.11.0. bevy 0.19 has
no `ButtonInput<GamepadButton>` resource - digital state is a private field on
the `Gamepad` COMPONENT (`bevy_input-0.19.1/src/gamepad.rs:383`, accessors at
524/529). Every `Option<Res<ButtonInput<GamepadButton>>>` reader was permanently
`None` in a real run; the tests passed only because they inserted the resource
themselves. Pause on Start, HUD on Select and NOVA OS on the right-stick click
all did nothing. `InputSources` reads the component, so it cannot recur.

`rcs_modifier` and `combat_stance` both held gamepad Left Trigger 2, both rigs
with `consume_input: false`, so one pull did both. Combat keeps the trigger - it
is the aim position and it pairs with fire on Right Trigger 2, which the shipped
turrets hold. The modifier takes Left Thumb, the only free standard button.
`no_two_fixed_rig_actions_share_a_gamepad_button` walks both lists together;
the radar hold/clear pair is exempt by name.

**Proof.** `nova_os_ui` 111, `nova_hud` 218, `nova_menu` 82, `nova_os` 24,
`nova_input` 21, `nova_ship` bindings 5/5. `cargo check --workspace
--all-targets` clean. Live on Xvfb :90: `screenshot_nova_os_apps` drove both
viewers through the real keyboard path, no panic.

**Design doc.** `design.html` gained `#modekeys`, "And so did the computer's own
keys", `#rawlanes` (the four stdin lanes) and the shipped section-firing table;
the action table is 33 rows. Republished to the artifact.

### Step 3.6 - when an action can fire (`07a306ba`)

Owner's question: can the channel allow only the valid actions, or show only
them? It can, and it is not cosmetic. Every action resolves to a KEY, and keys
are reused across surfaces on purpose - `G` is `autopilot_goto` in flight,
`map_goto` in the map viewer and `ship_mates` in the ship viewer; `W`, `[` and
`]` are each held twice. So a driver handed the whole table is told one key
means three things, and pressing an action that is not live does not fail: it
presses the key, and whatever IS live reads it as its own action.

`ActionContext` is a third axis beside `name` and `group`, on the action
itself. `Always` (2), `Flight` (15), `Viewer` (11), `ViewerApp(id)` (5).
`Viewer` is down at the prompt on purpose - there the keyboard is typing, and
`W` is a character - and it is the only nesting, so a named app runs inside the
shared viewer set.

`nova_input` is a leaf and cannot see `PauseStates` or the terminal mode, so
the subsystem that owns a context raises it. `nova_ship` raises `Flight` while
a player ship is on the field and no frozen variant holds the clocks;
`nova_os_ui` raises `Viewer` and the one `ViewerApp` that owns the screen. The
named apps are read off `InputBindings::contexts()` rather than listed in the
sync system, so a new NOVA OS app declares its context beside its actions and
nothing else moves.

`InputBindings::live()` is what a snapshot advertises and what a channel checks;
`conflicts()` pairs only actions whose contexts overlap. That generalises the
gamepad guard added with the LT2 fix - it covers the keyboard half too, and
`nova_menu` runs it over all 33 actions, being the one crate that sees every
owner's list.

**Where the seam is, per the owner:** `nova_input` defines the vocabulary and
answers which of it is live. Advertising the set and refusing a name outside it
is `nova_channel`'s job.

**Proof.** `nova_input` 27, `nova_ship` 685, `nova_os_ui` 112, `nova_menu` 84,
`nova_hud` 218, `nova_scenario` 236. `cargo check --workspace --all-targets`
clean. New behaviour tests: the flight context follows the ship and the freeze,
the viewer contexts follow the app that owns the monitor, and exactly one of
the three `G` actions is live at any instant.

**Design record.** `tasks/20260820-174148/nova-input.html`, published as
<https://claude.ai/code/artifact/7838fafd-f82a-4c51-a7fc-308449c03b07>. It is
the crate's own page: what shipped, the action table with contexts, what is
deliberately outside the table, and the queue - a capture UI (the Controls list
is still read-only), a rebind conflict guard, gamepad synthesis, whether
section weapons join the table, and the last hand-written half of the keycap
glyph coverage.

### Round 5 review, and the fixes it produced

Fixed in `9d224349`, `1a79bf21`, `819d5f06`, `489e68ab` and `7df3e75d`.

Six lanes over `e0a5092a..155afa73` with `--play`: 3 blockers, 22 majors, 30
minors, and nothing in the architecture. Every blocker and every major is
fixed, with ten of the minors. Disposition per finding is `REVIEW.md`; the
reasoning is `findings.html`, published as
<https://claude.ai/code/artifact/4c67ebc2-8ffe-4bff-b174-6834e0a20f81>.

**Three shapes accounted for most of it.** The capture had no owner - Escape
answered the capture AND the pause overlay, the refusal drew below the fold,
and three surfaces each picked an arbitrary key out of a `HashSet`. The table
could go inconsistent - the guard could not see live section bindings and the
load ran no conflict check at all. And a rebind did not reach everything - the
scenario rig, the HUD dock and a verb on a mouse button all kept the key they
shipped with.

**Two rules the table now keeps for itself.** `apply_overrides` applies every
stored row and THEN reconciles the whole table, so two rows trading keys load
and a stored row that a moved default landed on is put back. And `rebind`
refuses a spec the rebind screen could not have produced: a source in the wrong
device column, or a column the action ships empty.

**A release lets up what its press pushed.** `dispatch::apply` records the
source per name (`DrivenPresses`). Resolving the name twice let a rebind
between the halves strand the pressed source down for the rest of the run - on
the pad half with analog still at 1.0. `apply_stick` lands beside it, so the
declared stick is drivable rather than display-only.

**Deferred, by the owner.** The RCS stick's missing `DeadZone` and the Left
Thumb pairing are untouched: the controller defaults are being reconsidered as
a set after a playtest. Taking Left Trigger 2 off `rcs_modifier` put it on Left
Thumb, which the editor's Sandbox-return chord also reads - this range created
that collision, and it belongs to the same question. The fixed 560 px panel
height is a standing instruction, so the clipped FLIGHT row is a layout call.

**One regression made and caught.** Sizing every keycap by its short axis, the
fix for starved portrait art, grew all 26 letter caps two pixels - they measure
0.92. `keybind_dock`'s sizing tests caught it; the rule now starts below the
keycaps and `only_portrait_art_is_sized_by_its_width` pins it.

### Step 4 investigation - `nova_channel` at full GUI parity (branch `channel-design`, NOT landed)

Owner's bar for the crate that does not exist yet: everything the GUI player
can do - mouse moves, button clicks, typing into NOVA OS - must work over
stdin against `--norender`. Investigated against the tree; nothing shipped.
On the `channel-design` sprout by request, deliberately unlanded.

**Design record.** `tasks/20260820-174148/nova-channel.html`, published as
<https://claude.ai/code/artifact/4e65b957-8cd0-4eb5-90fd-570b5712533f>: the
parity ledger (16 player capabilities, each with its mechanism and status),
the wire schema for the raw lanes, the crate's system slots, and the spike
list in build order.

**The finding: one virtual window buys the whole pointer.** Headless input
dies in three independent places - every `nova_autopilot` gesture writer
resolves `PrimaryWindow` and no-ops (`input.rs:102-108, 401-409, 527-534`),
no camera ever computes `target_info` so UI layout collapses to 0x0
(`bevy_ui update.rs:137-153`), and `ui_picking` matches no camera
(`picking_backend.rs:126-131`). All three are the same missing ENTITY:
`get_render_target_info` reads the `Window` component, not the GPU
(`bevy_render camera.rs:275-284`), so spawning one ordinary
`Window`+`PrimaryWindow` with a fixed resolution revives layout, picking and
every existing gesture helper unchanged. `pointer_rig.rs:240-246` and
`tests/pointer_pin.rs` already use the idiom in miniature. This SUPERSEDES
the design page's "make the window optional on the helpers" note. Helping
fact: `--norender` is full `DefaultPlugins` minus only `WinitPlugin`, so the
whole picking stack is already present and idle; and the workspace has ZERO
legacy `Interaction` polling - every clickable surface is `bevy_picking`
observers, so one synthesized pointer serves all of them.

**The gate: headless has no NOVA OS.** `NovaOsUiPlugin` and `NovaHudPlugin`
are render-gated (`nova_core/src/lib.rs:300-303`) and their bindings register
with them, so the headless table holds 15 of 33 actions and `novaos_toggle`
is `Unknown` in exactly the channel's mode. Call: both plugins take the
`{ render: bool }` shape every other game plugin has. Registering bindings
without systems was rejected - a table advertising dead verbs is the lie the
registry exists to end.

**Corrections found.** (1) Determinism is not free: the shipping app seeds
from OS entropy (`EntropyPlugin::<WyRand>::default()`,
`nova_gameplay/src/plugin.rs:74`); every `with_seed` in the tree is a test
rig. Replay needs a seed knob. (2) Synthesized `KeyboardInput` needs a
`Released` twin or `keyboard_input_system` leaves phantom keys pressed
forever - latent in `type_text` today. (3) The prompt reads `event.text`
while `TextField` reads `Key::Character`, so the text lane writes both; the
key lane writes message AND `ButtonInput`, because the mode chords and the
rebind capture poll. (4) Any new `NOVA_*` env var must be registered in
`tests/env_contract.rs` and `docs/environment-variables.md` or CI fails.

**Wire, settled.** Five lanes in - `input` (+`phase`), `aim`, `text`, `key`,
`pointer`, plus a bare `tick` as the step instruction; `action`/`command`
parsed and refused naming 20260827-120347. Out: the snapshot grows `applied`
(per-line acks carrying `TriggerState` or `refused`), `input.live`/`contexts`,
and a `ui` block (pause rung, terminal model, pointer census of visible
`Name`d rects) - all additive, no schema bump (`snapshot.rs:127-130`).

**PoCs.** `tasks/20260820-174148/poc/`: `mock_game.py` implements the wire
over a toy world; `channel.py` is the driver client; `drive_flight.py`,
`drive_novaos.py`, `drive_pointer.py`, `agent_loop.py` all PASS over real
pipes - the radar tap/hold distinction, the context refusal on the map's `G`,
a release-over click on a census-advertised `Resume`, and an agent loop with
the clock gated on the driver. Rerunning them against the real binary via
`Channel(cmd=[...])` is the crate's acceptance test.

**Spikes, in build order.** (1) boot the full headless app with the virtual
window, assert `click_named("Resume")` fires the real observer; (2) NOVA OS
with `render: false` - the RTT/CRT surfaces with no GPU are the risk;
(3) the seed knob plus a byte-for-byte replay test; (4) the crate: reader
thread, two writer systems (`First` before `PickingSystems::Input` for the
pointer, `PreUpdate` after `InputSystems` before
`EnhancedInputSystems::Prepare` for buttons and axes), the runner via
`app.set_runner()` after `AppBuilder::build()`, `capture_snapshot` moved out
of `nova_probe`. Pointer synthesis home needs an owner call: depend on
`nova_autopilot` (bevy-only, env-inert, but starts shipping in release) or
move `input.rs` to a shared home; reuse is the recommendation.

### Step 4.1 spikes - the three risks, run against the real app (branch `channel-design`, NOT landed)

The owner's framing question for this round: does the channel have to
simulate the GUI, or can it use the in-memory backends the game already
keeps? Answer, now recorded in the design page's "The stance" section: the
backends exist (`NovaOsTerminal` is a resource, the pause menu is taffy
rects, the CRT is a projection), and the wire speaks at both levels - the
named-input lane is the multiplayer-packet level (dispatcher + context
validation, no keyboard), the pointer/text lanes are the human level where
bypass is refused on purpose (Playwright-style: resolve the `Name` to a
laid-out visible rect, then real events, so unreachable UI FAILS the run).

Spikes 1-3 from the build order now exist as ranges and all PASS headless -
no display, no GPU (`examples/systems/system_headless_{pointer,novaos,replay}.rs`):

1. **The virtual window holds.** `system_headless_pointer` boots
   `--norender --scenario shakedown_run`, spawns the `Window`+`PrimaryWindow`
   entity, ESC to the pause overlay, census (`Pause Overlay 1280x720`,
   `Resume Button 238x40 at (640,279)` - real taffy), clicks Resume through
   `bevy_picking` -> `On<Activate>`, game resumes. Exit 0.
2. **The render gate was never load-bearing.** Spike 2 REMOVES the gate on
   `NovaHudPlugin`/`NovaOsUiPlugin` outright (no `{render: bool}` flag -
   every GPU piece is bevy-guarded; `UiMaterialPlugin` no-ops without a
   render sub-app, `ui_material_pipeline.rs:55`). Headless registry now
   holds 33/33 actions; Tab opens the monitor, `type_text` lands `map` in
   the `NovaOsTerminal` resource, Enter launches the map app.
   OPEN owner call: headless measurement runs now carry HUD/monitor CPU
   systems - if probe noise matters, arm the plugins from the channel
   instead of unconditionally.
3. **Seed + pinned clock replay byte for byte.** `NOVA_SEED` added to
   `nova_gameplay::settings` (env contract + dev book rows; non-u64 refuses
   boot), `EntropyPlugin::with_seed` when set. With
   `TimeUpdateStrategy::ManualDuration(1/64 s)`, two seed-42 runs digest the
   probe snapshot identically (`bed90101044a6e0e`) and draw the same
   entropy sample; seed 43 draws differently (the passive world is
   RNG-free - the belt scatter uses scenario-authored seeds, entropy's
   consumers are combat-time). FINDING on the way: the first digest attempt
   diverged by exactly one 1/64 s tick of `elapsed` and NOTHING else - an
   outside observer counting from "I saw Playing" races the scenario start
   by one frame. Anchor on the world's own clock; the channel's step clock
   has no such race by construction.

Warts for the crate's log filter: `bevy_egui` warns every frame that the
virtual window has no winit backing; `bevy_gltf` notes missing
`CompressedImageFormatSupport` once at boot.

Still open: spike 4 (the crate itself; PoC drivers rerun via `--cmd` as
acceptance), the CRT blip click and slider drag (traced, not yet driven),
the gate owner call above, and the pointer-synthesis home call
(depend on `nova_autopilot` vs move `input.rs`).

### Step 4.2 spikes - the remaining ledger rows, run against the real app (branch channel-design, NOT landed)

Spikes 4-6 close the last three interaction rows headless
(`examples/systems/system_headless_{rebind,drag,crt}.rs`, all exit 0,
`--norender`, no display):

4. **The rebind by wire.** Full Settings walk by `Name` through the
   reconciled body (ESC -> Settings -> Controls tab -> FLIGHT group -> arm
   the `main_drive` Desk chip -> J), `NOVA_CONFIG_ROOT` pointed at a scratch
   dir BEFORE the app builds so the exit flush cannot touch the real
   settings.ron. The registry takes the override: keyboard column becomes
   `[Keyboard(KeyJ)]`, `overrides()` carries `main_drive`.
   FINDING (the round's best): the scenario loading screen fades out OVER
   the fresh pause overlay and eats every pick for ~1 s. A resolvable rect
   is NOT yet clickable - the first version pressed on a frame count and
   clicked the fade. The pointer lane's rule is now concrete: press only
   after the pick map reports the aim landed on the named widget, re-aim
   every frame while blocked, and a blocked aim NAMES its occluder
   (`the pointer is over ["Scenario Loading Screen"]`).
5. **The slider drag.** Volume track: press snaps (`TrackClick::Snap`) to
   0.5 at centre, two cursor legs +60 px along the 428 px track raise
   `MasterVolume` to 0.640 (predicted +0.14), release commits; the widget's
   `SliderValue` and the resource agree. The range also prints the proposed
   snapshot `ui` block with the modal open: 50 named nodes, logical rects,
   button flag, mode rungs, terminal model.
6. **The blip through the glass, no GPU anywhere.** The feared fallback
   never happens: `UiMaterialPlugin` registers the CRT material asset
   BEFORE its render-app check, so `--norender` takes the RTT arm and the
   whole chain is CPU (reconciler sizes the image, offscreen camera reads
   it, map lays out against it, forwarder un-warps window px -> image px).
   The walk mixes lanes: packet lane `novaos_next`/`novaos_reframe` (keys
   resolved from the registry) puts the ring on AST-1, frames it, cycles
   the ring OFF; human lane clicks through the warp inverse. Verdicts: the
   FORWARDED pointer hovers the blip and the window mouse does not, the
   click restores the ring, G engages `Autopilot::Goto` on exactly that
   contact. FINDINGS: the shakedown belt sits outside even max wheel zoom
   (scenario truth no trace surfaced; select+reframe is the wire's answer),
   and a driver that advances on a world change can skip its own key
   RELEASE - a held key never counts as just_pressed again, so the key
   lane must pair every press with its release.

Still open after this round: the crate itself (spike "the crate" with
`--cmd` acceptance), the `ui` block's real home in `capture_snapshot`
(census is driver-side today), the render-gate owner call, and the
pointer-synthesis home call.

## The crate round (2026-08-28, commit f5708bd1)

`crates/nova_channel` is real: protocol parser (mock refusal texts as unit
tests), five lanes behind two writer systems in the pinned slots (`First`
after MessageUpdateSystems / before PickingSystems::Input for the pointer,
`PreUpdate` after InputSystems / before EnhancedInputSystems::Prepare for
the rest), stdin reader thread, step/free runners with a boot gate (hold
the wire through `GameStates::Loading` + 2 settle frames; tick 0 = world
ready). Armed by `--channel step|free` (debug-only, requires `--norender`),
installed after the builder so its runner wins. `InputBindings::bundle` now
tags every action entity `ActionName`, which is what the ack reads its
`TriggerState` through after the frame. The `ui` census moved into
`nova_probe::capture_snapshot`; the header grew `t_game` (virtual seconds,
freezes with pause - `elapsed` is the scenario event clock and stalls
through the shakedown opening on its own cadence).

Acceptance: `drive_pointer.py` and `drive_novaos.py` PASS against the real
binary (`--cmd ".../nova-protocol --norender --scenario shakedown_run
--channel step"`, NOVA_CONFIG_ROOT isolated) and the mock alike. The real
world corrected the mock in five places, folded back into mock+drivers:
Resume Button (real Name), `nova_os.*` (group lowercases with underscore),
the 0.22 s CRT close drawer (driver steps through the slide), two real
frames of `t_game` between press and pause engaging, and press/release =
two acks (read the `start` ack, not the latest). Smoke: 120-tick
`flight.main_drive` hold took the real ship 0 -> 15 m/s on -Z, ack
Fired -> None. Tests: 6 + 36 + 80 pass on the touched crates.

Still open: the flight acceptance pair (drive_flight/agent_loop encode
mock facts - raider_1, 1-D kinematics, vel > 50; needs a purpose-built
loose --scenario-file and 3-D checks - owner call on scope), the
render-gate owner call (kept un-gated; revisit only if probe noise
matters), free mode's `late` flag (field rides every ack, runner never
sets it; step mode refuses past ticks so nothing lies), and retiring the
mock into the schema reference once the flight pair passes.

## The acceptance round (2026-08-28, commit 46530b9a)

All four drivers now PASS against the real binary. The flight pair got
`poc/acceptance.content.ron` - a loose --scenario-file, never installed:
one armed corvette (`player`), one hostile `raider_1` at 280 u down the
burn line under `engage_delay: 600`, each turret on its own free key
(U / I) so `section.<id>` presses one mount only. drive_flight and
agent_loop rewrote their checks in 3-D and refuse to run without --cmd.

Two real gesture rules the mock never knew, found while going green: the
radar sweep commits a COMBAT lock only while weapons are raised
(stance-down it banks a travel lock), and the latch is threshold PLUS an
acquisition dwell (lock_dwell_base 0.6 s, range-scaled) charged on the
candidate under the boresight - so stance moved before the sweep, the aim
beat after combat, and the hold grew to 90 ticks. The agent's kill is
real: raider `defeated` + `neutralized`, hull 907 -> 227, player
untouched, turret transcript on_target with 455 rounds spent (damage
lands in a late burst after bullet flight, past the loop's last print).

Also closed: free mode's `late` flag (passed-tick lines land in
`ChannelFrame::late_lines`; the ack funnel stamps them; 2 unit tests at
the funnel, 8 crate tests total), and the poc README retires mock_game.py
into the wire's executable schema reference (pointer/novaos still run
against it bare; the flight pair refuses without --cmd).

Still open: only the render-gate owner call (kept un-gated; revisit if
probe noise matters) and the console vocabulary, which is task
20260827-120347's scope. The branch stays unlanded until asked.

## The recording round (2026-08-28, commits 0a215e3f + abec9467)

The branch landed first (983d076c squash, 326fa773 changelog entry,
afdd6cf5 client hardening). Then the round the hand-test asked for:

`--record <dir>` (debug, requires --channel) films a run without a
window. A third AppBuilder assembly, `offscreen()`, keeps the headless
shape - no winit, no OS window, channel-spawned virtual PrimaryWindow -
but arms the GPU and the full visual plugin stack (pipelined rendering
disabled: a stepped driver wants the frame drawn when update returns).
nova_channel's recorder retargets every primary-window camera into one
window-sized image, spawns one Screenshot of it per tick with its own
numbered save_to_disk observer (completion order cannot scramble frame
order), and flushes in-flight captures after EOF. channel.py stitches
`<dir>.mp4` with ffmpeg on close; one tick is 1/60 s, so the movie is
real time however slowly the driver stepped.

Two integration truths the first recorded runs surfaced, both fixed in
the recorder rather than the game:

- bevy_ui routes roots to the highest-order camera TARGETING THE PRIMARY
  WINDOW when no IsDefaultUiCamera exists - retargeting alone left the
  HUD out of the frames and killed every UI hit-test. The recorder marks
  the camera that fallback would have picked and stands down whenever the
  game marks its own (menu ambience, render-scale blit).
- PointerInput::receive (PreUpdate, ProcessInput) re-applies the frame's
  window-targeted messages onto PointerLocation, so the pointer retarget
  must sit between ProcessInput and Backend; in First it is clobbered on
  exactly the frames a gesture arrives.

The egui half ran as its own lane (opus agent, sprout egui-headless,
landed abec9467): the inspector's egui stack now assembles only when
`WinitPlugin` is present, which silences the per-frame "Cannot access an
underlying winit window" WARN in both windowless shapes - 230
occurrences to zero on a headless boot, and the offscreen assembly is
covered by the same predicate for free.

Evidence: drive_pointer and drive_flight PASS recorded end to end; 415
gapless PNGs for the 414-tick flight (the census under recording now
carries the full HUD suite); frames show the burn, the hostile target
inspector, the tracer stream and the NEUTRALIZED overlay; rec4.mp4 is
6.9 s of real-time movie. Tests: 10 nova_core + 8 nova_channel.

Decided and deferred: the "playability runner" idea (one command that
takes a driver script and a scenario, runs the game, emits the video) is
NOT nova_probe - probe asserts expected results, this produces evidence
of play - and it is not built now either. It belongs to next sprint's
agent-play task; until then channel.py's close-time ffmpeg is the whole
pipeline. Agent tooling stays out of scope for this task.
