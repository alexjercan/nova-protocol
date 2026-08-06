# Clear compiler and rustdoc warnings for v0.10.0

- PRIORITY: 50
- TAGS: v0.10.0, quality, docs, warnings
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
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

- [x] Inventory every first-party warning from rustc, rustdoc and clippy on a
      clean tree; record counts and per-lint breakdown in NOTES.
- [x] Clear the 28 rustdoc warnings, choosing the fix by why resolution failed:
      drop the brackets where the target is private or not a dependency, give
      an explicit path where it is public but out of scope, disambiguate
      `nova_autopilot`, remove the redundant explicit target. Never widen an
      item to satisfy a link.
      (Amended: the original text said to drop brackets on ALL unresolved
      links. Review round 1 showed five had reachable public targets and needed
      relinking, not de-linking.)
- [x] Clear the 71 `doc_lazy_continuation` warnings at their cause - prose
      punctuation (`-`, `+`) starting a wrapped line, read as a list marker -
      by moving it up a line; indent only genuine list continuations.
      (Amended: the original text assumed indentation was the fix throughout.)
- [x] Clear the remaining 34 clippy warnings site by site, including removing
      the two stale `#[expect(...)]` attributes. No broad `allow`.
- [x] Re-run all three sweeps with warnings denied and confirm zero.

## Definition of Done

- Workspace compiler warnings are zero under the release feature set.
  (cmd: `nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`)
- Workspace rustdoc warnings are zero.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps`)
- Workspace clippy warnings are zero under the existing workspace lint config.
  (cmd: `nix develop --command cargo clippy --workspace --all-targets --features debug -- -Dwarnings`)
- Public paths and crate preludes remain unchanged; no new `allow` attribute
  widens a lint beyond a single justified site. (manual: review the diff)

## Notes

- Local clippy is run for this task only because the owner asked for a
  whole-project warning sweep. CI still owns the standing check.
- Do not run the full local test suite.
- `type_complexity` and `too_many_arguments` stay `allow` in
  `[workspace.lints.clippy]`; they are Bevy signature noise, not defects.
- The `proc-macro-error2` future-incompat note is third-party. Out of scope.
