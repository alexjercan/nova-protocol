# D3 resource and docking legibility: choices for the owner

Source audit only. Working tree (dirty with other lanes). No edit, no git write, no cargo, no game run.
Everything is source-derived. "UNVERIFIED" marks what no source read settles.
Paths are repo-relative. Line numbers are the working tree on 2026-11-06.

## Facts that shape every choice

- F1. Every shipped crewed hull carries a docking port. Test `every_crewed_ship_passes_the_post_build_checks_and_docks`
  (`crates/nova_authoring/src/base_content/ships/mod.rs:308-345`) skips only the derelict and the debris plate.
  So in New Game `NO DOCK PORT` fires only after the port is destroyed or refit away. It is rare for a first session.
  Refit can leave a hull with no port: Jobs has a refusal for it (`crates/nova_interface/src/jobs.rs:431`).
- F2. The tutorial trainer HAS a port. `training_picket` mounts `flank_collar(BLOCK_PORT_COLLAR_SECTION_ID ...)`
  (`crates/nova_authoring/src/base_content/ships/block.rs:589-594`).
  The comments at `scenarios/tutorial/range.rs:157-158` and `scenarios/drills.rs:222-224` say "no docking port". They look stale. Not run.
- F3. `dock_enabled` is false only on a parked hull (`crates/nova_scenario/src/objects/spaceship.rs:754-764`).
  No `SetShipCapability*` action exists for dock (`crates/nova_scenario/src/actions/mod.rs:341-381`).
  Drills and the range keep it true (`drills.rs:233`, `range.rs:167`). The player root keeps its own caps on Board (`spaceship.rs:768,788`).
  So the player has `dock_enabled` true in practice.
- F4. `SectionInactiveMarker` marks an editor-preview section, not a damaged one (`crates/nova_gameplay/src/markers.rs:233-237`).
  A free live port is a docking section, a child of the root, with no `DockedPort` and no inactive marker (`port.rs:132-145`, `has_free_port` at `port.rs:200`).
- F5. A dry ammo pull cannot happen with the safety on. `firing` counts only with `weapons_hot` (`crates/nova_hud/src/situation.rs:96-103`).
  The gauge shows on `weapons_hot || combat_lock` (`situation.rs:66-68`). So the gauge is already up, amber at 25 percent or less, when the pilot pulls
  (`ammo_readout.rs:716,746-751`, per audit section 2). Ammo has a gauge, not a word. No code change is needed for the "ammo empty" gap to be readable.
  A word is a separate owner decision. Not offered here as a choice.
- F6. RCS: no New Game hull lacks RCS (`crates/nova_scenario/src/actions/spawn.rs:333`, `capability.rs:82-93`). "Withheld" exists only in Basic Training
  (`range.rs:162`). Empty RCS draws ten dim pips whenever `spent > 0` (`flight_status.rs:553-557`). D4 stands: no display for withheld.
- F7. The `D` key is `dock` with a keyboard bind only (`crates/nova_ship/src/input/bindings.rs:62`). There is no pad bind (`bindings.rs:56-66`, D5).
  Any `D` teaching reaches keyboard pilots only. The dock chip keycap follows the live table and falls back to the key (`hints.rs:171-183`).

## Can a `D` keycap reuse the objective cue chip? No, not without a new API.

- The keycap is one per stack. `sync_objective_chips` reads the hard-coded action `"interface_toggle"` (`crates/nova_hud/src/objective_stack.rs:541-546`).
  `rebuild_objective_stack_children` spawns one `tab_footer` for the whole stack (`objective_stack.rs:512-535`).
  The footer word is `INTERFACE` while any shown chip is held, else `OBJECTIVES` (`objective_stack.rs:476-487`).
- A second objective `Undock` in `open_world` would stack above `Take a job`, under a Tab keycap labelled `INTERFACE`. It cannot show a `D` cap.
- Putting the letter in the objective text ("Undock: press D") needs no API. It hard-codes the key, breaks on rebind, and is keyboard only (F7).
- `ObjectiveChipHold` on it is sticky. D2 forbids that for other objectives. Without a hold it fades after 12 s or the first Tab (`objective_stack.rs:59,335-366`).
- It also needs a new `OnUndocked` handler. That handler must go LAST (index 4), by the rule at `open_world.rs` (comment above the `OnJobAccepted` handler):
  a save names spent `once` handlers by index.
