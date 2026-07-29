# REVIEW - Menus + editor spawn the shared nova_ui widget factories (105359)

- ROUND: 1 (out-of-context reviewer, fresh context)
- VERDICT: APPROVE (no MAJOR)
- DATE: 2026-07-29

All load-bearing claims verified against the code: the volume slider still
drives MasterVolume + its % label (block-meter reactive via `sync_slider_meters`
on the widget's `SliderValue` re-insert); the mods checkbox toggles EnabledMods
and repaints IN PLACE (bg+border+glyph) via `sync_mod_checkboxes`/`checkbox_colors`;
mods/scenarios row selection + hover are live through the `ListRow` reconciler
(Add/Remove<Selected> + Insert<Hovered>, Added override as a SYSTEM); all 9
`panel()` paint-decorator call-sites keep their own Node with border+radius (the
editor rail intentionally un-rounded); the required `Res<UiSkin>` params are safe
in every rig (register + plugin init UiSkin; editor tests never enter Editor);
and the test changes are stronger than the ones they replaced (block-meter
assertion, in-place-sync entity-holding, recursive label_of).

## Findings + resolution

- MINOR (scope note): the editor component-card row stays a bespoke
  `ThemedButton` (defensible - button/drawer-toggle semantics), unmentioned in
  the VERDICT carry-forward. FIXED (added the line).
- NIT (count): VERDICT said "nova_editor 3 call-sites"; actual is 2
  (`panel` + `badge`). FIXED (corrected to 2).
- NIT (robustness): `label_of` recurses to the first Text, which would grab a
  block button's `> ` cursor span - harmless (it only inspects plain buttons).
  FIXED (added a NOTE comment on `label_of`).
