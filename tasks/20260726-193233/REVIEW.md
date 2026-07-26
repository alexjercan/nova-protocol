# REVIEW - NOVA OS render-to-texture CRT pipeline (task 20260726-193233)

Branch `feature/nova-os-rtt-crt` vs `master`. Reviewer: out-of-context critical
pass. Scope: `crates/nova_gameplay/src/hud/drawer.rs`, `assets/shaders/nova_os_crt.wgsl`.

## Summary

The RTT pipeline is well-structured and the hard parts (picking through an image
target, the physical-pixel coordinate math, the WGSL/Rust uniform lockstep, the
render-target resize convergence) are correct. I traced the coordinate systems
against the bevy 0.19 source and they line up: `window.cursor_position()`,
`UiGlobalTransform.translation`, `ComputedNode.size()` and the picking backend's
`position * target_scaling_factor()` are ALL physical pixels with the image
target's `scale_factor = 1.0`, so the forward-pointer path is HiDPI-safe. The
uniform struct field order and std140 alignment both match exactly. No
divide-by-zero in the shader or the inverse barrel.

One real correctness bug stands out: `mirror_nova_os_hover` queries `Hovered`
with NO marker filter, so every frame the drawer is open it force-clears
`Hovered` on every hoverable entity in the world that the forwarded pointer does
not hit - clobbering the mouse-driven hover that `update_is_hovered` set in
PreUpdate. It is mostly latent today but directly breaks the window-space chin
knobs that the very next task (214617, which this branch scaffolds `power`/
`brightness`/`warp`/`bloom` for) will add. That is the blocking finding.

## Findings by severity

### MAJOR

**M1 - `mirror_nova_os_hover` clobbers hover on ALL window UI while the drawer is
open.** `drawer.rs:874-903`. The query is `Query<(Entity, &Hovered)>` with no
filter, and the final loop (`897-902`) writes `Hovered(false)` onto EVERY entity
carrying a `Hovered` component that is not in the forwarded pointer's hit set.
Trace: bevy's `update_is_hovered` runs in PreUpdate and sets `Hovered(true)` on a
window-space UI node the real mouse is over; `mirror_nova_os_hover` then runs in
Update (`run_if(in_state(PauseStates::Drawer))`), the forwarded pointer's
`HoverMap` never contains that window node (it targets the image render target),
so `hovered_set` lacks it and the system inserts `Hovered(false)`, overwriting the
mouse hover for the rest of the frame. Result: hover state on any window-space
interactive UI flip-flops (dead/flickering) whenever the computer is open.
- Today this is largely latent because the terminal's interactive nodes (scroll
  viewport, app-close button) live UNDER the content root and ARE correctly
  served by the mirror, and the chin controls row (`drawer.rs:3432`) is still
  inert. But `bevy`'s `Button` widget carries a `Hovered` component, so any
  `Button` present on the window during the drawer modal is already clobbered,
  and DECISION.md + TASK.md explicitly say the chin BRIGHT/SCAN knobs (task
  214617) are window-space nodes wired to THIS shader - they will regress the
  moment they land, on the branch built to enable them.
- Fix: scope the query to the terminal's own interactive nodes, e.g.
  `Query<(Entity, &Hovered), With<DrawerScrollViewportMarker>>` (the only
  `Hovered`-gated consumer is `scroll_drawer_panels`), or gate on descendants of
  `rtt.content_root`, or add a dedicated `NovaOsInteractive` marker. Do NOT walk
  the global `Hovered` set.

### MINOR

**m1 - Inverse pointer barrel ignores the shader's power-collapse remap.**
`drawer.rs:826-832` (`nova_os_inverse_barrel`) inverts only `barrel()`, but the
shader applies the power-collapse remap (`nova_os_crt.wgsl:67-76`) to the sample
UV BEFORE `barrel()`. During the open/close transition (power in (0,1)) the on-
screen content is squeezed, so a click would map to the wrong glyph. This is
acceptable in practice: at steady open, `power == 1` makes `open_h == open_w == 1`
and the remap is the identity (`cy == uv.y`, `cx == uv.x`), so hit accuracy is
exact whenever interaction actually matters; the transition is brief and non-
interactive. Worth a one-line comment on `nova_os_inverse_barrel` noting the
inverse is only exact at full power (matching the collapse identity), so a future
reader does not assume pixel-exact hits mid-transition.

