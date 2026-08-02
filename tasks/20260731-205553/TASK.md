# Clear compiler and rustdoc warnings for v0.10.0

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.10.0,quality,docs,warnings
- KIND: STORY
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955

## Story

`cargo check -p nova_gameplay --all-targets` emits 4 warnings, 2 from each
site:

```
warning: ambiguous import visibility: pub(crate) or pub(in crate::hud::nova_os_map)
  --> crates/nova_gameplay/src/hud/nova_os_map/mod.rs:45:21
warning: ambiguous import visibility: pub(crate) or pub(in crate::hud::nova_os_ship)
  --> crates/nova_gameplay/src/hud/nova_os_ship/mod.rs:55:39
```

Landed by the KISS pass on the NOVA OS drawer surfaces (20260731-170322).

`cargo doc -p nova_gameplay --no-deps --document-private-items` also warns:
the public module doc of `hud/ammo_readout.rs` intra-doc-links
`[`sync_ammo_gate`]`, a private `fn` in the same file. Predates both KISS
passes and is the exact shape of the promoted
`rustdoc-no-public-to-private-intra-doc-link` lesson. Fix by dropping the
link brackets, not by making the fn public.

Both found while working `20260731-170329`, whose scope forbade fixing
out-of-scope defects. Clear these known warnings, then use warnings-as-errors to
inventory and remove any additional compiler or rustdoc warnings exposed by the
v0.10.0 automation work. CI retains ownership of the full Clippy run.

## Steps

- [ ] Reproduce the known `nova_gameplay` check and private-link rustdoc
      warnings on the current tree.
- [ ] Fix import visibility without changing public paths or prelude exports.
- [ ] Drop the invalid rustdoc link markup without widening the private item.
- [ ] Run workspace check and documentation with warnings denied; fix every
      first-party warning in scope.
- [ ] Confirm the existing CI Clippy pass remains the full-suite warning check;
      fix any CI warning reported for this release without adding broad allows.

## Definition of Done

- Workspace compiler warnings are zero under the release feature set.
  (cmd: `nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`)
- Workspace rustdoc warnings are zero.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps`)
- Public paths and crate preludes remain unchanged except additive exports
  required by planned new crates. (manual: review public API and prelude diff)

## Notes

- Do not run full local Clippy or the full test suite unless requested. CI owns
  both per repository policy.
