# Review: portal client - fetch catalog + staged install/uninstall over the wire

- TASK: 20260715-163508
- BRANCH: feature/portal-fetch

## Round 1

- VERDICT: APPROVE (four MINORs + four NITs, all addressed or explicitly
  dispositioned before landing)

Out-of-context review pass over the full diff (416d5b80). Re-derived and
confirmed: the staged-install discipline holds on every traced early-return
path, native and wasm (guard failures touch job state only; per-file failures
drop staged memory; native commit failure sweeps with no surviving record;
wasm commit is one IDB transaction awaited to complete/error/abort with no
await between puts); sha256 comparison lowercase-hex both sides; total_size
never trusted; deviation 5's generation-free Committed invariant verified
(Committing blocks same-id install AND uninstall); ehttp 0.7.1 pulls
ureq/rustls (no openssl), single wasm-bindgen, dev-only tiny_http; the
location-derivation cases are unit-pinned and correct; all five close-out
sabotage claims consistent, one re-run personally (EnabledMods strip no-op ->
the exact R1.7 assertion failed, revert green). Counts reproduced: lib 38,
portal_install 6, mod_cache_install 7, demo_scenario 11; fmt/check/wasm32 all
green.

- [x] R1.1 (MINOR) URL containment overclaimed: `%2e%2e`/`?`/`#` pass the
  Path-component gates but change meaning per the WHATWG URL parse, letting a
  hostile catalog steer same-origin GETs above the portal base.
  - Response: fixed in bd91ee79 - validate_entry now also enforces the
    generator's published charset (url-safe segments for ids/versions,
    per-component charset for paths; dot-only rejected), the comment states
    the two enforced boundaries precisely, and a unit test rejects %2e%2e/?/
    uppercase before any fetch. The redundant standalone bundle path check was
    subsumed by bundle-in-validated-files (reasoned, accepted).
- [x] R1.2 (MINOR) wasm uninstall/reinstall race (detached removal could
  delete a fresh reinstall's IDB entries).
  - Response: fixed - PendingRemovals resource + Removed channel message; the
    install observer rejects pending ids first; guard unit-tested natively,
    wasm arm compile-gated. Sabotage-verified (guard forced-false -> test
    FAILED).
- [ ] R1.3 (MINOR) no timeout/cancel recovery anywhere - a never-firing
  callback wedges catalog fetch or a job forever.
  - Response: deliberately DEFERRED to 142916 (the UI owns the cancel/retry
    surface); recorded in that task's TASK.md and acknowledged in the module
    doc. Left unticked as a tracked deferral, not an unresolved dispute.
- [x] R1.4 (MINOR) unbounded staging from a hostile catalog.
  - Response: fixed - 32 MiB/file, 256 files, 128 MiB total anti-absurdity
    caps in validate_entry (honest note: a lying server can still send one
    oversized body; the caps bound what the catalog can command). Cap test
    trips all three.
- [x] R1.5 (NIT) stale Failed job survives uninstall. Fixed.
- [x] R1.6 (NIT) duplicate files[].path accepted. Fixed + test assert.
- [x] R1.7 (NIT) sync fs writes in Update. Accepted as repo idiom, no change.
- [x] R1.8 (NIT) await_transaction leak rationale undocumented. Fixed
  (cross-reference).

## Round 2

- VERDICT: APPROVE

Fix commit bd91ee79 verified: lib 41 passed (3 new pins: charset, caps,
pending-guard), portal_install 6, mod_cache_install 7, fmt clean, wasm gate
re-run green on the final tree, worktree clean. The same-function-coupling
honesty note on the caps/duplicate tests is recorded in TASK.md. No new
findings.
