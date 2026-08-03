# Notes: Clear compiler and rustdoc warnings for v0.10.0

## What changes

Before: three known first-party warnings survive a clean build.

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
