# Notes: Run the rebuilt fleet as CI will and record the sprint's correctness+perf evidence

Goal in one line: exercise the rebuilt fleet as a WHOLE - the only place that
happens - read the resulting report, and write down what it says.

## Problem Statement

Each of the six preceding tasks proved its own category with
`probe run <category>`. Nothing had ever run them in ONE invocation, so
cross-category effects were unproven: the run policy misfiring, the aggregate
report, or a category that only fails under `--all`'s sequencing. The epic
(`20260802-115955:37,45`) still owed an inspectable probe report as the
v0.10.0 demonstration.

The pain is unproven-as-a-whole. It is NOT:

- new production code - this is a verification pass;
- the CI smoke gate, which is a different run (see below);
- a committed artifact - the report is read, not stored.

## Owner decisions (2026-08-05) - these override anything below

| Question | Decision |
|-|-|
| Where to run | `master`, in place. No sprout worktree. |
| Where the report goes | probe's DEFAULT dir, `probe-runs/` (gitignored). NOT committed. |
| Baseline comparison | DROPPED. The retained v0.7.0 numbers are other scenarios; nothing comparable survives. |
| The command | one plain probe run, default output. No `--baseline`, no custom `--out`. |
| CI budget / `timeout-minutes: 60` | Not investigated. CI measures it on push. |
| The smoke flake (below) | PARKED by owner call. Diagnosis recorded, no fix attempted. |

## The gate and the evidence are two different runs

"as CI will" implied `probe run --all --fps` is what CI runs. It is not. No
workflow invokes probe at all. CI's fleet gate is
`xvfb-run --auto-servernum cargo test -p nova-protocol --test examples_smoke
--features debug` (`.github/workflows/ci.yaml:108`).

| | Gate | Evidence |
|-|-|-|
| What | `cargo test --test examples_smoke` | `probe run --all --fps` |
| Who | CI, every PR | the owner, once, here |
| Proves | reaches Playing, no panic, no command errors | correctness + frame time |
| Fails the build | yes | no |

## What the run said

`nix develop --command xvfb-run --auto-servernum cargo run -p nova_probe -- run --all --fps`
-> exit 0, `aggregate OK`, 17/17 examples OK, artifacts in
`probe-runs/d832b690/` (gitignored).

Run policy confirmed EMPIRICALLY, not just read off `CATEGORY_POLICIES`:

- 17 examples in `--all` = 5 `sections/` + 3 `systems/` + 5 `ui/` +
  3 `stress/` + `scene_baseline`.
- `screenshots/` reported `NOT PROBED` and contributed nothing.
- `sections/`, `systems/`, `ui/` each printed their fps-pass skip reason.
- `stress/` (+ `scene_baseline`) were the only frame-time captures.

### Frame times - the numbers nothing else outlives `target/` to keep

900 frames each, vulkan, NVIDIA GeForce RTX 3060 Ti, 1280x720, `dev` profile.

| example | mean ms | p50 | p95 | p99 | max | mean fps | 1% low |
|-|-:|-:|-:|-:|-:|-:|-:|
| many_bodies | 19.47 | 19.70 | 22.83 | 25.16 | 31.59 | 51.4 | 39.7 |
| many_sections | 20.29 | 20.14 | 26.25 | 32.71 | 44.36 | 49.3 | 30.6 |
| many_projectiles | 35.14 | 23.47 | 121.48 | 223.59 | 325.30 | 28.5 | 4.5 |
| scene_baseline | 21.80 | 21.33 | 24.79 | 31.50 | 41.98 | 45.9 | 31.7 |

These ARE the baseline: there is no earlier comparable series.

### Two caveats on the evidence, both benign

- Every row reads `measured 5/6`. The unmeasured check is
  `fps_within_baseline`, `SKIPPED` for "missing capture or baseline" - no
  `--baseline` dir was passed and `probe-runs/a6d06220` matched none of the 17.
  Read the verdict WITH `measured`; 5/6 here is the missing baseline, nothing
  else.
- The run spans two commits. The owner committed `694ab2a3` (scripts-only) at
  09:05 while the run was in flight, so example 17 (`many_projectiles`)
  stamped `694ab2a3` and examples 1-16 stamped `d832b690`. Probe resolves the
  sha per example at run time; this is the in-place-on-master cost, not a
  probe defect.

## What the run surfaced

`many_projectiles` is a genuine outlier: p95 121 ms, p99 224 ms, max 325 ms
against a 23 ms median, and a 4.5 fps 1% low. Spikes, not a low mean. Nothing
gates on it - the only frame-time check is a baseline comparison, which is
skipped - so it passed silently. FILED rather than fixed here.

## The smoke flake - CONFIRMED, and it blocks this task's close

Filed as `20260805-091151`, moved into the v0.10.0 sprint at priority 84 under
the epic. It is NOT fixed here.

The owner hit an intermittent failure the same morning:

