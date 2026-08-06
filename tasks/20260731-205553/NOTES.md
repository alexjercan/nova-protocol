# Notes: Clear compiler and rustdoc warnings for v0.10.0

## What changes

Before (as filed): three known first-party warnings survive a clean build. The
inventory below supersedes this - items 1 and 2 no longer reproduce, and the
real surface is 133 warnings across rustdoc and clippy.

1. `crates/nova_gameplay/src/hud/nova_os_map/mod.rs:40` -
   `pub(crate) use self::{app::*, contacts::*, scene::*};` re-exports items
   whose own visibility makes the effective visibility ambiguous
   (`pub(crate)` vs `pub(in crate::hud::nova_os_map)`).
2. `crates/nova_gameplay/src/hud/nova_os_ship/mod.rs:50` - same shape.
3. `crates/nova_gameplay/src/hud/ammo_readout.rs:43` - the module's `//!` doc
   intra-doc-links `` [`sync_ammo_gate`] ``, a private `fn` at line 174. That
   is the promoted `rustdoc-no-public-to-private-intra-doc-link` lesson: fix by
   dropping the link brackets, never by making the fn public.

Both import sites landed in the NOVA OS drawer KISS pass (`20260731-170322`);
the doc link predates it.

After: zero workspace compiler warnings under the release feature set and zero
rustdoc warnings, with no broad `allow` attributes and no widened visibility.

## Surfaces

| File | Why |
| --- | --- |
| `crates/nova_gameplay/src/hud/nova_os_map/mod.rs` | Ambiguous re-export visibility (line ~40). |
| `crates/nova_gameplay/src/hud/nova_os_ship/mod.rs` | Same (line ~50). |
| `crates/nova_gameplay/src/hud/ammo_readout.rs` | Public-to-private intra-doc link (line ~43). |
| The rest of the workspace | Whatever `-Dwarnings` turns up once the known three are cleared - unknown until the inventory pass runs. |
| `.github/workflows/ci.yaml` | Owns the full Clippy run; must stay the full-suite check. |

## Data and interfaces

None. Every fix is local: an explicit visibility on the re-export, and markup
removal in a doc comment. Public paths and crate preludes must be unchanged
(the `prelude` modules in both files re-export the intended public names
already, so the `pub(crate) use` lines can be narrowed without touching them).

## Sketches

Illustrative only.

```diff
-pub(crate) use self::{app::*, contacts::*, scene::*};
+pub(in crate::hud) use self::{app::*, contacts::*, scene::*};
```

```diff
-//! [`sync_ammo_gate`], so they surface while the weapons are hot or a group is
+//! `sync_ammo_gate`, so they surface while the weapons are hot or a group is
```

Inventory pass:

```
nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug
nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps
```

## Shape

```
known three  --fix--> local edits (visibility, doc markup)
                          |
                          v
   RUSTFLAGS=-Dwarnings check --workspace --all-targets --features debug
   RUSTDOCFLAGS=-Dwarnings doc --workspace --no-deps
                          |
             new warnings exposed by the release work
                          |
                          v
                 fix in scope, no blanket allow
                          |
                          v
                    CI clippy (unchanged owner)
```

## Inventory (2026-08-06, master @ 86689890)

Three sweeps run on a clean tree. Counts are unique sites; clippy repeats each
lib warning on the `lib test` target, which is why the raw log shows ~2x.

| Sweep | Command | First-party warnings |
| --- | --- | --- |
| rustc | `cargo check --workspace --all-targets --features debug` | 0 |
| rustdoc | `cargo doc --workspace --no-deps` | 28 |
| clippy | `cargo clippy --workspace --all-targets --features debug` | 105 |

The rustc sweep is already clean: the two ambiguous-import-visibility warnings
in the Story no longer reproduce (fixed in flight by a later KISS pass). The
only rustc-side output left is a third-party future-incompat note for
`proc-macro-error2 v2.0.1`, a transitive dependency - out of scope, no
first-party fix exists.

### Rustdoc: 28 warnings

| Lint | Count | Sites |
| --- | --- | --- |
| `private_intra_doc_links` (public doc -> private item) | 16 | nova_gameplay 13, nova_ui 2, nova_gameplay/audio 1 |
| `broken_intra_doc_links` (unresolved) | 6 | nova_assets 3, nova_probe 1, nova_gameplay 2 |
| `broken_intra_doc_links` (ambiguous `nova_autopilot` fn vs crate) | 5 | `nova_debug/src/harness.rs` |
| `redundant_explicit_links` | 1 | `nova_assets/src/lint_walk.rs` |

Per crate: nova_gameplay 16, nova_debug 5, nova_assets 4, nova_ui 2,
nova_probe 1.

