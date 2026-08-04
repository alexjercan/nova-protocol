# Review: example categories - contract + probe run policy

- TASK: 20260804-093855
- BRANCH: refactor/example-category-policy

Worktree: `/home/alex/.cache/sprouts/nova-protocol/refactor/example-category-policy`
Base: `master`

## Round 1

- REVIEWER: out-of-context `general-purpose` subagent `a8bfbedb7103e231f`
- VERDICT: REQUEST_CHANGES

The reviewer's prompt carried task ID, branch/worktree, dimensions and record format only.
Primary re-derived the MAJOR independently (`rg --hidden` sweep + reading
`AGENTS.md:94`, `env.rs:59-73`, `spec.rs:50-120`, the spec fixtures and both
new tests) and reran `cargo fmt --all --check` (clean) and
`cargo check --workspace --all-targets` (clean; only the pre-existing four
`nova_gameplay` ambiguous-import warnings and the `proc-macro-error2`
future-incompat note).

### Checks

| Command | Result |
|-|-|
| `cargo fmt --all --check` | clean (primary + reviewer) |
| `cargo check --workspace --all-targets` | clean (primary + reviewer) |
| `cargo test -p nova_probe` | 101 passed, 0 failed (reviewer) |
| `cargo test --test examples_smoke` | 7 passed, 0 failed, 462s (reviewer) |
| `! rg -n 'fps_exempt' crates/nova_probe web/src/wiki` | exits 1 - DoD proof green |
| `rg -n 'stress/' Cargo.toml web/src/wiki/dev/development.md` | hits both - DoD proof green |

Close-out Evidence re-derives accurately: 101 = 69 lib + 28 bin + 3
integration + 1 doc, and both `cmd:` proofs hold. No inflated claim found.

### Findings

