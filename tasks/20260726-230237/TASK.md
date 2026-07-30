# NOVA OS casing: playtest polish (dark-gray, bigger screen, plate)

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.9.0,feature,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

Playtest feedback on the just-landed casing/glass pass (20260726-193219, commit 01cdd852).

## Story

Polish the NOVA OS monitor from the owner's playtest so it matches the HTML PoC
better and reads as dark-gray moulded plastic (not blue).

## Steps

- [x] Enlarge the monitor: shrink the viewport insets so it sits almost at the
      screen edges (top status-bar chrome may overlap it). INSET_X ~16, Y ~14.
- [x] Remove the amber/orange accent slots at the top (the "orange lines"); the
      PoC has none. Drop the spawn, fn, marker, NOVA_OS_ORANGE, and the test
      assertion.
- [x] Recolour the plastic to the HTML dark-grays (not blue): case body
      #2f383f -> #161b20 -> #0a0d10 -> #05080a; case edge #05070a; bezel base
      neutral. Use the PoC :root values.
- [x] Brand plate: move it right so it clears the bottom-left screw (chin
      padding). Make it match the HTML: base darker than the surround, really
      dark edges, a top(dark)->bottom(light-ish grey) gradient, a delimiting
      border and a light lower catch so it reads recessed/inset in 3D.
- [x] Glass reflection: make the light circle a bit weaker.
- [x] Re-render the star mark PNG at higher resolution so it is not pixelated.

## Definition of Done

- Monitor nearly fills the viewport; no orange accent bars; plastic reads
  dark-gray; plate is recessed/darker and clears the screw; reflection softer;
  star crisp. (manual: AFTER capture vs the PoC, owner confirms)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay drawer)
