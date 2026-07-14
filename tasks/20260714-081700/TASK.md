# Editor sandbox UI/layout overhaul: readable build panel, palette, grid feedback

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: wontdo,editor

Spike: tasks/20260714-081636/SPIKE.md

Goal: make the sandbox editor legible and pleasant to build in. Today the whole
editor is one 1978-line `crates/nova_editor/src/lib.rs` that places sections on a
fixed grid by click, with a scrollable build panel and per-section keybind chips
but little else. Direction (user-stated for v0.6.0): rework the sandbox UI /
layout - a clearer section palette, better placement/grid feedback, panel
placement that does not fight the viewport, and general polish so building a ship
feels good. This is the UX half of the editor work; the persistence half lives in
20260708-162014 (save/load blueprints) and the two dovetail.

