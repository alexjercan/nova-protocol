# Remove the nova_portal_gen crate (gen-portal.py is the production generator); preserve publish-gate coverage

- STATUS: CLOSED
- PRIORITY: 38
- TAGS: v0.8.0,tooling,refactor

## Story

As the project owner, I want the dead `nova_portal_gen` Rust crate removed now
that `scripts/gen-portal.py` is the production portal generator on every real
path (deploy workflow + preview script), so the workspace stops carrying a tool
nothing invokes - WITHOUT losing the publish-gate test coverage the crate's
tests are currently the only committed home for.

The crate was kept one release as a parity oracle (task 20260718-152247) and
because two consumers block a bare delete: `nova_assets` dev-deps it and
`tests/portal_install.rs` calls `nova_portal_gen::generate()` as its real-wire
e2e; and `nova_portal_gen/tests/generate.rs` holds ~22 gate/rejection cases that
are the ONLY committed automated coverage of the publish gates (the Python
parity was proven by a one-off diff, not a committed test).

## Steps

- [x] Preserve the publish-gate coverage on the PRODUCTION tool: add a Rust
      integration test `crates/nova_assets/tests/gen_portal_gate.rs` that drives
      `python3 scripts/gen-portal.py` via `std::process::Command` over fixtures,
      porting the gate cases from `crates/nova_portal_gen/tests/generate.rs` -
      at minimum every REJECTION case (id-collision-with-shipped, missing
      content file, escaping content path, missing/undeclared resource ref,
      empty name/source/version, invalid id, missing manifest, unresolvable dep,
      malformed/undeclared dep ref, bare asset ref) each asserting a NON-ZERO
      exit, plus key positives (real `webmods/` publishes exit 0 with a
      catalog.json naming gauntlet + the-ledger; a valid synthetic mod
      publishes; determinism = two runs byte-identical). Table-drive the
      rejections to keep it compact. Skip-with-note if `python3` is absent
      (match the repo's self-skip pattern), so an unusual runner does not hard-fail.
- [x] Repoint `crates/nova_assets/tests/portal_install.rs`: replace the
      `nova_portal_gen::generate(...)` call (~line 336) with a subprocess run of
      `python3 scripts/gen-portal.py --source <src> --shipped
      ../../assets/mods.catalog.ron --out <portal_dir>`; assert it exits 0 and
      read `<portal_dir>/catalog.json` to confirm the fixture id publishes
      (replacing the `generated.entries` check). The rest of the e2e (serve the
      tree, fetch/install/enable/uninstall) is unchanged - it reads the portal
      tree from disk.
- [x] Delete the crate: `git rm -r crates/nova_portal_gen` (bin + lib + its
      `tests/generate.rs`). `rm -rf` the emptied dir if git leaves it.
- [x] Remove it from the workspace: drop `"crates/nova_portal_gen"` from BOTH
      `[workspace] members` AND `default-members` in the root `Cargo.toml`;
      remove the `nova_portal_gen = { path = ... }` dev-dependency from
      `crates/nova_assets/Cargo.toml` and fix the two comments there that name
      it (lines ~20, ~72).
- [x] Doc + reference sweep (grep `nova_portal_gen` tree-wide, fix every live
      surface): README crate table + any tools row, `AGENTS.md` crate table,
      `web/src/wiki/dev/{project-tour,architecture,mod-portal,guide-make-a-mod}.md`,
      and the `deploy-page.yaml` comment ("soon-to-be-removed nova_portal_gen"
      -> removed; gen-portal.py is THE generator). Leave historical task/spike
      files (152247 etc) as-is - they narrate the crate that existed.
- [x] Update `scripts/gen-portal.py`'s header comment if it calls itself a
      "port of the Rust nova_portal_gen crate" (the crate is now gone - reword
      to "the static mod-portal generator" and drop the "soon-to-be-removed"
      framing).

## Definition of Done

- `crates/nova_portal_gen` is gone and nothing references it
  (cmd: `! test -e crates/nova_portal_gen && ! grep -rn 'nova_portal_gen' Cargo.toml crates/*/Cargo.toml README.md AGENTS.md web/ .github/`).
- The publish gates are still tested against the production tool: the new gate
  test rejects a broken manifest and passes real webmods
  (test: `cargo test -p nova_assets --test gen_portal_gate`).
- The e2e portal-install test still passes driving gen-portal.py
  (test: `cargo test -p nova_assets --test portal_install`).
- The workspace builds without the crate
  (cmd: `cargo check --workspace --all-targets`).

## Notes

- Prior decision: task 20260718-152247 close-notes + its REVIEW.md R1.1 recorded
  this removal as the deferred follow-up and named portal_install.rs as the
  blocker. This task executes it.
- `deleted-content-tests-carry-engine-coverage`: `tests/generate.rs` is the only
  committed exercise of the publish gates - re-home that coverage on gen-portal.py
  BEFORE deleting, do not just drop it.
- gen-portal.py flags: `--source <dir> --shipped <catalog.ron> --out <dir>`
  (byte-parity with the Rust generator proven in 152247).
- CI: ci.yaml runs `cargo test --workspace` on ubuntu-latest (python3 present);
  deploy already runs gen-portal.py, so python3 is a CI given - but the skip-if-
  absent guard keeps a local run without python3 from hard-failing.
- nova_portal_gen is engine-free (no bevy), so it currently sits in
  default-members (cheap); removal drops it from both lists.

## Close-out

Deleted `crates/nova_portal_gen` (bin `main.rs` + lib `lib.rs` + its 22-case
`tests/generate.rs`), dropped it from the root `Cargo.toml` `members` +
`default-members`, and removed the `nova_assets` dev-dependency on it.

### How gate coverage was re-homed

New test `crates/nova_assets/tests/gen_portal_gate.rs` drives the PRODUCTION
tool `python3 scripts/gen-portal.py` via `std::process::Command` (absolute
`--source/--shipped/--out`, cwd = repo root resolved from `CARGO_MANIFEST_DIR`).
It self-skips with a printed note if `python3` is not runnable.

Rejection cases ported (TABLE-DRIVEN off one `Reject { name, with_shipped, build }`
row list, each asserting a NON-ZERO exit) - 15 rows, one per gate in the old
`generate.rs`:

- id-collision-with-shipped (needs `--shipped`)
- missing-content-file
- escaping-content-path (escaped target exists next to the mod dir; membership
  gate still rejects it)
- missing-resource-file
- content-ref-to-undeclared-resource (`self://` to an undeclared resource)
- empty-name
- empty-source (no mods under `--source`)
- empty-version
- invalid-id (`Bad_Mod`)
- missing-bundle-manifest
- unresolvable-dependency
- dep-ref-to-non-declared-dependency
- dep-ref-to-undeclared-resource-of-dependency
- malformed-dep-ref (`dep://art`)
- bare-asset-ref (scheme-less `textures/cubemap.png`)

Positives ported as individual tests: `real_webmods_publishes_and_lists_both_mods`
(exit 0, catalog.json names gauntlet + the-ledger), `synthetic_valid_mod_publishes`
(exit 0, catalog names the mod), `generation_is_deterministic` (two runs over
webmods produce byte-identical trees - compared over the whole tree, not just
catalog.json). Positive-membership cases from generate.rs that were NOT rejections
(declared_resources_publish, content-ref-to-declared-resource, dep-ref-to-declared-
dependency-resource, dep-ref-to-base-publishes) are implicitly exercised by the
real-webmods positive and the fixture rig being valid; they were not each re-added
as their own row since they assert acceptance, which the two publish positives
already cover.

Hash/size recompute-from-copied-bytes and sorted-order assertions from the old
`real_webmods_publish_and_hashes_verify` were NOT re-ported verbatim: they poke
`PortalCatalog` internals (typed `entries`/`files`) which this test does not
deserialize. Byte-level correctness is instead covered by the determinism test
(whole-tree byte identity) and by the existing `webmods_validation.rs` deep-load
test; the size/sha fields are produced by the same code path the deploy runs.

### How portal_install now generates

`crates/nova_assets/tests/portal_install.rs` (~line 336) no longer calls
`nova_portal_gen::generate`. It runs `python3 scripts/gen-portal.py --source
<fixture> --shipped <abs assets/mods.catalog.ron> --out <portal_dir>` as a
subprocess (repo root from `CARGO_MANIFEST_DIR`, absolute paths), asserts exit 0,
reads `<portal_dir>/catalog.json` and asserts it names `FIXTURE_ID`
("fixture-slalom"). Everything downstream (serve tree over tiny_http, fetch /
install / enable / uninstall) reads the on-disk portal tree and is unchanged.

### Verification (via `nix develop --command cargo ...`)

- `cargo check --workspace --all-targets`: clean (workspace builds without the crate).
- `cargo test -p nova_assets --test gen_portal_gate`: 4 passed (15 rejections + 3 positives).
- `cargo test -p nova_assets --test portal_install`: 9 passed (e2e now drives gen-portal.py).
- `cargo fmt` applied; `cargo fmt --check` clean.
- DoD grep over `Cargo.toml crates/*/Cargo.toml README.md AGENTS.md web/ .github/`: no matches.

Beyond the DoD scope I also updated the doc-comments in `nova_mod_format`,
`nova_assets::portal`, `webmods_validation.rs`, `cubemap_meta_app_config.rs`,
`portal_install.rs`, and the two webmods bundle comments + gauntlet README that
named `nova_portal_gen` as the LIVE generator, repointing them to
`scripts/gen-portal.py`. Historical narration (tasks/spikes, gauntlet CHANGELOG
1.0.0, LESSONS.md, and the one intentional lineage note in gen_portal_gate.rs's
docstring) was left as-is.

### Difficulties

- The sandbox's `cargo`/`rustc` on PATH could not execute (nix store glibc
  interpreter mismatch). Resolved by running everything through `nix develop
  --command cargo ...`, which provides a working toolchain - so this task was
  fully test-verified locally, not just CI-deferred.

### Self-reflection

- Re-homing coverage onto a subprocess loses the fine-grained `err.0.contains(...)`
  message assertions the Rust test had (each row only asserts NON-ZERO exit).
  That is the right tradeoff for a black-box tool test (asserting exact stderr
  wording would couple the test to phrasing), but it means a gate that rejects for
  the WRONG reason would still pass. The `synthetic_valid_mod_publishes` positive
  guards the rig, so a broken fixture cannot silently make all rows "pass". If
  message-precision matters later, assert a stable substring of stderr per row.
- The python3 skip-guard is the one place a real failure could be masked: if CI
  ever lost python3, this test would silently skip rather than fail. Mitigated
  because deploy also runs gen-portal.py (a missing python3 breaks deploy loudly)
  and ci.yaml is ubuntu-latest where python3 is guaranteed. A stricter option
  would be to fail (not skip) when `CI` is set; not done here to match the repo's
  existing self-skip convention.
