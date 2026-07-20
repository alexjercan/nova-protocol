# Review: port nova_portal_gen to a Python build-time script

- TASK: 20260718-152247
- BRANCH: refactor/portal-gen-python

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. The reviewer independently re-ran the parity oracle (`diff -r` empty,
`cmp` on catalog.json identical, 455480 bytes, 2 mods) and reconstructed the 10
rejection cases (both tools exit 1 with byte-identical stderr in every case),
plus ~20 adversarial RON-shape probes - all parity. The core port is correct;
both MAJORs are completeness/accuracy gaps (a missed author-facing doc, and a
removal plan that missed a live consumer), not correctness bugs.

- [x] R1.1 (MAJOR) tasks/20260718-152247/TASK.md - the removal decision missed
  a real consumer: `nova_assets` dev-deps `nova_portal_gen`
  (`crates/nova_assets/Cargo.toml:73`) and `tests/portal_install.rs:336` calls
  `nova_portal_gen::generate(...)` as its real-wire e2e; the "webmods_validation
  doesn't drive it" note hid the real driver.
  - Response: fixed - amended the Rust-crate-fate note to name
    `crates/nova_assets/{Cargo.toml dev-dep, tests/portal_install.rs}` as the
    blocker the removal task must port (shell out to gen-portal.py) or wait on,
    and corrected the misleading webmods_validation framing.
- [x] R1.2 (MAJOR) web/src/wiki/dev/guide-make-a-mod.md:287,323,338 - the
  primary "how to publish a mod" guide still told authors to
  `cargo run -p nova_portal_gen`.
  - Response: fixed - all three (the publish command, the pre-publish check
    step, and the mermaid `Gen` node) now name `python3 scripts/gen-portal.py`.
- [x] R1.3 (MINOR) Trunk.toml:28 - the local-testing dev comment still showed
  the Rust invocation.
  - Response: fixed - now `python3 scripts/gen-portal.py ...`.
- [x] R1.4 (NIT) scripts/gen-portal.py:719 - `import json` was lazy inside
  `generate()`.
  - Response: fixed - hoisted to the module-level import block.
- [x] R1.5 (NIT) scripts/gen-portal.py:335 - the `_is_struct_head` comment
  claimed raw-string keys were allowed, but only `"` and bare idents are.
  - Response: fixed - comment now states raw-string keys are not handled and do
    not occur in webmods manifests.

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (round-1 fixes are doc/comment/import-location only, no
  generator logic changed; verified mechanically)

Confirmed each round-1 fix against the new diff: guide-make-a-mod.md has no
remaining `nova_portal_gen` invocation (grep clean; only the amended TASK.md and
the removal-plan prose mention the crate now), Trunk.toml points at the Python,
`import json` is module-level, the RON-reader comment is accurate, and the
removal decision names portal_install.rs. Re-ran the load-bearing claim myself
after the edits: `diff -r` on the real webmods is byte-identical (gauntlet
1.2.0, the-ledger 1.5.0 / 8 files) and the empty-version rejection still exits 1
on both tools. No new issues introduced.
