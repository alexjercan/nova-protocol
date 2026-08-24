# Prevent CI web release build termination

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.11.0

## Failure

Release run 32694711257 compiled the wasm graph normally, then spent 43m28s
without completing the final `nova_core` release link. The hosted runner
terminated Trunk with exit 143. The v0.10.0 web release had completed its whole
Cargo build in 16m35s, before the game graph grew by about 14,000 lines.

This is not a source compile error. A local build can finish because it has more
capacity than the GitHub-hosted runner.

## Decision

Use thin LTO only for the wasm release and Pages jobs through
`CARGO_PROFILE_RELEASE_LTO`. Keep the checked-in release default and native
`dist` profile on fat LTO. The separate `wasm-opt` step still performs the final
wasm size optimization.

## Proof

- `CARGO_PROFILE_RELEASE_LTO=thin nix develop --command trunk build --release`
  passed from a clean thin-LTO cache in 8m24s, including the post-build metadata
  hook. It generated 246 sidecars and installed `dist/`.
- `nix develop --command cargo metadata --no-deps --format-version 1` passed.
- `git diff --check` passed.
- Actionlint parsed both edited workflows. Its only finding is the pre-existing
  `actions/setup-dotnet@v3` version at `release.yaml:193`.
