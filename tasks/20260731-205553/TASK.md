# Clear the standing nova_gameplay check and doc warnings

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT

## Context

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

Both found while working 20260731-170329, whose scope forbade fixing
out-of-scope defects.

## Definition of Done

- No `ambiguous import visibility` warning. (cmd: `nix develop --command cargo check -p nova_gameplay --all-targets`)
- No `public documentation for ... links to private item` warning. (cmd: `nix develop --command cargo doc -p nova_gameplay --no-deps --document-private-items`)
- Public paths and the nova_gameplay prelude unchanged. (manual: review diff)
