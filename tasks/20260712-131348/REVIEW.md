# Review: Ammo HUD readout for weapon sections

- TASK: 20260712-131348
- BRANCH: ammo-hud-diegetic

## Round 1

- VERDICT: APPROVE

Diff reviewed: `hud/ammo_readout.rs` (new) + `hud/mod.rs` wiring (module,
prelude, plugin, setup/remove observers), feature commit 5a1d31b, against the
default branch. TASK.md Goal/Steps/Notes read as the spec.

Verified against the Goal - the diff delivers it:

- One `Entity`-anchored `screen_indicator` per player weapon section that
  carries a `SectionAmmo`, reconciled idempotently; infinite-ammo weapons (no
  component) draw nothing. Chunked content: turret ring lights >=1 while any
  round remains, torpedo bar is one pip per round of capacity. Debug number on
  F10, `HudTier::Instrument`, spawned/despawned with the player ship. All ticked
  steps are actually implemented.
- Checks: `cargo fmt`, `cargo check --workspace`, `cargo clippy -p nova_gameplay`
  all clean; the 9 new headless tests pass. Tests assert behavior (reconcile
  spawn/despawn on death and on ammo-removal, infinite/other-ship exclusion,
  fraction->lit buckets, torpedo lit==rounds, debug number text + visibility),
  not just execution. No existing tests were weakened.

Independent re-derivation (shared-session blind spot): the load-bearing claim
that HUD-tier hiding and the self-driven debug number coexist correctly was
verified against `apply_hud_visibility` rather than assumed. That system only
mutates (a) `HudTier` roots without `ScreenIndicatorMarker` and (b) nodes with
`ScreenIndicatorMarker`. The debug number and pips are grandchildren carrying
neither, so the system never stomps the number's driver-owned `Hidden`; the
layer (HudTier, no marker) tier-hides the whole subtree via ancestry, and the
readout node (marker) is also directly tier-hidden. No `HudSelfDrivenVisibility`
opt-out is required (unlike the gravity sphere, which self-drives *and* is a
tagged root). Confirmed correct.

Findings:

- [ ] R1.1 (NIT) hud/ammo_readout.rs:149 - the anchor offset is a fixed
  `(RING_PX*0.6, -RING_PX*0.6)` px, so at extreme distance the gauge sits the
  same pixel offset from a tiny weapon and can read as slightly detached. Fine
  for the common range and consistent with the fixed-size choice; only worth
  revisiting if playtest shows it floating. No change required.

- [ ] R1.2 (NIT) hud/ammo_readout.rs:52 - the F10 debug toggle and the
  `AmmoReadoutDebug` resource are unit-tested at the driver level but the key
  binding itself is not exercised (no dev-overlay harness exists yet). Matches
  how the other HUD toggles are tested; acceptable.

No BLOCKER/MAJOR/MINOR findings. The two NITs are discretionary. APPROVE.
