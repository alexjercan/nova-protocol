# Review: Retire the BCS harness surface and refresh the automation docs

- TASK: 20260802-183406
- BRANCH: docs/retire-bcs-harness-surface

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (BLOCKER) .claude/skills/probe/SKILL.md:176 - the probe skill still
  teaches `BCS_HARNESS_DEADLINE` (lines 176 and 181), but probe now sets
  `NOVA_AUTOPILOT_DEADLINE` (`crates/nova_probe/src/bin/probe/native/env.rs:6`
  imports `completion::DEADLINE_ENV`; line 101 sets it), so the documented
  override is dead prose - exactly the Story's failure mode. The DoD absence
  proof only reads green because `rg` skips dot-directories by default; the
  same pattern with `--hidden` returns 4 hits. Rename both occurrences to
  `NOVA_AUTOPILOT_DEADLINE`, and add `--hidden --glob '!.git/**'` to the DoD
  proof cmd so `.claude/`, `.github/` and `.gitignore` are actually swept.
  - Response: fixed - both occurrences in `.claude/skills/probe/SKILL.md` now
    read `NOVA_AUTOPILOT_DEADLINE`, and the DoD proof cmd gained
    `--hidden --glob '!.git/**'`. The widened sweep is what surfaced R1.3 and
    R1.5 too; it now returns 0 hits.
