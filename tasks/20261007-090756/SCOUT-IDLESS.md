# SCOUT-IDLESS: a torpedo whose live target has no `EntityId`

Scout report (scout-idless, read-only, given inline and saved here by the
main worker). The main worker added the "Main worker check" section.

## 1. Which kinds can be a `TorpedoTargetEntity`

`TorpedoTargetEntity` is set once at launch (`torpedo_section/mod.rs:646-647,656-657`).
- Player: `input/player/intent.rs:220-254` commits the `CombatLock`, with
  no kind filter.
- AI: `input/ai/torpedo.rs:215-245`. It filters to ship roots (line 232).
- Scripted: `torpedo_section/scripted.rs:45-77`. It filters to ship roots
  (line 62).

The `CombatLock` comes from `radar_pick` (`input/targeting/radar.rs:39-209`).
It filters only by line of sight and, in combat, by canisters (line 184).
Sensing passes any `RigidBody::Dynamic` (`input/targeting/sensing.rs:271-276`).
An unsigned body gets `unsigned_lock_range` 5.0 units = 50 m
(`targeting/state.rs:79`).

| Kind | Signature | Durable key | Evidence |
|---|---|---|---|
| Ship | yes | `EntityId` | `sections/signature.rs:101-157` |
| Asteroid | yes | `EntityId` | `objects/asteroid.rs:345-373` |
| Torpedo | yes | none | `torpedo_section/bay.rs:365-375,416` |
| Cargo canister | yes | `CargoCanisterRuntimeId` (frozen, allocator resumed) | `cargo_intake_section.rs:279,304` |
| Ship wreck fragment (persistent) | no | none (`FrozenBody.id` is `None`) | `nova_world/src/frozen.rs:358-359,463` |
| Carved rock chunk (transient) | no | none | `asteroid_carve.rs:559-590` |
| Detached piece (transient) | no | none | `integrity/explode.rs:420-448` |
| Shed fixture (transient) | no | none | `sections/fixture.rs:321-339` |

Only the player can launch at an id-less body. A torpedo or canister is
lockable at signature range. An unsigned body is lockable only within
50 m.

## 2. Matching after a Load

`live_by_id` (`save/transients.rs`) matches by `EntityId` only. Nothing
else matches a thawed wreck fragment, chunk, piece or fixture to its
saved self.

## 3. Live behavior when the link is lost

`update_target_position` (`torpedo_section/projectile.rs:12-57`) removes
`TorpedoTargetEntity` (line 36) and keeps `TorpedoTargetPosition`.
Guidance loses the lead term (`projectile.rs:392-432`). Detonation then
fuzes at half the blast radius around the point, not on the target skin
(`projectile.rs:292-296`). Test: `torpedo_survives_target_loss_and_freezes_position`
(`projectile.rs:997-1042`).

## 4. Lifetimes

- Torpedo `projectile_lifetime` is 100 s (`nova_authoring/src/base_content/sections/torpedo_bay.rs:180`),
  or 60 s in the menu duel (`scenarios/main_menu/duel.rs:123`).
- Debris lives 30 s (chunk and piece, `explode.rs:446`) or 12 s (shed
  fixture, `fixture.rs:338`).
- The save settling bound is 600 advancing frames (`save/session.rs:39`).

A wait (Unsettled) can last up to the debris lifetime: 12-30 s, which is
720-1800 frames at 60 fps. That is past the bound, so the save fails.

## 5. Scout options

- (a) `Frozen(last position)`.
- (b) Unsettled.
- (c) Mint ids for all debris.
- (d) Refuse.

The scout recommended (a).

## Main worker check

- The scout table missed two kinds that I added: torpedoes and wreck
  fragments.
  - A torpedo is lockable at signature range, and the player can torpedo
    an enemy torpedo.
  - A ship wreck fragment is persistent, not a transient. Its frozen
    record keeps an `EntityId` only if the body has one (`frozen.rs:358-359`),
    and a severed wreck has none.
- Option (a) changes the shot. Live, the torpedo leads the moving body and
  fuzes on its skin. After a Load, it would fly to a stale point and fuze
  at half the blast radius. That is a loss of the link, made explicit.
- A key that the save already has covers most kinds with no loss:
  - Canister: its `CargoCanisterRuntimeId`.
  - Chunk, piece, fixture or torpedo: these are saved transients in the
    same record list (T3), so the key is their index in
    `WorldSaveState.transients`. The ref resolves on the frame that all
    transients spawn.
  - Wreck fragment: this is the only kind with no key. Either it gets an
    `EntityId` when it is severed (`FrozenBody.id` then keeps it, and
    `live_by_id` finds it), or it uses explicit `Frozen(last position)`.
