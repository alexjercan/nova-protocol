# Spike: worktree build ergonomics

- TASK: 20260719-002512
- DATE: 2026-07-20
- STATUS: RECOMMENDED

## Question

Two build-ergonomics gotchas, currently documented instead of fixed:
1. a fresh sprout worktree pays a full cold build (~8 min measured in a past
   cycle) because naively sharing `CARGO_TARGET_DIR` with the main checkout is
   KNOWN BROKEN here (stale-binary incident);
2. `cargo test -p <crate>` alone does not compile for crates whose tests lean
   on workspace feature unification (`serde`).
What is the SAFE fast-build mechanism, and what is the crate-scoped-test fix?

## Context (grounded in the tree)

- The nix devshell (`flake.nix`) sets `buildInputs` + `LD_LIBRARY_PATH` only -
  NO `sccache`, `RUSTC_WRAPPER`, or `CARGO_TARGET_DIR`. Any cache tool lands in
  the flake (`nix-devshell-for-cargo`: the toolchain comes from the flake, not a
  per-user install).
- Feature-gated test derives live in `nova_scenario` (11), `nova_gameplay` (12),
  `nova_core` (1); `serde` is a feature on assets/gameplay/events/menu/modding/
  mod_format/scenario. `nova_scenario`'s `serde = ["dep:serde",
  "bevy/serialize", "nova_gameplay/serde"]`.

## Root cause of the shared-target clobber (named, per step 1)

From retro 20260709-131502: with `CARGO_TARGET_DIR` pointed at the main
checkout, a master A/B run rebuilt master's `nova_gameplay` into the shared dir,
and the next worktree build LINKED it - the "branch" smoke ran master code.

Mechanism: **cargo's fingerprint keys on crate name + version + features +
profile + rustc, NOT the source ROOT path.** Two checkouts of the same
workspace produce the same fingerprint for a given crate, so in one shared
target dir their artifacts alias each other; cargo sees the existing artifact as
up-to-date and skips the rebuild, linking whichever checkout compiled it last.
This is not a cargo bug to work around - it is the defined behavior, so any
"share the target dir" scheme (including a dedicated warm `CARGO_TARGET_DIR`
reused across worktrees) has the same hazard.

## Options considered - fast worktree builds

- **A. sccache as a nix-devshell `RUSTC_WRAPPER` (RECOMMENDED).** sccache caches
  each rustc invocation's OUTPUT keyed by a hash of the preprocessed source +
  flags + compiler version, in a shared cache dir. Each worktree keeps its OWN
  `target/`, so cargo's per-worktree fingerprinting and linking are untouched -
  the clobber mechanism cannot occur. A worktree only gets a cache HIT when the
  source CONTENT matches a prior compile (so unchanged deps - bevy, avian, the
  whole pinned tree - are 100% hits across worktrees; changed nova_* crates miss
  and compile fresh). There is NO path where a worktree links code from
  different source, because the cache key IS the source content. Expected shape:
  the ~8-min cold build is dominated by the dep tree (identical across
  worktrees), so the 2nd+ worktree should drop to the nova_* recompile + link +
  cargo overhead. Wire it: add `sccache` to the devshell packages and set
  `RUSTC_WRAPPER = "sccache"` (+ `CARGO_INCREMENTAL = 0`, which sccache
  requires). Safe for CI (transparent: a cold cache == a cold build).
- **B. Seed the sprout's `target/` (cp --reflink / hardlink from main).**
  Rejected: seeding master's artifacts into a new worktree reintroduces the
  fingerprint-aliasing risk - the seeded artifacts carry master's fingerprint,
  and a branch crate that cargo mis-judges as up-to-date links master's code.
  Trades the whole point (safety) for a copy; sccache gets the sharing safely.
- **C. A dedicated shared `CARGO_TARGET_DIR` (not the live main one).** Rejected:
  it is still one target dir shared across worktrees, so it has the SAME
  name+version fingerprint collision as sharing the main checkout's. sccache
  subsumes the intent without the shared-dir hazard.

## Options considered - the incremental tradeoff (the real open question)

sccache requires `CARGO_INCREMENTAL=0`. That HELPS cold / fresh-worktree builds
(the agent workflow: new worktree per task) but REMOVES incremental speedup for
the main checkout's iterative edit-rebuild loop (the human workflow). Two ways:
- set it devshell-wide (simplest; accepts slower main-checkout iteration), or
- scope `RUSTC_WRAPPER`/`CARGO_INCREMENTAL` to the SPROUT shell only (the sprout
  skill exports them on `sprout new`), leaving the main checkout on incremental.
  The sprout skill lives in nix.dotfiles (external), so this is a follow-up
  there. The impl task should MEASURE both to pick.

## Options considered - crate-scoped tests

The fix is settled by the ledger (`crate-solo-tests-miss-unified-features` x6,
promoted to AGENTS.md): give each affected crate a self dev-dependency that
enables its own feature, so `cargo test -p <crate>` builds with the feature on
without a sibling to unify it:

```toml
[dev-dependencies]
nova_scenario = { path = ".", features = ["serde"] }
```

`required-features` on the test targets is the WRONG fix (it SKIPS the tests
when the feature is off, so a plain `cargo test -p X` silently runs nothing).
Affected: nova_scenario, nova_gameplay, nova_core (sweep every crate with
feature-gated test code, not just these three). Mechanical once the pattern is
chosen; verify each `cargo test -p <crate>` compiles standalone and the full
`--workspace` run is unchanged.

## Recommendation

1. **sccache in the flake devshell** for fast, SAFE worktree builds - it shares
   compilation by content hash without a shared target dir, so the stale-binary
   incident provably cannot recur. Measure cold-vs-warm on a quiet host and
   decide devshell-wide vs sprout-scoped `CARGO_INCREMENTAL=0`.
2. **Self dev-dep feature fix** for crate-scoped tests, swept across the
   workspace.
These are independent and land separately. Measurement + wiring is big enough to
be its own task (per this task's Notes), so both are seeded below and this task
closes as the spike record.

## Open questions

- The incremental tradeoff (devshell-wide vs sprout-scoped) - resolved by the
  measurement in the seeded build-cache task.
- sccache + proc-macros / build scripts: sccache caches the proc-macro crate's
  own compile (a hit) but not its expansion (never cached anyway); build scripts
  run per-build. Confirm no correctness surprise during wiring (there should be
  none - these are compile-output caches).
- Does the sandbox `nix develop --command cargo` friction (a bare-PATH cargo
  glibc mismatch seen this session) interact? Orthogonal to caching, but worth a
  line in the build-cache task's docs.

## Next steps (seeded)

- tatr 20260721-000229: wire sccache into the nix devshell; measure cold-vs-warm
  fresh-worktree build; update AGENTS.md + development.md + the
  `worktree-shares-main-target` ledger entry.
- tatr 20260721-000249: fix crate-scoped tests with the self dev-dep feature
  pattern across the workspace; prove `cargo test -p <crate>` standalone;
  update AGENTS.md + the `crate-solo-tests-miss-unified-features` ledger entry.
