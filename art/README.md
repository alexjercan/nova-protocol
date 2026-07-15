# Source art

Non-runtime source art for Nova Protocol. Nothing here is loaded by the game or
shipped in a build - it is kept in the repo so the runtime assets can be
regenerated.

Keep it OUT of `assets/`: that directory is copied wholesale into every shipped
build (web via Trunk `copy-dir`, native via `release.yaml`), so anything there
adds to the download whether or not the game loads it.

## Contents

- `blender/` - the Blender sources (`.blend`) the runtime `assets/gltf/*.glb`
  models are exported from. `.blend1`/`.blend2` autosave backups are gitignored.
