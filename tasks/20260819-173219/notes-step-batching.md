# The two mechanical fixed-step items, taken to a number

`notes-fixed-step.md` ranked two changes that needed no design decision. One
pays and shipped; the other buys nothing measurable AND changes what the game
does, so it is rejected twice over.

Measured 2026-08-21 on `1a86b140`, headless (`NOVA_NORENDER=1`), never under
Xvfb, `dev` profile, i9-12900F, one lane on the box. Arms are paired and
INTERLEAVED from prebuilt binaries, so no arm pays for a rebuild inside a sweep.
Regimes are selected on each run's own step CSV (the investigation's
`NOVA_STEP_DIAG`, extended here with a per-step `CollisionStart` message count),
never on window placement.

**The load band is the load-bearing part of the method.** The arena capture
lands wherever the fight happens to be, and an arm whose fights end sooner reads
faster for that reason alone. Every step number below is taken over steps
matched on DYNAMIC BODY COUNT - 650-750 in the arena, 1,800-2,200 in point
defence - and each table carries the mean body count so the match can be
checked. Whole-run frame stats are reported beside them, not instead of them.

## Item 2 - single-threaded fixed loop: KEPT

`FixedFirst` through `FixedLast` run on Bevy's `SingleThreadedExecutor`, set in
`AppBuilder::assemble` before any plugin adds a system. Avian's
`PhysicsSchedule` and `SubstepSchedule` are left multithreaded.

Arena, `wfc_arena --ship amber --ship onyx`, two sweeps pooled (base n=17,
fixed-only n=8, fixed+physics n=17, physics-only n=9). Per-run stats over steps
with 650-750 dynamic bodies, then rank stats across runs (Mann-Whitney vs base):

| arm | step median | step p95 | step worst | "other" | bodies |
|---|--:|--:|--:|--:|--:|
| base | 7.89 | 15.16 | 16.26 | 5.37 | 698 |
| fixed schedules | **6.08** (p=0.032) | **13.26** (p=0.040) | 14.38 (p=0.239) | **4.01** (p=0.013) | 694 (p=0.631) |
| fixed + physics | **5.40** (p=0.025) | **12.77** (p=0.008) | **13.50** (p=0.002) | **3.61** (p=0.009) | 694 (p=0.245) |
| physics only | 7.54 (p=0.796) | 15.76 (p=0.245) | 16.39 (p=0.439) | 5.45 (p=0.897) | 695 (p=0.897) |

"other" is wall minus every avian phase timer - the per-step bookkeeping the
investigation sized at 66% of a step. It moves by 1.4 ms, which is where the
win comes from and matches the ~1.4 ms of executor self time the trace
attributed.

Frame level, whole run, same pooled sweeps:

| arm | frame p99 | worst frame | 1% low fps |
|---|--:|--:|--:|
| base | 36.92 | 49.17 | 27.09 |
| fixed schedules | **21.04** (p=0.001) | **28.25** (p=0.001) | **47.53** (p=0.001) |
| fixed + physics | **21.78** (p=0.000) | **26.15** (p=0.000) | **45.92** (p=0.000) |
| physics only | 40.59 (p=0.038 WORSE) | 53.89 (p=0.038 WORSE) | 24.64 (p=0.038 WORSE) |

**The fixed schedules are the whole win; adding the physics schedules buys
nothing beyond them, and the physics schedules ALONE are a small regression.**
That asymmetry is the reason the change stops at the fixed loop: the solver's
`par_for_each` passes are the one part of a step that saturates threads, and
single-threading them gives that parallelism back at no gain.

Second workload, `stress_point_defense`, 6 interleaved rounds, steps matched at
1,800-2,200 bodies (base 2,045 vs 2,035, p=0.522):

| arm | step median | step p99 | step worst | "other" |
|---|--:|--:|--:|--:|
| base | 3.17 | 11.89 | 14.81 | 2.32 |
| fixed schedules | **2.84** (p=0.004) | **7.91** (p=0.025) | **10.79** (p=0.025) | **2.01** (p=0.004) |

No frame regression there (1% low 134.5 -> 158.0, p=0.631), which is the point
of the cross-check: a saturated sensor scene is the workload most likely to
want the threads back, and it does not.

### The landing check, on binaries with no instrument in them

Everything above carries `NOVA_STEP_DIAG`, which locks a mutex and counts every
collider once a step - real time, in both arms. So the landed change was
re-measured against a CLEAN `1a86b140` build, neither binary instrumented, 8
interleaved rounds, frame stats only:

| metric | master | landed | p |
|---|--:|--:|--:|
| frame p99 | 27.82 | 22.50 | 0.115 |
| worst frame | 41.73 | **25.78** | 0.036 |
| 1% low fps | 36.65 | 44.45 | 0.115 |

Same direction, smaller separation, and only the worst frame clears p<0.05 at
n=8 - which is what the instrument's own cost predicts: an uninstrumented base
already runs at a 36.7 fps 1% low against the instrumented 27.1, so there is
less to take back. Read the instrumented tables for the SHAPE and this one for
the level.

Behaviour: `system_blast_penetration` and `system_destruction_finale` produce
BYTE-IDENTICAL outcome markers; `system_hull_damage` differs only in a
centre-of-mass drift of 8.3e-9 against 2.4e-7, both zero to the assertion.
`stress_point_defense` lands inside the base spread on every count it pins
(peak rounds 2,420 against 2,419-2,421; intercepts 7 against 3-9; aim error
19.5 deg against 18.5-19.6). `system_player_path` and `system_turret_gunnery`
pass. Three ranges FAIL both before and after this change, identically -
`system_blast_penetration` (never reaches `Playing`), `system_section_severing`
(`ShipIntegrityPlugin` added twice), `system_hull_damage`
(`assert_com_follows_sections` expects a missing entity). Pre-existing, not
touched here, and worth a task of their own.