The 16 private-link warnings are all the promoted
`rustdoc-no-public-to-private-intra-doc-link` shape: drop the brackets, never
widen the item. The 6 unresolved links name items that do not exist in scope
(wrong path or renamed); drop the brackets too unless a correct path exists.
The 5 `nova_autopilot` ones are genuinely ambiguous - `nova_debug` has a
`nova_autopilot` fn and depends on the `nova_autopilot` crate; the fn is the
intended referent, so `nova_autopilot()`.

### Clippy: 105 warnings

`type_complexity` and `too_many_arguments` are `allow` in
`[workspace.lints.clippy]` (Bevy system signatures); with them re-enabled the
count is 197. This inventory respects the workspace config.

| Lint | Count |
| --- | --- |
| `doc_lazy_continuation` (doc list item without indentation) | 71 |
| `write_with_newline` | 4 |
| `inconsistent_digit_grouping` | 6 |
| `manual_contains` | 4 |
| `manual_chunks_exact` / const chunk size | 2 |
| `assertions_on_constants` | 2 |
| `redundant_closure` | 2 |
| `field_reassign_with_default` | 2 |
| `empty_line_after_doc_comment` | 2 |
| `incorrect_clippy_lint_expectation` (unfulfilled) | 2 |
| one-off (`ptr_arg`, `clone_on_copy`, `unnecessary_to_owned`, `unnecessary_lazy_evaluations` x2, `derivable_impls`, `question_mark`, `duplicated_attributes`) | 8 |

`doc_lazy_continuation` dominates and is purely mechanical: a wrapped `- item`
continuation line must be indented under the bullet. Spread over 17 files;
`nova_gameplay` holds most.

The 2 unfulfilled `#[expect(...)]` attributes (`keybind_dock.rs:687`,
`torpedo_target.rs:330`) are stale suppressions - the underlying lint stopped
firing, so the attribute must go, not be re-broadened.

## Result

All three sweeps are zero. Nothing was silenced with a broad `allow`; two
targeted `#[expect(clippy::inconsistent_digit_grouping)]` were added on the
shakedown belt seeds, whose `<date>_<index>` grouping is meaning, not magnitude.

Notable non-mechanical edits:

| Site | Change |
| --- | --- |
| `nova_assets/src/bin/content.rs` | A wrapped line started with `+`, opening a phantom nested list that made 17 continuations lazy. Moved the `+` up a line. |
| `nova_assets/tests/ledger_ch3_channel.rs` | `5b.` is not a list marker; renumbered 5b/6/7 -> 6/7/8. |
| `nova_gameplay/src/mesh/builder.rs` | `chunks_exact(N)` -> `as_chunks::<N>().0`. Same remainder-dropping semantics; the malformed-boundary NOTE was updated to match. Covered by `mesh::` tests (9 pass). |
| `nova_gameplay/src/hud/nova_os/content.rs` | `or_else` with an if-chain -> early return + if-chain. |
| `nova_scenario/src/actions/mission.rs` | Hand-written `Default` -> `#[derive(Default)]` + `#[default]`. |
| `nova_probe/src/recorder.rs` | Test `rig(&PathBuf)` -> `rig(&Path)`. |
| `nova_gameplay/src/hud/beacon_chips.rs` | An orphaned doc comment documented nothing; moved to the `ScreenIndicatorSize::Content` site as a plain comment. |
| `keybind_dock.rs`, `torpedo_target.rs` | Removed two `#[expect(clippy::type_complexity)]` that could never fire - the lint is `allow` workspace-wide. |
| `nova_os_map/scene.rs` | Removed a duplicated (and workspace-redundant) `#[allow(clippy::too_many_arguments)]`. |

The rest is doc rewrapping: a hyphen or `+` at the start of a wrapped line reads
as a markdown list marker, so most of the 71 `doc_lazy_continuation` hits were
fixed by moving the punctuation up one line rather than by indenting.

The `nova_debug` `nova_autopilot` links resolve to the FUNCTION
(`nova_autopilot()`), except the module doc's "over the nova_autopilot drivers",
which means the crate (`mod@nova_autopilot`).

## Consequences and open questions

- Cost: the two known fixes are minutes. The unbounded part is the `-Dwarnings`
  inventory, whose size is unknown until it runs - and `20260802-120025` /
  `20260802-120029` will churn the whole example fleet before this task lands,
  which is exactly why it sits last in the sprint.
- The correct narrowed visibility for the two re-exports needs a look at who
  actually consumes those glob items; `pub(in crate::hud)` is the guess, not a
  verified answer.
- Per repo policy this task must not run the full local test suite or a full
  local Clippy (CI owns both); the two commands above are check/doc only, which
  is allowed.
- Open: whether examples count as "workspace" for the zero-warning bar.
  `--all-targets` includes them, so a churned fleet can reopen this task; the
  DoD should be evaluated after `20260802-120029` lands, not before.
- No dependency edges: it can land any time, but re-running it after the fleet
  rebuild is the only way the result stays true.
