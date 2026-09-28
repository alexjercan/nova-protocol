# Review: dual-lock target inset (combat + travel priority, kill cam, caption readout)

Scope: crates/nova_hud/src/target_inset.rs, crates/nova_hud/src/torpedo_target.rs,
crates/nova_hud/src/lib.rs, crates/nova_ship/src/input/targeting/state.rs,
web/src/wiki/hud.md, web/src/widgets.ts, CHANGELOG.md. examples/ and tasks/ excluded
per instructions.

## Verdict

MINOR findings only. The state machine (`InsetSlotType`, `inset_subject`,
`drive_inset_camera`, `drive_inset_frame_state`) matches the approved design and is
correctly ordered (`drive_inset_camera -> ApplyDeferred -> (drive_inset_frame_state,
show_confirmed_destruction)`). No BLOCKER or MAJOR issues found in the reviewed diff.

## Findings

### MINOR - web/src/widgets.ts:670-676 - switchboard widget cannot demonstrate the new travel-lock-alone behavior it now describes

- Evidence: `web/src/widgets.ts:671-676` still gates the "Target viewfinder" row's
  `on` flag purely on `s.combatLock`:
  ```
  e(
      "Target viewfinder",
      "chrome",
      s.combatLock,
      s.combatLock
          ? "frame hot-red, corner ticks out"
          : "arrives with a combat lock, or a travel lock alone"
  ),
  ```
  `HudSituationsModel` (`web/src/widgets.ts:605-612`) and the `hud-context`
  switchboard's `KEYS` list (`web/src/widgets.ts:5139-5146`) have no `travelLock`
  field/toggle at all. The new source comment above the interface
  (`web/src/widgets.ts:600-602`) says outright: "the viewfinder shows the combat
  lock, else the travel lock (target_inset.rs inset_subject), which this model does
  not toggle."
- Consequence: a player reading `web/src/wiki/hud.md`'s "HUD switchboard" widget and
  toggling COMBAT LOCK off will see "Target viewfinder ... OFF" with the new detail
  text claiming it "arrives with a combat lock, or a travel lock alone" - a claim the
  demo cannot show since there is no way to turn a travel lock on. The widget's stated
  behavior and its interactive behavior now disagree for this element.
- This is docs/demo-only; it does not affect gameplay or the Rust state machine. Per
  the project's widget-first docs philosophy, either add a `travelLock` toggle to the
  model/switchboard, or drop the "or a travel lock alone" clause from the detail text
  until the demo can show it.

### Unverified / code-reasoned, not test-covered

- Same-entity-in-both-slots (`CombatLock == TravelLock == Some(X)`): traced by
  reading, not exercised by a test. `inset_subject` (`crates/nova_hud/src/target_inset.rs:637-646`)
  checks `combat` before `travel` via `.or_else`, so both `drive_inset_camera` and
  `drive_inset_frame_state` independently resolve the same `(Combat, X)` pair - no
  observed way for image and caption to disagree here, but no test pins it.
- The pre-existing edge case where a player clears `CombatLock` in the same frame a
  target is marked `IntegrityDestroyMarker` but *before* it despawns: `confirmed_gone`
  requires `!q_alive.contains(last.target)` (`crates/nova_hud/src/target_inset.rs:755-758`),
  so the kill cam is skipped and the panel falls straight to travel/hidden that frame.
  Traced this pattern unchanged from the pre-diff single-lock code (same structure
  existed before travel-lock support), so it is not a regression introduced here - not
  re-verified against current game logic for whether `CombatLock`-clear and despawn are
  ever non-atomic in practice.

## Confirmed correct (given evidence)

- **Ordering fix**: the `ApplyDeferred` sync point added at
  `crates/nova_hud/src/target_inset.rs:496-502` makes a kill cam started via
  `Commands` in `drive_inset_camera` visible to `drive_inset_frame_state` and
  `show_confirmed_destruction` in the same frame - matches the design requirement and
  is exercised by `a_destroyed_combat_target_holds_the_kill_cam_before_the_travel_target`
  (`target_inset.rs:2026-2077`), which asserts the caption is `""` and the DESTROYED
  ribbon is `Inherited` in the very frame the kill cam starts.
