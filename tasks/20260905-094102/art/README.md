# Panel style candidates

HD vector panels in the style of the campaign portraits: the same CRT bezel,
phosphor palette and scanlines as `art/portrait-candidates/`, drawn as flat
shapes at 1920x1080 instead of pixel blocks. Each candidate shows The Strike,
beat 2 of the main comic. `hd_panels.py` draws all of them; run it and
rasterise with `magick <file>.svg <file>.png`.

| File | Framing |
| --- | --- |
| `the-strike-a-wide` | Wide shot. Meridian broadside, the bolt from Severance emerging from the moonlet, Cutter One small in the junk. |
| `the-strike-b-cockpit` | From inside Cutter One. Two helmets, the console, the readout with the carrier link lost. |
| `the-strike-c-flash` | The instant of the hit. Ships as silhouettes against the flash, junk in the light. |
| `the-strike-d-aftermath` | The two halves drifting apart, embers, the five skiffs arriving. |
| `the-strike-e-page` | A three-panel page: the moonlet with one amber light, the hit, Cutter One hiding among the junk. |

The generator draws every hull and prop from a small set of functions
(`meridian`, `severance`, `cutter`, `skiff`, `moonlet`, `junk`, `bolt`,
`burst`, `helmet`) so a chosen framing can be reused for other beats. The
design page keeps its line-art boards until a style is chosen.
