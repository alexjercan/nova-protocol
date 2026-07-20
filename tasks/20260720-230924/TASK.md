# Remove the nova_portal_gen crate (gen-portal.py is the production generator); preserve publish-gate coverage

- STATUS: OPEN
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

- [ ] Preserve the publish-gate coverage on the PRODUCTION tool: add a Rust
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
- [ ] Repoint `crates/nova_assets/tests/portal_install.rs`: replace the
      `nova_portal_gen::generate(...)` call (~line 336) with a subprocess run of
      `python3 scripts/gen-portal.py --source <src> --shipped
      ../../assets/mods.catalog.ron --out <portal_dir>`; assert it exits 0 and
      read `<portal_dir>/catalog.json` to confirm the fixture id publishes
      (replacing the `generated.entries` check). The rest of the e2e (serve the
      tree, fetch/install/enable/uninstall) is unchanged - it reads the portal
      tree from disk.
- [ ] Delete the crate: `git rm -r crates/nova_portal_gen` (bin + lib + its
      `tests/generate.rs`). `rm -rf` the emptied dir if git leaves it.
- [ ] Remove it from the workspace: drop `"crates/nova_portal_gen"` from BOTH
      `[workspace] members` AND `default-members` in the root `Cargo.toml`;
      remove the `nova_portal_gen = { path = ... }` dev-dependency from
      `crates/nova_assets/Cargo.toml` and fix the two comments there that name
      it (lines ~20, ~72).
- [ ] Doc + reference sweep (grep `nova_portal_gen` tree-wide, fix every live
      surface): README crate table + any tools row, `AGENTS.md` crate table,
      `web/src/wiki/dev/{project-tour,architecture,mod-portal,guide-make-a-mod}.md`,
      and the `deploy-page.yaml` comment ("soon-to-be-removed nova_portal_gen"
      -> removed; gen-portal.py is THE generator). Leave historical task/spike
      files (152247 etc) as-is - they narrate the crate that existed.
- [ ] Update `scripts/gen-portal.py`'s header comment if it calls itself a
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
