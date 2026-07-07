# Review: Blast collision fires inconsistently (ordering bug)

- TASK: 20260706-162912
- BRANCH: fix/blast-collision-ordering

## Round 1

- VERDICT: APPROVE

Diff touches three things: `blast_damage` gains `CollisionEventsEnabled` (blast.rs),
`on_blast_collision_deal_damage` is rewritten to treat the blast as the event's "self" side
(plugin.rs), and the FIXME is removed. Two new regression tests plus the updated existing blast
tests.

Verified:

- Root cause is correct and the fix matches it. avian's `trigger_collision_events`
  (narrow_phase/mod.rs) raises `CollisionStart` once per collider in the pair that has
  `CollisionEventsEnabled`, with that collider as `body1`. The blast never enabled events, so it
  depended on the target's events - exactly the "(object, blast) only" symptom. Enabling events
  on the blast and keying the observer on the blast-as-`body1` ordering makes it fire against
  every overlapped collider, mirroring `area.rs`.
- No double-dip. When the target also has events, avian raises both orderings; the observer's
  `q_blast.get(body1)` fails on the target-as-self ordering, so damage lands once. The
  `a_blast_deals_damage_only_once_when_the_target_also_has_events` test asserts exactly 60
  (not 120), and the falloff test asserting exactly 60 confirms the impact observer contributes
  nothing spurious now that the blast carries events (the static blast has no
  `LinearVelocity`/`ComputedMass`, so the impact path early-returns).
- The regression test is meaningful, not a tautology: it constructs a target that genuinely
  lacks `CollisionEventsEnabled` (collider added without `Health`, so the enable observer skips
  it), asserts that precondition, then shows the blast still deals 60. On the old code no event
  would be raised for that target at all, so it would take zero.
- Damage target is consistent: always the object's own collider (`collider2` now, `collider1`
  before) - the Health-bearing entity - so ship sections and asteroid nodes are hit correctly.
- Full suite green: `cargo test --workspace` (50 nova_gameplay incl. 2 new, examples_smoke
  under Xvfb), `cargo clippy --workspace --all-targets` clean (only the pre-existing
  `hull_section.rs` `struct update` warning, outside this diff).

No BLOCKER/MAJOR/MINOR findings. Small, well-targeted fix with a genuine regression test.
