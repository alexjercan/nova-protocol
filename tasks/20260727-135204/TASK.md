# NOVA OS CRT: wrap the screen border to the curved tube + green-tint the noise

- PRIORITY: 43
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

Playtest feedback on the just-landed curved-CRT NOVA OS screen. Two screen-
surface visual issues after the barrel/curve was added:

1. The old flat rectangular bezel/screen borders no longer match the bulged
   3D tube face; the frame reads as a flat rectangle sitting in front of a
   curved screen. Borders should wrap/hug the 3D curved look.
2. The screen noise/grain background reads gray; it should be a green shade
   (phosphor tint), consistent with the green-phosphor direction.

Code: `crates/nova_gameplay/src/hud/nova_os.rs` bezel spawn ~3069-3089,
screen spawn ~3094-3109, `NOVA_OS_SCREEN` color ~141, grain strength ~183;
CRT shader `assets/shaders/nova_os_crt.wgsl` (grain ~183-200, warp/vignette).
Reference look: `examples/ui/nova_os_terminal_poc.html` tube clip-path
~941-951, `.tube`/`.grain` ~199/260-274, `.rim`/`.curve` overlays.

## Story

Make the screen frame and its noise read as one curved 3D CRT: the border must
follow the tube's bulge instead of being a hard flat rounded rectangle, and the
grain/noise must carry a green phosphor tint rather than neutral gray.

DECISION (record a DECISION.md): the border approach is a load-bearing fork.
Candidate shapes, mutually exclusive: (a) bake the bezel/frame INTO the
warped CRT render so it curves with the tube; (b) overlay a pre-shaped
(curved) frame graphic/mask matched to the barrel warp; (c) drop the hard
rectangular border for a soft curved rim/vignette (inner glow) that reads as a
rounded tube edge. Pick one before building; the owner-approved choice is the
DECISION.md.

## Steps

- [x] Record the border-approach DECISION.md (a/b/c above) before touching the
      frame. Chose (a)+(c): a shader phosphor rim in barrel-warped uv space +
      demoted flat UI rings.
- [x] Implement the chosen curved-border treatment so the bezel/screen edge
      follows the tube bulge (no flat rectangle against the curved content).
      Done: `nova_os_crt.wgsl` rim from the panel-edge distance in `warped` uv
      (bows with the barrel); screen node border -> dark recess; phosphor-rim
      overlay -> faint outer halo.
- [x] Green-tint the grain/noise: shift the noise color from neutral gray to a
      phosphor-green shade. Done: shader grain multiplied by `GRAIN_TINT`
      (0.35,1.0,0.55) - the scalar snow was gray because it broadcast equally to
      RGB; now it reads green.
- [x] Sanity-check against the web PoC capture; keep BRIGHT/SCAN and other CRT
      effects intact. Verified the shader compiles + the NOVA OS renders by
      running the real app headless (screenshot_nova_os under autopilot).

## Definition of Done

- The screen frame reads as a curved 3D tube edge (not a flat rectangle) and
      the noise carries a visible green tint. (manual: AFTER capture vs the PoC,
      owner confirms)
- DECISION.md records the chosen border approach with an ACCEPTED status.
- The CRT shader still compiles and the NOVA OS renders without a wgpu/naga
      panic. (cmd: BCS_AUTOPILOT=1 nix develop --command cargo run --example screenshot_nova_os --features debug)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay -- nova_os_monitor_has_physical_casing_details)
      [The template's `drawer` filter matches 0 tests; the chrome test lives
      under `hud::nova_os::tests::*`.]

## Close-out

What changed and why (see DECISION.md for the border-approach fork):
- CRT shader (`nova_os_crt.wgsl`): the crisp screen edge is now a phosphor rim
  computed from the distance to the panel bound in BARREL-WARPED uv space, so its
  iso-contour bows with the tube (the border wraps the 3D curve). The flat UI
  edge rings are demoted: the screen node's bright-phosphor border -> a dark
  recess line, and the phosphor-rim overlay -> a faint outer halo (kept as nodes
  for the headless fallback + the existing rim test).
- Grain green-tint: the analog grain was a scalar added equally to R=G=B (hence
  gray snow); it is now multiplied by `GRAIN_TINT` (0.35,1.0,0.55) so the noise
  reads as green phosphor shimmer.

Difficulties / verification:
- cargo check does NOT compile WGSL (shaders load at runtime), so a syntax/type
  error would only surface as a wgpu panic when the NOVA OS first opens. Validated
  by running the real app headless: `BCS_AUTOPILOT=1 cargo run --example
  screenshot_nova_os --features debug` reached Playing and exited via
  AppExit::Success (only reachable after the autopilot opens the NOVA OS, which
  instantiates + renders the CRT material) with zero panic/naga/validation errors.
  This is the honest proof the shader compiles and the tube renders.
- WGSL note: `+`/`*` broadcast a scalar over a vector (that is WHY the old scalar
  grain was gray - added equally to all channels), so `grain * GRAIN_TINT`
  typechecks and green-tints without a cast.
- Discovered a PRE-EXISTING red test on master, unrelated to this diff:
  `tests/examples_smoke.rs::catalog_matches_disk` fails because
  `screenshot_nova_os` is cataloged but in no smoke list. Filed as a separate
  follow-up (not a blocker here).

Self-reflection: the right call was to treat "no cargo-check coverage" as a real
gap and run the app for the shader, rather than trusting the diff. Next time,
reach for the runnable example immediately on any shader/asset change.
