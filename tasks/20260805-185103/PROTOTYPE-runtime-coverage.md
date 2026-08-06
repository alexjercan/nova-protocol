# Prototype: runtime coverage - the example declares by WIRING

- DATE: 20260806
- STATUS: PROPOSED (no step yet; see "Proposed step" below)
- TASK: 20260805-185103
- TAGS: tooling, testing, refactor

Owner's rule, verbatim: *if it adds the frametime plugin it does frametime,
simple as that*.

Round 2 of this document. An out-of-context reviewer read draft 1 against the
source and found two blockers; both are folded in below and marked `[R1]`. The
sections that changed are "The check side" (the resolution is a 2x2, not a
3-way), "The verdict fold", "Retiring `NOT_PROBED`", and everything under
"What the first draft missed".

## The problem, stated once

`nova_probe` is a set of small plugins that each gather one kind of evidence
(timeline, invariants, frame time). What an example can be JUDGED on is exactly
which of those it wires. Today nothing reads the wiring. Coverage is declared
by hand in four places instead:

| Table | Location | Reads the example? |
| --- | --- | --- |
| `CATEGORY_POLICIES` | `crates/nova_probe/src/catalog.rs:144` | no |
| `NOT_PROBED` | `crates/nova_probe/src/bin/probe/native/spec.rs:8` | no |
| `NOT_SMOKED` + the smoke lists | `tests/examples_smoke.rs:32-88` | no |
| the `add_plugins` lines | `examples/*/*.rs` | IS the truth, unread |

Two consequences, both live today:

- **`frame_time` contradicts the wiring in 12 files.** All 17 probed examples
  call `add_plugins(nova_probe::nova_frametime())`, but only `stress/` is
  `frame_time: true`, so `clean_pass_env` never sets `NOVA_PERF` for the other
  12 (`native/env.rs:131`). Twelve inert plugin lines.
- **Green and wrong is reachable by name.** `probe run screenshot_ui` is
  allowed (`spec.rs:102`). No timeline is produced, so `run_completed`,
  `reached_playing` and `invariants_held` return `Skipped`, `process_exit` and
  `log_clean` pass, and `overall_verdict` prints **OK** at `measured 2/6`. The
  hand tables are the only thing keeping that out of CI.

The information needed to fix this is already half-present: `RunManifest`
carries `armed_timeline` / `armed_invariants` / `armed_fps`, which is PROBE's
side of the handshake (what probe turned on). The EXAMPLE's side (what it
wired) does not exist. `timeline_skip_detail` (`run_report/checks/mod.rs:111`)
already reasons about the difference in prose, then throws it away by returning
`Skipped` either way.

## The mechanism

One new module, `crates/nova_probe/src/contract.rs`. Every probe plugin
declares itself into a resource at `build()` time, BEFORE its own arming
guard; one Startup system writes the resource out beside the run's other
artifacts. Probe reads it back with the rest.

```rust
//! crates/nova_probe/src/contract.rs  (sketch)
//!
//! What the EXAMPLE claims, declared by the plugins it wires. Probe's
//! manifest says what probe ARMED; this says what the app can produce.
//! The pair is what lets a report tell "makes no such claim" (N/A) apart
//! from "claimed it, was armed, and produced nothing" (a failure).

/// One evidence surface, one plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    /// `nova_timeline()` - the run-timeline recorder.
    Timeline,
    /// `nova_invariants()` - the continuous invariant checks.
    Invariants,
    /// `nova_frametime()` - the frame-time capture.
    FrameTime,
}

/// Env param (via `perf_param`, so `NOVA_PERF_CONTRACT`) naming the path
/// the contract is written to. Unset (a hand-run example) writes nothing.
pub const CONTRACT_PARAM: &str = "contract";

/// The capabilities this app wired.
#[derive(Resource, Default, Debug)]
pub struct ProbeContract {
    declared: BTreeSet<Capability>,
}

/// Declare `capability`. Called from a plugin's `build`, ABOVE its arming
/// guard: wiring is the claim, arming is probe's answer to it.
pub fn declare(app: &mut App, capability: Capability) {
    if !app.world().contains_resource::<ProbeContract>() {
        app.init_resource::<ProbeContract>();
        app.add_systems(Startup, write_contract);
    }
    app.world_mut()
        .resource_mut::<ProbeContract>()
        .declared
        .insert(capability);
}

/// Write `probe-contract.json` once, at Startup, via temp-file + rename so
/// a watcher can never read a half-written file. Failing to write is an
/// ERROR, never silence: a lost contract would read as "claims nothing".
fn write_contract(contract: Res<ProbeContract>) { /* ... */ }
```

The reviewer confirmed the mechanism itself is sound on bevy 0.19: `build`
already holds `&mut App`, `world()`/`world_mut()` at build time are fine,
`add_systems(Startup, ..)` works before or after `DefaultPlugins`, the
`contains_resource` guard makes repeated `declare` calls idempotent, no example
builds a second `App` at runtime, and nothing here uses `SubApp`.

The three plugin diffs are one line each:

```diff
 impl Plugin for RunRecorderPlugin {
     fn build(&self, app: &mut App) {
+        // Declared by WIRING, not by arming: an example that adds the
+        // recorder claims a timeline even on a run that did not arm one.
+        contract::declare(app, Capability::Timeline);
         let Some(path) = self
             .out
             .clone()
             .or_else(|| perf_param(TIMELINE_PARAM).map(PathBuf::from))
         else {
             return;
         };
```

```diff
 impl Plugin for FrameTimePlugin {
     fn build(&self, app: &mut App) {
+        contract::declare(app, Capability::FrameTime);
         if !perf_armed() {
             return;
         }
```

```diff
 impl Plugin for InvariantsPlugin {
     fn build(&self, app: &mut App) {
+        contract::declare(app, Capability::Invariants);
         let Some(_) = perf_param(INVARIANTS_PARAM) else {
             return;
         };
```

Probe's side gets simpler, not larger: `clean_pass_env` keeps arming timeline +
invariants unconditionally and adds
`NOVA_PERF_CONTRACT=<out>/probe-contract.json`. An unwired plugin is inert, so
arming costs nothing. `fps_skip_reason` and the whole `CategoryPolicy` type go
away: the answer to "does this example make a frame-time claim" is no longer a
category opinion, it is whether the example called `nova_frametime()`.

## The check side: a 2x2, not a 3-way `[R1]`

Draft 1 resolved each check as `NotDeclared | DeclaredButAbsent | Present` and
failed the run on `DeclaredButAbsent`. That is wrong, and wrong in the common
case: probe deliberately does not arm every declared surface.

- `probe run scene_baseline` without `--fps` never sets `NOVA_PERF`
  (`native/env.rs:127-138`), so a declared `FrameTime` produces no
  `frametime.csv`.
- Sweep cells strip the recorder and invariants on purpose
  (`native/run.rs:149-154`), and the fps pass strips them again
  (`run.rs:188`).

Under draft 1 all three of those FAIL. The missing axis is the one the manifest
already carries, so `armed_timeline` / `armed_invariants` / `armed_fps` STAY:

| Declared (example) | Armed (probe) | Artifact | Outcome |
| --- | --- | --- | --- |
| no | - | - | **N/A** - makes no such claim, naming the capability |
| yes | no | - | **N/A** - this run did not ask for it, citing the manifest |
| yes | yes | absent | **FAIL** - armed and silent |
| yes | yes | present | graded as today |

```rust
// crates/nova_probe/src/run_report/checks/reached_playing.rs  (sketch)
pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    match artifacts.resolve(Capability::Timeline) {
        Input::NotDeclared => Check::not_applicable(NAME, THRESHOLD, NotDeclared(Timeline)),
        Input::NotArmed => Check::not_applicable(NAME, THRESHOLD, NotArmed(Timeline)),
        Input::ArmedButAbsent => Check::fail(NAME, THRESHOLD, "..."),
        Input::Present(timeline) => { /* ... grade as today ... */ }
    }
}
```

`CheckStatus::Skipped` is replaced by a status carrying WHY, so the fold can
act on it and the report can print the right word:

```rust
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    /// The check does not apply to this run, with the reason as a VALUE.
    NotApplicable(NotApplicable),
}

pub enum NotApplicable {
    /// The example wires no such plugin - it makes no claim.
    NotDeclared(Capability),
    /// Wired, but this run did not arm it (no `--fps`, a sweep cell).
    NotArmed(Capability),
    /// An operator-supplied input was not given (`--baseline`).
    InputNotSupplied(&'static str),
    /// An input WAS supplied but cannot be compared (a baseline sharing no
    /// run labels). Never a pass - see the fps note below.
    InputNotComparable(&'static str),
}
```

`timeline_skip_detail`'s reasoning is not deleted, it is promoted: its two
branches are exactly `NotDeclared` and `NotArmed`, as values instead of prose.

## The verdict fold `[R1]`

Draft 1's fold could still print OK for a run that graded nothing: its only
zero-coverage guard was "contract declares nothing -> UNPROBEABLE", so a run
that declared three capabilities and graded none of them (every sweep, and
every pre-contract run dir) fell through to the final `else`.

| Condition | Verdict |
| --- | --- |
| any `Fail` | FAIL |
| any `Warn` | WARN |
| nothing graded at all | NO_DATA (kept, unchanged) |
| contract declares nothing | UNPROBEABLE |
| no DECLARED capability graded | UNPROBEABLE |
| every declared+armed capability graded Pass | OK |

`measured_count` and the `"measured": "n/total"` field STAY. The skill contract
tells agents to read verdict together with measured
(`.claude/skills/probe/SKILL.md:126,136`), and deleting the field to make a
point would break the only consumer that behaves correctly today. Its
definition tightens: measured counts checks that produced a judgement, and N/A
is not one.

`process_exit` and `log_clean` need no capability: they read the manifest and
the captured stdio, which probe owns for every run.

## What this deletes

| Deleted | Because |
| --- | --- |
| `CategoryPolicy`, `CATEGORY_POLICIES`, `category_policy` | the example answers both axes itself |
| `every_category_has_a_probe_policy` (`examples_smoke.rs:195`) | nothing left to be total over |
| `fps_skip_reason`, `example_fps_skip_reason` (`native/env.rs:25-51`) | the reason is now `NotDeclared(FrameTime)` |
| `RunManifest::fps_skipped` | same |
| 12 inert `nova_frametime()` lines | wiring now MEANS capture, so the ones that do not want it must stop asking |

`RunManifest::armed_*` STAYS `[R1]`. `catalog.rs` keeps the `[[example]]`
parser - spec resolution still needs to know what an example is. It loses every
opinion about what probe does with one.

## Retiring `NOT_PROBED` - SETTLED: option 3, shipped in `c62436a8`

`render_scale_shot` must not be launched by `--all`: it only exits under
`NOVA_SHOT`, so under probe's Xvfb it runs to the deadline. Draft 1 proposed
killing the child as soon as an empty contract is observed. The reviewer showed
that does not hold up as stated:

- The writer is registered BY `declare`, so an example that wires nothing never
  writes an empty contract - the observable is an ABSENT file, which is also
  what you see one millisecond after spawn, after a panic in `DefaultPlugins`,
  and after an Xvfb failure. "No contract after N seconds" is a timeout
  constant replacing a name table, and it kills a slow cold Startup under
  llvmpipe.
- `run_supervised` has exactly two outcomes, `Completed` and `TimedOut`
  (`native/supervise.rs:100-113`), both of which land in
  `PassRecord { success, timed_out }`. An abort recorded as either one reads as
  `process_exit` FAIL, so "UNPROBEABLE" would also print FAIL.
- `render_scale_shot`'s actual property is "no self-ending autopilot", which no
  `Capability` in the enum expresses. Wiring `nova_timeline` into it would make
  it "probeable" and it would still time out.
