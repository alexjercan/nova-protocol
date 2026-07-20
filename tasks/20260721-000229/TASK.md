# Wire sccache into the nix devshell for safe fast worktree builds (measured)

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.8.0,tooling,testing

## Story

As an agent/human working nova-protocol in sprout worktrees, I want a fresh
worktree's build to reuse compilation from other checkouts SAFELY, so the
per-task cycle stops paying a full ~8-min cold build - without ever risking the
stale-binary incident that killed naive `CARGO_TARGET_DIR` sharing.

Decided by spike 20260719-002512: sccache as a nix-devshell `RUSTC_WRAPPER` is
the safe mechanism. It caches each rustc invocation's OUTPUT by CONTENT hash in
a shared cache, while each worktree keeps its own `target/` - so cargo's
per-worktree fingerprinting/linking is untouched and the name+version fingerprint
collision (the incident's mechanism) cannot occur. Unchanged deps (bevy, avian,
the pinned tree) become 100% cache hits across worktrees; only changed nova_*
crates recompile.

## Steps

- [ ] Add `sccache` to the `flake.nix` devshell packages and set
      `RUSTC_WRAPPER = "sccache"` (+ `CARGO_INCREMENTAL = "0"`, which sccache
      requires) - OR scope those to the sprout shell only (see the measurement
      step; the sprout-scoped variant lives in nix.dotfiles as a follow-up).
- [ ] MEASURE on a quiet host (`quiet-host-before-measuring`): a cold build
      (empty sccache), then a fresh sprout worktree's first build WITH a warm
      sccache. Record both wall-clock numbers here. The warm-worktree number is
      the DoD's before/after.
- [ ] Decide the incremental tradeoff from the numbers: devshell-wide
      `CARGO_INCREMENTAL=0` (simplest, slower main-checkout iteration) vs
      sprout-scoped (main checkout keeps incremental). Record the decision.
- [ ] Confirm no correctness surprise: build + run a smoke (e.g.
      `cargo run -p nova_assets --bin content -- lint`) from a warm-cache
      worktree and confirm it is not stale (sccache is content-keyed, so this
      should be trivially true - record it as the safety check).
- [ ] Docs: rewrite the AGENTS.md "Build, run, test" worktree paragraph to the
      new reality, put the fast-build recipe in web/src/wiki/dev/development.md,
      and update the LESSONS.md `worktree-shares-main-target` entry (sccache
      makes fresh worktrees fast; still NEVER share CARGO_TARGET_DIR). Note the
      `nix develop --command cargo` bare-PATH friction if relevant.

## Definition of Done

- A fresh sprout worktree's first build is measurably faster than a cold build,
  numbers recorded here (cmd: the two timed builds).
- The mechanism has a written why-it-cannot-reproduce-the-stale-binary-incident
  (content-hash keying, per-worktree target dir).
- CI is unaffected (sccache transparent: cold cache == cold build).
- AGENTS.md + development.md + LESSONS.md reflect the new reality; the old
  gotcha text survives nowhere.

## Notes

- Spike: tasks/20260719-002512/SPIKE.md (Option A + the incremental tradeoff).
- flake.nix devshell today: buildInputs + LD_LIBRARY_PATH only, no cache.
- CI builds cold anyway and must stay honest - sccache with an empty cache is a
  cold build, so CI is safe either way.
