# Credits

This is the single source of truth for Nova Protocol's asset attributions. It
ships with every build: the web build copies `credits/` into `dist/credits/`
(Trunk), and the native release bundles it alongside the executable
(`.github/workflows/release.yaml`). `dist/credits/` is a build artifact and is
gitignored - never hand-edit it, edit this directory.

The game's own code and original assets are covered by the top-level
[`LICENSE`](../LICENSE) (MIT, (c) Alexandru Jercan). This file lists the
THIRD-PARTY material the build includes and the licenses it carries.

## Original assets

All game assets are original to the project unless listed under "Third-party
assets" below:

- **Sounds** (`assets/sounds/*.wav`) - generated placeholders produced by
  `scripts/gen-placeholder-sounds.py`; see `assets/sounds/README.md`.
- **3D models** (`assets/gltf/*.glb`) - exported from the project's own Blender
  sources in `art/blender/`.
- **Textures** (`assets/textures/*.png`) and **UI icons**
  (`assets/icons/*.png`, `assets/banner.png`) - authored for the project.
- **Shaders** (`assets/shaders/*.wgsl`) - written for the project.

## Third-party assets

- **Bevy icon** - [MIT License](licenses/Bevy_MIT_License.md).

## Third-party code

The engine and libraries are Rust crate dependencies (Bevy, avian, and their
transitive deps), licensed MIT / Apache-2.0. Their license texts are not yet
aggregated here; generating a complete dependency-license manifest (e.g. with
`cargo-about`) is a tracked follow-up and out of scope for this asset audit.
