# NOVA OS screen: playtest polish (caret-at-0, kill black gaps, greener grain, white star)

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.9.0, feature, ui, hud

Second playtest pass on the just-landed NOVA OS screen (siblings 20260727-135200/135204/135217). Four small, cohesive visual fixes on the ONE screen; owner said "the rest looks really good".

## Feedback items (verbatim intent)

1. CARET: before typing any character the block caret is not visible; it must show even with 0 characters typed.
2. BLACK GAPS: the screen still has black gaps. Stretch the picture more and let it run "below" the casing lines so there is no visible black between the content and the casing.
3. GRAIN: too much gray noise; make it a bit more green.
4. STAR: the star mark in the status-bar item is black; make it white (or light) instead.

## Root causes (already traced)

1. The block caret (`NovaOsTerminalCaretMarker`, nova_os.rs ~4078) is absolute with `top:0 bottom:0`, so it stretches to its parent `NovaOsPromptInputWrapMarker`. When the input is empty all three text nodes in the wrap are "", so the wrap collapses to 0 height and the caret stretches to nothing. Fix: keep the input wrap (or prompt line) at a full text-line min-height so the caret is always full height.
2. The barrel warp (`nova_os_crt.wgsl` barrel(), line ~121) pushes edge/corner UVs past [0,1]; `rgb * in_bounds` (line ~202) zeroes those to tube-black, and `NOVA_OS_SCREEN_PAD_PX` (18px) adds a black margin. Fix: add CRT OVERSCAN (a shader const, derived to cover warp=0.12 corners at ~1.03, so no uniform-struct change - respects the encase field-order/alignment lesson) so the bowed edges land under the bezel, and trim the screen padding so the glass reaches the casing.
3. `GRAIN_TINT = vec3(0.35, 1.0, 0.55)` in nova_os_crt.wgsl is still fairly gray. Fix: push it greener (lower R and B relative to G).
4. `assets/icons/nova_crt_mark.png` opaque pixels are near-black (~17,17,17) with white only in the transparent bg, so an in-engine tint cannot lift it (black x tint = black). Fix: recolor the asset's opaque pixels to white (preserve alpha), and tint it a NOVA phosphor color in-engine via ImageNode color for theme consistency.

## Definition of Done

1. (manual) Open NOVA OS with no input typed: the amber block caret is visible and blinking at the prompt start.
2. (manual) No black gap shows between the screen picture and the casing; the warped picture bleeds under the bezel with no black corners/margin.
3. (manual) The analog grain reads as a green phosphor shimmer, not gray snow.
4. (manual) The status-bar star mark renders white/light (phosphor-tinted), not black.
5. (cmd) Shader + asset load clean at runtime: `BCS_AUTOPILOT=1 cargo run --example screenshot_nova_os --features debug` exits AppExit::Success with no naga/wgpu panic (the only thing that actually compiles the WGSL - see lesson wgsl-not-covered-by-cargo-check).
6. (cmd) `cargo fmt --check` clean and any newly touched tests green; full suite runs in CI.

## Steps

- [x] Fix caret visibility at 0 chars: gave `NovaOsPromptInputWrapMarker` a `min_height` of the text line box (`DRAWER_LINE_FONT_PX * 1.2`) so the absolute caret never stretches to a collapsed 0-height box when all three text pieces are empty.
- [x] Kill the black gaps: added `NOVA_OS_OVERSCAN = 0.93` to nova_os_crt.wgsl; the sampled UV is scaled in toward centre after the barrel warp so the bowed corners land under the bezel with no interior tube-black. NO padding change needed - the `MaterialNode` is `position:absolute; inset:0` and already fills under the screen padding; the black was purely the warp margin. DECISION.md records the overscan choice.
- [x] Green the grain: `GRAIN_TINT` 0.35/1.0/0.55 -> 0.15/1.0/0.35 in nova_os_crt.wgsl.
- [x] White the star: recoloured assets/icons/nova_crt_mark.png opaque pixels to white (alpha preserved; RGBA). Both call sites (objective_hint.rs, nova_os.rs plate) render it native on dark chrome, so white shows without a per-site tint - kept the existing "native colours" design.
- [x] Validate: `BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example screenshot_nova_os --features debug` exits AppExit::Success (WGSL compiled, no naga panic); `cargo fmt --check` clean. Screenshots confirm: green grain, content fills to the bowed glass edge (no black corners), caret renders (amber block after `nova> lo`). Empty-input caret is by-construction (min-height removes the sole collapse cause) - the welcome shot caught a blink-off (caret alpha toggles at 1.25 Hz), so the on-phase empty caret is left to owner manual acceptance. Star not shown in NOVA-OS shots (flight status bar hidden while the OS is open); asset is pixel-verified white.

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED
