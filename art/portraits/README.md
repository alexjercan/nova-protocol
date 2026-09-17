# Portrait sources

Generated SVG sources for the green CRT comms portraits. Run
`scripts/generate-campaign-portraits.py` from the repository root to regenerate
these files and the shipped PNGs under `assets/base/portraits/`. A mod that
ships faces of its own writes them under its own asset tree.

Two casts live here:

- The training voices, `player` and `range-control`, drawn in the HUD's own
  green.
- Season one's crew, `jonah`, `leila`, `tomas`, `rina`, `samir`, `nadia` and
  `elena`. These take their skin, hair, eyes and coat straight from the story's
  character palettes in `scripts/nova_illustration/colors.py`, so a reader of
  the book meets the same faces the illustrations use. Change a palette there
  and the game portraits follow on the next run. `nadia` and `elena` speak over
  the radio, so they carry a blue channel chip instead of a green one.

The source uses a 32x32 hard-pixel grid and renders at 512x512. The HUD displays
each portrait at 48x48. Keep faces, headsets, and role silhouettes distinct at
that final size; the crew are told apart mostly by coat colour, which is why the
palettes are shared with the story rather than re-picked.

Every PNG must also be listed in `assets/base/base.bundle.ron`, or the scenario
that names it loads without a face.
