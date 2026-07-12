# Editor: visible + editable section keybinds (v0.5.0)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,feature

## Goal

Make editor keybinds easier to assign and discover. Today the only way to
bind a key to a section is to hold that key while plopping the section down.
That is undiscoverable and awkward. Build on top of it:

1. Show the current keybind on the section (in-editor label/overlay) so you
   can see at a glance which key triggers each placed section.

2. Clicking a section opens/allows changing its keybind, so you can rebind
   after placement instead of having to delete and re-plop while holding a key.

The existing hold-key-while-placing flow stays as the fast path; this adds
visibility and post-hoc editing on top of it.

## Notes

- Keep the hold-to-bind placement path working as-is.
- Needs step breakdown via /plan before /work.