- [x] R1.2 (MAJOR) tasks/20260802-183406/DECISION.md:1 - the record ignores the
  tatr DECISION schema, so `tatr check 20260802-183406` is red with 10
  `bad-record-schema`/`bad-decision-status` errors and would land lint-red
  history. All 50 other `tasks/*/DECISION.md` files conform. Regenerate via
  `tatr scaffold 20260802-183406 DECISION` and refit the existing prose: a
  `# Decision: <title>` heading, the `- DATE:`/`- STATUS:`/`- TASK:`/`- TAGS:`
  lines, and `## Context`, `## Decision`, `## Alternatives considered`,
  `## Consequences` sections. The two current sections are two decisions; the
  schema wants one record each, or one record whose `## Decision` states both.
  - Response: fixed - rewritten as one conforming record ("the dead BCS names
    get one searchable home, and the absence proof only asserts BCS") with the
    full header and all four sections; both original decisions are one
    decision with two consequences, so they fit one record. `tatr check
    20260802-183406` now exits 0.
- [x] R1.3 (MINOR) .github/workflows/ci.yaml:92 - the smoke-step comment still
  says examples spawn "under BCS_AUTOPILOT/Xvfb"; the env is now
  `NOVA_AUTOPILOT`. Change `BCS_AUTOPILOT` to `NOVA_AUTOPILOT`.
  - Response: fixed - the comment names `NOVA_AUTOPILOT`.
- [x] R1.4 (MINOR) CHANGELOG.md:31 - the breaking entry lists `NOVA_SHOT_DIR`
  among the `BCS_* -> NOVA_*` renames, but that variable was always spelled
  `NOVA_SHOT_DIR` (`git log -S BCS_SHOT_DIR --all` is empty; `master~5`'s
  `scripts/gen-web-screenshots.py:31` already reads
  `NOVA_SHOT_DIR=target/reel BCS_REEL=1`). Drop `NOVA_SHOT_DIR` from the
  renamed list so a reader greps for names that actually existed.
  - Response: fixed - dropped from the renamed list, with an explicit
    parenthetical that `NOVA_SHOT_DIR` was always spelled that way and is
    unchanged, so a reader does not assume it was missed.
- [x] R1.5 (MINOR) .gitignore:14 - comment reads "Default output of the
  nova_debug screenshot harness (BCS_SHOT)". Change `BCS_SHOT` to `NOVA_SHOT`.
  - Response: fixed - the comment names `NOVA_SHOT`.
- [x] R1.6 (NIT) crates/nova_gameplay/src/settings.rs:386 - test comment says
  "otherwise the bcs harness envs decide"; the envs read at line 95 are
  `NOVA_AUTOPILOT`/`NOVA_SHOT`/`NOVA_REEL`. Change "the bcs harness envs" to
  "the harness envs".
  - Response: fixed - comment now reads "otherwise the harness envs decide".
    `cargo check --workspace --all-targets --features debug` re-run green.

Verification, re-derived in-session on top of the out-of-context round:

- R1.1 re-derived independently: `.claude/skills/probe/SKILL.md`,
  `.github/workflows/ci.yaml` and `.gitignore` are all tracked
  (`git ls-files`), and the DoD pattern with `--hidden --glob '!.git/**'`
  returns those 4 hits. `DEADLINE_ENV` resolves to `NOVA_AUTOPILOT_DEADLINE`
  (asserted at `crates/nova_probe/src/bin/probe/native/env.rs:311`).
- R1.3 re-derived: `git log --oneline -S 'BCS_SHOT_DIR' --all` returns nothing.
- Proof 1 (absence sweep): PASS as written, exit 0. Fails under `--hidden`.
- Proof 2 (`rg -n "NOVA_AUTOPILOT" AGENTS.md`): PASS, line 74.
- Proof 3 (`rg -n "nova_autopilot" CHANGELOG.md`): PASS, lines 28, 36, 37.
- Proof 4: `nix develop --command cargo check --workspace --all-targets
  --features debug` PASS (rerun in-session, `Finished`); `cd web && npm run ci`
  PASS per the out-of-context reviewer (a fresh sprout needs `npm ci` first).
- Proof 5 is `manual:` and stays a pending user check; correctly left unticked.
- Steps: all five ticks match the diff. The retained-`bevy_common_systems`
  table in the close-out matches an independent sweep - no `debug::harness` or
  `completion` import from `bevy_common_systems` survives in `crates/`,
  `examples/` or `tests/`.
- Prose is ASCII-clean per the global AGENTS rules; no em dashes, smart quotes
  or unicode arrows in the diff.
- The two DoD proof-cmd narrowings after the PLAN gate are justified in
  `DECISION.md` and correct on the merits.

Process signal: de-future-tensing `web/src/wiki/dev/automation-harness.md` is
in no literal Step, but that page taught `BCS_HARNESS_DEADLINE` and the Story
is "leave the prose true", so it belongs here. The gap the Steps missed is the
same one R1.1-R1.4 name: the Step list enumerated `web/`, `AGENTS.md` and
`CHANGELOG.md` by hand instead of deriving the file set from the sweep, so
every dot-directory surface fell outside it.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

All six round-1 findings verified fixed by the round-2 out-of-context
reviewer, each against a live grep rather than the Response text:

- R1.1: both `.claude/skills/probe/SKILL.md` lines read
  `NOVA_AUTOPILOT_DEADLINE`; the widened DoD sweep exits 1 with 0 hits, and a
  broader `rg --hidden "BCS_"` under the same excludes is also empty.
- R1.2: `tatr check 20260802-183406` exits 0; the record matches the header
  and four sections used by `tasks/20260802-183403/DECISION.md`.
- R1.3: `.github/workflows/ci.yaml:92` reads `NOVA_AUTOPILOT/Xvfb`.
- R1.4: `git log -S BCS_SHOT_DIR --all` shows no historical rename.
- R1.5: `.gitignore:14` reads `(NOVA_SHOT)`.
- R1.6: `crates/nova_gameplay/src/settings.rs:386` reads "otherwise the
  harness envs decide"; `cargo check` green.

The boxes above are ticked on that reviewer's confirmation.

- [ ] R2.1 (MINOR) CHANGELOG.md:32 - `web/src/wiki/dev/automation-harness.md:60`
  promises "the CHANGELOG's breaking entry spells out the old spellings", but
  the entry only spelled the `BCS_* -> NOVA_*` glob and
  `BCS_HARNESS_DEADLINE`. `BCS_SHOT` and `BCS_REEL` appeared nowhere outside
  `tasks/**`, so a stuck `BCS_REEL=1` script greps the repo to nothing and
  DECISION.md's "one searchable home" does not hold for them. Spell all four
  renames literally; the DoD sweep already excludes `CHANGELOG.md`.
  - Response: fixed - the entry now reads `BCS_AUTOPILOT -> NOVA_AUTOPILOT`,
    `BCS_SHOT -> NOVA_SHOT`, `BCS_REEL -> NOVA_REEL`,
    `BCS_HARNESS_DEADLINE -> NOVA_AUTOPILOT_DEADLINE`, with the
    not-a-pure-prefix-swap note the wiki page also carries. All four old names
    are now greppable in exactly one live file.
- [ ] R2.2 (NIT) tasks/20260802-183406/DECISION.md:23 - "CHANGELOG.md line 295"
  (and the same ref at line 48) is off by one, and the R2.1 fix moves it again.
  Drop the line number and quote the entry instead.
  - Response: fixed - both refs now name the entry by its opening words
    ("Examples are a testable curriculum"), so no CHANGELOG edit can stale
    them.
- [ ] R2.3 (MINOR) tasks/20260802-183406/TASK.md:6 - the round-1 fix commit
  `3cd2edb2` records `ACTIVITY: WORKING`; the `REVIEWING` correction exists
  only as an uncommitted working-tree edit. Commit it so the branch's landed
  record is `REVIEWING`.
  - Response: not a revert - `d2277182` committed REVIEWING, the
    `rewind --to WORKING` that the REQUEST_CHANGES loop requires moved it back,
    and `3cd2edb2` correctly recorded WORKING because that was the activity
    while the fixes were being written. The `tatr flow` back to REVIEWING ran
    after that commit, so its edit is in-flight by construction and lands with
    this round-2 commit. Verified: the rest of `3cd2edb2`'s TASK.md hunk is the
    close-out, not a stale copy.

Proofs re-run for round 2, all first-hand: absence sweep 0 hits (`rg` exit 1);
`rg -n "NOVA_AUTOPILOT" AGENTS.md` line 74; `rg -n "nova_autopilot"
CHANGELOG.md` lines 28, 38, 39; `cargo check --workspace --all-targets
--features debug` and `cd web && npm run ci` both exit 0;
`tatr check 20260802-183406` exit 0. The `manual:` proof (read the retained
`bevy_common_systems` imports) was independently re-derived by the round-2
reviewer and matches the close-out table, but stays a pending user check.

The diff is ASCII-clean: `rg '[^\x00-\x7F]'` over `git diff master...HEAD`
returns nothing.

Not verified: the examples-smoke suite was not run, so DECISION.md's claim
that `examples_name_drivers_through_the_nova_harness` requires
`nova_debug::harness` is read from source only.