- [x] **R1.1 (MAJOR)** `.claude/skills/probe/SKILL.md:165-172` still documents
      the two mechanisms this diff deletes. It instructs the reader to list
      one-shot examples "in the root `Cargo.toml` under
      `[package.metadata.nova_probe] fps_exempt`", says the report prints
      "fps-exempt", and states "Outside `perf/`, `--fps` defaults to a short
      60/240 window so a bare `probe run gameplay --fps` fits the deadline".
      All three are false after this branch: the key is inert (`Cargo.toml:27`
      now says so), the note is `category ... carries no frame-time pass`
      (`env.rs:26-33`), and `resolve_fps_window` (`env.rs:66-73`) returns one
      180/900 window with the per-category default explicitly removed. This is
      not a pre-existing problem - the diff invalidates it, and root
      `AGENTS.md:94` names this file as the probe doc surface ("Probe details:
      `.claude/skills/probe/SKILL.md`"). It survived because the DoD proof
      scans only `crates/nova_probe web/src/wiki` and ripgrep skips hidden
      directories by default.
      **Change:** rewrite that paragraph as category policy, mirroring
      `development.md:507-520`, and re-derive with
      `rg --hidden -g '!.git' -g '!tasks' 'fps_exempt|fps-exempt'`.
      Response: fixed. `SKILL.md:165-180` is rewritten as category policy
      mirroring `development.md:511-521` - it names the `CATEGORY_POLICIES`
      table and its smoke gate, states the unprobed-category behavior
      (`--all` skips and records, a bare `probe run screenshots` errors), and
      replaces the 60/240 paragraph with the single 180/900 window. The
      neighbouring `SKILL.md:46-49` claim that "`--all` skips the NOT_PROBED
      list" was stale for the same reason and now names both axes. Re-derived
      with `rg --hidden -g '!.git' -g '!tasks' -g '!CHANGELOG.md'
      'fps_exempt|fps-exempt|60/240'`: the only survivors are
      `SKILL.md:168`, which is the negation ("there is no `fps_exempt` list
      any more"), and `Cargo.toml:27,33`, the inert key owned by
      `20260804-093910`/`094006`.

- [x] **R1.2 (MINOR)** `.claude/skills/probe/SKILL.md:25` advertises
      `probe run ui  # a whole category (sections|gameplay|ui|screenshots|perf)`.
      `screenshots` is now rejected outright by `spec.rs:105-113`, so the
      documented spec errors.
      **Change:** drop `screenshots` from that list (or annotate it as not a
      probe target) and name `stress`/`systems`. Same file as R1.1; fix
      together.
      Response: fixed. The line now names only categories that resolve today
      (`sections|gameplay|ui|perf`), flags `screenshots` as ERRORS and
      `systems|stress` as contract-only until populated, and the `--all` line
      below it says "the catalog minus unprobed categories and NOT_PROBED".
      Round 2 found the same defect in a higher-authority surface; see R2.1.

- [x] **R1.3 (MINOR)** `crates/nova_probe/src/bin/probe/native/spec.rs:60-73`
      pushes a CATEGORY name into `AllManifest::excluded`, whose doc comment
      (`aggregate.rs:53-55`) and serialized shape both declare the slot to be
      an example: `(example, reason) pairs`. `index.json` can now carry
      `"example": "screenshots"`, a value that will not join against the
      example catalog, and the terminal renders it as
      `screenshots  NOT PROBED - ...` as though it were an example. The two
      axes are deliberate (per DECISION) but the consumer-facing contract was
      not updated to admit the second one.
      **Change:** update the `excluded` doc comment and the "Not probed
      (deliberately)" rendering to state that an entry may name a category, or
      tag each entry with its axis.
      Response: fixed, by tagging the axis in the value rather than changing
      the schema. `spec.rs:64` now records a category as `screenshots/` with
      the contract's trailing slash - the same way `Cargo.toml`,
      `development.md` and every reason string already write categories - so
      it reads unambiguously as a category in `index.json`, the HTML list and
      the terminal line, and can never collide with an example name. Both doc
      comments (`aggregate.rs:53-60`, `spec.rs:25-30`) now state that the
      list carries two axes and that a name may not be in the example
      catalog. The `index.json` key is left as `example`: renaming it is a
      second breaking schema change this task was not scoped for, and the
      trailing slash makes the value self-describing without one.

- [x] **R1.4 (MINOR)** `crates/nova_probe/src/bin/probe/native/env.rs:43-50`
      flips fail-OPEN to fail-CLOSED. Previously an example missing from the
      catalog, or a catalog read error, resolved to "not exempt" so the
      capture still ran - the deleted comment called that "by design so a
      catalog hiccup never silently suppresses a real capture". Now
      `unwrap_or_default()` yields `""`, `category_policy("")` returns the
      unknown default (`frame_time: false`), and the fps pass is skipped with
      the malformed reason ``category `/` carries no frame-time pass``. The
      doc comment claims this is unreachable and it is today (`sweep::run_spec`
      validates names against a loaded catalog; `--platform web` returns early
      with `fps_skipped: None`), so this is latent, not live.
      **Change:** make the unknown-example case explicit - either keep the
      capture (`None`) or emit a reason naming the missing example instead of
      formatting an empty category into a backtick-slash string.
      Response: fixed. `example_fps_skip_reason` (`env.rs:35-51`) now uses
      `?` on both the catalog load and the name lookup, so an unresolvable
      category returns `None` and the capture RUNS - the fail-open behavior
      the deleted comment called out, restored deliberately rather than by
      accident. The doc comment states the choice and why the alternative
      (formatting an empty category) was rejected.

- [x] **R1.5 (MINOR)** `crates/nova_probe/src/bin/probe/native/spec.rs:253-272`
      weakens `resolve_all_and_explicit_excluded`: `excluded` went from an
      exact `assert_eq!` to `.contains(...)`. That exact assertion was the only
      thing pinning the dedupe branch this same diff introduces
      (`spec.rs:68-71`, `if !excluded.contains(&entry)`), and the fixture
      catalog has two `screenshots` members (`fixtures.rs:15-16`) so the branch
      is genuinely exercised - it just is not asserted. The new test's
      `.find(...)` at `spec.rs:230-234` carries the message "recorded once, by
      category" but checks only that at least one entry exists. `examples` was
      strengthened in the same edit (exact vector, replacing `len() == 5`), so
      this reads as an oversight rather than a decision.
      **Change:** restore an exact `assert_eq!` on `resolved.excluded`, or
      assert the screenshots entry count is 1 where the message claims it.
      Response: fixed, both ways. `resolve_all_and_explicit_excluded` gets its
      exact `assert_eq!(resolved.excluded, vec![...])` back, covering both
      axes in catalog order, with a comment naming the dedupe it pins. And
      `category_run_policy_selects_passes_per_category` swaps `.find(...)` for
      a `.filter(...).collect()` plus `assert_eq!(recorded.len(), 1)`, so the
      "recorded once, by category" claim is now actually asserted. Verified
      RED by hand: deleting the `if !excluded.contains(&entry)` guard fails
      both.

- [x] **R1.6 (NIT)** Production `NOT_PROBED` (`spec.rs:8`) has exactly one
      entry, `render_scale_shot`, which lives in `screenshots` - now excluded
      wholesale by the category gate that runs first. The per-EXAMPLE axis
      therefore has no live member under `--all` or category expansion; it
      only still reaches the explicit-name note at `sweep.rs:48-51`. The unit
      tests had to retarget `EXCLUDED` to `playable` precisely because of
      this, i.e. they exercise a configuration that does not exist on disk.
      DECISION's "`NOT_PROBED` stays - it is per-EXAMPLE, an orthogonal axis"
      is true in shape but no longer true of any real example.
      **Change:** record a line in the task record (or seed a follow-up) so a
      later task decides whether the entry earns its keep.
      Response: recorded, not changed. The finding is correct and the
      observation is now written into DECISION.md ("The per-EXAMPLE axis has
      no live member") with the reason it is left standing: `NOT_PROBED` is
      still the mechanism behind the explicit-name note at `sweep.rs:48-51`,
      which `render_scale_shot` DOES reach, and deleting the entry would make
      `probe run render_scale_shot` run a real-GPU pixel capture under Xvfb
      with no printed warning. Whether the axis survives once `screenshots/`
      settles belongs to `20260804-093910` (which owns that directory), and
      the DECISION note names it.

### What holds

- All seven Steps are implemented as written. The one deviation -
  `resolve_fps_window` collapsing to a single window rather than keeping a
  per-category default - is recorded in DECISION with its reason (the
  non-frame-time branch is unreachable under the policy).
- All five DoD bullets hold under independent re-derivation, including both
  `cmd:` proofs.
- `category_run_policy_selects_passes_per_category` and
  `every_category_has_a_probe_policy` are real behavior/pinning tests that
  would fail on regression, and neither judges example content - they clear
  the owner's "do not test our tests" bar stated in Notes.
- No test was deleted without its mechanism; the four `fps_exempt` unit tests
  went with the parsers they covered.
- The scope boundary held: no example moved, no directory renamed,
  `catalog_matches_disk` untouched and green, smoke consts left for
  `093910`/`093934`/`094006`. `development.md` and `Cargo.toml` describe the
  tree as it actually is, with `examples/gameplay` and `examples/perf` still
  present and the transitional rows carrying `# remove with <task-id>`.

### Pending manual checks

None. No `manual:` proof in the DoD.

### Round 1 verdict rationale

**REQUEST_CHANGES** - one open MAJOR (R1.1: the probe SKILL.md, named by
`AGENTS.md` as the probe doc surface, still documents the deleted `fps_exempt`
mechanism and the deleted 60/240 window). R1.2 is the same file and lands with
it. R1.3-R1.5 are MINOR and do not block on their own, but R1.5 restores a
test assertion this diff removed and is cheap to take in the same round.

### Inspection commands

```
cd /home/alex/.cache/sprouts/nova-protocol/refactor/example-category-policy
git diff master...HEAD
rg --hidden -g '!.git' -g '!tasks' 'fps_exempt|fps-exempt|60/240' .
sed -n '20,30p;160,178p' .claude/skills/probe/SKILL.md
sed -n '43,73p' crates/nova_probe/src/bin/probe/native/env.rs
sed -n '50,120p;225,280p' crates/nova_probe/src/bin/probe/native/spec.rs
nix develop --command cargo test -p nova_probe
```

## Round 2

- REVIEWER: fresh out-of-context `general-purpose` subagent `a4a33f156816ff9d9`
- VERDICT: APPROVE

Not the round-1 reviewer, and not the author's context. Scope: verify each round-1 fix, plus regressions from the fix commit
`1fab3a65` only. The primary re-derived R2.1 independently before accepting it
(`sed` on `cli.rs:5-22` plus
`git diff --stat master...HEAD -- .../cli.rs`, which is empty).

### Round-1 findings, verified

| Finding | Round-2 verdict | Evidence the reviewer used |
|-|-|-|
| R1.1 (MAJOR) | CONFIRMED FIXED | The `--hidden` sweep returns exactly three lines - the `SKILL.md:168` negation and `Cargo.toml:27,33` - and zero `60/240`. Rewritten prose checked claim-by-claim against `catalog.rs:150-207` and `env.rs:69`. Independent sweep of `.claude/`, `AGENTS.md`, `README.md`, `.github/`, `scripts/` found one further surface: R2.1. |
| R1.2 (MINOR) | CONFIRMED FIXED in the named file; same defect found elsewhere -> R2.1 |
| R1.3 (MINOR) | CONFIRMED FIXED | Traced `screenshots/` verbatim through all three consumers (`aggregate.rs:74-76` JSON, `aggregate.rs:308-317` HTML, `sweep.rs:270-272` terminal) and the `from_json` round-trip; no consumer parses or joins it, and no test still expects a bare `screenshots`. |
| R1.4 (MINOR) | CONFIRMED FIXED | `env.rs:44-50` returns `None` on both failure paths; sole caller `run.rs:64` turns `None` into `armed_fps == true` (`run.rs:338`, `375`, `180`), i.e. the capture runs. No caller depended on the old `Some(...)`. |
| R1.5 (MINOR) | CONFIRMED FIXED, reproduced RED | Reviewer independently deleted the dedupe guard: exactly the two named tests failed (26 passed, 2 failed), `git checkout` restored byte-identical, re-run green at 28 passed. |
| R1.6 (NIT) | PUSHBACK ACCEPTED | Confirmed `spec.rs:101-107` matches an explicit name BEFORE either policy gate, so `probe run render_scale_shot` resolves and reaches the `NOT_PROBED` note at `sweep.rs:48-52`. Deferral to `20260804-093910` judged the right owner. |

### New findings

- [x] **R2.1 (MINOR)** `crates/nova_probe/src/bin/probe/native/cli.rs:12-13`.
      The probe `USAGE` string - what the tool prints as its own `--help` -
      still read `a category dir (sections|gameplay|ui|screenshots|perf);
      --all runs the whole catalog minus NOT_PROBED`. Both halves are false
      after this branch: `probe run screenshots` is now a hard error
      (`spec.rs:112-118`) and `--all` skips unprobed categories as well as
      `NOT_PROBED`. `cli.rs` is untouched by the branch, but the branch's
      behavior change is precisely what makes it false - the same argument
      that made R1.1 a finding rather than a pre-existing problem, and a
      higher-authority surface than the skill doc.
      **Change:** mirror the SKILL.md wording.
      Response: fixed in the round-2 commit (`docs(probe): correct the CLI
      usage text for the category run policy`). `cli.rs:11-14` now names the categories
      that resolve today, states that an unprobed category errors, and
      describes `--all` as "minus unprobed categories and minus NOT_PROBED -
      two axes, both recorded in the report".

- [x] **R2.2 (NIT)** `.claude/skills/probe/SKILL.md:25` named `systems` and
      `stress`, neither of which has a catalog member yet (this task moves no
      examples), so `probe run stress` fails with "unknown example or
      category". The line annotated `gameplay|perf` as transitional but gave
      no signal for the two contract-only rows.
      Response: fixed in the same commit - the line now reads `today:
      sections|gameplay|ui|perf; systems|stress once populated; screenshots
      ERRORS`.

- [ ] **R2.3 (NIT)** `crates/nova_probe/src/bin/probe/native/spec.rs:112-117`.
      The unprobed-category error appends `spec_help(catalog)`, which
      enumerates every catalog category including the one just rejected - so
      `probe run screenshots` prints "not a probe target" and then lists
      `screenshots: screenshot_reel, ...` under "examples by category".
      Circular rather than misleading (the error leads with the honest
      answer), and pre-existing in shape.
      Response: NOT fixed here, deliberately. `spec_help` is the catalog
      listing, and the catalog genuinely does contain that category - marking
      unprobed rows in it is a display concern that belongs with the
      `screenshots/` settlement in `20260804-093910`, which may remove the
      question entirely. Left open as a NIT; it blocks nothing.

### Checks (round 2)

| Command | Result |
|-|-|
| `cargo fmt --all --check` | clean (primary + reviewer) |
| `cargo check --workspace --all-targets` | clean; only the pre-existing four `nova_gameplay` glob-ambiguity warnings and the `proc-macro-error2` note (primary + reviewer) |
| `cargo test -p nova_probe` | 101 passed, 0 failed - 69 lib + 0 + 28 bin + 3 integration + 1 doc (primary + reviewer) |
| `cargo test --test examples_smoke` | 7 passed, 0 failed, 141s (reviewer); 7 passed, 0 failed, 111s (primary) |
| `! rg -n 'fps_exempt' crates/nova_probe web/src/wiki` | exits 1 - DoD proof green |
| `rg -c 'stress/' Cargo.toml web/src/wiki/dev/development.md` | `Cargo.toml:2`, `development.md:4` - DoD proof green |
| dedupe guard deleted | 2 tests FAIL, as R1.5 claims; restored, 28 passed |

### Pending manual checks

None. The DoD carries no `manual:` proof.

### Round 2 verdict rationale

**APPROVE.** Every round-1 finding is closed - five fixed and independently
re-verified (R1.5 reproduced RED and restored), R1.6's pushback accepted on
the exact path it names. R2.1 and R2.2 were found and fixed within this round.
R2.3 stays open as a NIT with a named owner and blocks nothing. All five DoD
bullets hold, both `cmd:` proofs are green, and the scope boundary is intact:
no example moved, no directory renamed, `catalog_matches_disk` green, smoke
consts untouched for `093910`/`093934`/`094006`.

### Inspection commands

```
cd /home/alex/.cache/sprouts/nova-protocol/refactor/example-category-policy
git diff master...HEAD
rg --hidden -g '!.git' -g '!tasks' -g '!CHANGELOG.md' 'fps_exempt|fps-exempt|60/240' .
sed -n '5,22p' crates/nova_probe/src/bin/probe/native/cli.rs
nix develop --command cargo test -p nova_probe
nix develop --command cargo test --test examples_smoke
```
