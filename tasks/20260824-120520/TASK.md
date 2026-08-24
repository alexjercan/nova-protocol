# Node editor foundations: edit contexts and stable section ids

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.12.0,editor

Child of the v0.12.0 editor epic (`20260812-131912`), second in the spine
after `20260824-011329`. Research: `tasks/20260815-231945/EDITOR-STATE.md`
sections 1 and 3a.

## Goal

The editor holds more than one thing at a time: per-root build state, an
enter/exit edit-context model, and section ids that survive a restart. This
is the enabler for prefab stamping and world editing; nothing user-visible
needs to change yet beyond "two ships can exist".

## The single-root debt, itemised

- `PlayerSpaceshipConfig` is a SINGLETON resource
  (nova_editor/src/config.rs:13-33) keyed by LIVE preview entities; it must
  become a per-root component on each edited root. Its shape is fine; its
  residence is wrong.
- `Single<...>` assumptions to convert: the commit observer takes
  `Single<Entity, With<SpaceshipPreviewMarker>>` (placement.rs:722), the
  preview solver takes `Single<&Children, ...>` (placement.rs:486), the skin
  sync takes a single root (skin.rs:73). Grep for the marker; this list is
  the checklist, not the total.
- A current-context handle the systems filter on: "editing ship X" vs "in
  the world". Camera and rail scope per context.

## Stable ids

- Section config ids are stringified live `Entity` ids (placement.rs:155-158)
  and the scenario input_mapping is keyed by them (scenario.rs:463-466).
  Entity ids do not survive a process restart; a saved file is impossible
  until this changes.
- Mint stable ids at placement time (sequential per root is enough), store
  them in the build state, and re-key `rebuild_editor_preview_on_enter`
  (placement.rs:258-317) off them.

## Enter/exit semantics (settled)

Switch the edit root; relax validation inside the context; validate at the
boundary on exit (Godot edit context + Cosmoteer blueprint mode). Lowering
only ever runs on a context that passed exit validation.

## Done when

- Two ships can be edited in one session; entering one scopes every editor
  system to it; exiting returns to the outer context.
- Section ids survive exit/re-enter and are entity-independent.
- The existing editor ranges stay green (state waits from `20260824-011329`,
  not SETTLE).
