# NOTES - 20260731-170335

KISS pass on the 15 HUD chrome and objective-surface files. Comment axis
across all 15; structure axis is one justified exception plus no splits.

## Structure axis

Prod/tests split measured at each file's `mod tests` boundary - prod is
everything above its `#[cfg(test)]` attribute. That attribute is the file's
first `#[cfg(test)]` in 14 of the 15; `mod.rs` is the exception, where two
`#[cfg(test)] mod <rig>;` declarations sit at lines 47 and 52 while the
`#[cfg(test)]` guarding `mod tests` is at 965.

| File | Lines | Prod | Tests | Concern |
|-|-|-|-|-|
| keybind_dock.rs | 1911 | 836 | 1075 | the contextual keybind dock + the anchored verb cues (one sizing/paint path, see the exception below) |
| mod.rs | 1463 | 964 | 499 | the HUD umbrella: tier/visibility vocabulary, `NovaHudPlugin`, the spawn/despawn observers |
| objective_stack.rs | 1049 | 551 | 498 | the top-centre objective notification stack |
| velocity.rs | 887 | 622 | 265 | the direction sphere (velocity + gravity sources) |
| comms_panel.rs | 772 | 416 | 356 | speaker-attributed story text |
| maneuver_instruments.rs | 704 | 435 | 269 | the engaged maneuver's chips + holo ring |
| objective_markers.rs | 578 | 342 | 236 | the gold world-anchored objective chip |
| flight_status.rs | 551 | 338 | 213 | the diegetic speed/mode chips |
| objective_feedback.rs | 530 | 281 | 249 | objective change cues (chime, blip, ghosts) |
| beacon_chips.rs | 472 | 319 | 153 | the cyan nav-beacon chip |
| holo_instruments.rs | 406 | 261 | 145 | world-space holo geometry |
| chip_layout_rig.rs | 363 | 298 | 65 | test-only live-tree layout rig |
| key_glyphs.rs | 340 | 271 | 69 | the label -> keycap-art mapping |
| readout.rs | 312 | 214 | 98 | the scenario-variable readout strip |
| situation.rs | 221 | 118 | 103 | the per-frame contextual situation sense |

Nothing was split. Every file below the threshold is one concern, and per the
epic rubric length alone is not a reason to cut.

`mod.rs` at 1463 is under the 1500 threshold, so it needs no exception. It is
also genuinely one concern - the HUD umbrella - and its prod half is 964
lines of vocabulary types plus one plugin `build`.

### Exception: keybind_dock.rs stays whole at 1911 lines

DoD 4 allows an over-1500 file when NOTES justifies it as one cohesive
concern. This is that justification.

The dock and the anchored verb cues are not two concerns that happen to share
a file - they share the CODE:

- `keycap_sizing_tests` (the second `#[cfg(test)]` module, from line 1667)
  asserts that the cue and the dock size their keycaps through ONE path. Split
  the cues out and that test either dies or has to reach across a module
  boundary into private items.
- The cue tests reuse the dock's rig (`glyph_app`, `all_available_hints`,
  `chips`) - `keycap_sizing_tests` imports `super::tests::{...}` directly.
