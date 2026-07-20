# Port nova_portal_gen to a Python build-time script (interim mod portal generator before the hosted API), matching the catalog.json wire schema

- STATUS: CLOSED
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

- [x] Capture the exact `catalog.json` wire schema from `nova_mod_format` (the
      portal wire types) and the current `nova_portal_gen::generate` behavior:
      file discovery rules, id = dir name, version handling, hashing
      algorithm, shipped-catalog id-collision check
      (`--shipped assets/mods.catalog.ron`), output layout, and the manifest
      validation gates it runs (bundle parses, publishable meta, content files
      exist, unique ids, deps resolve) - the gates are part of the behavior,
      not an extra.
- [x] Write `scripts/gen-portal.py` reproducing that behavior byte-for-byte on
      the catalog.json and the hashed file tree. Stdlib only if possible; pin
      any dependency in the nix flake.
- [x] Add a parity check: run both the Rust bin and the Python script over
      `webmods/` and diff the output trees; keep the Rust bin as the oracle
      until parity holds on real webmods (gauntlet multi-version history,
      the-ledger multi-file bundle).
- [x] Update the deploy workflow (`.github/workflows/deploy-page.yaml`),
      `scripts/preview-web.sh` if it invokes the generator, and the docs:
      `web/src/wiki/dev/mod-portal.md` + README tools section
      (20260718-152205).
- [x] Decide the removal point for `nova_portal_gen` (same release after
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

## Rust-crate fate (decision)

KEEP `nova_portal_gen` for ONE release as the byte-for-byte parity ORACLE, then
remove it. Reasoning: parity is proven now (diff -r empty on the real webmods,
plus 10 matched rejection cases), but `scripts/gen-portal.py` is brand new and
carries a hand-written RON reader; leaving the Rust generator in the tree for
one release costs nothing (it is engine-free and no longer on the deploy hot
path) and lets a `cargo run -p nova_portal_gen ... && diff` re-confirm parity if
the Python is ever touched. The deploy workflow, local preview, README, and
mod-portal.md now name the Python script as THE generator; the README crate
table and mod-portal.md both record the crate is superseded and slated for
removal. Follow-up: file a removal task for `crates/nova_portal_gen` at the next
release cut. The removal is NOT just deleting the crate + workspace member +
its own `tests/generate.rs` + the README crate row - it has a live in-tree
CONSUMER that must be ported or retired first (review R1.1): `nova_assets` takes
`nova_portal_gen` as a dev-dependency (`crates/nova_assets/Cargo.toml:73`) and
`crates/nova_assets/tests/portal_install.rs` calls `nova_portal_gen::generate(...)`
directly as its "real wire" end-to-end portal-install test. So the removal task
must EITHER port `portal_install.rs` to shell out to `scripts/gen-portal.py`
(losing the typed API but keeping the coverage), OR let the crate live as a
test-only library until `portal_install.rs` is otherwise retired. Note:
`webmods_validation` does NOT drive the generator (it uses the real bevy
loaders); the real consumer is `portal_install.rs`, which the first close-note
missed.

## Close-out

What changed:
- Added `scripts/gen-portal.py` - a stdlib-only (json, hashlib, shutil,
  pathlib, argparse, sys) Python port of `nova_portal_gen::generate` + its CLI.
  Includes a minimal hand-written RON reader (comments, strings/raw-strings,
  chars, numbers, seq/map/tuple/struct/enum) sufficient to (a) read
  `mods.catalog.ron` ids, (b) read `*.bundle.ron` fields, and (c) parse
  `*.content.ron` to a value tree for the self://, dep://, and bare-asset ref
  walks. All manifest publish gates are reproduced with identical error text.
- Deploy workflow `.github/workflows/deploy-page.yaml` and
  `scripts/preview-web.sh` now invoke `python3 scripts/gen-portal.py`.
- Docs: `web/src/wiki/dev/mod-portal.md` (generator section, mermaid participant,
  publishing steps, all three local-dev invocations) and `README.md` (tools
  table + crate list) name the Python script as THE way to publish.
- No nix flake change: stdlib only, nothing to pin.

Parity commands + results (clean):
- Build/run oracle then port over the real webmods and diff:
  `cargo run -q -p nova_portal_gen -- --source webmods --shipped assets/mods.catalog.ron --out /tmp/rust-portal`
  `python3 scripts/gen-portal.py --source webmods --shipped assets/mods.catalog.ron --out /tmp/py-portal`
  `diff -r /tmp/rust-portal /tmp/py-portal`  ->  EMPTY (byte-for-byte identical,
  incl. `cmp catalog.json` identical: 2-space indent, no trailing newline,
  non-ASCII unescaped, ModMeta field order name/description/author/version/
  dependencies/icon/screenshots with `icon: null` and empty `screenshots: []`).
  Covers the tricky inputs: the-ledger's 5-content multi-file bundle (8 files)
  and gauntlet (the published version is the manifest's meta.version = 1.2.0;
  the "multi-version history" lives only in its CHANGELOG.md, which is copied
  verbatim - there is no multi-version dir tree in the source).
- Rejection parity (10 cases, each identical message + exit=1 on BOTH tools):
  empty version, missing content file, shipped-id collision (`example`), empty
  portal, bare (scheme-less) asset ref, malformed `dep://`, undeclared `dep://`
  target, undeclared `self://` resource, two `*.bundle.ron` at root, and a
  dependency that resolves to nothing. Plus a positive case exercising declared
  resources + `icon: Some(...)` + screenshots + a valid `self://` + `dep://base`
  ref: `diff -r` empty.

Difficulties:
- JSON formatting parity: `serde_json::to_string_pretty` writes 2-space indent,
  `": "` after keys, no trailing spaces, and NO trailing newline; Python's
  `json.dumps(obj, indent=2, ensure_ascii=False)` + `write_text` (no added `\n`)
  matches exactly. `ensure_ascii=False` is required (serde does not escape
  non-ASCII). Field order is guaranteed by building the dicts in Rust struct
  declaration order. All three landed on the first `cmp`.
- The RON reader was the real work. The content files are large and use enums,
  tuples, chars, and heavy comments; the port only needs String leaves, so the
  reader flattens structs -> dicts and tuples/variants -> lists and never
  interprets numbers/chars/bare-idents (wrapped in a `_Unit` sentinel the ref
  walk ignores). Block-comment nesting and `Some(x)` unwrapping were the fiddly
  bits; verified indirectly by the byte-for-byte parity (a mis-parse would drop
  or fabricate a ref and change a gate outcome).
- File-sort parity: Rust sorts `PathBuf`s; Python sorts on `path.parts` to match
  component-wise ordering rather than raw string ordering.

Self-reflection:
- Building the Rust oracle FIRST and diffing against a captured target made this
  fast and self-proving - the DoD was the test. Next time on a byte-parity port
  I would do exactly this again.
- The hand-written RON reader is the one fragile spot; it is justified because a
  stdlib-only constraint rules out `ron`, and parity is pinned by the oracle for
  one release. Worth keeping the oracle around precisely because of it.
