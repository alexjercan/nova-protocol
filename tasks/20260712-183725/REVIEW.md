# Review: Editor keybind label bg + deselect button

- TASK: 20260712-183725
- BRANCH: fix/editor-keybind-ux

## Round 1

- VERDICT: APPROVE

Small UX fix, self-reviewed. The "Select / Rebind" button carries
`ButtonValue(SectionChoice::None)` and is applied by the existing, already-tested
`button_on_setting::<SectionChoice>` observer (SelectedOption highlight follows),
so it needs no new logic - it closes the gap that made rebind unreachable (no
button set None; nothing else deselected). The chip background is a cosmetic Node
change (BackgroundColor + padding + border_radius as a Node field, not a
component - the API trap that failed the first compile and was fixed). Existing
editor tests (label reconcile + rebind + Escape) unchanged and green.

Checks: cargo check --workspace --all-targets clean; nova_editor 8/8; cargo fmt
clean. No behavior branch added, so no new unit test.
