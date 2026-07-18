# Port nova_portal_gen to a Python build-time script (interim mod portal generator before the hosted API), matching the catalog.json wire schema

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.8.0,tooling,modding,refactor

## Story

As the project owner, I want the mod portal generator to live as a small
Python build-time script instead of a Rust crate, so that the interim tool
(which the hosted-mods API will eventually replace wholesale) is cheap to
iterate on and cheap to throw away.

It is a static generator today (`nova_portal_gen`, engine-free, no bevy) that
scans `webmods/`, copies + content-hashes every file, and emits `catalog.json`
+ `<id>/<version>/<files>`. Being engine-free is exactly what makes it safely
portable (contrast the meta-gen spike 20260718-152255, where Bevy coupling
makes porting questionable).

## Steps

- [ ] Capture the exact `catalog.json` wire schema from `nova_mod_format` (the
      portal wire types) and the current `nova_portal_gen::generate` behavior:
      file discovery rules, id = dir name, version handling, hashing
      algorithm, shipped-catalog id-collision check
      (`--shipped assets/mods.catalog.ron`), output layout, and the manifest
      validation gates it runs (bundle parses, publishable meta, content files
      exist, unique ids, deps resolve) - the gates are part of the behavior,
      not an extra.
- [ ] Write `scripts/gen-portal.py` reproducing that behavior byte-for-byte on
      the catalog.json and the hashed file tree. Stdlib only if possible; pin
      any dependency in the nix flake.
- [ ] Add a parity check: run both the Rust bin and the Python script over
      `webmods/` and diff the output trees; keep the Rust bin as the oracle
      until parity holds on real webmods (gauntlet multi-version history,
      the-ledger multi-file bundle).
- [ ] Update the deploy workflow (`.github/workflows/deploy-page.yaml`),
      `scripts/preview-web.sh` if it invokes the generator, and the docs:
      `web/src/wiki/dev/mod-portal.md` + README tools section
      (20260718-152205).
- [ ] Decide the removal point for `nova_portal_gen` (same release after
      parity, or one release as oracle) and record it; also update the
      `webmods_validation` test if it drives the Rust generator directly.

## Definition of Done

- `python scripts/gen-portal.py --source webmods --shipped
  assets/mods.catalog.ron --out site/mods` (flags may differ) produces a tree
  byte-identical to the Rust generator's over the current webmods, including
  rejection behavior on a deliberately broken manifest.
- Deploy workflow and local preview use the script; docs name it as THE way to
  publish.
- The Rust crate's fate (removed or oracle-for-one-release) is decided and
  recorded.

## Notes

- Wire types are in `crates/nova_mod_format`; generator in
  `crates/nova_portal_gen/{src/lib.rs,src/main.rs}`.
- This is explicitly interim: the plan is to scrap portal generation when the
  mods API lands. Do not over-engineer the Python; match the schema and move
  on.
- Coordinate wording with 20260718-231601 (publish-vs-load split doc): the
  generator's gates are the "publish" half of that split.