- **Priority**: `inset_subject` (`target_inset.rs:637-646`) and the "fresh framable
  combat lock preempts" branch (`target_inset.rs:763-765`) give combat unconditional
  priority over travel, including for a non-zoomable combat lock (NO-SIGNAL still wins
  over a framable travel target) - covered by
  `a_non_zoomable_combat_lock_holds_no_signal_over_a_framable_travel_target`
  (`target_inset.rs:1422-1458`, replaces the old single-lock test of the same shape).
- **Stale lock treated as absent**: a `CombatLock` left pointing at a despawned entity
  with no confirmed-destruction proof falls through to the travel target the same
  frame, without ever showing NO-SIGNAL - verified by the third segment of
  `the_combat_lock_owns_the_inset_and_the_travel_lock_takes_it_back`
  (`target_inset.rs:2002-2019`), which leaves `CombatLock` dangling on the despawned
  entity and asserts `framed_target() == Some(depot)`. This is a real behavior fix over
  the pre-diff code (previously a dangling lock's failed anchor lookup was
  indistinguishable from a beacon and rendered NO-SIGNAL indefinitely).
- **No lock writes**: both new priority/fallback and kill-cam tests assert
  `CombatLock`/`TravelLock` are unchanged by the inset systems
  (`target_inset.rs:2015-2023`, `target_inset.rs:2072-2076`).
- **State bookkeeping**: `Live`/`NoSignal`/`Hidden` branches
  (`target_inset.rs:789-814`) always clear `TargetInsetKillCam`,
  `TargetInsetLastFramed` and `TargetInsetDestroyedTarget` together when leaving the
  kill cam, so a later re-lock cannot resurrect a stale kill cam or destroyed-target
  marker (chrome-hide, expiry-with-no-fallback, and fresh-combat-preempt paths all
  checked by reading; expiry-with-travel-fallback checked by the new kill-cam test).
- **Caption/image agreement**: `drive_inset_frame_state` blanks the caption
  unconditionally whenever any `TargetInsetKillCam` exists
  (`target_inset.rs:933-937`), regardless of what the travel lock is doing, so no
  travel caption can sit over a frozen kill-cam image - this is the mechanism that
  satisfies "caption must never describe one target over another target's image" for
  the one case (kill cam) where the shown image and the live locks can otherwise
  diverge.
- **Ordering vs. gameplay**: `NovaHudSystems` is configured
  `.after(SpaceshipSectionSystems).before(NovaCameraSystems)`
  (`crates/nova_hud/src/lib.rs:104-116`), so `CombatLock`/`TravelLock` are stable for
  the whole HUD pass within a frame - no risk of a gameplay system mutating either lock
  between `drive_inset_camera` and `drive_inset_frame_state`.
- **Units/formatting**: the new caption reuses `torpedo_target::{closing_line,
  closing_speed, distance_line}` (now `pub(crate)`,
  `crates/nova_hud/src/torpedo_target.rs:359,372,382`) rather than re-implementing the
  engine-to-meters conversion - one conversion point, no dual path, matches AGENTS.md.
- **Docs/changelog**: `web/src/wiki/hud.md` and the `CombatLock`/`TravelLock` doc
  comments in `crates/nova_ship/src/input/targeting/state.rs` accurately describe the
  new priority/fallback/kill-cam behavior; the `CHANGELOG.md` entry (164 chars, under
  the 200-char rule) sits under the correct `### Interface & HUD` heading. No stale
  references to the old single-lock behavior found elsewhere in the tree (grepped for
  the retired doc phrasing and the old `target_inset.rs:658-707` line-number citation).
- **No leftovers**: no adapters, aliases, or dual code paths found; the old
  single-lock test this change replaced
  (`a_non_zoomable_lock_holds_the_panel_with_no_signal`) was renamed/rewritten rather
  than left alongside a new one.

## Skipped checks

- No workspace-wide tests or Clippy run (per instructions).
- No game or GPU run performed (per instructions; a `frames/`/`xvfb.log` artifact
  already present under `tasks/20260928-083106/` suggests a proof worker may be running
  one concurrently - not inspected, per instructions to ignore `tasks/`).
- Did not verify at runtime whether `CombatLock` clearing and target despawn are
  guaranteed atomic within the same frame elsewhere in the gameplay/scenario code (see
  "Unverified" above) - traced by reading only.
- Did not check `examples/` (explicitly excluded).
