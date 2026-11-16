# Low-ammo warning: feature map

Slice: the hold reserve of each ammo item the player's live weapons reload
from; LOW below one magazine (at least 2 rounds), DRY at 0.

## Owner and state
- Owner: `nova_ship` sections. Magazine `SectionAmmo { rounds, capacity }`
  and `SectionReload { item, .. }` live on the weapon SECTION
  (`crates/nova_ship/src/sections/ammo.rs:150-170,296-310`).
- Reserve: the ship root's `ShipInventory` count of `SectionReload::item`
  (`nova_gameplay/src/inventory.rs`). It drains only through
  `tick_section_reload` (`ammo.rs:425-484`, FixedUpdate after
  `SpaceshipSectionSystems`, `sections/mod.rs:326-334`) and rises through
  market buys, jobs and cheats (Update).
- No reload item means no warning: a `ReloadConfig::Disabled` or unlimited
  weapon has no `SectionReload`.

## New
- `sections/ammo_warning.rs` (nova_ship): `AMMO_RESERVE_FLOOR`,
  `AmmoReserveWarningType { Low, Dry }`, pure `ammo_reserve_warning`,
  `AmmoReserveWarnings` (player root, per item), global event
  `AmmoReserveCrossed { ship, item, warning, reserve }`, system
  `track_player_ammo_reserve` in Update `SpaceshipSectionSystems`.
- First sight of an item (spawn, Load, rebuilt weapon, cheat off) records
  the level silently; only a worsening edge triggers. A rise to `None`
  re-arms both edges; a rise to Low re-arms DRY.

## Consumers
- Notice: `nova_hud/src/hud_notice.rs` observer like
  `post_supply_refused_notice` (item name from `GameItems`):
  `PDC ROUND LOW: 400`, `TORPEDO DRY`. The slot plays its `hud_notice` cue.
- Audio: `nova_ship/src/ship_audio/cues.rs` observer plays the player
  ship's authored `ShipFeedbackSounds::ammo_dry`
  (`base/sounds/ammo_dry.wav`, the cockpit ammo gauge voice), Cockpit
  category. Authored-or-silent.
- Steady mark: `nova_hud/src/ammo_readout.rs` gauge child tag `LOW`/`DRY`
  above each gauge whose reload item is warned. Shown only while the
  gauges show (weapons hot or combat lock).

## Ordering
FixedUpdate reload drains hold -> Update `SpaceshipSectionSystems`
(tracker triggers event) -> observers (notice message, cue) -> Update
`NovaHudSystems` notice show; PostUpdate gauge tag driver.

## Proof
- Unit: the rule (`ammo_warning.rs` tests).
- ECS: reload drain past both lines fires one LOW then one DRY; restock
  re-arms (`ammo_warning.rs` tests).
- Notice line: `hud_notice.rs` test.
- Example: `examples/playable/ammo_warning_sandbox.rs`, autopilot walk
  fires the PDC down past both lines; captures the LOW mark and notice.

## Docs
`web/src/wiki/hud.md` (notice, gauges), `web/src/wiki/combat-weapons.md`
(reserve), `CHANGELOG.md` Interface & HUD, `examples/playable/README.md`.
