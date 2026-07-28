# Review: Fix dead-code warning ShipBlock.section

- TASK: 20260728-124443
- BRANCH: fix/ship-block-section-dead-field

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session (trivial diff - a dead-code field removal + a query
  re-source; no behavior change, mechanically verifiable)

No findings.

Verification:
- `cargo check -p nova_gameplay` (non-test build, dead_code lint active): exit 0,
  `grep -c "never read"` == 0. The warning that motivated the task is gone.
- `cargo test -p nova_gameplay --lib nova_os_ship`: 12/12 pass, including
  `blocks_stay_uniform_green_regardless_of_status` and
  `ship_app_renders_blocks_and_selects_section` (the selection path the outline
  tint depends on).
- `cargo fmt` clean.
- Confirmed `ShipBlock.section` now has a production reader in
  `update_ship_blocks` (via the parent lookup), and `ShipBlockOutline` is a unit
  marker used only as a `With<>` filter - no dead code either way.
