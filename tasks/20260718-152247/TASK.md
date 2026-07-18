# Port nova_portal_gen to a Python build-time script (interim mod portal generator before the hosted API), matching the catalog.json wire schema

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.8.0,tooling,modding,refactor

## Goal

Per the user's direction: the mod portal generator should become a Python
build-time script. It is a static generator today (`nova_portal_gen`,
engine-free, no bevy) that scans `webmods/`, copies + content-hashes every file,
and emits `catalog.json` + `<id>/<version>/<files>`. Since the hosted-mods API
will eventually replace the portal entirely (and the .rs code scrapped), a
Python script is the cheaper interim home and easier to iterate on.

## Steps

- Capture the exact `catalog.json` wire schema from `nova_mod_format` (the
  portal wire types) and the current `nova_portal_gen::generate` behavior:
  file discovery rules, id = dir name, version handling, hashing algorithm,
  shipped-catalog id-collision check (`--shipped assets/mods.catalog.ron`),
  output layout.
- Write `scripts/gen-portal.py` reproducing that behavior byte-for-byte on the
  catalog.json and the hashed file tree.
- Add a parity check: run both the Rust bin and the Python script over
  `webmods/` and diff the output trees; only remove `nova_portal_gen` once they
  match (or keep the Rust bin one release as the oracle).
- Update the deploy workflow (`.github/workflows/deploy-page.yaml`) and
  `web/src/wiki/dev/mod-portal.md` + README tools section to call the script.

## Notes

- Wire types are in `crates/nova_mod_format`; generator in
  `crates/nova_portal_gen/{src/lib.rs,src/main.rs}`.
- This is explicitly interim: the plan is to scrap portal generation when the
  mods API lands. Do not over-engineer the Python; match the schema and move on.