**m2 - `setup_drawer` firing twice orphans the `NovaOsRtt` resource (pre-existing
singleton assumption).** `drawer.rs:2818-2887`. A second `Add<PlayerSpaceshipMarker>`
without a prior `Remove` would run `setup_drawer` again, `insert_resource`
(`2855`) overwrites `NovaOsRtt` with the second camera/root/pointer/image, so the
first `NovaOsRtt` handle set is dropped from the resource. It is NOT a leak while
alive (the first camera + first material still hold strong handles to the first
image) and `remove_drawer` despawns BOTH sets on ship removal (it queries by
marker, `3730-3742`), after which both images/materials GC by last-strong-handle
drop. So no new leak vs master - but the whole HUD already assumes a singleton
player ship (every `On<Add, PlayerSpaceshipMarker>` observer in `hud/mod.rs`), so
this is consistent with, not worse than, the existing invariant. Note only; no
action required unless multi-ship is ever real.

**m3 - `PointerInput` press/release still emitted at the parked (-1000,-1000)
position when the cursor is off-panel.** `drawer.rs:838-857`. Every real mouse
click anywhere writes a Press/Release `PointerInput` for the custom pointer at the
parked position. Harmless (outside the image viewport -> no hits, per
`picking_backend.rs:135-139`) but it churns the pointer-event pipeline on every
click regardless of drawer focus. Optional: skip emitting when `in_image` is
`None`.

**m4 - Offscreen image asset relies on implicit handle-drop GC on teardown.**
`remove_drawer` (`3722-3746`) despawns the camera + removes `NovaOsRtt`, and the
sampling `MaterialNode` despawns with `DrawerRootMarker`, so the last strong
`Handle<Image>` and `Handle<NovaOsCrtMaterial>` drop and both assets GC. This is
the standard bevy pattern (same as `render_scale.rs`, which also never calls
`images.remove`). Correct, but there is no explicit free and no test asserting the
image/material asset count returns to zero after teardown - see t2.

### NIT

**n1 - First-frame 2x2 -> 1x1 -> real-size resize churn is invisible but noisy.**
`setup_drawer` seeds a 2x2 image (`2825`); `reconcile_nova_os_target` resizes to
`computed.size().round().as_uvec2().max(ONE)`, which is (1,1) before the first
layout, then the real size. Because the camera is `is_active = false` until
`open > EPSILON` (`2834`, and re-gated in reconcile `238-241`), no garbage/stretched
frame is ever presented; by the time openness ramps up the size has converged. Fine
as-is; the 2x2 seed could be dropped to a comment-justified 1x1 for tidiness.

**n2 - Bloom taps can sample outside [0,1].** `nova_os_crt.wgsl:98-101` gathers at
`warped + offs*texel*2`, which near the panel edge reads outside the image. The
default image sampler is `ClampToEdge` (bevy default `ImageSamplerDescriptor`), so
this pulls the (dark, tube-region) edge texel rather than wrapping the opposite
side, and `in_bounds`/vignette mask the rim anyway. No artifact; noting only that
the safety depends on the default clamp address mode remaining unchanged.

**n3 - `RenderLayers` on the content root is inert but harmless.** `drawer.rs:2867`
adds `RenderLayers::layer(20)` to the content root Node. UI is routed purely by
`ComputedUiTargetCamera` / `UiTargetCamera` (confirmed in `bevy_ui_render`), so the
layer does nothing for the UI subtree; it only matters on the camera (`2860`) to
keep world 2D sprites out. Keeping it on the root is defensible as documentation of
intent but it is not load-bearing - the reasoning recorded in NOTES is sound.

