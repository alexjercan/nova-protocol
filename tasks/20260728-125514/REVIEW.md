# Review: NOVA OS ship app - minimize the blip overlay

- TASK: 20260728-125514
- BRANCH: feature/ship-blip-minimize

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/nova_os_ship.rs - the just-spawned
  dot sets its transparent non-selected border as `NOVA_OS_AMBER.with_alpha(0.0)`
  while `project_ship_blips` sets it as `NOVA_OS_PHOSPHOR.with_alpha(0.0)`. Both
  are fully transparent (no visual difference), but pick one constant for clarity.
  - Response: fixed - `project_ship_blips` now uses `NOVA_OS_AMBER.with_alpha(0.0)`
    to match the spawn (the visible selected border is amber).

- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/nova_os_ship.rs (test deletion) -
  deleting `integrity_bar_and_ammo_pips_track_live_data` also dropped the only
  assertion pinning `integrity()`'s "unknown health reads full/nominal" edge, which
  `status()`/`status_color()` (now driving the dot + panel) still rely on. Add a
  one-line assert so the edge stays covered.
  - Response: fixed - added `unknown_health_reads_nominal`: an unknown-health
    fixture's `status()` == "nominal" and `status_color()` == `NOVA_OS_PHOSPHOR`.

Verification notes (out-of-context reviewer, re-confirmed in-session):
- Non-test `cargo check -p nova_gameplay` clean, 0 `never read` - the removed
  `bar_fraction`/`ammo_pips` helpers and `ShipBlip.bar_fill`/`ammo` fields left
  nothing dangling.
- DoD greps: `ammo_pips|bar_fill|bar_fraction` -> none; `srgb` -> 1 (unchanged).
- `blip_is_status_dot_with_labelled_marker` is meaningful (fails if the dot were
  not status-coloured, the label dropped, or pips remained).
- `project_ship_blips` uses disjoint `&mut` queries; the just-spawned-blip guard is
  preserved; `q_bg.get_mut(blip)` recolours only the dot, not the pill child.
- DECISION supersede chain is bidirectional and correctly scoped (only the
  blip-status part of `20260728-115435`; blocks/kind/outline still stand).

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (both round-1 findings were NITs - a transparent-colour
  constant alignment and a one-line edge assertion; mechanically verifiable, no
  re-run of the out-of-context reviewer warranted)

- R1.1 verified: the non-selected border constant now matches the spawn (amber,
  transparent); no behavioural change.
- R1.2 verified: `unknown_health_reads_nominal` added and passing; the nominal
  edge is pinned again.
- Full suite `cargo test -p nova_gameplay --lib nova_os_ship` -> 16 pass; `cargo
  fmt` clean.

Pending user checks (open `manual:` DoD items, not resolved by APPROVE):
- No "500 circles" - ammo pips gone (confirmed in the screenshot capture).
- Labels readable, not tiny green-on-green - dark backing pill (confirmed in the
  capture; pending owner playtest).
- A critical section's dot reads amber at a glance without selecting it.
