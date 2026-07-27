# Review: NOVA OS block caret over the first completion letter

- TASK: 20260727-135200
- BRANCH: feature/nova-os-ghost-caret

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

DoD proofs run by the reviewer: `nova_os_block_caret` + `nova_os_inline_completion`
tests PASS (2 passed); `cargo check -p nova_gameplay --all-targets` clean.

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/nova_os.rs:2283-2284 - the caret
  `left` used `chars * DRAWER_LINE_FONT_PX * 0.6`, but `0.6em` is the PoC's
  decorative block-cursor WIDTH, not the font's glyph advance (IosevkaTerm is
  narrower), so the caret drifts cumulatively (~a full cell by ~6 chars). The PoC
  MEASURES the rendered width; it does not multiply by 0.6.
  - Response: fixed in ef694c2d. Replaced the char-count formula with
    `position_nova_os_block_caret`, which sets `caret.left` from the before-text
    node's measured `ComputedNode.size.x` (converted physical->logical via
    `inverse_scale_factor`) - exactly the PoC's measure-based approach,
    font-agnostic and drift-free. Corrected the misleading constant doc comment
    (nova_os.rs:84-87) so it no longer claims 0.6em is the glyph advance.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/nova_os.rs:4656,4694 - the test
  asserted against the same `chars * cell_px` formula the code used, so it could
  never catch the drift.
  - Response: fixed in ef694c2d. The test now stamps a synthetic `ComputedNode`
    (size 57.6 physical, `inverse_scale_factor` 0.5) on the before-text node and
    asserts the caret copies the CONVERTED logical width (28.8), i.e. it pins the
    measure+convert wiring, not a formula. It still asserts the caret is
    `Absolute` (which the old flex spacer was not).
- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/nova_os.rs:4078-4086 - the absolute
  caret relies on `top:0/bottom:0` stretch for its height instead of an explicit
  height.
  - Response: left as-is (reviewer said none required). The block correctly sizes
    to the line-box height; the input row has no extra vertical padding, and the
    comment already notes the stretch. Acknowledged as an accepted trade-off.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

DoD proofs re-run by the reviewer: `nova_os_block_caret` + `nova_os_inline_completion`
PASS (2 passed); `cargo check -p nova_gameplay --all-targets` clean, no unused
imports from the removed formula.

The reviewer confirmed R1.1 and R1.2 are GENUINELY resolved: the caret is now
measure-driven (`before.size().x * inverse_scale_factor()`), the old
`chars * 0.6` formula is gone from `rebuild_terminal_ui`,
`NOVA_OS_CARET_WIDTH_FRACTION` is used only for the caret's drawn width,
`position_nova_os_block_caret` is registered under `run_if(in_state(NovaOs))`,
`q_before.single()` degrades safely, and the test pins the measure+convert path
(would fail if the scale conversion were dropped, and structurally cannot pass
the old code since it names the new system).

- [x] R2.1 (NIT) crates/nova_gameplay/src/hud/nova_os.rs:4723 - the stamped test
  width `57.6` made the expected `28.8` numerically coincide with the old
  `3 chars * 9.6px` cell output, so the number did not self-advertise as
  measure-derived.
  - Response: fixed - changed the synthetic width to `50.0` (`* 0.5 = 25.0`),
    which is not any `chars * 9.6` multiple, so the asserted number itself proves
    measure-derivation. Re-ran the test: passes.
- [x] R2.2 (NIT) crates/nova_gameplay/src/hud/nova_os.rs:2306 - reading
  `ComputedNode` in `Update` is one frame behind layout, so the caret sits at
  `left=0` for one frame after a rebuild.
  - Response: accepted as-is (reviewer agreed no change required) - imperceptible
    for a 0.85-alpha blinking block, and self-correcting.

Pending user checks (manual DoD, cleared at flow Finish):
- Owner types a partial command and confirms the block caret sits ON the first
  completion letter (no one-cell gap), with no drift on longer inputs.
