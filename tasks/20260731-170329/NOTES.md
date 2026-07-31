# NOTES - 20260731-170329

KISS pass on the 12 combat-readout HUD widgets. Comment axis only; the
structure axis is a justified no-op (below).

## Structure axis: no splits, deliberately

DoD 4 (`wc -l`, no file over 1500) passes outright - the largest is
`screen_indicator.rs` at 1428. Per the epic rubric and child 1's lesson,
splitting is driven by multiple clear concerns, not by line count. Each file
here is one widget: a bundle constructor, a plugin, its systems, and its
tests. Nothing was cut.

Prod/tests split measured at the last `#[cfg(test)]` in each file.

| File | Lines | Prod | Tests | Concern |
|-|-|-|-|-|
| screen_indicator.rs | 1428 | 604 | 824 | the world-anchor -> UI-node projection widget every other file consumes |
| target_inset.rs | 1387 | 812 | 575 | the RTT target portrait (second camera + frame styling) |
| ammo_readout.rs | 1248 | 603 | 645 | per-mount ammo counts + the debug gate |
| torpedo_target.rs | 1188 | 559 | 629 | torpedo lock readout + wind-down decay |
| lock_crosshairs.rs | 651 | 406 | 245 | lock reticle |
| edge_indicators.rs | 597 | 373 | 224 | offscreen arrows + labels |
| turret_lead.rs | 441 | 186 | 255 | lead pips |
| allegiance_markers.rs | 433 | 279 | 154 | IFF markers |
| lock_dwell_ring.rs | 346 | 185 | 161 | dwell progress ring |
| emphasis.rs | 311 | 192 | 119 | shared emphasis styling |
| component_lock.rs | 310 | 183 | 127 | per-section markers |
| item_highlights.rs | 256 | 172 | 84 | pickup highlights |

No prod half reaches 850 lines; the largest is `target_inset.rs` at 812, and
the longest file overall (`screen_indicator.rs`) is 604 prod lines against
824 of tests. That prod half is a single public widget API plus the systems
implementing it; extracting a `mod` would only move the seam, not remove
one.

## Comment axis: what was cut

Three categories, ~64 sites across 12 files:

- **Dead `docs/spikes/*.md` pointers (6 sites)**, in the module docs of
  `item_highlights`, `component_lock`, `edge_indicators`,
  `screen_indicator`, `ammo_readout`, `target_inset`. `docs/` holds only
  `README.md`; none of the six referenced files exist anywhere in the tree.
  Dead **as paths** - the spike content itself survives as
  `tasks/<id>/SPIKE.md` - so the pointer was the rot, not the record.
  A seventh, a bare `(DECISION.md)` in `torpedo_target.rs`, went with them:
  unresolvable as written (no task path), though a matching record does
  exist at `tasks/20260730-123009/DECISION.md`.
- **tatr-ID and date provenance clauses (~58 sites)** - `(task
  20260728-175747)`, `(tasks ... / ...)`, `spike 20260713-110039`,
  `(playtest 2026-07-13)`, `(review R1.1)`, `(user decision 2026-07-13, task
  ...)`. The rubric puts this in the task record, not the source. Four of
  these survived the first sweep because they carry no tatr ID and so are
  invisible to the DoD 3 grep - two `(playtest 2026-07-13)` and two
  `(review Rn.n)` - and were caught by review (R1.4).
- **Spike question-labels** - `Q4a`, `Q5a`, `Q6a`, `Q7a`, `Q8a`, `B1`, `F4`,
  `F5`, `F7`, and the spike-label uses of `F11`. Meaningless without the
  deleted spike docs.

Three test assertion strings lost their labels with them, the only
non-comment lines in the whole diff:

- `"the panel holds through the beacon (Q4a)"` -> `... beacon"`
- `"hot: the frame goes lock-red (Q5a color)"` -> `... (color half)"`
- `"hot: the armed ticks appear (Q5a shape)"` -> `... (shape half)"`

