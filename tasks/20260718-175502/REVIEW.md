# Review: SHIFT keybind hint + disable RCS in the mainline campaign

- TASK: 20260718-175502
- BRANCH: feat/rcs-keybind-disable

## Round 1

- VERDICT: APPROVE

Reviewed commit 83900c71 vs master. Delivers both asks: the SHIFT hint shows only
when RCS is granted (mirroring the other verb hints), and RCS is disabled in the
mainline campaign via the scenario builders.

Independently verified (shared-session blind-spot guard):
- The keybind hint mirrors the proven `radar` hint exactly (fixed label +
  `verb_granted`); the contextual renderer already hides an ungranted verb's row,
  so "only when RCS enabled" falls out. `rcs_hint_shows_shift_only_when_the_verb_is_granted`
  flips available with the verb (non-vacuous); the keybind suite (12) is green
  after adding the field + the 7th row.
- The disable is authored in the BUILDERS (the `.content.ron` are generated +
  parity-guarded), then regenerated. `content_ron_parity` (2) is green, so
  committed == builders. Each regenerated scenario `.content.ron` carries exactly
  one `DisableVerb(Rcs)`, on the player controller (grep-confirmed).
- REGRESSION CHECK: the shakedown test `the_new_game_player_starts_with_goto_withheld`
  uses `.any(|m| DisableVerb(v) == verb)` per-verb, NOT an exact set/count - so
  adding `DisableVerb(Rcs)` to the controller gate does not break it (it still
  withholds Goto/Lock/Orbit; Rcs is additional and never re-granted by a beat).
- The disable targets ALL player-flyable base scenarios: asteroid_field (inline
  controller), shakedown_run (racer controller_gate), broadside + broadside_gunship
  (the shared `player_ship()`). asteroid_next has no player; menus are backdrops.

No BLOCKER/MAJOR. Nits only:

- [ ] R1.1 (NIT) crates/nova_gameplay/src/hud/keybind_hints.rs (RCS row order) -
  the RCS row sits last (index 6, below COMPONENT). Fine, but a by-eye pass in a
  playtest could reorder it nearer the flight verbs if it reads better. No code
  concern.
  - Response:
- [ ] R1.2 (NIT) the disable is scoped to the base player-flyable scenarios; if a
  future non-menu scenario adds a player ship it must remember `DisableVerb(Rcs)`
  until the rework. A shared player-builder would centralize it, but that is a
  larger refactor and out of scope. Left as-is.
  - Response:
