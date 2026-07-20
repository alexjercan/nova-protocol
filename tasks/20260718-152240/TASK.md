# content lint: merge balance audit into lint and emit a human-friendly per-mod report that pinpoints where each finding occurs (file + location)

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.8.0,tooling,modding

## Story

As a modder debugging my bundle, I want a single `content lint --target
<my-mod>` command that checks BOTH reference/geometry correctness AND combat
balance, and hands me a report that says which file, which id and which field
each finding is about, with a suggested fix, so that I can repair my mod
without grepping my own content against a bare stdout message or remembering
that "audit" is a separate command.

The `content` CLI (`crates/nova_assets/src/bin/content.rs`) has `lint` and
`audit` as two subcommands today. `audit` prints a combat balance sheet and
grades static fairness findings (`nova_assets::balance`); `lint` surfaces
reference errors (unknown prototypes, dangling NextScenario chains, unseated
mounts, unmatched ObjectiveComplete - `nova_scenario/src/lint.rs`). The
decision (user, 2026-07-19): checking whether a scenario is fair/balanced is
still linting, so fold `audit` INTO `lint` - one command, both classes of
finding. Both speak in stdout lines today; a modder debugging a multi-file
bundle (The Ledger has five content files) deserves a document.

## Steps

- [x] Merge `audit` into `lint`: `lint` runs reference/geometry checks AND the
      balance/fairness audit in one pass, over the whole tree or one
      `--target <mod>`. Remove the separate `audit` subcommand. Preserve every
      current exit-code rule (non-zero on any Error, on stale balance ack) and
      keep the balance ack mechanism (`shipped_acks` / `partition_findings`).
- [x] Keep the embedded checkers where they live - `nova_scenario::lint`
      (reference/geometry) and `nova_assets::balance` (balance) stay as
      libraries; only the CLI surface unifies. The `content_lint_gate` and
      `balance_audit_gate` tests keep exercising their walks (rename/retarget
      the gates if the CLI subcommand they invoke changes, but keep both
      coverage sets).
- [x] Give `lint --target <mod>` a report mode (e.g. `--report <path>` or
      `--format md|html`) that writes a per-mod report grouping findings by
      severity and, for each, the source location: file (relative to the mod),
      the offending id/prototype/field, and a short explanation + suggested fix.
      Reference findings and balance findings appear in the same report.
- [x] Ensure both finding types carry enough location context to point at the
      file and element; enrich the finding types if they currently only have a
      message.
- [x] Markdown is the baseline output (diffable, pasteable); HTML optional if
      it is cheap and matches the perf-report styling (20260718-152230).
- [x] Prove it on the shipped mods: generate the report for `the-ledger`,
      `gauntlet` and `example` and check every finding's location actually
      points where the report says (seed one deliberate reference error AND one
      deliberate balance problem to verify both classes surface).
- [x] Add an input-binding-overlap check: flag any `PlayerControllerConfig`
      `input_mapping` section bound to a key that the flight rig also binds
      (W/S/Space/RightTrigger drive FlightBurnInput etc, consume_input: false),
      because the section then silently double-drives flight. Report it as a
      finding with the file + section id + the colliding key. Follow-up from
      20260718-235837, where "guns" on Space burned the ship off its mark and
      broke the 10_playable CI smoke.
- [x] Document the unified command + report mode in the dev wiki
      (guide-make-a-mod.md pre-publish check, development.md tools list) and the
      README tools section (20260718-152205). Note the `audit` -> `lint` merge
      so old invocations are repointed.

## Definition of Done

- One command (`content lint`) produces both reference and balance findings; a
  per-mod report names file + element + explanation + suggested fix for every
  finding; zero findings produces a clean report, not an empty file.
- The separate `audit` subcommand is gone; its balance coverage (and ack
  handling, and stale-ack failure) lives under `lint`.
- The existing library consumers of `nova_scenario::lint` and
  `nova_assets::balance` (CLI, the lint/balance gate tests, runtime merge
  sweep) still pass.
- A deliberately broken test mod's report pinpoints the planted reference error
  AND the planted balance problem, and a test pins that behavior.

## Notes

- Decision recorded here (user, 2026-07-19): audit is a kind of lint, so the
  two CLI subcommands merge into one `lint`. This supersedes the earlier
  "consider folding" step, which is now settled as YES.
- Linters stay embedded: `nova_scenario::lint` (reference/geometry) reused by
  the CLI, the `content_lint_gate` test, and the runtime merge sweep;
  `nova_assets::balance` reused by the CLI and the `balance_audit_gate` test.
  Keep those working - only the CLI subcommand surface changes.
- `resolve_target` already maps a mod name/id to a dir (webmods/, assets/mods/,
  base); the report just needs a nicer sink than println!.
- Coordinates with 20260719-092952 (base-mod gen moves to build-time and the
  `gen` subcommand is removed): together these shrink the `content` bin to a
  single `lint` command. Umbrella: 20260718-152304.
- Input-binding-overlap check (added 2026-07-19, follow-up from 20260718-235837):
  the flight rig binds W/S/Space/RightTrigger to FlightBurnInput and friends
  with consume_input: false, so a content `input_mapping` section reusing one of
  those keys double-drives flight. The lint should enumerate the flight-rig
  bindings (`crates/nova_gameplay/src/input/player.rs`) and flag any content
  section that overlaps. See LESSONS.md `input-mapping-overlays-flight-rig`.
- The docs review (2026-07-18) recommends the wiki's pre-publish checklist
  point at this report once it exists (see 20260718-231601's publish-vs-load
  split item) - coordinate wording.

## Outcome (closed 2026-07-20)

What changed:
- The `content` bin lost its `audit` subcommand; `lint` now runs all three
  checker families in ONE pass - `nova_scenario::lint` (reference/geometry/
  resource), `nova_assets::balance` (fairness), and a new flight-rig
  input-overlap check - over the whole tree or one `--target <mod>`.
- New `nova_assets::content_report` module: a located `Finding` model
  (bundle + file + severity + category + element + message + suggested fix)
  with Markdown (baseline) and HTML (perf-report-styled, self-contained)
  renderers. `lint --target <mod> --report <path> [--format md|html]` writes
  the per-mod document; a `.html` path implies HTML. A clean mod still gets a
  document, not an empty file.
- File PROVENANCE: `WalkedBundle.content` now carries the bundle-relative file
  per content item, and `file_of(id)` resolves a scenario/section id to its
  source file, so every finding points where it says. `lint_walk::collect_tree`
  / `collect_target` build the unified report; `collect_target` walks the repo
  for context but reports only the target, and scopes acks to the target so
  another mod's ack is not falsely stale.
- Balance code path unified: `balance::audit_bundles_to_audits` is the single
  audit body, fed the whole repo by `audit_content_tree` (gate) and the walked
  target set by the report, so a `--target` audit and the tree audit agree.
- Input-overlap: `nova_gameplay` exposes `InputSource`, `binding_source`, and
  `flight_rig_reserved_sources()` (the 21 discrete flight-rig keys, each with
  its verb). A drift-guard test (`flight_rig_reserves_exactly_these_sources`)
  builds the REAL rig and asserts the reserved list == the rig's queried
  discrete-button sources, so authoring and lint cannot drift. The check is a
  WARN (content loads, just misbehaves) - shipped content is clean of overlaps
  (turrets fire on Mouse(Left)/Gamepad(RightTrigger2), the flight rig burns on
  the RightTrigger BUMPER, different buttons), so no gate reddens.

Exit-code rule preserved: non-zero on any Error. Reference errors and
spawned-dead balance stay Error; a STALE ack is now an Error-grade finding
(it fails the build), which lets the exit code key cleanly on `error_count()`
without a separate flag. close-spawn and input-overlap are Warns and never gate.

Proof: `content_report_gate` plants a reference error, a spawned-dead hostile
and a guns-on-Space overlap in a fixture mod and asserts the report pinpoints
each with the right file + element; a second test walks the-ledger / gauntlet /
example and asserts every located finding points at a real file. Verified the
CLI by hand on all three shipped mods (md + html) and the whole-tree lint
(0 errors, the ledger cross-handler warn + 2 acked balance findings, exit 0).

Difficulties:
- `PlayerControllerConfig.infinite_ammo` is not serde-defaulted, so the fixture
  RON needed it explicitly (parse panic named the field).
- A unit test set `bundles: ["m"]` while its finding helper hardcoded bundle
  "the-ledger"; `by_bundle()` filters findings to listed bundles, so the finding
  vanished from the rendered table. Real callers always list every finding's
  bundle (built from report_ids), so this was a test-fixture bug, not a code
  one - fixed the fixture.

Self-reflection: the reserved-binding drift guard is the piece most likely to
save a future session - a hand-listed key set would silently rot as the rig
changes; building the rig and diffing sources makes it self-correcting. Next
time, author fixture RON by copying a known-good shipped block wholesale before
trimming, rather than hand-writing from the struct - would have skipped the
missing-field round-trip. Coordinates with 20260719-092952 (removes `gen`),
still OPEN: both shrink the `content` bin; done independently here.
