# Retune RCS sensitivity: bump mouse a little, scroll a lot (playtest)

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.7.0, input, playtest

## Goal

Playtest 2026-07-18 (after the delta-control change, task 20260718-185826):
RCS "feels a lot better", but the sensitivity is still low. Bump the mouse
aim gain a little and the scroll (vertical) step a lot.

## Steps

- [x] `RCS_AIM_SENSITIVITY` (input/player.rs): 0.02 -> 0.03 (mouse a little more).
- [x] `RCS_SCROLL_STEP` (input/targeting.rs): 0.25 -> 0.75 (scroll a lot more),
  and refresh the doc comment - it still describes the pre-delta-control
  "held virtual-joystick offset the pilot scrolls the opposite way to null",
  which the decay now makes transient.
- [x] Re-run the two affected tests
  (`rcs_mouse_motion_sets_intent_from_the_delta_only_while_active`,
  `rcs_scroll_drives_the_vertical_axis_only_while_active`) - both use symbolic
  constants / sign assertions, so they stay green after the retune.

## Notes

Constants only; the delta+decay model (task 20260718-185826) is unchanged.
Mouse maps per-frame delta pixels -> clamped [-1,1] intent; scroll sets one
notch of the vertical axis per gesture, then decays. Values are feel-tunable.
