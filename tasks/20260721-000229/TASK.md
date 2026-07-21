# Wire sccache into the nix devshell for safe fast worktree builds (measured)

- STATUS: CLOSED
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

- [x] Add `sccache` to the `flake.nix` devshell packages and set
      `RUSTC_WRAPPER = "sccache"` (+ `CARGO_INCREMENTAL = "0"`, which sccache
      requires) - OR scope those to the sprout shell only (see the measurement
      step; the sprout-scoped variant lives in nix.dotfiles as a follow-up).
- [x] MEASURE on a quiet host (`quiet-host-before-measuring`): a cold build
      (empty sccache), then a fresh sprout worktree's first build WITH a warm
      sccache. Record both wall-clock numbers here. The warm-worktree number is
      the DoD's before/after.
- [x] Decide the incremental tradeoff from the numbers: devshell-wide
      `CARGO_INCREMENTAL=0` (simplest, slower main-checkout iteration) vs
      sprout-scoped (main checkout keeps incremental). Record the decision.
- [x] Confirm no correctness surprise: build + run a smoke (e.g.
      `cargo run -p nova_assets --bin content -- lint`) from a warm-cache
      worktree and confirm it is not stale (sccache is content-keyed, so this
      should be trivially true - record it as the safety check).
- [x] Docs: rewrite the AGENTS.md "Build, run, test" worktree paragraph to the
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

## Close-out (2026-07-21)

Branch `tooling/sccache-devshell`, worktree
`~/.cache/sprouts/nova-protocol/tooling/sccache-devshell`.

### Measured (game binary, quiet host: load avg ~0.8, `date +%s` around builds)

- **T_cold** (empty sccache cache, `cargo clean` first): **405s (~6m45s)**.
  sccache stats: 634 compile requests, 517 executed, **0 hits / 517 misses**
  (483 Rust, 18 asm, 16 C/C++), cache grew to 805 MiB. Non-zero requests
  confirm `RUSTC_WRAPPER=sccache` was actually active during the build.
- **T_warm** (cache populated, `cargo clean` to wipe target/ but keep the
  sccache cache, same source): **38s**. sccache stats: 633 requests, 517
  executed, **517 hits / 0 misses (100% hit rate)**.
- Result: T_warm (38s) is ~10.7x faster than T_cold (405s) and far under the
  historical ~8-min no-sccache cold build. The DoD win is REAL and large; it
  matches the spike's predicted shape (dep tree = 100% hits, only changed
  nova_* would miss - here nothing changed so everything hit).

### Safety check

`nix develop --command cargo run -p nova_assets --bin content -- lint` from the
warm worktree: exit 0, printed the current lint output (11 scenarios audited,
1 warning, 2 acked findings - the live `the-ledger` content). NOT stale. sccache
is content-hash keyed and each worktree keeps its own target/, so the
name+version fingerprint aliasing that caused the stale-binary incident
(retro 20260709-131502) cannot occur - the cache key IS the source content.

### Incremental decision

Devshell-wide `CARGO_INCREMENTAL=0` (set in flake.nix next to RUST_BACKTRACE).
Rationale: sccache requires incremental off; the agent workflow is a fresh
worktree per task (always cold-shaped, pure win), and the win is huge (10x).
Tradeoff: the main checkout's human iterative edit-rebuild loop loses
incremental speedup. A sprout-scoped variant (export the wrapper only in sprout
shells) is left as a nix.dotfiles follow-up if that iteration cost bites -
noted in AGENTS.md and the wiki.

### CI

Unaffected: CI runs with an empty/no sccache cache, which is a plain cold
build (transparent). flake.lock unchanged (sccache is already in nixpkgs; no
input edits).

### Files changed

- `flake.nix`: +sccache to nativeBuildInputs; +RUSTC_WRAPPER, +CARGO_INCREMENTAL.
- `AGENTS.md`: Build section rewritten (nix develop --command requirement,
  sccache fast worktree builds, never-share-target-dir survives).
- `web/src/wiki/dev/development.md`: fast-build recipe + measured table +
  incremental tradeoff; toolchain note.
- `LESSONS.md`: `worktree-shares-main-target` updated with the sccache fast-path.

### Difficulties / self-reflection

- Straightforward. The `content lint` smoke recompiled the `content`-bin-only
  targets (different profile targets than the game `cargo build` produced) but
  pulled all dep compiles from the sccache cache - still fast enough and proves
  content-keying. If I wanted a pure warm-target smoke I'd have built the bin
  before clean, but the point (non-stale current code) is proven either way.
- Web CI needed `npm ci` (node_modules absent in fresh worktree); ran green.