- Retiring the list puts it into `--all`, where a non-OK row makes
  `aggregate_exit` return failure (`aggregate.rs:172-179`, `sweep.rs:275-280`).

Three ways out, owner picks:

1. **Keep one launch-side opt-out.** Honest, one row, and it survives the
   deletion of everything else. The claim it encodes ("not self-ending") is
   genuinely static.
2. **Add a `SelfEnding` capability** declared by the autopilot/completion
   layer, plus `RunOutcome::Unprobeable` and a bounded contract wait. Fully
   runtime, and it makes the property explicit instead of implied - but it
   needs a third outcome through the supervisor and an aggregate-exit rule.
3. **Let it fail.** `--all` is non-green by construction.

Recommend 2 if the supervisor change is acceptable, else 1. Do not ship draft
1's version.

**Owner picked 3.** `NOT_PROBED`, its `resolve_spec` parameter, the
`excluded_reason` branch and the sweep's printed note are deleted; only
`NOT_PROBED_CATEGORIES` remains. The two consequences the reviewer listed are
accepted rather than answered: an example that cannot survive a probe run FAILS
in the report instead of being listed away, and `--all` may be non-green by
construction. The fleet is unchanged TODAY - `render_scale_shot` was the list's
only member and it lives in `screenshots/`, which `--all` already skips by
category - so the change is latent until an unprobeable example lands in a
probed category. Option 2 stays available if that day comes: nothing here
forecloses `SelfEnding` + `RunOutcome::Unprobeable`.

## What the first draft missed `[R1]`

1. **The wasm split was wrong.** `capture` is compiled for both targets
   (`lib.rs:84`), so `FrameTimePlugin::build` calling `contract::declare` means
   `contract` cannot be native-only. It needs a wasm stub whose `declare` is a
   no-op, in the shape `recorder` already uses (`lib.rs:105-137`). Separately,
   a web run has no filesystem and therefore never writes a contract
   (`native/web.rs:83-148`): the web path must be handled explicitly, not left
   to read as "declares nothing".
2. **Backward compatibility.** Every existing `probe-runs/<sha>/<example>/`
   dir has no `probe-contract.json`, and `probe report` re-renders those
   (`native/report.rs:33-70`). ABSENT must mean "pre-contract run dir" and must
   not collapse into "declares nothing". `RunArtifacts.contract` is
   `Option<ProbeContract>` for exactly that reason.
3. **Stale contracts.** `RUN_ARTIFACTS` (`native/run.rs:29-41`) does not list
   `probe-contract.json`, and probe reuses a run dir at the same commit. A run
   whose child dies before Startup - or an example that just lost its
   `nova_frametime()` line in this very step - would inherit the previous run's
   claims. Add it to the list.
4. **Multi-pass provenance.** Clean, sweep cells and the fps pass all target
   one out dir. Only `clean_pass_env` sets `NOVA_PERF_CONTRACT` (the trace and
   samply passes must not), and the write is temp-file + rename, so the rule is
   "the clean pass owns the contract".
5. **Capabilities the enum does not cover.** `probe_marker` (and with it the
   27-slug section invariant roster that `examples_smoke.rs:324-432` is
   currently the only thing pinning - and step 6 deletes that file), the
   trace/profile pass, and `nova_screenshot`. The first is the one to decide:
   markers are the strongest correctness claims in the repo. The other two are
   deliberately out of the model and the doc should say so.
6. **`log_clean`'s reach.** Risk "a failed contract write is ERROR and
   `log_clean` catches it" holds only for logs the report loads: `run.log`,
   `fps-run.log`, `run-*.log` (`artifacts.rs:61-87`). `trace-run.log` and
   `samply-run.log` are excluded by design.
7. **`fps_within_baseline` has a third N/A reason**, hence
   `InputNotComparable`: a supplied baseline that shares no run labels
   (`checks/fps_within_baseline.rs:51-58`). It must never become Pass. Note
   probe often supplies a baseline automatically
   (`sweep.rs:82-106`), so "the operator did not supply one" is rarely the real
   reason.
