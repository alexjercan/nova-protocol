# Editor: gallery section picker with previews, dropdowns, filters

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.11.0,editor,ui

Goal: the editor's section selection becomes a parts_viewer-style gallery -
3D preview tiles with labels, category dropdowns, text filter, focus preview
with turntable. Owner: "that's how the editor selection for sections should
look like, but with dropdowns and filters" (2026-08-12).

Context:
- Reference UX: examples/screenshots/parts_viewer.rs (task 20260812-100246)
  - grid gallery, paging, Enter-to-focus turntable, ship view. Owner loved
  it; lift its patterns, but the editor version browses SECTION PROTOTYPES
  from the catalog (not art/ candidates).
- Current editor selection: study examples/ui/editor.rs and the editor crate
  path first; record the delta in the task.

Scope:
- Gallery grid of catalog section prototypes with live 3D previews + names.
- Filters: category dropdown (structure/propulsion/weapon/...), text search.
- Focus mode: turntable preview, stats readout (size, mass, HP, behavior).
- Selection feeds the existing placement flow (link-point snapping arrives
  with task 20260812-131005; do not block on it).
- Keyboard + mouse; gamepad is task 20260714-001140, not here.

DoD:
- UI harness coverage (ui/ example or extension of editor.rs walk) proving
  browse -> filter -> focus -> select -> place.
- probe green; screenshots refreshed where the editor appears in wiki/web.
