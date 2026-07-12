# Editor keybind labels: add background + a deselect/select-mode button

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0,editor,ux,playtest

## Goal

Two playtest fixes to the editor section-keybind feature (task 20260712-163912):

1. The keybind chip is hard to read against the 3D scene - give it a background
   (a dark rounded pill) so it stands out.
2. You cannot deselect a build/delete tool, so you can never reach
   `SectionChoice::None` - which is where clicking a section arms a rebind. This
   makes the rebind feature unreachable. Add a way to deselect (return to
   select/rebind mode).

## Steps

- [ ] Deselect: add a palette button (top of the section list in
  `setup_editor_scene`, nova_editor/lib.rs) with
  `ButtonValue(SectionChoice::None)`, labeled e.g. "Select / Rebind". The
  existing `button_on_setting::<SectionChoice>` observer already applies it, so
  clicking it sets None (select mode) and the SelectedOption highlight follows.
- [ ] Label background: in `sync_section_keybind_labels`' label spawn, add a
  `BackgroundColor` (dark, ~0.75 alpha) plus small padding and `BorderRadius` to
  the label `Node` so the gold text reads as a pill chip over the scene. Keep the
  gold `TextColor`.
- [ ] Verify `cargo check --workspace --all-targets` + `cargo test -p nova_editor`
  + `cargo fmt`. (The deselect is wired via the existing tested button path; the
  background is a cosmetic Node change - no new logic to unit test. Add/adjust a
  test only if a behavior branch is introduced.) CHANGELOG line.

## Notes

- Root cause of (2): rebind arms only in the `SectionChoice::None` click arm, and
  the palette only had Section(id)/Delete buttons - no button set None, and
  nothing else deselects. The OnEnter reset sets None initially, but the moment
  you pick any tool you're stuck until scene reload.
- Relevant: nova_editor/lib.rs `setup_editor_scene` palette (~503), the label
  spawn in `sync_section_keybind_labels`, `button`/`ButtonValue`/`button_on_setting`.
