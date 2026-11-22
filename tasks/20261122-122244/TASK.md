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

## Build (2026-11-22)
- Tag removed: `AmmoReadoutReserveMark`, its spawn on both gauge kinds and
  `drive_ammo_reserve_marks` are gone (`crates/nova_hud/src/ammo_readout.rs`).
- Flash: `AmmoReadoutReserveFlash` (`Pending` / `Running { started }`), armed
  by the `arm_ammo_reserve_flash` observer on `AmmoReserveCrossed` into LOW
  only; `drive_ammo_reserve_flash` runs after `drive_ammo_readouts` and lifts
  every pip's HSL lightness 45% toward white and a dark pip's alpha to 0.42 at
  peak. Three squared-sine pulses over 1.5 s, hue kept, then removed. The
  magazine breath is amber, 0.9 s, dims lit pips and loops; the flash
  brightens the whole ring in its own hue and stops.
- Gate decision: flash on the next frame the gauge is drawn while still LOW,
  once. Evidence: `SectionReload::advance` runs off the trigger with no
  `WeaponsHot` check (`crates/nova_ship/src/sections/ammo.rs`), and the gauge
  layer is shut outside combat (`sync_ammo_gate`), so a drain usually crosses
  the line with the gauges down. A pending flash is dropped when the reserve
  leaves LOW first.
- DRY: no cue. An empty magazine on a dry reserve is `is_low_ammo` with no
  active reload, so the hue is the theme accent at `DIM_ALPHA` 0.16 on all
  pips and nothing is lit to breathe: a dim amber track.
- Left as is for the owner: the notice `PDC ROUND LOW: 400` / `PDC ROUND DRY`
  (`hud_notice.rs` `post_ammo_reserve_notice`) and the one `ammo_dry` gauge
  sound per crossing (`nova_ship` `ship_audio/cues.rs`).
- `ammo_warning_sandbox` walk: shoots before, flash peak, after the flash,
  and empty on a dry hold; asserts the flash ends, DRY flashes nothing, and
  a restock flashes again.
- Frames: `~/Pictures/nova-ammo-flash/` (`before-tag-*` old build,
  `after-{before,flash,low,dry}` new build, `crop-*`, `sheet-after.png`).
  Top lit pip 249,187,50 at rest, 249,204,105 at flash glow 0.62; a spent
  pip 115,87,37 at rest, 154,126,68 in the flash; dry ring about 114,83,40
  on every pip, no text.
- Tests: `cargo test -p nova_hud --lib ammo_readout` 28 passed, four new.
  Walk passed on a scratch asset root: the repo's generated base RON lagged
  other lanes' in-flight intake and role-goal code at run time.

## Verification
- Asserted state for the flash and no tag; frames of the gauge at full,
  low and dry.

## Done when
- The gauge carries no tag text, flashes softly on low, and the owner has
  played it.
