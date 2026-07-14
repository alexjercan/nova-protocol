# Mods main-menu panel: list installed mods with enable/disable toggles, Explore online coming-soon, base locked-enabled

- STATUS: OPEN
- PRIORITY: 58
- TAGS: menu, modding, spike

Spike: tasks/20260714-174000/SPIKE.md
Depends on: 20260714-174120 (catalog + EnabledMods).

Goal: a "Mods" main-menu section - THE GOAL of the flow (see the `demo` mod in the list
and enable it). Add a "Mods" button to the main menu that toggles a modal `ModsPanel`
(mirror the existing `SettingsPanel`: hidden panel, `Visibility` toggle). Inside: a
scrollable list (reuse the `EditorScrollPanel` + `Overflow::scroll_y()` + wheel-scroll
pattern) of INSTALLED-catalog entries, each a row with the mod name/description + an
enable/disable toggle button whose label+colour reflect `EnabledMods`; the `base` row is
shown enabled and LOCKED (no toggle). Plus a disabled "Explore online (coming soon)"
button and a Back button. A toggle observer flips `EnabledMods` and (via 174120's
re-merge) applies live. Follow nova_menu's `button()`/`observe(handler)` idiom and the
editor palette's data-driven list iteration. `spike` until planned.