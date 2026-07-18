# Spike: worktree build ergonomics - safe shared/cached cargo target for sprouts + crate-scoped cargo test works standalone

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.8.0,spike,tooling,testing

## Story

As an agent (or human) working nova-protocol in sprout worktrees, I want fresh
worktrees to build fast and `cargo test -p <crate>` to just work, so that the
per-task cycle stops paying a full cold build and nobody needs the
feature-unification incantation from AGENTS.md ever again.

Two related build-ergonomics problems, both currently documented as gotchas
instead of fixed:

1. A fresh sprout worktree has an empty `target/` and pays a full cold build
   (~8 min was measured in a past cycle). Naively sharing `CARGO_TARGET_DIR`
   with the main checkout is KNOWN BROKEN here - artifacts clobbered and a
   worktree binary silently linked master's code (ledger:
   `worktree-shares-main-target`, retro 20260709-131502).
2. `cargo test -p nova_scenario` alone does not compile: its serde round-trip
   tests lean on workspace feature unification (ledger x6:
   `crate-solo-tests-miss-unified-features`), so crate-scoped test runs need
   `--features serde`, a unifying sibling, or workspace-wide runs.

## Steps

- [ ] Reproduce and root-cause the shared-target clobber first: read the
      20260709-131502 retro, then probe how cargo fingerprints workspace
      members across two checkouts sharing one target dir (same crate names +
      versions, different source paths). Name the exact mechanism before
      designing around it - the old incident is evidence it CAN go wrong, not
      an explanation of why.
- [ ] Evaluate the candidate mechanisms for fast worktree builds, each with a
      timed probe (cold vs warm numbers on this machine, quiet host per
      `quiet-host-before-measuring`):
  - [ ] sccache (or an equivalent compiler cache) as a nix devshell default -
        shares compiled artifacts safely across checkouts by content hash.
  - [ ] Seeding the sprout's `target/` at creation time (cp --reflink /
        hardlink from the main checkout) - measure whether cargo accepts or
        invalidates the seeded cache, and whether stale-binary risk returns.
  - [ ] A per-project shared cache that is NOT the live main checkout's
        target (e.g. a dedicated warm cache dir refreshed from master), if
        direct sharing proves unsafe.
- [ ] Pick the winner, wire it (nix devshell env for this repo; a sprout-side
      hook belongs in nix.dotfiles as a follow-up task there if needed), and
      record the measured before/after cold-build numbers.
- [ ] Fix crate-scoped tests at the root: give each affected crate a self
      dev-dependency enabling its own feature
      (`[dev-dependencies] nova_scenario = { path = ".", features = ["serde"] }`)
      or `required-features` on the test targets - whichever keeps CI's
      feature matrix honest. Sweep ALL workspace crates for the same trap,
      not just nova_scenario (grep test modules for feature-gated derives).
- [ ] Prove it: `cargo test -p nova_scenario` (and each previously affected
      crate) compiles and runs standalone with no extra flags; run the full
      workspace suite once to confirm nothing else moved.
- [ ] Docs in the same task: rewrite the AGENTS.md "Build, run, test"
      paragraph to the new reality, update the
      `crate-solo-tests-miss-unified-features` ledger entry (fixed-at-root,
      like the tatr collision), update `worktree-shares-main-target` and the
      sprout skill's "worktree facts" note in nix.dotfiles if the verdict
      changes it, and put the fast-build recipe in
      web/src/wiki/dev/development.md.

## Definition of Done

- A fresh sprout worktree's first build is measurably faster than a cold
  build (numbers recorded in this task), via a mechanism with a written
  explanation of why it cannot reproduce the stale-binary incident - or a
  documented decision that safe sharing is not possible, with the probe
  evidence.
- `cargo test -p nova_scenario` works standalone; no crate in the workspace
  needs a feature incantation for its own tests.
- AGENTS.md, LESSONS.md, development.md and (if the worktree verdict changed)
  the sprout skill all reflect the new reality; the old gotcha text survives
  nowhere.

## Notes

- Spike-shaped: the target-sharing half needs probes before a design; the
  test-fix half is likely mechanical once the pattern is chosen. If the
  probes get big, seed a separate implementation task and keep this as the
  spike record (SPIKE.md in this folder).
- Prior evidence: ledger entries `worktree-shares-main-target` (x1,
  CORRECTED), `crate-solo-tests-miss-unified-features` (x6),
  `nix-devshell-for-cargo` (toolchain comes from the flake - any cache tool
  must land in flake.nix, not a per-user install).
- Interactions to check: the nix devshell LD_LIBRARY_PATH, CI (which builds
  cold anyway and must stay unaffected), and `cargo check --all-targets`
  habits from the work skill.
- Source: AGENTS.md "Build, run, test" gotcha paragraph; requested by the
  user on 2026-07-19.
