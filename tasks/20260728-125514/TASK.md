# NOVA OS ship app: minimize + make readable the blip overlay (tiny label, ammo as numbers, defer detail to side panel)

- STATUS: OPEN
- PRIORITY: 28
- TAGS: v0.9.0,feedback,ui,hud

## Story

As a player using the `ship` app, I want each blip's on-view overlay to be
minimal and readable, so the schematic is not cluttered with tiny green-on-green
text - the FULL detail belongs in the side inspector panel, not floating on the
3D view.

Playtest verdict (2026-07-28) on the landed legibility change (`20260728-115435`):

- KEEP: "the cubes look really good, I like a lot the idea of having a smaller
  cube inside a frame that WORKS" - the uniform-green fill + bright wireframe
  outline + gap is a win; do not touch it.
- FIX 1 (readability): "the text makes it hard to read, it's really small and
  green on green" - the blip label + integrity bar text is hard to read.
- FIX 2 (ammo): "the ammo count is weird because it's literally 500 circles :D" -
  a high-capacity weapon renders a pip per round. Use NUMBERS instead (or rethink)
  so a large magazine reads cleanly.
- FIX 3 (minimalism): "the first two should be extremely minimal because we will
  have the side panel anyway" - the blip overlay should carry the bare minimum; a
  label with the section name + some way to tell the section KIND makes sense, but
  the detail (integrity %, ammo, status words) moves to the side panel.

## What it should do

- Cut the per-blip overlay down to the essentials: a readable label (section
  name/code + a kind indicator) and keep it legible against the phosphor scene
  (contrast, a backing/outline, or show label only for the SELECTED/hovered blip
  rather than all at once - decide at plan time).
- Replace the ammo PIP row with a compact numeric readout (e.g. `2/6`) or drop
  ammo from the blip entirely and leave it to the side panel.
- Reduce or remove the per-blip integrity bar if the side panel already carries
  status - the blip should not duplicate what the panel shows. Decide how much
  the blip keeps once the panel exists.
- Do NOT change the block rendering (fill + wireframe outline + gap) - that is the
  part the owner explicitly likes.

## Notes

- The overlay lives in `spawn_ship_blip` / `project_ship_blips` in
  `crates/nova_gameplay/src/hud/nova_os_ship.rs`: today each blip is a dot + a
  glyph label + an integrity bar (track+fill) + (weapons) an `ammo_pips()` line.
  `ammo_pips()` renders one `●`/`○` per round - that is the "500 circles".
- Depends on / coordinates with the side-inspector-panel task
  (`20260728-115430`): the panel is where the full detail goes, which is WHY the
  blip can shrink. Sequence this after (or alongside) the panel so we do not strip
  the blip before the detail has a home.
- Load-bearing forks to settle at plan time (surface to owner): (a) show labels
  for ALL blips or only the selected/hovered one; (b) does the blip keep any
  status cue (small bar / dot colour) or defer status entirely to the panel; (c)
  ammo as blip number vs panel-only. These change WHAT is built, not just styling.
- Depends on: 20260728-115430. Follows `20260728-115435`.
