# Review: Clear compiler and rustdoc warnings for v0.10.0

- TASK: 20260731-205553
- BRANCH: master (landed in place, no feature branch - owner directive)

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

Reviewed `946d03a6` against `86689890`. No BLOCKER or MAJOR. The five MINOR
findings are one shape: a broken intra-doc link was de-linked where the target
was in fact a reachable public item, so the correct fix was an explicit path,
not dropping the brackets. Fixed in `3c479f20` and confirmed by the same
reviewer.

- [x] R1.1 (MINOR) crates/nova_assets/src/portal/transport.rs:42 -
  `PortalPlugin` was de-linked although a valid public target exists
  (`portal/mod.rs:283`, inside `pub mod portal`); relink as
  `[`PortalPlugin`](super::PortalPlugin)`.
  - Response: Fixed in 3c479f20 as suggested.
- [x] R1.2 (MINOR) crates/nova_assets/src/portal/mod.rs:30 - `EnabledMods` was
  de-linked but is publicly re-exported at `nova_assets/src/lib.rs:54`; relink
  as `[`EnabledMods`](crate::EnabledMods)`.
  - Response: Fixed in 3c479f20 as suggested.
- [x] R1.3 (MINOR) crates/nova_assets/src/merge.rs:38 -
  `mark_downloaded_bundles_loaded` was de-linked but is a `pub fn`
  (`mod_set.rs:333`) re-exported at `lib.rs:53`; relink as
  `[`mark_downloaded_bundles_loaded`](crate::mark_downloaded_bundles_loaded)`.
  - Response: Fixed in 3c479f20 as suggested.
- [x] R1.4 (MINOR) crates/nova_gameplay/src/camera/mod.rs:69 -
  `SpaceshipSystems` (line 69) and `NovaGameplayPlugin` (line 74) were
  de-linked but both are `pub` in `pub mod plugin` (`plugin.rs:27`, `:38`);
  relink via `crate::plugin::`.
  - Response: Fixed in 3c479f20; both use the `crate::plugin::` path.
- [x] R1.5 (MINOR) crates/nova_probe/src/contract.rs:27 - `perf_param` was
  de-linked but is a `pub fn` in `pub mod capture`; relink to
  `crate::capture::perf_param`.
  - Response: Fixed in 3c479f20 as suggested.
- [x] R1.6 (NIT) tasks/20260731-205553/NOTES.md - the Result table says a
  duplicated `#[allow(clippy::too_many_arguments)]` was removed at
  `nova_os_map/scene.rs`, but the diff removes BOTH copies; reword.
  - Response: Reworded in 3c479f20.
- [x] R1.7 (NIT) crates/nova_os/src/shell.rs:78 - the rewrap left an orphan
  short line ("future queued/over-time,") mid-paragraph; re-flow it.
  - Response: Re-flowed in 3c479f20.

Verified by the reviewer, re-derived in session:

- All three DoD proof commands exit 0 on `946d03a6` and again on `3c479f20`.
  The only remaining output is the third-party `proc-macro-error2`
  future-incompat note, which TASK.md declares out of scope.
- Every behavior-touching edit is semantics-preserving: `chunks_exact(N)` ->
  `as_chunks::<N>().0` (same remainder drop, `mesh::` tests pass),
  `or_else` -> early return, struct-literal init, derived `Default` with
  `#[default] Number` matching the deleted hand impl, `then` -> `then_some`,
  `write!`+`\n` -> `writeln!` at all four sites.
- Both `assert!` -> `const _: () = assert!(...)` conversions still evaluate,
  and now fail at compile time rather than test time - strictly stronger.
- No test was deleted or weakened; every `#[cfg(test)]`/`tests/` edit is an
  assertion rewrite that preserves the predicate.
- Only two suppressions were added, both narrow `#[expect]` with reasons on the
  shakedown seeds. No public path or prelude changed.
- The de-links that remain all target private, `pub(crate)`/`pub(super)` or
  non-dependency items and are correct.

Process signal: the task was filed against three known warnings; the real
surface was 133. The plan's own open question ("the unbounded part is the
`-Dwarnings` inventory") called this correctly, and the inventory-first step
kept the scope legible instead of discovering it mid-fix.

Pending user checks: the `manual:` DoD item - review the public API and prelude
diff. The diff touches no `pub` signature and no `prelude` block; only
`nova_scenario::actions::mission::HudReadoutFormat` gains a `Default` derive
(same default value as the deleted hand impl).

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

The same reviewer verified `3c479f20` and confirmed R1.1 through R1.7 fixed.
Link resolution was checked in the GENERATED HTML, not merely by the absence of
a rustdoc warning: all six relinked anchors point at the intended public items
(`nova_assets::mark_downloaded_bundles_loaded`, `nova_assets::EnabledMods`,
`nova_assets::portal::PortalPlugin`, `nova_gameplay::plugin::SpaceshipSystems`,
`nova_gameplay::plugin::NovaGameplayPlugin`, `nova_probe::capture::perf_param`).
All three DoD commands re-run by the reviewer: exit 0, 0, 0.

Two new findings on the fix commit, neither blocking:

- [x] R2.1 (MINOR) tasks/20260731-205553/REVIEW.md:4 - `3c479f20` committed the
  scaffold stub (`BRANCH: TODO`, `REVIEWER: TODO`, a placeholder
  `R1.1 (MAJOR) file:line - TODO`) whose content contradicts the round it
  claims to record; fill in the real round or drop the file.
  - Response: The stub was swept in by the fix commit's `git add`. Replaced
    with the real Round 1 and this round.
- [x] R2.2 (NIT) crates/nova_assets/src/merge.rs:38 - the relink left an orphan
  fragment ("re-triggers this") on its own line, and the same ragged wrap in
  `portal/mod.rs:30` and `contract.rs:27`; re-flow those three paragraphs.
  - Response: All three re-flowed.

Process signal: the fix commit staged a directory (`tasks/20260731-205553`)
rather than the files actually edited, which is how the placeholder REVIEW.md
landed. Staging explicit paths would have caught it.

Inspection commands:

```
git show 946d03a6
git show 3c479f20
nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug
nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps
nix develop --command cargo clippy --workspace --all-targets --features debug -- -Dwarnings
```
