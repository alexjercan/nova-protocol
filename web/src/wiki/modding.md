# Modding

Nova Protocol is data-driven: scenarios and mods are authored as **RON data
files**, not code. Everything the game ships - the campaigns, the demo mod - is
content loaded through the same pipeline your own content uses, so anything the
base game does, a mod can do too. This page is the front door; the guides below
have the detail.

## Author a scenario

A scenario is a list of event handlers - each pairs an event (a ship destroyed,
an area entered, an objective met) with filters that gate it and actions that
mutate the world. You write it in RON with the existing vocabulary, no Rust. See
[Author a scenario (RON)](../dev/guide-author-scenario/) for the full grammar and
a worked objective loop, and the [Scenarios](../scenarios/) page for what a
scenario places into the world.

## Package and share a mod

Content ships as a **bundle** - a folder with a manifest listing its scenarios
(and any section prototypes). Add it to the game to test it, then publish it to
the mod portal so other players can install it in-game. See
[Make and publish a mod](../dev/guide-make-a-mod/) for the end-to-end lifecycle,
and [Modding data format (RON)](../dev/modding-ron/) for the bundle / catalog /
portal reference.

## Extend the engine

New event kinds, filters, actions or scenario-object types need a small Rust
change - that is a contributor task, covered in
[Extend the scenario engine](../dev/guide-extend-scenarios/).