```
ERROR nova_autopilot::autopilot: autopilot: step `menu_newgame: release New Game`
  stalled after 90.0s (run 91.7s, state MainMenu)
test result: FAILED. 8 passed; 1 failed
```

It did not reproduce on the first attempt - a full `examples_smoke` run on
`d832b690` passed 9/9 in 86.45s, each run-based category taking >60s (so no
silent `DISPLAY`-missing skip). It DID reproduce on the workspace suite run,
in the same category but a different example, which is what proves it is the
shared `click_named` mechanism rather than one example's script:

```
autopilot: step `editor: release Sandbox` begins      06:18:53.986944
ERROR nova_autopilot::completion: harness completion: deadline (120s) expired
  with collectors still pending: ["autopilot"]        06:20:52.074566
example editor exited with Some(1)
```

Roughly 1 in 3 full runs (2 failures across 3 `examples_smoke` runs on
2026-08-05).

State stayed put, so the release never produced an `Activate`. Two candidates,
one root cause - the click beat has no OBSERVED precondition:

| # | Candidate | Mechanism | Log signature |
|-|-|-|-|
| 1 | Press lands before hover resolves | `click_named` warps the cursor and presses in the SAME frame (`crates/nova_autopilot/src/input.rs:157-158`); the picking backend raycasts a system later, so the widget never enters `Pressed` and the release emits no `Activate` | none - just the stall |
| 2 | Button not laid out yet | the preceding beat waits `frames(SETTLE)` = 10 (`examples/ui/menu_newgame.rs:100-102`), not an observed state; `resolve` then warns and returns WITHOUT pressing | `autopilot: click on \`New Game Button\` found no laid-out UI node with that Name` |

Candidate 2 is ELIMINATED: the reproduction's log carries no
`found no laid-out UI node` warn, so the node resolved and the press WAS
issued. Candidate 1 is the confirmed mechanism.

Both are the epic's own anti-pattern - advancing on a frame count instead of
observed state (`20260802-115955:32`). `editor.rs` is accidentally immune for
the hull CARD only, because a tooltip assertion hovers it in an earlier beat
(`editor.rs:146-171`) - its `Sandbox Button` click is not, which is what
failed. Every other `.on_enter(click_named(...))` call site carries the same
race.

## Out of band: the prelude bookkeeping tests

Removed in `2a8bd05b` on owner instruction, not as part of this task's plan.
`crates/nova_autopilot/tests/prelude.rs` maintained a hand-written `EXPORTED`
list beside the prelude's own `pub use` list, so adding a correctly
re-exported item went red until the duplicate caught up - which is what
`ui_node_rect` did. The file is now `tests/env_contract.rs`, keeping only the
env-name contract test.

## The closing run - 2026-08-05, after the flake fix

Re-run of both DoD commands once `20260805-091151` landed (`87bcb956`, "let a
driven run own the pointer"). Both green.

`probe run --all --fps` - exit 0, run id `cafae048`. All 17 probed categories
PASS every correctness check: `process_exit`, `run_completed`,
`reached_playing`, `invariants_held`, `log_clean`. Zero invariant violations,
zero panic/ERROR lines. The per-category policy held against the RUN again:
`screenshots/` NOT PROBED, `stress/` the only 6/6 (frame-time) category,
everything else 5/6 correctness-only.

`cargo test --workspace --features debug` - exit 0, 1543 passed, 0 failed,
2 ignored, 72 test binaries. `examples_smoke` 9/9, including
`catalog_matches_disk` and `every_category_has_a_probe_policy`. No
`ui_reach_playing_without_panic` flake - the failure that blocked this task.
The pre-fix categories that flaked, `menu_newgame` and `editor`, both passed.

### The aggregate WARN is not a regression

Probe auto-selected `probe-runs/d832b690` as baseline - this task's OWN
pre-fix run from the same morning. Deltas:

| category | delta vs d832b690 | status |
|-|-:|-|
| many_projectiles | -20.8% | PASS (improved) |
| many_sections | +1.7% | PASS |
| many_bodies | +14.2% | WARN |
| scene_baseline | +14.8% | WARN |

Both WARNs are the documented soft gate - "frame numbers are host-noisy;
reviewer judges". OWNER CALL, and the reason the gate is soft: these are NEW
examples, so there is no past to compare them against. `d832b690` is not a
reference point, it is the same sprint's first capture. The comparison is
noise by construction, not evidence of a regression. This does not gate the
close; the thing that gated it was the failing test, and that is fixed.

### Caveat on attribution

The evidence was gathered in the shared checkout while another session was
committing to `master` and running Bevy examples on the same host. HEAD moved
`7789c4c8 -> cafae048 -> 1b738932 -> 37943668` across the two runs, so the
runs attach to a commit range rather than one commit, and the host was not
quiet during the frame-time capture. Accepted by the owner - the correctness
verdicts are what this task needed, and those do not depend on a quiet host.