**`F11` in `ammo_readout.rs` was kept everywhere it appears.** There it is
the real `KeyCode::F11` (`const DEBUG_TOGGLE_KEY: KeyCode = KeyCode::F11;`),
not a spike label. Easy to strip by pattern; checked by hand.

## DoD 3: zero remaining hits

`grep -rnE '//.*[0-9]{8}-[0-9]{6}'` over the scope returns nothing, so there
is no deliberate-reference list to record. No surviving comment needed an ID
to make sense; the four that carried a real constraint were rewritten to
state it directly (below).

## Promoted to NOTE:

Four comments guard a value or a schedule slot, so per AGENTS.md they were
kept and marked rather than pruned:

- `turret_lead.rs` build - pips consume THIS frame's intercept; Update put
  the pip a frame behind a moving target.
- `edge_indicators.rs` build - the label mirrors visibility the widget writes
  in PostUpdate; mirroring from Update lags a frame.
- `screen_indicator.rs` build - projection must sample the same camera pose
  the frame renders with; bcs moves the chase camera in PostUpdate.
- `ammo_readout.rs` build - the debug mirror is UNGATED on purpose, to stay
  in phase with nova_debug's equally ungated `toggle_debug_mode`. Explicit
  "do not re-add a state gate here".

`torpedo_target::wind_down_alpha`'s doc gained a `NOTE:` that it is a
function of the decay clock alone, deliberately not of session time.

## What was deliberately NOT cut

Following child 1's precedent (`20260731-170322/NOTES.md`): a literal read of
the rubric would make every bare `//` comment fluff, but reading them, they
are overwhelmingly the categories the rubric says to keep - why a branch
exists, what an ECS ordering buys, what a magic constant is in world units.
Child 1 landed with 486 bare comments and 1 NOTE marker; this pass follows
the same line.

## Verification

| Proof | Result |
|-|-|
| `cargo check --workspace --all-targets` | green (exit 0) |
| `cargo fmt --check` | clean (exit 0) |
| item-name multiset vs master | identical, 485 items |
| non-comment lines in `git diff -U0` | 3, all test assertion strings above |
| plugin `build` bodies vs master | byte-identical (no executable line changed) |
| `cargo doc -p nova_gameplay --no-deps --document-private-items` | 14 warnings on master AND on this branch; the single in-scope one (`ammo_readout` public doc links to private `sync_ammo_gate`) is pre-existing |
| `cargo test -- --list` name diff | **SKIPPED** - see below |

The doc baseline needed a redo. The first one used a loose `grep -c
'^warning'` against a warm cache and reported 2; the honest measurement is
`git stash` -> `touch crates/nova_gameplay/src/lib.rs` -> rerun -> `git stash
pop`, which showed master at the same 14. A baseline taken with a different
extraction than the after-run is not a baseline.

The test-name-list proof was not obtained. Running the test binaries locally
is out per the standing directive (the link step OOMs this box); CI runs the
suite on the PR. Since no executable line changed and no `#[test]` was added,
moved, or renamed, the name list cannot have moved - but that is an argument,
not a measurement, and is recorded as such.

## Defect found, deferred to backlog

`cargo check -p nova_gameplay --all-targets` emits 4 `ambiguous import
visibility` warnings, all from child 1's landed files and outside this scope:

```
crates/nova_gameplay/src/hud/nova_os_map/mod.rs:45:21   (x2)
crates/nova_gameplay/src/hud/nova_os_ship/mod.rs:55:39  (x2)
```

and `cargo doc` warns that `ammo_readout`'s public module doc intra-doc-links
the private `sync_ammo_gate` - pre-existing on master, and the exact shape of
the promoted `rustdoc-no-public-to-private-intra-doc-link` lesson.

Per the task constraint ("any defect found becomes a backlog task, not a fix
in this commit") all of these are filed as 20260731-205553, not fixed.
