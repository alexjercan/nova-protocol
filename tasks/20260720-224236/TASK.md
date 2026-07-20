# Relocate nova_meta_gen to tools/ as a workspace-member build tool (out of crates/)

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: v0.8.0,tooling,refactor,web,spike

## Story

As the project owner, I want the web-only `.meta` sidecar generator out of my
game's `crates/` list and framed as build tooling, so the native game's crate
graph is not cluttered by a tool it never uses - without breaking the tool's
pin to the game's exact Bevy (version AND features).

Decided by spike 20260718-152255 (round 2, Option A): the tool must stay Rust
and ask Bevy (round 1 - the metas carry version-specific loader type-names +
non-defaulted settings, so a hardcode drifts), and it must stay a WORKSPACE
MEMBER (feature unification is what auto-pins the `wav` feature via
nova_modding -> nova_gameplay; leaving the workspace silently drops `.wav`
sidecars). So the move is: relocate the crate OUT of `crates/` into a top-level
`tools/` dir, keep it a workspace member, and optionally exclude it from bare
builds via `default-members`.

## Steps

- [x] `git mv crates/nova_meta_gen tools/nova_meta_gen` (keep the PACKAGE name
      `nova_meta_gen` - Trunk invokes `-p nova_meta_gen`, so the package name
      must not change; only the directory moves). `rm -rf` the emptied
      `crates/nova_meta_gen` if `git mv` leaves it behind
      (`git-mv-leaves-empty-parent`).
- [x] Fix the crate's own relative path-dep in `tools/nova_meta_gen/Cargo.toml`:
      `nova_modding = { path = "../nova_modding" }` -> `{ path =
      "../../crates/nova_modding" }`. (Workspace-inherited fields -
      `version`/`edition`/`license`/`[lints] workspace = true` - keep working
      because it stays a member.)
- [x] Update `[workspace] members` in the root `Cargo.toml`:
      `"crates/nova_meta_gen"` -> `"tools/nova_meta_gen"`.
- [x] Add `default-members` to the root `[workspace]` listing the game crates +
      the root package (`"."`) but NOT `tools/nova_meta_gen`, so a bare
      `cargo build`/`test` at the root no longer compiles the web-only tool.
      (See Notes for the maintenance caveat.)
- [x] Confirm the move preserved feature unification: build the tool in its new
      home and run it on a temp dir containing a `.wav`, and check a
      `sample.wav.meta` IS written (proves the `wav` feature still unifies in
      via nova_modding -> nova_gameplay from `tools/`). If the `.wav` sidecar is
      skipped, unification broke and the move regressed the tool.
- [x] Repoint the path-shaped doc reference in `README.md:148`: the
      `See [`crates/nova_meta_gen`](crates/nova_meta_gen/)` link ->
      `tools/nova_meta_gen`; keep the `-p nova_meta_gen` invocation as-is. Sweep
      the README crate table + `AGENTS.md` crate table rows for `nova_meta_gen`
      and note it now lives under `tools/` (grep the old path tree-wide:
      `grep -rn 'crates/nova_meta_gen'`, `keep-docs-in-sync-with-code`).
- [x] Update the tooling-inventory umbrella note
      (`tasks/20260718-152304/TASK.md`) so its catalog records meta-gen under
      `tools/`, not `crates/`.
- [x] Verify the web hook path end to end: `Trunk.toml`'s `post_build` hook
      (`cargo run -p nova_meta_gen`) still resolves and runs (a `trunk build`,
      or at minimum `cargo run -p nova_meta_gen -- --assets <temp>` from the
      repo root, succeeds and writes sidecars).

## Definition of Done

- `nova_meta_gen` lives under `tools/` and is gone from `crates/`
  (cmd: `test -d tools/nova_meta_gen && ! test -e crates/nova_meta_gen`).
- It still builds and its `wav` feature still unifies: the tool writes a
  `.wav.meta` for a `.wav` asset from its new location
  (cmd: `cargo run -p nova_meta_gen -- --assets <tmp-with-a-wav> && test -f <tmp>/sample.wav.meta`).
- A bare `cargo build` at the repo root does NOT compile `nova_meta_gen`
  (cmd: `cargo build 2>&1 | grep -q 'Compiling nova_meta_gen' && echo LEAKED || echo ok` on a clean target, or inspect `cargo build --dry-run`/`-v`).