- Extracting a shared rig or a shared sizing helper to serve two modules
  would be a NEW ABSTRACTION, which this pass explicitly forbids ("moves,
  renames and deletions only - no new abstractions").

So the honest options were "split and break the shared-sizing guarantee" or
"split and build an abstraction the pass forbids". Neither is worth it. Note
also that 1075 of the 1911 lines are tests; the prod half is 836.

If the file is split later, the shared keycap sizing path is the seam to
extract first, and it should be a task of its own with the sizing test moved
onto the extracted path.

## Comment axis: what was cut

Same three categories as the previous child (20260731-170329). Measured over
the whole branch change to `crates/` (`git diff -U0 8eabd5d5^ -- crates/`,
which spans the pass commit and the review-round fixes): 240 insertions, 271
deletions, 134 hunks across the 15 files, and 140 removed lines carrying a
tatr ID, a bare `YYYY-MM-DD` date, a `review Rn.n` clause or a `docs/`
pointer. Adding `playtest round` to the pattern takes it to 143.

- **tatr-ID provenance clauses** - `(task 20260728-175747)`,
  `(tasks 20260724-134312 / 20260724-161545)`, `(spike 20260728-175742
  note)`, `(review R1.4 of 20260712-093831)`. The ID belongs in the task
  record, not in the source.
- **Bare-date and playtest provenance** - `(playtest 2026-07-12)`,
  `(owner playtest 2026-07-30)`, `(owner decision, 2026-07-30 plan gate)`,
  `(user request 2026-07-10)`, `(playtest round 4)`, `measured ... on
  2026-07-30`, `(seen on the lifeline walk, 2026-07-29)`. These carry no
  tatr ID, so the previous child's narrower grep could not see them - the
  R1.4 finding that widened DoD 3 for this task.
- **`(review Rn.n)` clauses** and **record pointers** - one
  `tasks/20260730-122843/DECISION.md` pointer in `keybind_dock::chip_visible`
  and one `see the task's DECISION.md` in `objective_stack`. In every case
  the constraint prose the clause hung off was kept; only the pointer went.
  No `docs/spikes/*.md` pointers existed in this scope (the previous child
  cleared the dead ones in its own).

One dead-history block comment was deleted outright, in `mod.rs`: a
four-line note that the always-on compact objectives panel and its four
functions "were REMOVED in task 20260724-134312". The surviving fact - that
objectives surface via the objective stack and the NOVA OS monitor, not a
panel - is stated at the `ObjectivesPlugin` registration a few lines above,
so nothing was lost.

## Promoted to NOTE:

Twelve comments guard a value, a schedule slot or a non-obvious ordering, so
per AGENTS.md they were kept and marked rather than pruned: eleven `NOTE:`
and one `TODO(20260710-231927):` - written with the ID in the parens, so a
bare `grep 'TODO:'` will not find it. This is the full marker inventory for
the scope:

| Site | What it guards |
|-|-|
| `mod.rs:241` | the contextual layer is bounded on BOTH sides of the widget drivers |
| `mod.rs:266` | visibility enforcement must run after the screen-indicator projection |
| `mod.rs:306` | `NovaOsMapPlugin` must be added AFTER `NovaOsPlugin` (app registry) |
| `mod.rs:311` | `NovaOsShipPlugin` must be added AFTER `NovaOsPlugin` (command registry) |
| `mod.rs:521` | `try_remove`, not `remove` - teardown despawns in the same flush |
| `mod.rs:559` | the gravity sphere drives its own visibility; the restore must not overrule it |
| `key_glyphs.rs:25` | a runtime-rebound key cannot sit behind the one-shot preload collection (the `TODO(20260710-231927):`, see below) |
| `objective_stack.rs:433` | fade and breath are both FRACTIONS of the rest tone, never absolute alpha |
| `objective_stack.rs:601` | the emphasis driver must stay in PostUpdate, not the Update chain |
| `holo_instruments.rs:144` | the ribbon's ship end reads the eased root `Transform`, not avian `Position` |
| `objective_feedback.rs:13` | `GameObjectives` is write-on-diff, so `resource_changed` means a real change |
| `readout.rs:194` | each readout row is a member of the HUD chip family |

The last five were promoted by the earlier files' pass; the first six plus
`key_glyphs` came from this session's `mod.rs`/`keybind_dock.rs` work and the
sweep behind it.

## DoD 3: one deliberate reference

The widened grep (tatr IDs, bare `YYYY-MM-DD` dates, `review Rn.n`, `docs/`
pointers) returns exactly ONE hit over the 15 scoped files:

- `key_glyphs.rs:25` - `TODO(20260710-231927)`: a runtime-rebound key needs a
  `server.load` that cannot sit behind the one-shot preload collection.
  20260710-231927 (keybind hint icons and key remapping) is OPEN, and the
  epic rubric says deferred work keeps its tatr ID as a
  `TODO:`/`FIXME:`/`BUG:`. The first cut of this pass demoted it to a bare
  `NOTE:` and dropped the ID, which the rubric does not allow for deferred
  work; review R1.1 caught it.

Every other constraint that used to lean on an ID was rewritten to state
itself.

## What was deliberately NOT cut

Following both earlier children: a literal read of the rubric would make
every bare `//` comment fluff, but read one by one they are overwhelmingly
the categories the rubric says to keep - why a branch exists, what an ECS
ordering buys, what a magic constant means in world units. Those stay.

Four rewrapped comment blocks were reflowed beyond the minimum edit
(`comms_panel` and `objective_markers` module docs, `objective_markers`'s
diamond doc, `maneuver_instruments`'s radius-spoke bullet) because the
substitution had left lines past the file's ~80-col fill. One more,
`keybind_dock`'s `EMPHASIS_PERIOD_SECS` doc, was reflowed so its sentence
dash is no longer line-initial - a line starting `- ` is parsed by CommonMark
as a list item and renders as a stray bullet (the R1.1 finding on the
previous child).

## Defects found

One, deferred per the task constraint: `holo_instruments`'s module doc says
the trajectory ribbon is a straight line "until the arrival solve becomes
gravity-aware", but the gravity-aware arrival task (20260710-193500) is
CLOSED, so the doc may already be describing a superseded state. Checking
that means reading the autopilot, which is outside this pass. Filed as
20260731-232634.

`cargo check` still emits the 4 `ambiguous import visibility` warnings from
`nova_os_map/mod.rs` and `nova_os_ship/mod.rs`, already filed as
20260731-205553 by the previous child and outside this scope.

## Verification

| Proof | Result |
|-|-|
| `cargo check --workspace --all-targets` | green (exit 0) |
| `cargo fmt --check` | clean (exit 0) |
| DoD 3 widened grep over the 15 files | zero hits (exit 1) |
| DoD 4 `wc -l` | one file over 1500: `keybind_dock.rs` at 1911, justified above |
| `cargo test --lib -p nova_gameplay hud::` | 307 passed, 0 failed |
| `cargo test --lib -p nova_gameplay` | 785 passed, 0 failed, 1 ignored (exit 0) |
| non-comment lines in `git diff -U0` over `crates/` | 1, a deleted blank line where the dead-history block came out |

That last row is the strongest no-behavior-change evidence available: apart
from one blank line, every changed line in the diff starts with `//`, `///`
or `//!`, so no executable line moved and no test was added, renamed or
removed. The 785-test lib run is the same suite, passing, on this tree.

Unlike the previous child, the tests DID run here: this branch carries the
link-RAM fix (20260731-210651), so `cargo test --lib` fits in the box.

Not run: the workspace suite. It is CI's job on the PR, and the known
pre-existing `nova_assets` failure
(`scenario::shakedown::tests::an_early_derelict_kill_skips_to_the_fight`,
filed as 20260731-215407) makes a local full run exit 101 regardless of this
diff.
