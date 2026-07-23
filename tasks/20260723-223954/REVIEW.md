# Review: ch5 gravity round 2 - thrusterless base clear of tiny wells

- TASK: 20260723-223954
- BRANCH: feature/ch5-gravity-r2

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [ ] R2.1 (NIT) webmods/the-ledger/ledger_ch5_the_raid.content.ron:2521 - my
  planning shorthand "only piloted ships feel gravity" is imprecise: AI ships DO
  feel gravity (`gravity.rs:226` opts them in; "piloted" = player OR AI). This
  strengthens the PR rather than weakening it - AI ships genuinely feel the pull,
  so keeping them clear of the wells is load-bearing and the geometry gaps are
  what protect them. The shipped RON comment is accurate ("placed clear of every
  planetoid well"); no code change.
  - Response: ACCEPTED, no change. The load-bearing surfaces (the base RON
    comment, the geometry) are correct; the imprecise shorthand only lived in the
    planning prose. Noting for record accuracy.

What the out-of-context reviewer verified (I had computed the same geometry
in-session before adopting):

- Geometry recomputed from the RON (base (0,15,-580); leash 200; SOI = 8*radius):
  base vs wells p1 448.7/64, p2 411.9/72, p3 360.2/64 - OUTSIDE all three;
  raider closest approach (basedist - 200 - SOI) p1 184.7, p2 139.9, p3 96.2 -
  all positive, so no raider reaches any well at full leash stretch. No raider
  spawn in a well; base + 4 raiders 614-732u from the player (no spawned-dead);
  planetoids 270-328u from the player in the z -150..-230 approach corridor.
- Rig test meaningful: `assert_eq!(thrusters, 0)` (fails if RCS remained),
  `leash == None` (fails on any leash); the bundle pin was UPDATED to 1.12.0 (a
  real new value); `grep '"1.11.0"'` finds only the dated CHANGELOG history entry.
- lint 0 error/warning/finding (6 scenarios balance-audited, 1 pre-existing ack);
  ledger_ch5_raid 11/11, ledger_ch4_ending 10/10, webmods_validation 1/1;
  `cargo fmt --check -p nova_assets` clean.
- The "AI can't fly wells" limitation is honestly deferred to backlog task
  20260723-224003 (OPEN).

Pending manual check (human-acceptance gate, batched at Finish): playtest ch5 -
the base holds its position; NO AI ship (base or the four fighters) falls into a
well; the wells are gentle approach scenery; torpedoes on R; launches from the
picker.
