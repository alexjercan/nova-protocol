# Hidden dev mods: catalog hidden flag keeps screenshot-reel out of the Mods menu

- STATUS: OPEN
- PRIORITY: 20
- TAGS: modding,menu

Spike: tasks/20260714-202515/SPIKE.md (option W)

Goal: dev/tooling mods must not appear in the player-facing Mods menu. Add a
serde-default `hidden: bool` field to the shipped catalog's `ModEntry`
(nova_modding), set `hidden: true` on the `screenshot-reel` entry in
`assets/mods.catalog.ron`, and filter hidden entries out of the menu list (in
`ModCatalog` construction or the nova_menu rendering - decide in /plan). The mod
still ships, still loads at startup, and `examples/13_screenshot_reel.rs` keeps
enabling it by id via `EnabledMods` unchanged. Update the modding docs to mention
the flag.

