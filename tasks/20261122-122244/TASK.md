# Ammo gauge: soft flash on low reserve, no LOW/DRY tag

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.19.0, hud, combat

## User facts
- Owner 2026-11-22: "I don't like the 'DRY' text on the PDC/torpedo bay, I
  mean it's obvious when it's empty, maybe some other kind of
  'notification' when you run low, like the thing flashes, but not text on
  top of it, and when it's dry it's obvious it's darkened anyway, so it's
  fine, but when it's low it can flash or something, but not obnoxious".
- Owner decision: the low/dry HUD notice above the dock bar stays.

## Delivery
- Remove the `LOW`/`DRY` tag from the ammo gauge. A soft, short flash on
  low reserve; an empty gauge stays dark with no text.
- Build running (2026-11-22).

## Verification
- Asserted state for the flash and no tag; frames of the gauge at full,
  low and dry.

## Done when
- The gauge carries no tag text, flashes softly on low, and the owner has
  played it.
