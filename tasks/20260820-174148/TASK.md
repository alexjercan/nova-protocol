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
