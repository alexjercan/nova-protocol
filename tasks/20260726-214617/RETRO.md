# Retro - NOVA OS chin controls (20260726-214617)

Landed one commit: working BRIGHT/SCAN knobs + SND/PWR buttons on the NOVA OS
monitor chin, with detent persistence. Review APPROVEd round 1 (NIT/MINOR only).

## What went well

- The RTT sampling shader already reserved both `brightness` and
  `scanline_strength` uniforms (task 193233 was re-slotted ahead for exactly
  this), so the knobs wired to the final shader once. Extending
  `animate_nova_os_crt` to read the settings each frame was a 4-line change.
- Pinning the SCAN default detent to the shipped `NOVA_OS_CRT_SCANLINE_STRENGTH`
  and BRIGHT default to 1.0 kept the out-of-box look byte-identical even though
  `animate` now overwrites both uniforms unconditionally - the reviewer
  independently re-derived that this equality is what preserves the shipped
  image.
- Making `setup_drawer`'s new `NovaOsMonitorSettings` param `Option<Res<>>`
  (surfaced by grepping the ~8 `add_observer(setup_drawer)` call sites BEFORE
  running) let every existing test rig stay green with zero edits.
- Persistence reused the `MasterVolume` pattern verbatim (serde-defaulted
  fields, snapshot + apply, `is_added` init guard), so old settings files still
  load.

## What went wrong

- Two test-rig panics cost two build cycles (~2 min each):
  1. `BorderRadius` is a `Node` FIELD in bevy 0.19, not a standalone component -
     a cascade of "not a Bundle" errors.
  2. `nova_os_font` panics when an `AssetServer` is present but `Assets<Font>` /
     `Assets<Image>` are not registered. First I wrongly dropped `AssetPlugin`
     (which `init_asset` needs), then landed on the right combo: `AssetPlugin` +
     `init_asset::<Font>()` + `init_asset::<Image>()` + the material.

## What to improve next time

- `reuse-known-good-stack` applies hard to TEST RIGS: `spawn_drawer_shell_with_crt`
  already encoded the exact asset setup my chin rig needed. Copying its
  `init_asset` block verbatim from the start would have skipped BOTH font/image
  panics. Reach for the nearest passing in-repo rig before reasoning about which
  plugins/assets a headless render-capable test needs.

## Follow-ups / notes

- NIT (deferred): `NOVA_OS_CASE_LIT` doubles as the SND "unlit" color via
  `nova_os_lit_color(false)` - a clearer `_UNLIT` alias would read better. Not
  worth a second commit; noted for a future cleanup pass.
- Manual acceptance still owed: eyeball the BRIGHT/SCAN effect on real hardware
  (the CRT+RTT scene OOMs local lavapipe - `gpu-example-local-skip`). SCAN top
  detent is 0.20 per the owner's 2026-07-27 call.
</content>
