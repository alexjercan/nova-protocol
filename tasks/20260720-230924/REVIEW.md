# Review: remove the nova_portal_gen crate (preserve publish-gate coverage)

- TASK: 20260720-230924
- BRANCH: refactor/remove-portal-gen

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. It confirmed the crate is fully removed (deleted, dropped from
`members` + `default-members`, dev-dep gone, Cargo.lock clean, DoD grep clean),
did a CASE-BY-CASE comparison of the deleted `generate.rs` against the new
`gen_portal_gate.rs` (all 15 rejection cases re-homed, none dropped), ran
gen-portal.py on 3 fixtures itself to confirm they reject for the RIGHT reason,
and re-ran the tests (`gen_portal_gate` 4 passed, `portal_install` 9 passed,
`cargo check --workspace` clean, `npm run ci` green). All findings MINOR/NIT.

- [x] R1.1 (MINOR) gen_portal_gate.rs - rejection rows asserted only a non-zero
  exit, not the error reason (a gate rejecting for the WRONG reason would pass).
  - Response: fixed. Added a `stderr_contains: &'static str` field to `Reject`
    and assert it per row, porting the substrings from the deleted generate.rs's
    `err.0.contains(...)` checks (mapped to gen-portal.py's actual messages:
    "collides with a SHIPPED catalog id", "is not a file inside", "listed
    resource file", "references undeclared mod resource", "meta.name is
    required", "refusing to publish an empty portal", "meta.version is
    required", "is invalid: use lowercase ascii", "no *.bundle.ron at the mod
    root", "is neither a portal mod", "is not a declared dependency", "of
    dependency", "malformed dependency resource ref", "with no scheme"). All 15
    pass, so each gate rejects for its own reason today, and a future
    reason-regression now fails the test.
- [x] R1.2 (MINOR) gen_portal_gate.rs:48 - the python3 self-skip could silently
  drop the only gate coverage on a runner without python3, including CI.
  - Response: fixed. `python3_missing()` now hard-fails (asserts) when the `CI`
    env var is set - CI never silently skips - while a local runner without
    python3 still self-skips per the repo convention.
- [ ] R1.3 (NIT) webmods/gauntlet/CHANGELOG.md:28 - the frozen `## 1.0.0` entry
  still says "published ... by `nova_portal_gen`".
  - Response: left as-is. A CHANGELOG entry is a frozen historical record of
    what shipped at 1.0.0 (when the crate was the generator); rewriting past
    release notes would be dishonest. Not in the DoD grep scope; grep is clean.
- [ ] R1.4 (NIT) gen_portal_gate.rs - the old positive's sha256/size recompute +
  sorted-order asserts (which poke typed `PortalCatalog` internals) were not
  re-ported.
  - Response: accepted as a scoping call. Hash INTEGRITY is covered end to end
    by `portal_install.rs` - its install path downloads each file and verifies
    it against the catalog's `sha256` + `size`, so a wrong hash from
    gen-portal.py fails the e2e. Ordering is covered by the whole-tree
    byte-identity determinism test. Re-deserializing catalog.json into
    `PortalCatalog` to re-add direct asserts is a possible future strengthening,
    not a coverage gap that ships unguarded.

No BLOCKER/MAJOR. Gate coverage genuinely transferred (and, after R1.1,
faithfully - reason-checked, not just exit-code).
