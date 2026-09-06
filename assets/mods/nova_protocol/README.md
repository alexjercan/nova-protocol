# Nova Protocol

The story campaign, shipped as a mod. It is enabled on a fresh install and
listed in the Scenarios picker under the **Nova Protocol** campaign header.
New Game does not start it: New Game starts the base game's training range
(`tutorial`, "Basic Training").

## What it carries

- `campaigns/nova_protocol.content.ron` - the campaign: its chapters in play
  order. One so far.
- `scenarios/first_shift.content.ron` - "An Ordinary Shift", the first chapter.
- `portraits/*.png` - the green CRT faces of the story's named voices. The
  player's own face is base art, borrowed as `dep://base/portraits/player.png`.
- `thumbnails/first_shift.png` - the picker plate.

## Generated, not hand-written

Both content files are generated from the Rust builders under
`crates/nova_authoring/src/mod_content/nova_protocol/`:

```sh
cargo run content gen
```

The `content_ron_parity` test fails on a hand edit. Change the builders, then
regenerate. The portraits come from `scripts/generate-campaign-portraits.py`
and the thumbnail from `scripts/gen-scenario-thumbnails.py`.

## Turning it off

Disable it from the main-menu Mods section, or drop its id from `EnabledMods`.
The campaign header and its chapter leave the Scenarios picker; nothing else
changes.
