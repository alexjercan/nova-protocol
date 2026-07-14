# Review: Editor UI rework (baseline)

- TASK: 20260714-204219
- BRANCH: editor/ui-rework

## Round 1

- VERDICT: APPROVE

Reviewed with an out-of-context adversarial pass (independent agent) plus a
same-session re-derivation. The reviewer enumerated every system/observer/resource
in the OLD `editor_plugin` and confirmed each is still wired in the new module set
(cross-checked independently here too - all 10 `add_systems`, 5 `button_on_interaction`
observers, `on_add/remove_selected`, `button_on_setting::<SectionChoice>`, the 4
placement observers, and the 4 resource inits are present; only the two tooltip
observers are additive). Autopilot compatibility, the raycast-not-blocked footprint,
tooltip lifetime, `Pickable` settings, scenario cleanup, and the planetoid config
were all verified correct. Both NITs below were addressed anyway (cheap correctness
wins).

- [x] R1.1 (NIT) `crates/nova_editor/src/ui/tooltip.rs` - `hide_component_tooltip`
  despawned ALL tooltips on any card `Out`; if `Over(cardB)` is ever delivered
  before `Out(cardA)` when crossing directly between cards, B's fresh tooltip would
  be wrongly removed and not respawn (Over fires only on enter). Latent, not active
  (Bevy normally fires Out-before-Over for siblings), but fragile.
  - Response: fixed. `Tooltip` now carries the source card `Entity`; `hide` only
    despawns the tooltip whose `card == out.entity`, so a newly-entered card's
    tooltip survives an out-of-order `Out`. `show` still clears any stale tooltip
    before spawning, preserving at-most-one.

- [x] R1.2 (NIT) `crates/nova_editor/src/scenario.rs` - the planetoid test asserted
  `surface_gravity.is_some()` but not the value.
  - Response: fixed. Now asserts `surface_gravity == Some(40.0)` (the load-bearing
    "well" property).

### Verified correct (no change needed)

- Raycast not blocked at screen centre: RAIL_W(150)+DRAWER_W(280)=430 < 512; root
  passes pointer events through the centre/right, panels block over themselves.
- Card children carry `IGNORE` `Pickable`, so hover stays on the card (no tooltip
  flicker). Tooltip carries `DespawnOnExit(Editor)` + at-most-one; no leak.
- Autopilot: `Name "Create New Spaceship Button V2"` + the card `Name`s +
  `EditorButton`+`ButtonValue<SectionChoice>` preserved; `button_on_setting` still
  fires on `insert(Pressed)`. Confirmed by the headless `09_editor` run.
- Scenario: no dangling `other_spaceship`/objective references; planetoid config
  idiomatic. Tests are genuine regressions against the old scenario.
- No test weakened or lost in the split; `pub(crate)` visibility + imports clean.