- What does reuse existing parts with a live keycap: the DOCK chip itself, plus `HintEmphasisSet`/`HintEmphasisClear` (see choice 1b). Nothing new in the API.

## Choice 1 (cheapest). The DOCK chip reads `UNDOCK` while docked. Optional 1b adds a gold pulse.

On-screen text: chip word `UNDOCK` (key cap `D` beside it). Normal word stays `DOCK`.

Exact Rust:
- `chip_word(hints: &FlightVerbHints, index: usize) -> &'static str` at `crates/nova_hud/src/keybind_dock.rs:693`.
  Call site `let word = chip_word(&hints, **chip);` at `keybind_dock.rs:676`, inside `update_dock` (`:595`).
- Add a `docked: bool` argument. Pass `situations.docked` (`HudSituations`, `situation.rs:54`, set from `Has<DockedShip>` at `situation.rs:94-108`).
  Match arm: `("DOCK", _) if docked => "UNDOCK"`.
- `update_dock` already runs when `situations.is_changed()` (`keybind_dock.rs:632-640`), so no gate change.
- Pulse, by name `DOCK_VERBS[6]` (`keybind_dock.rs:89-91`), is not touched.

Posted by: the existing chip text write at `keybind_dock.rs:676-686`. No new path.

New names: none. One new parameter on a private fn.

Option 1b (stackable, no new API): `open_world` `fresh_start` (not `start`) gets `show_hint_emphasis("DOCK")`
(`crates/nova_authoring/src/scenario_helpers.rs:157`, prelude `:15-19`).
Append a LAST handler: `OnUndocked`, `once: true`, filter `Entity(other_id: Some("player"))` (`web/src/create/events.md:746-790`), action `clear_hint_emphasis("DOCK")`.
Do NOT set it in `OnResume`: a Load after the undock would pulse for ever, because the `once` handler is spent.
Emphasis is cleared on teardown (`keybind_dock.rs:151-153`).
Risk of 1b: the pulse runs until the first undock. A pilot reading Jobs docked sees it a long time. That is a nag. Skip 1b if D2's spirit (no sticky) matters.

False-positive check:
- Docked ship: correct. `on_dock_input` undocks first, before the capability gate (`flight_rig.rs:739-769`). It undocks even a hull that does not fly the pair.
- Docked pair without the helm: still correct. `D` releases (`flight_rig.rs:754-757`).
- Dead station (looting dock): correct, same release.
- Parked hull, scripted drill, tutorial beats: no `DockedShip` on the player, so the word stays `DOCK`.
- Inactive section: not involved.
- Layout: `UNDOCK` is 6 letters, `DOCK` 4. The row is centred, so the other chips shift when the dock state changes.
  hud.md:65 promises "no chip moves". The same break already exists for STOP to MATCH in the other lane's code. Note it.

Wiki lines it touches:
- `web/src/wiki/hud.md:65` ("plus the verb word" and the STOP/MATCH clause; add the DOCK/UNDOCK clause and "no chip moves").
- `web/src/wiki/sections/docking.md:22` stays true; one clause could name the chip.
- `web/src/wiki/getting-started.md`: has no word "undock" for New Game. One line needed.

Shared files, read-only `git status`:
- `crates/nova_hud/src/keybind_dock.rs` is DIRTY (+143 lines). `chip_word` itself is the other lane's uncommitted addition
  (STOP to MATCH; `git diff` adds `fn chip_word` and its tests at `keybind_dock.rs:1105-1176`). Risk: HIGH if edited before that lane lands. Land after it.
- `open_world.rs` is DIRTY (this task's D1/D2 lane). 1b touches it. Same owner.
- `hud.md`, `docking.md`, `getting-started.md` are DIRTY.

Player fact added: `D` is the key that leaves the dock, read off the chip they already see inverted.
Gap left: nothing for no free port, geometry, no lock, RCS empty, ammo empty. Keyboard only (F7).

## Choice 2. `NO DOCK PORT` (D3), corrected

On-screen text: `NO DOCK PORT` (one slot, 4 s of game time, once on the edge; `web/src/wiki/hud.md:66`).

Proposed D3 condition: "travel lock is a dock target and the hull has no free live port". Verified against source and CORRECTED:
- "dock target" is not the same as "travel lock is set". `admission.refusal` returns `None` for a non-ship-root target
  (`admission.rs`, `judge`: `self.ships.get(target)` fails, query is `With<SpaceshipRootMarker>`). An asteroid or planet lock has `None`.
  A bare "refusal is None" test would post `NO DOCK PORT` on every asteroid lock. A root check is needed.
- Order matters. `admission.refusal` must stay first, so `Combat` stays silent (rule at `hints.rs:117-119`, arm at `:122`).
  A target that refuses must not be replaced by a hull fact it did not cause.
- `docked.is_none()` already holds in the existing `match` (`hints.rs:259-260`).
- Add `capabilities.dock_enabled` (the chip's own gate, `hints.rs:311-318`). Add `hardware` as a guard against the
  assemble and rehull gap (UNVERIFIED: whether a lock survives a rehull or thaw with sections not yet spawned; if it does, a 4 s false line shows).

Exact Rust, `update_flight_verb_hints` at `crates/nova_ship/src/input/player/hints.rs`:
- Replace the `let dock_refusal = match (ship, travel) { ... }` block at `:259-267`.
- Current arm: `admission.refusal(ship, target).and_then(|refusal| { name; dock_refusal_line(name, refusal) })`.
- New arm shape:
  ```
  match admission.refusal(ship, target) {
      Some(refusal) => dock_refusal_line(name, refusal),
      None if capabilities.dock_enabled && hardware
          && q_roots.contains(target) && !ports.has_free_port(ship) =>
          Some("NO DOCK PORT".to_string()),
      None => None,
  }
  ```
- `has_free_port` is `pub(crate)` (`port.rs:200`), same crate as `hints.rs`. `ports: DockingPorts` is already a parameter (`hints.rs:158`).
  Its only caller today is `crates/nova_ship/src/flight/autopilot.rs:896`, which disengages with `DisengageType::NoDockingPort`.
- Update the doc on `FlightVerbHints.dock_refusal` (`hints.rs:100-110`): it names only station lines now.

Posted by: the existing `dock_refusal` field and `post_dock_refusal_notice` (`crates/nova_hud/src/hud_notice.rs:165-177`). Edge-latched on string change.
Two consecutive stations both read `NO DOCK PORT`: the second does not repost. Correct, the fact is about the hull.

New names: one private query `q_roots: Query<(), With<SpaceshipRootMarker>>` as a system parameter. No new type, no new field.

False-positive check:
- Docked ship: excluded (`docked.is_none()`).
- Parked hull: the player does not fly it. `dock_enabled` false also excludes it.
- Scripted drill, tutorial beat: trainer has a port (F2), so no line. If F2 is wrong the range would show it on every lock. Verify by one tutorial run.
- Dead station: shown only if the player's own port is gone. True, a loot dock needs a port.
- Inactive section: n/a (F4).
- Lock on asteroid or planet: silent (root check).
- Target ship that has no port: silent here. The pilot gets a dark chip. Gap.
- Combat ship: silent by design.
- Rehull or thaw transient: UNVERIFIED, guarded by `hardware`.

Wiki lines it invalidates:
- `web/src/wiki/sections/docking.md:66` ("Only two refusals post a line" and "a hull of yours with no free live port ... says nothing").
- `web/src/wiki/hud.md:66` (notice list).
- `web/src/create/stations.md:124` stays true (it speaks of a station's refusal).

Shared files:
- `crates/nova_ship/src/input/player/hints.rs` is DIRTY (+360 lines, the STOP/MATCH and device-label lane). Its hunks sit in the same function, at `:152-178`, `:222-244`, `:270-285`. The `dock_refusal` block `:259-267` is untouched by them. Risk: MEDIUM, a textual merge in one function.
- `hud_notice.rs` is clean. `docking.md` and `hud.md` are dirty.

Player fact added: your own hull has lost its port.
Gap left: almost all of New Game (F1). No lock, geometry, target without a port, Combat ship, RCS, ammo.
Stale comments to fix at the same time: `range.rs:157-158`, `drills.rs:222-224`.

## Choice 3. A `RCS EMPTY` notice when Shift is held on an empty magazine

On-screen text: `RCS EMPTY` (HUD notice slot, 4 s).

Exact Rust:
- New system `post_rcs_empty_notice` in `crates/nova_hud/src/hud_notice.rs`, chained beside `post_dock_refusal_notice` in the `Update` set (`hud_notice.rs:149-157`).
- Condition: player root `Has<RcsActive>` and `RcsBudget::is_empty()` (`crates/nova_ship/src/flight/state.rs:~262`; both in the `nova_ship` flight prelude).
  Latch with `Local<bool>`: post on the rising edge, reset when the condition is false.
- Use `RcsActive`, not `RcsIntent != 0`. `RcsActive` is the Shift modal (`flight_rig.rs:795-815`).
  The autopilot MATCH also writes `RcsIntent`, and its short end already posts `MATCH: RCS EMPTY` (`hud_notice.rs:188-198`). `RcsActive` avoids a double line.

Posted by: `HudNotice` (`hud_notice.rs:67`), same slot and cue as the other lines.

New names: the system `post_rcs_empty_notice`. No new type. The `MatchRefusal` path is not reused: its name and its `MATCH:` text are about MATCH.

False-positive check:
- Docked ship: `RcsActive` is released without the helm (`release_rcs_without_the_helm`, `flight_rig.rs:~840`), so silent. A docked pair you fly can still trim; then it is true.
- Parked hull, drill, tutorial: the capability off means `on_rcs_modifier_start` never inserts `RcsActive` (`flight_rig.rs:812-815`). Silent. A Basic Training pilot who has not been given RCS gets no reason (F6, fine).
- Empty because the budget just drained while Shift is held: true, one line. Refill needs a pause with no command, so the tank stays empty while held (`state.rs` doc, `:171-180`).
- Dead station, inactive section: n/a.

Wiki lines it invalidates: none false. Add to `web/src/wiki/hud.md:62` (RCS magazine) and to the notice list at `hud.md:66`.
`web/src/wiki/flight-autopilot.md:90-95` gives the empty behaviour; one clause.

Shared files: `hud_notice.rs` is clean. `crates/nova_hud/src/flight_status.rs` is DIRTY and is NOT touched. `hud.md` is dirty.

Player fact added: the tank is empty, so Shift does nothing. Gap left: withheld RCS (not in New Game), ammo, all docking reasons.

## Choice 4 (largest). Press-time DOCK reasons, a new message modelled on `MatchRefusal`

This replaces choice 2. It fires on the key press, not on the lock. So it has no passive false positives.

On-screen text, first failing check wins, in the gate order of `DockingPair::is_eligible` (`port.rs:116-118`):
- `DOCK: NO LOCK` (press with no travel lock; `flight_rig.rs:761-764` today logs at debug).
- `DOCK: NO FREE PORT` (no free live port on either hull; `nearest_pair` is `None`).
- `DOCK: TOO FAR` (`!pair.gap_holds()`: face gap over the 10 m capture distance plus `GAP_SLACK`; the docking sight draws only inside 8 units, `docking_sight.rs:63,319`).
- `DOCK: NOT FACING` (`!pair.facing_holds()`, 15 degrees).
- `DOCK: TOO FAST` (`!pair.motion_holds()`, speed or spin).

Exact Rust:
- `on_dock_input` (`crates/nova_ship/src/input/player/flight_rig.rs:739`): at the `let Some(target) = ...` else branch (`:761-764`) write the NO LOCK refusal.
- `on_docking_connection_request` (`crates/nova_ship/src/sections/docking_section/connection.rs:328`): at `no eligible port pair, refused` (`:354-357`),
  call `ports.nearest_pair(first_ship, second_ship)`, pick the first failing gate, write the message.
  Only when `request.entity` is the player (`PlayerSpaceshipMarker`; AI ships send the same request).
- The admission refusal branch (`:350-353`) stays silent. Station lines were posted at the lock. Combat stays silent, as at `hints.rs:117-119`.
- Reader: `post_dock_press_notice` in `hud_notice.rs`, shaped like `post_match_refusal_notice` (`hud_notice.rs:180-186`).

New names (all need the owner, per the interface rule):
- `DockPressRefusal(pub DockPressRefusalType)` (a `Message`), `DockPressRefusalType { NoLock, NoFreePort, TooFar, NotFacing, TooFast }`.
  Home: `crates/nova_ship/src/sections/docking_section/` (clean) with an `add_message` there and in `HudNoticePlugin`
  (`hud_notice.rs:144` has the `MatchRefusal` precedent for the second registration).
  Do NOT put the registration in `crates/nova_ship/src/input/player/mod.rs` (DIRTY).
- `fn dock_press_refusal_line(DockPressRefusalType) -> &'static str` in `hud_notice.rs`.
- `post_dock_press_notice`.

False-positive check:
- Docked ship: `on_dock_input` releases first and returns (`:754-757`); no refusal.
- Parked hull: the player does not fly it; `dock_enabled` false returns before the lock check (`:757-760`), still silent.
- Scripted drill, tutorial beat: a press the pilot makes with no lock would show `DOCK: NO LOCK`. That is true. No tutorial beat asks for D (grep of `scenarios/tutorial/*.rs` found no DOCK beat).
  Tests that count notices: UNVERIFIED, none looked for.
- Dead station: geometry lines apply, correct.
- Inactive section: excluded from `ports`, so `NO FREE PORT`, correct.
- Pause or suspended control: returns first (`:747-749`), silent.
- AI dock requests: must be filtered to the player or the HUD shows lines for NPC ships.
- `nearest_pair` ranks one pair. The failing gate named is the nearest pair's, not necessarily the nearest to passing.

Wiki lines it invalidates:
- `web/src/wiki/sections/docking.md:66`: "Every other refusal only darkens the DOCK chip and says nothing. That covers ... a port pair that fails the envelope, a hull of yours with no free live port, ... and a press with no travel lock."
  Becomes false for all three. A ship that refuses stays silent.
- `web/src/wiki/hud.md:66` (notice list). `web/src/wiki/sections/docking.md:46-60` stays true.

Shared files: `flight_rig.rs`, `connection.rs`, `docking_section/*`, `hud_notice.rs` are all CLEAN. `docking.md` and `hud.md` are DIRTY. Lowest code-conflict risk of the four.

Player fact added: why a press did nothing: no lock, no port, too far, not facing, too fast. This is the whole docking-constraint set the audit lists, except Combat ship and `dock_enabled` off.
Gap left: `D` undock is not taught (use choice 1). RCS and ammo (choice 3, F5). Combat stays dark and silent. Keyboard only (F7). A pilot who never presses `D` learns nothing.
Cost: the biggest. About 60 lines, one new message type, two writers, one reader.

## Recommendation

Choose choice 4 as the docking-reason change. It replaces `NO DOCK PORT` (choice 2), because its own-port case reads `DOCK: NO FREE PORT`.
Reasons:
- Choice 2 rarely fires in New Game (F1).
- Choice 4 covers five of the six docking gaps the owner listed, and fires only on a press, so it has no passive false positives.
- It touches no dirty Rust file.
Choice 1 (the `UNDOCK` word) is the stackable next step for the `D` gap. It waits for the other lane to land `chip_word`.
Choice 3 is optional. Ammo needs no change (F5).

Owner decision 2026-11-06: choice 4 approved. Delivered with one writer, `on_dock_input`, not the two above:
`start.rs` and `redock_bought_hull` send `DockingConnectionRequest` for the player with no press, so the request observer cannot write. See TASK.md D3.
