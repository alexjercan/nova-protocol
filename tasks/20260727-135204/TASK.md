# NOVA OS CRT: wrap the screen border to the curved tube + green-tint the noise

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.9.0,feature,ui,hud

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

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

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

- [ ] Record the border-approach DECISION.md (a/b/c above) before touching the
      frame.
- [ ] Implement the chosen curved-border treatment so the bezel/screen edge
      follows the tube bulge (no flat rectangle against the curved content).
- [ ] Green-tint the grain/noise: shift the noise color from neutral gray to a
      phosphor-green shade (shader grain color and/or `NOVA_OS_SCREEN` /
      terminal surface tint), matching the PoC's green tube background.
- [ ] Sanity-check against the web PoC capture; keep BRIGHT/SCAN and other CRT
      effects intact.

## Definition of Done

- The screen frame reads as a curved 3D tube edge (not a flat rectangle) and
      the noise carries a visible green tint. (manual: AFTER capture vs the PoC,
      owner confirms)
- DECISION.md records the chosen border approach with an ACCEPTED status.
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay drawer)