- The Trunk `post_build` hook resolves unchanged (`-p nova_meta_gen`) - the web
  build still generates sidecars (cmd: `cargo run -p nova_meta_gen -- --assets <tmp>` succeeds from repo root).
- No stale `crates/nova_meta_gen` path references remain in docs
  (cmd: `grep -rn 'crates/nova_meta_gen' README.md AGENTS.md web/ tasks/20260718-152304/TASK.md` returns nothing).

## Notes

- Spike: tasks/20260718-152255/SPIKE.md (round 2 = the location decision).
- Leaf tool: nothing depends on `nova_meta_gen`; Trunk invokes it BY PACKAGE
  NAME (`cargo run -p nova_meta_gen`, Trunk.toml post_build), so the hook is
  unchanged by a directory move - only path-shaped references move.
- `main.rs` has NO hardcoded/workspace-relative path: `--assets` defaults to
  `$TRUNK_STAGING_DIR/assets` (Trunk hook) or `assets`, so the relocation needs
  no code change.
- Only ONE path-dep to fix (the crate's own `nova_modding` path) and ONE
  path-shaped doc link (README:148). `Trunk.toml` references the tool by NAME
  only (no path), so it needs no functional change.
- `default-members` maintenance caveat: it is an ALLOWLIST, so every FUTURE
  game crate must be added to it too or a bare build silently skips it. Mitigate
  by keeping CI on `cargo {check,test} --workspace` (which ignores
  default-members and still covers `tools/`), so nothing rots un-built. If this
  footgun is unwanted, drop the `default-members` step - the directory move
  alone still gets the tool out of `crates/`; bare builds would then still link
  its bin (cheap - the bevy lib is already built for the game).
- Web-only: native's real-filesystem 404 lets Bevy fall back to defaults; only
  the web SPA-fallback-200-HTML trap needs pre-written sidecars.
- Keep the package name `nova_meta_gen` (do NOT rename to `meta-gen`): the Trunk
  hook, README tools row and any `-p` invocation key on it; a rename widens the
  diff for no gain. Directory `tools/nova_meta_gen` matches the package name to
  minimize surprise.
- Coordinates with the tooling-inventory umbrella 20260718-152304 (keep its
  catalog in sync: meta-gen moves to tools/).

## Outcome (closed 2026-07-20)

Relocated `crates/nova_meta_gen` -> `tools/nova_meta_gen` via `git mv` (history
preserved), keeping the PACKAGE name `nova_meta_gen` so the Trunk hook
(`cargo run -p nova_meta_gen`) and every `-p` invocation are unchanged. Fixed
the crate's own `nova_modding` path-dep (`../nova_modding` ->
`../../crates/nova_modding`). In the root `Cargo.toml`: moved its `[workspace]
members` entry to `tools/nova_meta_gen` and added `default-members` (the 14 game
crates + the root package `"."`, NOT the tool) so a bare `cargo build`/`test`
skips the web-only tool while `--workspace` and `-p` still reach it.

Correctness verified: the tool builds and RUNS from its new home and still
writes a `sample.wav.meta` for a `.wav` asset - proving the `wav` feature
unification (via nova_modding -> nova_gameplay) survived the move, which was the
whole risk the spike flagged. `cargo metadata` confirms it is a workspace member
(16) but NOT a default-member (15). Docs repointed: README tools row + link ->
`tools/nova_meta_gen`, README + AGENTS crate-table rows note the `tools/`
location + default-members exclusion, and the 152304 catalog records the move.

No code changed (main.rs already defaults `--assets` to `$TRUNK_STAGING_DIR/
assets`, no hardcoded workspace paths). The tool's own tests (`tests/generate.rs`
+ unit tests) compile from the new location.

Self-reflection: a clean, mechanical relocation - the spike's up-front finding
(feature unification requires workspace membership) made the design obvious, and
the `.wav.meta` check turned that abstract risk into a concrete pass/fail proof.
The `default-members` allowlist is the one maintenance cost (new game crates
must be added there too); flagged in Notes with the CI-stays-on-`--workspace`
mitigation.
