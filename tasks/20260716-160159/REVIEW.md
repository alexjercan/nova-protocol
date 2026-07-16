# Review: Broadside playtest tuning - enemy-only torpedoes, infinite ammo

- TASK: 20260716-160159
- BRANCH: fix/broadside-playtest-tuning

## Round 1

- VERDICT: APPROVE
- Reviewer note: same-session self-review, chosen deliberately for a
  ~60-line mechanical diff (the out-of-context pass is reserved for
  substantial branches); compensated by re-deriving the load-bearing bits
  rather than reading the diff alone:
  - the regenerated RON delta is EXACTLY the three intended changes
    (binding removed, infinite_ammo flipped, torpedo section removed) -
    inspected hunk by hunk, nothing else moved;
  - mutation check on each pin: reverting the flag fails the infinite-ammo
    assert, restoring the section fails the NO-torpedo assert, gutting the
    gunship fails the new tubes>=2 pin (which guards the ENEMY half of the
    verdict - the screening beat needs tubes);
  - parity x2 green, broadside_assault 8/8 green, fmt clean;
  - live example-19 walk re-run green on the tuned data (kill path is
    damage-driven, unaffected by design - verified, not assumed);
  - prose sweep: no shipped copy claims a player torpedo (the PDC-screening
    lines were already defense-only); the closed slice task's NOTES.md
    retains its historical "full loadout" wording as a record of what
    shipped THEN, with this task recording the change.

No findings. The one judgment call - infinite ammo vs finite-but-bigger -
is recorded in the close record with its trigger for revisiting (ship
management/resupply, pairs with the ammo HUD task 20260716-123556).
