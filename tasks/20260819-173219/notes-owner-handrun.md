# The owner's hand-run, before and after the bucket fix

Same host, same subject (`wfc_ships`), same protocol both times: load the
gallery by hand, read the FPS counter. Not a probe capture - no pinned fixed
step, no fixed window - so these are not comparable with `notes-ablation.md`
absolutes. They ARE comparable with each other, which is what makes them
worth keeping: it is the only before/after measured outside the harness.

## The readings

| ships | before (`master` at `84955113^^`) | after (`6b3bfc87`) |
|--:|---|---|
| 1 | - | **36-48 FPS** (20.8-27.8 ms) |
| 3 | 25 FPS (40.0 ms) | - |
| 11 | 9 FPS (111.1 ms) | - |
| 17 | **7 FPS (142.9 ms)** | **16 FPS (62.5 ms)** |

## The two lines

Least squares on the before points; two-point fit on the after points, given
as a range because the 1-ship reading is a range.

| line | floor | per ship |
|---|--:|--:|
| before | 21.2 ms | **7.43 ms** |
| after | 18.2-21.9 ms | **2.39-2.61 ms** |

**The floor did not move and the per-ship term fell to a third.** That is
exactly the shape the fix was designed to have, and the unmoved intercept is
the check: a fix that had accidentally touched anything scene-wide would have
moved it. 21.2 ms also lands on the harness's independently measured 21.5 ms
shipped-line floor, on a different protocol.

Per-ship 7.43 -> 2.4-2.6 is **2.9x**, better than the arena's 2.2x. Expected:
`notes-ablation.md` section 5 established the gallery went from ZERO private
materials to one per section at `0ee9cbb0`, because `damage_tint.rs` skipped
unaligned bodies and `SectionCracksPlugin` does not. The gallery had the most
to give back.

## What the 1-ship reading says, and it is the finding

**One ship holds 36-48 FPS. The floor is 18-22 ms of that.** So 75-90% of a
single-ship frame is scene cost that no ship optimisation can touch, and the
gallery has a hard ceiling near **46 FPS with nothing in it**.

The floor was already named as a phase 4 item behind the phase 3 batch. This
reading re-ranks it: at every ship count the owner actually plays, the floor
is now either the largest term or the only one.

## The 7 ms spread at one ship, unexplained

36-48 FPS is a 33% spread on a scene with one static hull. Steady-state cost
does not do that. Two candidates, both cheap to separate:

1. **The fixed-step amplifier.** Un-pinned, bevy runs `delta / 15.625 ms`
   steps. At 20.8 ms that is 1.33 steps, at 27.8 ms it is 1.78 - so the frame
   alternates 1 and 2 steps and each step costs `s`. A 7 ms spread implies
   `s` near 7 ms. Test: `NOVA_PERF_MAX_DELTA=0.015625` should collapse the
   spread. The harness measured the pin as +1.5% on this subject, so it costs
   nothing to try.
2. **`process_pipeline_queue_system`**, already recorded at 68 ms mid-run.
   Periodic, main-thread, deliberate.

Neither is a regression. Both change what a hand-read FPS counter MEANS, and
the owner reads that counter to judge every change in this epic.

## Where 60 FPS actually stands, for the release's own subject

The epic's target is a `wfc_arena` 4v4 - eight hulls that fight, so the arena's
4.76 ms/ship applies, not the gallery's 2.4.

| | frame | FPS |
|---|--:|--:|
| today, 4v4 | 21.5 + 8 x 4.76 = 59.6 ms | ~17 |
| floor halved, per-ship to 3.0 | 10.8 + 24.0 = 34.8 ms | ~29 |
| floor to zero, every DRAWN cost gone | 0 + 8 x 2.00 = 16.0 ms | ~62 |

The last row is the bound, not a plan: 2.00 ms/ship is the measured non-drawn
half - physics, colliders, AI, health, integrity - which the epic protects.

**So 60 FPS at 4v4 is not reachable by presentation work alone.** 30 FPS is,
and it needs the floor and one more per-ship halving together. Recorded rather
than asked, per the plan's rule 6; it is a target question, not a change to
what the game does.
