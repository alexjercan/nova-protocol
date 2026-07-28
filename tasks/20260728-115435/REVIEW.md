# Review: NOVA OS ship app - clearer section rendering

- TASK: 20260728-115435
- BRANCH: feature/ship-section-legibility

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context (in-session pass re-ran the suite + re-verified the regression pin and DoD cmd proofs before adopting)

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/nova_os_ship.rs:13-18 - the module
  `//!` header still describes the interactive layer as "projected clickable UI
  blips labelled with their code"; the blip now also carries a per-kind glyph, an
  integrity bar, and (for weapons) ammo pips. Consider extending that sentence so
  the header stays a faithful map of the file. Not a correctness issue - the
  header never claimed block colour encodes status, so nothing went stale, it is
  only now incomplete.
  - Response: fixed in 47aa5cec - extended the module header to describe the
    uniform-green fills + bright box outlines and the glyph/bar/pips on the blip.

- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/nova_os_ship.rs:281-283 -
  `bar_fraction()` returns HP fraction via `integrity().unwrap_or(1.0)`, so an
  `inactive` (neutralized-by-inactivity) section that still has full Health reads
  a full bar while its fill/pips colour goes dim. Internally consistent (bar ==
  HP, colour == status) but a neutralized-yet-full-HP section shows a long dim
  bar. If that reads oddly in playtest, gate the fraction to 0 when
  `status() == "neutralized"`.
  - Response: left as-is by design. The bar means "HP remaining" and the colour
    means "status"; a neutralized-but-intact section genuinely still has its HP,
    so a long dim bar is truthful. Zeroing the bar would conflate "neutralized"
    with "destroyed". Flagged in the Work Log's manual-acceptance list so the
    owner can judge it in playtest; will gate the fraction then if it reads oddly.

Verification notes (out-of-context reviewer, re-confirmed in-session):
- `cargo test -p nova_gameplay --lib nova_os_ship` - all 12 tests pass, including
  the 4 new ones (`kind_glyph_distinct_per_kind`,
  `integrity_bar_and_ammo_pips_track_live_data`,
  `blocks_stay_uniform_green_regardless_of_status`,
  `blip_carries_kind_glyph_and_integrity_bar`).
- DoD `cmd:` proofs both pass: `cuboid_edges` defined + used; `grep srgb` hits
  only the pre-existing `SHIP_VIEW_BG` (no new hue constant; palette intact).
- Regression pin `blocks_stay_uniform_green_regardless_of_status` genuinely fails
  on revert (before the change the block carried the status-bucket material);
  off-origin fixture per `spatial-fixture-off-the-trivial-point`.
- `project_ship_blips` splits the old combined `&mut` query into disjoint
  per-component queries; the `&ShipBlip` borrow drops (Copy fields) before the
  mutable updates; just-spawned blips wait a frame (guarded). No `&mut` aliasing.
- `ammo_pips()` arithmetic total == capacity for `rounds < capacity` and
  `rounds > capacity`; no overflow/underflow.
- `cuboid_edges()` LineList + unlit `StandardMaterial` confirmed rendering via the
  `screenshot_nova_os` harness (real GPU, exit 0) - retires the render-path risk.
- DECISION.md records the load-bearing choice with Context, three rejected
  alternatives, and the forcing monochrome-palette constraint; Work Log matches
  the code.

Pending user (`manual:`) checks - APPROVE does not resolve these; owner playtest:
- While orbiting, sections read as separated boxes, not a blob.
- Hull / thruster / controller / PDC / torpedo each visually distinguishable at
  default framing.
- A damaged section's bar reads short/amber and a full weapon shows full pips
  (the screenshot range had no weapon section, so ammo pips were verified only by
  unit test + ECS tree, not on screen - a visual check on an armed ship is open).
- Clicking a block still selects its section (blip picking not regressed).
