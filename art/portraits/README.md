# Campaign portrait sources

Generated SVG sources for the green CRT comms portraits. Run
`scripts/generate-campaign-portraits.py` from the repository root to regenerate
these files and the shipped PNGs: the base game's faces (the player, Range
Control) under `assets/base/portraits/`, the Nova Protocol story mod's under
`assets/mods/nova_protocol/portraits/`.

The source uses a 32x32 hard-pixel grid and renders at 512x512. The HUD displays
each portrait at 48x48. Keep faces, headsets, and role silhouettes distinct at
that final size.
