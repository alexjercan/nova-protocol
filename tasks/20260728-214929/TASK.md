# Adopt input-prompt key glyphs across game + web key-UI (Alt style)

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, ui, assets
- KIND: TASK
- FLOW STEP: DROPPED
- PLAN STATUS: DRAFT

## Story

Owner direction (2026-07-28, during the UI-rework spike): the FREE Input Prompts
pack (JulioCacko, CC0) has nice key/mouse glyph icons that would upgrade every
place the game or web UI names a key. The spike already imported the
`Keyboard_Mouse` set (Alt primary, Dark, White; full glyph range) under
`examples/ui/assets/input-prompts/` with a provenance NOTICE, and the HUD PoC
dock uses the Alt keycaps. This task looks into adopting them for real, beyond
the throwaway PoCs.

Style ranking (owner): Alt > Dark > White; the other pack styles and the gamepad
sets were not imported.

## Candidate surfaces

- DONE-BY-EPIC: flight HUD keybind cluster / verb cues -> the icon-chip dock
  ships with 20260728-175742 (this task no longer owns that surface).
- Editor: the "placed section shows its bound key as a chip" surface.
- NOVA OS help / hint lines that name keys.
- Web wiki keybinds page + tutorial (currently `<kbd>` text).
- Menu/settings Controls reference (keyboard + gamepad rows).

## Open questions to resolve first

- RESOLVED 2026-07-28 (owner directive; tasks/20260728-233707/DECISION.md):
  canonical home is `assets/input-prompts/keyboard/Alt/` (Alt only, license
  in credits/), relocated by 20260728-233707; the KeyCode -> glyph mapping
  table + the HUD dock adoption land with 20260728-175742. This task keeps
  the REMAINING surfaces below.
- Key-name -> glyph-file mapping for gamepad if the gamepad sets get
  imported later (keyboard mapping ships with 175742).
- Sizing/tint: the Alt glyphs are dark keycaps with a white letter; confirm they
  read on both the phosphor HUD and the light web pages, or pick per-surface style
  (Alt on dark, Dark/White where needed).
- Whether to import the gamepad sets (trademark note in the NOTICE applies) for
  the Controls reference's gamepad column.

## Definition of Done

Direction-level for now (this is a "look into it" task). At minimum when it runs:
a decision on the canonical asset location + mapping approach recorded in a
DECISION.md, and at least one real (non-PoC) surface switched to the glyphs with
a render eyeball.

## Notes

License is CC0 (public domain, attribution appreciated not required); provenance
moves to `credits/CREDITS.md` + `credits/licenses/` with 20260728-233707 (the
old `examples/ui/assets/input-prompts/NOTICE.md` is absorbed there). Depends on
nothing hard, but best sequenced after the HUD restyle child (20260728-175742)
so the HUD dock is the first real adopter.

## Interim note (2026-07-30, task 20260730-122940)

The keycap SIZING path is settled and this task inherits it: every glyph is
trimmed to the cap it actually paints (an alpha scan at load, in
`nova_gameplay::hud::key_glyphs`), and a `KeyCap` sizes any site by pinning the
HEIGHT and deriving the width from the art's aspect
(`KeyCap::node`/`KeyCap::apply`). Every new surface adopted here - web key-UI,
NOVA OS help, editor chips - goes through `KeyCap` rather than setting a square
box; the "Sizing/tint" bullet above now only has the TINT half left to decide.
Rationale in `tasks/20260730-122940/DECISION.md`.


## Dropped

- REASON: this was done already
