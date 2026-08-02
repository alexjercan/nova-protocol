# fix(nova_gameplay): ambiguous import visibility in the nova_os_map/ship mod.rs re-exports

- PRIORITY: 0
- TAGS: backlog, bug, chore
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Context

Uncovered by 20260731-170359 (the nova_menu KISS pass), which does not touch
nova_gameplay. `cargo check --workspace --all-targets` emits four warnings:

```
warning: ambiguous import visibility: pub(crate) or pub(in crate::hud::nova_os_map)
  --> crates/nova_gameplay/src/hud/nova_os_map/mod.rs:45:21
warning: ambiguous import visibility: pub(crate) or pub(in crate::hud::nova_os_ship)
  --> crates/nova_gameplay/src/hud/nova_os_ship/mod.rs:55:39
```

Landed by 20260731-170322 (the NOVA OS HUD split). Rustc says this will become
a hard error in a future release, so it is a real deadline, not style.

## Steps

- [ ] Spell the intended visibility explicitly at both sites.
- [ ] Verify: `nix develop --command cargo check --workspace --all-targets`
      emits no `ambiguous import visibility` warning.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets 2>&1 |
   grep -c 'ambiguous import visibility'` - returns 0.
2. cmd: `nix develop --command cargo fmt --check` - clean.


## Dropped

- REASON: duplicate. Warning remains real, but fully covered by broader 20260731-205553, including the related rustdoc warning.
