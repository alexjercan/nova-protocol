# Show SHIFT keybind hint when RCS granted + disable RCS in the mainline campaign

- STATUS: CLOSED
- PRIORITY: 4
- TAGS: v0.7.0, feature, input, scenario, spike

## Goal

Two pieces of the user's ask (2026-07-18), so RCS ships present-but-off:

1. Show the SHIFT keybind hint ONLY when the ship grants the `Rcs` verb, exactly
   like the other verb hints (the cluster already hides an ungranted verb's row).
2. Disable RCS in the mainline campaign (player ships withhold the `Rcs` verb)
   until the rework - the same per-scenario `DisableVerb` opt-out the other verbs
   use, NOT a change to the shared base controller (which the pirate reuses).

RCS stays a normal granted-by-default verb (user model); this only turns it off
in the shipped campaign content and surfaces its keybind consistently.

## Steps

- [x] Add `pub rcs: VerbHint` to `FlightVerbHints`
  (crates/nova_gameplay/src/input/player.rs:98) and populate it in
  `update_flight_verb_hints` (~player.rs:245) like the `radar` hint:
  `key: cycle_label("SHIFT", rig_exists)`, `available: verb_granted(FlightVerb::Rcs)`,
  `anchor: None`.
- [x] Render the row (crates/nova_gameplay/src/hud/keybind_hints.rs): extend
  `ROW_VERBS` to include `"RCS"` (index 6), map it in `row_hint`
  (`6 => &hints.rcs`, keeping 5 = component_cycle), and add `row(6)` to the
  cluster `children!` (keybind_hint_cluster_hud). The existing "row renders only
  while its verb is actionable" logic hides SHIFT when RCS is withheld - i.e.
  "only when RCS enabled" for free.
- [x] Test (mirror the existing hint tests, player.rs:~1602): with a live
  controller that GRANTS Rcs, `update_flight_verb_hints` sets `rcs.key == "SHIFT"`
  and `rcs.available == true`; with `WithheldVerbs([Rcs])`, `rcs.available ==
  false`. (The row-hide is the renderer's existing behavior, already tested for
  the other verbs.)
- [x] Disable RCS in the mainline campaign: add `DisableVerb(Rcs)` to the PLAYER
  controller's `modifications` in the base player-flyable scenarios
  (assets/base/scenarios/: asteroid_field, shakedown_run, broadside,
  broadside_gunship - whichever spawn the player with a controller). Mirror the
  existing `DisableVerb(Goto/Lock/Orbit)` blocks. Only the PLAYER controller, not
  enemy ships (verb affects RCS-terminal for any granting ship, but the intent is
  the player-facing feature off).
- [x] Verify the scenarios still load (nova_assets scenario tests / a headless
  smoke) and that a player ship in them has `Rcs` withheld.
- [x] NOTES.md + spike Fix record.

## Notes

Spike: tasks/20260718-122508/SPIKE.md (the RCS family). RCS is a normal verb
(granted by default) - user call 2026-07-18. Reference points:
- Verb hints: `FlightVerbHints`/`VerbHint` player.rs:75-115,
  `update_flight_verb_hints` player.rs:167 (the `radar` hint at ~286 is the
  exact SHIFT pattern - `cycle_label` label + `verb_granted`).
- Renderer: `ROW_VERBS` keybind_hints.rs:47, `row_hint` :212, cluster
  `children!` :147 (rows are contextual - hidden when unavailable).
- Disable pattern: `DisableVerb(...)` in scenario RON `modifications`
  (assets/base/scenarios/shakedown_run.content.ron:134); the shared base
  controller deliberately does NOT bake withholding (sections.rs:235).