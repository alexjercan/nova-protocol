# Package the game as a nix flake output: nix run .#default

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.12.0,tooling,nix

v0.12.0, standalone tooling. Owner ask 2026-08-24: `nix run .#default`
should launch the game, so it can be consumed as a nix package.

## Goal

Add `packages.default` (and the matching `apps.default`) to `flake.nix` so
`nix run` / `nix build` / `nix profile install` work on the game. The flake
currently exports only `devShells.default` (flake.nix:36).

## What the package must get right

- **Binary**: the root crate `nova-protocol` (Cargo.toml:2), release
  profile. The workspace pins nightly 2026-07-03 - the same pin the
  devShell builds (flake.nix:31, kept in lockstep with
  rust-toolchain.toml); the package must use it too, not stable.
  The flake already imports rust-flake's flakeModules (flake.nix:15-16);
  check whether its crane-based packaging gives this for free before
  hand-rolling `buildRustPackage` with an overridden rustPlatform.
- **Assets travel with the binary.** The game loads `assets/` at runtime and
  registers a `mods://` source before AssetPlugin
  (nova_core/src/lib.rs:189-192). Install `assets/` into the store and
  point the binary at it (BEVY_ASSET_ROOT in a wrapper, or a wrapper that
  cds to the share dir). Decide whether `webmods/` ships in the package or
  stays a dev-only source; record the call here.
- **Runtime libraries.** The devShell exports LD_LIBRARY_PATH for
  vulkan-loader, wayland, x11, libxkbcommon, alsa, udev (flake.nix:56-68).
  A package cannot rely on the shell: wrap the binary (wrapProgram with
  makeLibraryPath) or patchelf the runpath, covering both wayland and x11.
- **Build reproducibility**: sccache/RUSTC_WRAPPER is a devShell convenience
  and must NOT leak into the package derivation; cargo vendoring via the
  lock file.

## Non-goals

- No NixOS module, no cachix, no cross-compilation, no wasm packaging - the
  web build has its own CI path.
- Do not restructure the devShell; it stays as-is.

## Done when

- `nix run .#default` launches the game from a clean checkout outside the
  devShell, on x11 and wayland.
- `nix build` produces a result/ whose binary finds assets from the store.
- `nix flake check` passes; the pinned-nightly lockstep comment
  (flake.nix:27-30) is extended to cover the package.
- CI is untouched or extended deliberately, not accidentally rebuilding the
  world.