## Item 1 - batch the `CollisionStart` consumers: REJECTED, twice

Built in full: a `CollisionStarts` system param in `nova_gameplay` that reads
avian's `CollisionStart` MESSAGE stream and re-orients it exactly as avian's
`trigger_collision_events` does, with `on_impact_collision_deal_damage`,
`collect_nova_blast_collision` and `resolve_bullet_hit` converted from observers
to systems draining it once per step in `PhysicsStepSystems::Finalize`, after
`CollisionEventSystems`.

### The stream is provably identical; the game still changes

The delivery equivalence is not an argument, it is a test: one app with an
observer and a drain both recording what they are handed over a sensor round
sweeping two targets - one with collision events on both sides, one with events
on a single side - asserts the two sequences are EQUAL. They are.

The health snapshot is unchanged too, and for a reason worth writing down:
observers queue their commands into the world queue and nothing flushes between
two `world.trigger` calls, so a step's contacts already shared ONE pre-damage
snapshot under the observers. The drain does not change that.

The game changes anyway. `stress_point_defense`, 8 interleaved rounds each arm,
frame rate statistically identical between arms (fps p=0.753), which rules out
frame-rate coupling as the explanation:

| count | base | batched |
|---|--:|--:|
| peak live rounds | 2,420 (2,419-2,421) | **2,190** (2,169-2,214) p=0.001 |
| torpedoes intercepted | 5 (3-7) | **10** (9-12) p=0.001 |
| peak torpedoes in the envelope | 54 (54-56) | **49** (48-50) p=0.001 |
| mean aim error, deg | 19.27 | 18.30 p=0.001 |

Peak live rounds is a SATURATION level, not a trajectory: 16 base runs across
three separate sessions span 2,419-2,421. A 9.5% shift in it is a rule-level
difference, not chaos.

### Which conversion does it: NEITHER, and that is the finding

Two more interleaved arms, 5 rounds each, on the same peak-round count:

| arm | peak live rounds | vs base |
|---|--:|--:|
| base (all three observed) | 2,421 | - |
| only the bullet resolver drained | 2,421 | p=0.210 |
| impact + blast drained, bullet observed | 2,421 | p=0.210 |
| all three drained | 2,192 | **p=0.009** |

**Either conversion alone is behaviour-preserving. Both together are not.** So
the thing the game depends on is not WHERE a consumer runs but that the impact
and bullet consumers INTERLEAVE per contact: under observers the queue reads
`e1.impact, e1.bullet, e2.impact, e2.bullet`, and under two systems it reads
every impact then every bullet. Two systems in two crates cannot be interleaved
back without merging them, so this is not a scheduling flag away.

I did not isolate WHICH non-commutative pair inside that interleaving carries
the 9.5%. The best candidate that survives inspection is `HealthApplyDamage`
carrying a `source` entity that the bullet drain despawns in the same flush, so
block ordering can strand attribution that per-contact ordering never did. It is
a candidate, not a measurement.

### And it does not pay anyway

Arena, 9 interleaved rounds, matched at 650-750 dynamic bodies:

| metric | base | batched | p |
|---|--:|--:|--:|
| step median | 7.89 | 7.70 | 0.493 |
| step p95 | 15.16 | 15.51 | 0.456 |
| step worst | 16.26 | 17.12 | 0.655 |
| "other" | 5.37 | 5.26 | 0.456 |
| frame p99 | 36.92 | 37.47 | 0.609 |
| 1% low fps | 27.09 | 26.69 | 0.609 |

Nothing moves, at the mean or in the tail. The estimate that ranked this first
was 0.4-0.8 ms typical and "several ms on a cascade step", and the reason it
does not land is visible in the new CSV column: **an arena fight step raises
100-180 `CollisionStart` messages, not thousands.** The 12,673-event figure the
estimate leaned on was a WHOLE-RUN total; per step it is two orders of magnitude
smaller, and a few hundred observer dispatches are not a millisecond. Cascade
steps (>= 50 messages) show the same nothing: median 7.27 -> 8.00 (p=0.850).

**Verdict: rejected. The code is not landed.** It costs a measurable gameplay
change for no measurable time, and the gameplay change is not the owner's to
approve because there is nothing on the other side of the trade.

## What this leaves

The fixed step's remaining ~6 ms at a 1v1 fight is still dominated by per-step
bookkeeping, and the ranked list's item 3 - "a round is not a physics body" - is
untouched and still the largest lever. Nothing measured here changes its
estimate; the executor win is orthogonal to it, since it removes scheduling
overhead rather than bodies.

One method note for whoever measures next: the executor arm looked like a 65%
win on whole-run frame stats AND like a lighter fight (fight-regime steps 110
against base 159, mean bodies 607 against 665). Both were true, and only the
matched band separated them - a faster sim ends the arena's fight sooner, which
flatters every whole-run average it produces. Match on body count first.

## Raw data

`measurements/step-batching/` beside this note: the pooled arena arm table
(`arena_arms.txt`), the matched-band tables for arena and point defence
(`arena_band.txt`, `pd_band.txt`), the uninstrumented landing check
(`landing_check.txt`), the point-defence outcome-marker
distributions that carry the gameplay finding (`pd_markers.txt`), and the
three-arm ablation that localised it (`pd_ablation.txt`). Step CSVs and the
binaries stayed outside the repo.
