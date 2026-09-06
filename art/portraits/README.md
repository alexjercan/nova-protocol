# Portrait sources

Generated SVG sources for the green CRT comms portraits. Run
`scripts/generate-campaign-portraits.py` from the repository root to regenerate
these files and the shipped PNGs: the base game's faces, the player and Range
Control, under `assets/base/portraits/`. A mod that ships faces of its own
writes them under its own asset tree.

The source uses a 32x32 hard-pixel grid and renders at 512x512. The HUD displays
each portrait at 48x48. Keep faces, headsets, and role silhouettes distinct at
that final size.