## Test adequacy

The two updated tests are meaningful:
- `drawer_screen_samples_offscreen_image` (`5576+`) asserts exactly one sampling
  surface, that terminal content is parented under `rtt.content_root` (not the
  screen node), and that `material.source == rtt.image` - all the right structural
  invariants for the RTT swap.
- `nova_os_crt_material_receives_resolution_time_and_power` (`5625+`) exercises
  `animate_nova_os_crt` end to end including the new `power <- DrawerOpenness` feed.

Gaps (none blocking, but worth a follow-up):
- **t1 - No test for `mirror_nova_os_hover`.** The M1 over-broad clobber, and the
  intended behavior (forwarded pointer sets `Hovered` on the scroll viewport, and
  CLEARS it when the pointer leaves), are entirely untested. A unit test that
  seeds a `HoverMap` for `nova_os_pointer_id()` and asserts (a) the viewport gets
  `Hovered(true)`, (b) an unrelated window node with `Hovered(true)` is NOT
  cleared would have caught M1.
- **t2 - No teardown test.** Nothing asserts `remove_drawer` removes `NovaOsRtt`
  and that the `Assets<Image>`/`Assets<NovaOsCrtMaterial>` counts drop back to
  zero (the leak-vs-GC question in m4).
- **t3 - No reconcile/resize test.** `reconcile_nova_os_target`'s resize detection
  (`img.size() != desired`), the `.max(UVec2::ONE)` floor, the `is_active`/
  visibility gate on openness, and `projection.set_changed()` on swap are all
  untested. A headless test can drive a `ComputedNode` size + `DrawerOpenness` and
  assert the image resizes and the camera `is_active` flips.
- **t4 - The forwarded-pointer math (`forward_nova_os_pointer` /
  `nova_os_inverse_barrel`) has no unit test.** At minimum a round-trip
  `barrel(inverse_barrel(uv)) == uv` property test would pin the inverse and guard
  m1's "identity at full power" assumption.

## Verdict rationale

The architecture is correct and the coordinate/uniform/alignment work is solid and
verified against engine source. M1 is a genuine correctness bug: an unfiltered
global `Hovered` write that fights `update_is_hovered` for all window UI while the
drawer is open, and it will regress the chin controls this branch exists to enable.
It is a small, localized fix (scope the query). Given the branch's stated purpose,
that should be fixed before merge; the MINOR/NIT items and test gaps can ride along
or become a quick follow-up.

VERDICT: REQUEST_CHANGES

---

## Round 2 - resolution (author, 2026-07-27)

**M1 (MAJOR) - FIXED.** `mirror_nova_os_hover` now manages `Hovered` ONLY on
entities rendered through the image (the content root and its descendants, via an
`iter_ancestors` check against `rtt.content_root`); window-space UI is never
touched, exactly the scoping the review prescribed. Regression test added:
`mirror_hover_serves_content_but_never_clobbers_window_ui` - a `Hovered(true)`
window node survives the mirror while a content-root node hit by the forwarded
pointer is mirrored to `Hovered(true)`. Without the `through_image` guard the
window assertion fails (the exact M1 clobber). `cargo test -p nova_gameplay drawer`
= 57 passed.

**MINOR/NIT items** - accepted as-is (non-blocking, per the review):
- inverse-barrel ignores the power-collapse remap: exact only at full power;
  interaction during the brief open/close animation is not load-bearing.
- double-`setup_drawer` resource orphan: consistent with the existing
  singleton-player-ship invariant (the overlay path had the same assumption).
- parked-pointer `PointerInput` on off-panel clicks: harmless (position parked
  off-image -> no hit).
- first-frame 2x2->1x1 resize churn: invisible (camera inactive until openness>0).

Remaining test gaps (reconcile/resize, teardown, forwarded-pointer math) are noted
for the degauss/polish follow-up (20260727-014148); the load-bearing hover path is
now pinned.

VERDICT: APPROVE (M1 resolved + pinned)

VERDICT: APPROVE
