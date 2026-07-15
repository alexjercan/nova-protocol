# Explore online tab: fetch portal catalog, install/uninstall/update from the menu, offline handling

- STATUS: OPEN
- PRIORITY: 14
- TAGS: modding,menu,wasm

Spike: tasks/20260714-202515/SPIKE.md
Depends on: 20260715-142906 (download runtime) and 20260715-142911 (two-pane
screen).

Goal: the "Explore online (coming soon)" placeholder becomes real. The Explore
tab fetches `catalog.json` from the portal on tab open (spinner while loading;
on failure an error + retry button, falling back to the last cached catalog
with a "stale" note - cache the fetched catalog in the small-prefs store).
Rows mark already-installed entries and show an Update badge when the installed
version differs from the catalog's (v1: exact string compare). The details
panel gains Install / Uninstall / Update actions wired to the download runtime,
with progress + error states. Installing registers the mod into the installed
set live (existing re-merge); it appears on the Installed tab immediately.
THE GOAL of the spike family is met here: browse the portal, install the demo
mod, enable it, play it - on native and web.

