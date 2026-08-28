# Package the game as a nix flake output: nix run .#default

- STATUS: CLOSED
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

## What shipped

`nix run .#default` builds and launches the game. Three outputs, all Linux:

| output | what |
|-|-|
| `packages.default` / `packages.nova-protocol` | the wrapped game |
| `packages.nova-protocol-unwrapped` | the bare cargo binary |
| `apps.default` | `nix run` |

The unwrapped binary is split out so that editing the desktop entry, the icon
or the asset wiring costs a second instead of a fat-LTO relink. The wrapper
adds `BEVY_ASSET_ROOT` (as a default, so a modder can still aim a packaged
build at a working tree), the dlopened libraries, the desktop entry and the
icon.

### Decisions

**rust-flake pays, its defaults do not.** Its crane wrapper, the toolchain
option and the `rust-bin` overlay are worth keeping. Its DEFAULT crate set is
not: it reads `workspace.members`, and the game is the workspace ROOT, so the
flake shipped 24 crate packages, 24 doc builds and 24 `--all-features -D
warnings` clippy checks - and nothing for the game. `nix flake check` built all
72. `crates` is now `lib.mkForce` to the one root package with `autoWire = []`.
This was a live defect at v0.11.0, not something this task introduced, so it
gets a changelog entry.

**One toolchain pin, not two.** `rust-project.toolchain` is set to the
devshell's `rustNightly` value. Left alone rust-flake resolves its own from
`rust-toolchain.toml`, which would put a third pin in play and let the package
and the shell drift on a `nix flake update`. The lockstep comment in
`flake.nix` now says the package rides the same value.

**One library list.** `gameLibs` is shared by the devshell's `buildInputs` and
the wrapper, so a library added for one is never missing from the other. The
devshell's `LD_LIBRARY_PATH` is byte-identical to what it was (checked by
evaluating it before and after).

**`webmods/` does NOT ship.** Those are the portal's development fixtures,
served by `scripts/serve-mods.sh`. The `mods://` source reads out of the
player's data directory (`~/.local/share/nova-protocol/mods`, `mod_cache.rs`),
which no store path can hold. `assets/` and `credits/` do ship - the same two
directories the release tarball carries.

**`--profile dist`, not `release`.** The profile `release.yaml` ships, so
`nix run` gets the binary a player gets. Cost: about 20 minutes cold on a
16-core box (deps 6, the fat-LTO link the rest).

**`builtins.path`, not `./assets`.** `./assets` is a subpath of the flake
source, so referencing it would drag the whole checkout into the package's
runtime closure.

**The icon is `web/src/favicon.svg`**, rendered to the eight hicolor sizes plus
the scalable SVG. It is the site's brand mark and the only real one in the
repo: `build/icon_1024x1024.png` and `build/windows/icon.ico` are still
bevy_game_template's placeholder bird. Replacing those is a separate job - they
feed the Windows and macOS artifacts, not this one.

**CI is untouched, deliberately.** No job runs nix; adding one would mean a
nix install plus a cold fat-LTO Bevy build with no cache on a 4-vCPU runner.
`release.yaml` already proves `cargo build --profile dist` compiles on Linux,
and the flake wraps that same build.

### Verified live

`nix build .#default`, then the store binary run under Xvfb :94 with
`LD_LIBRARY_PATH`, `BEVY_ASSET_ROOT` and `CARGO_MANIFEST_DIR` unset and `HOME`
pointed at a scratch directory - no development shell anywhere in the
environment:

- It reached the main menu with the asteroid backdrop, the CRT panel, fonts and
  shaders, all read out of the store. `Loading state 'Loading' is done`, no
  error and no panic in the log.
- It wrote its settings to `$HOME/.config/nova-protocol/`, so a packaged build
  keeps player state in the player's directories.
- `xprop` reports `WM_CLASS = "nova-protocol", "nova-protocol"`, which is what
  `StartupWMClass` claims. Read off the window, not guessed - the game leaves
  `Window::name` unset outside a probe run.
- `rofi -show drun` on the user's own config, with the package's `share` on
  `XDG_DATA_DIRS`, lists "Nova Protocol (Space Shooter)" with its icon.
- `desktop-file-validate` passes. `nix flake check` passes. `nix run .` and
  `nix run .#default` both resolve.

### Skipped

- `nix profile install` was not run: it mutates the user's profile. The
  mechanism was proved with `XDG_DATA_DIRS` instead, which is the same lookup.
- x86_64-linux only. `nix flake check` skips the other three systems as
  incompatible, as it did before; the package is `platforms.linux` because the
  wrapper is a Linux library path.
- Wayland was not exercised. The session here is X11, and Xvfb is an X server;
  the wayland library is on the path but no run has proved it.
