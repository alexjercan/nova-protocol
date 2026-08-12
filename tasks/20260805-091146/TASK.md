# many_projectiles spikes: p99 224ms against a 23ms median

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog,perf,examples,wontdo

## Story

Surfaced by the v0.10.0 fleet run (`20260804-095507`), which was the first
`probe run --all --fps` over the rebuilt `stress/` category.

`many_projectiles` is the fleet's one frame-time outlier, and its problem is
SPIKES, not a low mean. 900 frames, vulkan, RTX 3060 Ti, 1280x720, dev:

| example | mean ms | p50 | p95 | p99 | max | mean fps | 1% low |
|-|-:|-:|-:|-:|-:|-:|-:|
| many_bodies | 19.47 | 19.70 | 22.83 | 25.16 | 31.59 | 51.4 | 39.7 |
| many_sections | 20.29 | 20.14 | 26.25 | 32.71 | 44.36 | 49.3 | 30.6 |
| **many_projectiles** | **35.14** | **23.47** | **121.48** | **223.59** | **325.30** | **28.5** | **4.5** |
| scene_baseline | 21.80 | 21.33 | 24.79 | 31.50 | 41.98 | 45.9 | 31.7 |

Its median (23.47 ms) sits with the others. Its p95 is 5x the p95 of every
other run, and the 4.5 fps 1% low means the worst frames are ~20x the median.
Something periodic is stalling - projectile spawn/despawn batching, collision
broadphase growth, or an allocation cliff are the obvious suspects, none
confirmed.

Nothing gates on this. The only frame-time check is `fps_within_baseline`,
which is `SKIPPED` on every example because no baseline is stored, so the run
went `OK` with the spikes in it.

## Notes

- Evidence is NOT in the repo by design (`20260804-095507/DECISION.md`); the
  numbers above are the record. Regenerate with
  `nix develop --command xvfb-run --auto-servernum cargo run -p nova_probe -- run many_projectiles --fps`.
- Worth deciding alongside this: whether `stress/` gets a stored baseline so
  this class of regression can fail a check rather than be read off a table.

## Closed 2026-08-12: wontdo, resolved by intervening work

The stored run at c2dde47d (same host/GPU/resolution/profile) shows
p95 28.7 / p99 40.4 / 1% low 24.8 fps - in family with the other stress
examples. The baseline question landed via 20260808-195933:
`fps_within_baseline` now compares stored baselines in CI.
