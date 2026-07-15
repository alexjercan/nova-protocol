# Mods screen rework: two-pane Installed|Explore layout with search, quiet enable toggles, details side panel

- STATUS: OPEN
- PRIORITY: 14
- TAGS: modding, menu, ui

Spike: tasks/20260714-202515/SPIKE.md (option AA)
Depends on: 20260715-142849 (bundle meta feeds the details panel).

Goal: rework the Mods menu into a Factorio/Wesnoth-style two-pane screen.
LEFT: tab bar (Installed | Explore online), a search box, and a scrollable list
of rows - name, version, author, with a QUIET per-row enable checkbox (not a
big toggle button) on the Installed tab; base shown locked. RIGHT: a details
side panel for the selected mod rendered from its bundle meta - title, author,
version, description, dependencies, icon/screenshots as a stretch goal - plus
the action buttons (Enable/Disable here; Install/Uninstall/Update belong to the
Explore task). Built from the existing nova_menu idioms (button()/observe(),
scroll panel, theme::*). This task ships the Installed tab fully working; the
Explore tab renders as a placeholder until 20260715-142916 wires it.

