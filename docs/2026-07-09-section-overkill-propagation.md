# Overkill damage no longer kills a whole ship (task 20260709-144906)

## What changed

`HealthApplyDamage` in `bevy-common-systems` is an
`#[entity_event(propagate, auto_propagate)]`, so a hit on a ship section bubbles
up the `ChildOf` chain to the ship root, whose aggregate `Health` is the sum of
its sections. The old `on_damage` clamped the *section's* health to zero but
forwarded the raw, unclamped `amount` to every ancestor. A 1000-damage hit on a
100 hp section therefore subtracted 1000 from the root aggregate in the same
frame, latching `HealthZeroMarker` on the root before `aggregate_ship_health`
could recompute it - and the whole ship died through disable -> destroy despite
healthy sections.

The fix is in bcs `on_damage` (`src/health/mod.rs`): apply at most the node's
remaining health, and set the bubbling event's `amount` to what actually landed
(`applied = amount.min(health.current)`). Entity-event propagation reuses the
same event instance for each ancestor (see `event/trigger.rs`: the propagation
loop passes `event.into()` every iteration), so mutating `damage.amount` is
exactly what the next node up sees. A node that is already destroyed or at zero
health propagates zero, so hitting a corpse charges its parents nothing.

nova side: a physics-level regression test
(`overkill_on_one_section_does_not_kill_the_ship` in `integrity/glue.rs`) drives
a two-section ship, hits one section with 10x its health, and asserts the other
section and the root survive with roughly one section's worth of health lost.
The `aggregate_ship_health` doc comment was updated to record that the bubbled
amount is now clamped, and why the "last section dies -> root dies" flow still
works (with one section left, the aggregate equals it, so the clamped amount is
exactly enough to zero the root).

## Why this mechanism

Two options were on the table: (A) clamp the propagated amount in bcs, or (B)
keep it nova-side by making `aggregate_ship_health` the sole source of root death
and stopping propagated overkill from marking the root. (A) was chosen: it is the
correct, general fix for *any* aggregate-health hierarchy (the propagation model
only makes sense if a parent loses what its child actually lost), it preserves
the existing propagation-drives-root-death design instead of fighting it, and it
keeps the fix where the bug is. (B) would have left the same latent bug for every
other consumer of bcs health propagation.

## Difficulties

- **Verifying the fix worked required a cross-repo build.** nova pins bcs by git
  `rev`, so the local bcs edit had no effect until nova was pointed at it. A
  temporary `[patch."https://github.com/alexjercan/bevy-common-systems"]` to the
  local bcs worktree let the full nova pipeline run against the fix without a
  push. The patch is a verification-only artifact and is not committed; landing
  the nova branch is blocked on pushing bcs and bumping the pinned rev.
- **The first regression assertion was too strict.** It asserted the root health
  was exactly `100.0`; it came back `99.978`. The 0.022 hp is negligible
  section-to-section contact damage between the two touching unit-cube sections
  in avian - a physics artifact the sibling COM test never noticed because it
  only checks mass/COM. The assertion was relaxed to a 1.0 hp tolerance, which
  still cleanly separates "lost ~one section (~100)" from "ate the 1000 overkill
  (0)". The lesson from the last two retros held: an exact-equality assertion
  over a value a real physics step touches is a false-precision trap.

## Verification

- bcs: 3 new health unit tests (overkill clamp, preserved fatal bubble,
  corpse-hit charges nothing) + full lib suite, 126 passing.
- nova: `integrity::glue` module, 9 passing (incl. the new regression), built
  against the bcs branch via the temporary patch. `cargo fmt --check` clean in
  both repos. Broad clippy/example suites left to CI per repo policy; the 06/11
  range smokes were not re-run (behavior-invariant: in-play blast damage never
  exceeds section hp, so the clamp cannot change normal play).
