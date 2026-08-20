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

## These readings turned out to be the TRUSTWORTHY instrument

Corrected 2026-08-20 after `notes-floor.md`. What follows replaces an earlier
reading of this page that derived a "floor" from these numbers.

The owner runs on a real display. **The harness ran under `xvfb-run`, which adds
about 13.7 ms of per-pixel CPU copy at 720p** - a software X server has no
scanout, so presenting is a memcpy of the window. So the harness's absolutes
were inflated and these hand-runs never were.

The floor lane then measured the same subject on a real display and landed on
top of these readings:

| ships | owner, by hand | lane, real display |
|--:|---|---|
| 1 | 36-48 FPS | 27.60 ms, **36 FPS** |
| 2 | ~30 FPS (1v1) | 34.82 ms, **29 FPS** |

**Two instruments, built independently, agreeing to within a frame at two ship
counts.** That is the cross-check that should have been run before any budget
was derived from the harness, and it was free the whole time.

## What the fitted intercept actually was

Both lines put ~20 ms at zero ships - 21.2 ms from the owner's 3/11/17 row above,
19.4 ms from the lane's 1/2/3 row. **The measured empty scene is 3.02 ms.**

Those are consistent. The intercept is not scene cost: it is a fixed cost that
appears the moment there is ONE hull, and a straight line fitted through ship
counts >= 1 reports it at x = 0. It is real, it is worth about 16 ms, and it now
has a name - `Prepare` + `PrepareMeshes`, building per-instance buffers and bind
groups over 986 mesh instances.

The cost curve is SUBLINEAR: first hull 24.6 ms, each further hull about 8.

## The 7 ms spread at one ship

Still unexplained, and the fixed-step amplifier is now ruled out: the lane
measured `fixed_steps min=0 max=1`, mean 0.19 on a real display. It never runs
two steps in a frame, so it cannot be alternating between one and two.

What remains is vsync interval quantisation on the owner's own display, which is
a property of the counter rather than of the game. It is a reason to judge
changes on harness numbers rather than the FPS counter, not a defect.

## Where 60 FPS stands

Measured, real display, frozen gallery: empty 3.02 ms, one hull 27.60, two
34.82, three 44.04.

**1v1 misses 60 FPS by 18.15 ms, and essentially 100% of that is per-ship.**
There is no scene overhead worth cutting - 3.02 ms is 18% of the whole budget
and the largest single item inside it measures 1.15 ms.