8. **The contract is the BUILD's claim, not the file's.** Every example wires
   probe inside `#[cfg(feature = "debug")]` (`examples/ui/editor.rs:52-59`).
   Probe always builds `--features debug` (`run.rs:124`), so this is a doc
   point rather than a bug, but it belongs in the module doc.
9. **`NOVA_PERF_CONTRACT` is process-global env.** A stray value in a dev shell
   makes every hand-run example write a contract. Give the contract the same
   explicit-path override `RunRecorderPlugin::out()` has (`recorder.rs:72-80`).

## Consumers to migrate

Every reader of `CheckStatus::Skipped` or the measured figure, named:

| Consumer | Evidence |
| --- | --- |
| `measured_count`, `overall_verdict` | `run_report/checks/mod.rs:81-105` |
| `checks_json` `"measured"` | `checks/mod.rs:137-153` |
| aggregate row `measured` | `aggregate.rs:29-31`, read at `sweep.rs:202-206` |
| `render_index` status cells - a literal `if status == "SKIPPED"` and a `class="status-{lowercased}"` that would emit `status-n/a` | `aggregate.rs:285-296`, rules at `report.rs:252-255` |
| aggregate honesty banner (counts OK/WARN/FAIL/NO_DATA/ERROR; an UNPROBEABLE row would be in the table but in no count) | `aggregate.rs:248-260` |
| `verdict_severity` - "UNPROBEABLE" is unrecognized and ranks as FAIL; fail-closed, but make it deliberate | `aggregate.rs:172-179` |
| per-run HTML header + reviewer checklist ("SKIPPED = not measured") | `html.rs:44-57`, `html.rs:286` |
| the agent contract | `.claude/skills/probe/SKILL.md:126,136,166`, `.claude/skills/release/SKILL.md:62` |

## Ordering against the rest of the task

Step 5 flips `screenshots/` to probed and wires `nova_timeline` into the six
screenshot examples. Under this design the flip has nothing to flip - wiring
the recorder IS the flip - so this lands FIRST and step 5 shrinks to "wire the
six, plus the `log_clean` command-error gate". Step 6 (delete the smoke suite)
is unaffected except that `every_category_has_a_probe_policy` no longer needs
relocating: it is gone. The `probe_marker` question in point 5 above overlaps
step 6's deletion of the invariant roster.

## Open questions for the owner

1. Does `armed_*` survive as the second axis (recommended), or does the
   contract drive arming - i.e. probe arms `NOVA_PERF` whenever the contract
   declares `FrameTime`, and `--fps` becomes implied?
2. ~~`NOT_PROBED`: option 1, 2 or 3 above?~~ ANSWERED: option 3, let it fail.
3. Do `systems/` (`player_path`, `outcomes`) keep a real frame-time claim, or
   is `stress/` the only home, as the current table asserts? This decides which
   of the 12 `nova_frametime()` lines are deleted.
4. Does `probe_marker` become a `Capability`?

## Proposed step

Insert into `TASK.md` after step 4, as step 4.1 - the rest of the sequence
keeps its numbers:

> - [ ] 4.1. **Coverage becomes runtime: the example declares by wiring.** New
>   `nova_probe::contract` - each probe plugin declares its `Capability` in
>   `build()` above its arming guard, one Startup system writes
>   `probe-contract.json` (temp + rename), and `RunArtifacts` reads it.
>   Resolution is contract x `RunManifest::armed_*`: undeclared or unarmed is
>   N/A, armed-and-absent is a FAIL. `CheckStatus::Skipped` becomes
>   `NotApplicable(NotDeclared | NotArmed | InputNotSupplied |
>   InputNotComparable)`, and the verdict fold gains UNPROBEABLE for a run that
>   graded no declared capability. Deletes `CategoryPolicy` +
>   `CATEGORY_POLICIES` + `category_policy`, `fps_skip_reason`,
>   `RunManifest::fps_skipped`, and the 12 inert `nova_frametime()` lines.
>   Migrates every consumer listed in `PROTOTYPE-runtime-coverage.md`.
