# Review: Salvage crate pickup polish (20260714-090002)

Branch: `feat/salvage-pickup-polish`

## Verdict: APPROVE (after one MAJOR fixed in-cycle)

## Scope reviewed

- `NovaSfx::SalvagePickup` cue + `SALVAGE_PICKUP_VOLUME` (nova_gameplay/audio.rs),
  loader entry in `NOVA_SFX_FILES`, prelude export.
- `salvage_pickup` generator entry + regenerated placeholder WAV.
- Pickup observer + dedup in nova_scenario's `SalvageCratePlugin`.
- Shakedown crate spacing (~29-37u -> >=53u) + pin test.

## Findings

### R1 (MAJOR, FIXED in-cycle) - the cue multi-dinged, once per section collider

A player ship is one `RigidBody` with many section colliders (`base_section.rs`
gives each section its own `Collider::cuboid`). Avian's `CollisionStart` fires
per collider-pair, not per body-pair (confirmed in the avian 0.7 source: the
event carries `collider1/2` and `body1/2`). An empirical probe (3 section
colliders flying through one crate sensor) produced **3 dings**, not 1.

The first cut observed raw `CollisionStart` gated to the player, so it inherited
this: one pickup would fire the ding 3-4 times. This is the same multi-collider
exposure the existing scenario `add_one` crate counting has, but the counting is
protected by the pickup-despawn landing before later sections enter - fragile,
and not something the cue should lean on.

Fix: dedup per crate entity via a `DingedCrates` set - `insert` returns true only
on first contact, collapsing the burst to one ding regardless of collider count
or despawn timing. Pruned by an `On<Remove, SalvageCrateMarker>` observer so the
set stays bounded and correct across entity-index reuse. Pinned by
`a_multi_collider_player_dings_once_per_crate` (self-guarding: 0 without the
collision, 3 without the dedup, both fail).

### R2 (MINOR, accepted) - README "Required files" table not extended

`assets/sounds/README.md` lists only the 5 original core cues; the objective and
lock UI cues added later are already absent, so `salvage_pickup` matches
precedent. `NOVA_SFX_FILES` + the generator's `SOUNDS` dict are the real source
of truth. Left as-is to avoid a partial fix; a full table refresh is a separate
docs chore.

## Tests

- `a_player_flying_into_a_crate_dings_once` - happy path, real physics, delivery
  guard (0 before the pass).
- `a_multi_collider_player_dings_once_per_crate` - the R1 regression pin.
- `a_non_player_body_through_a_crate_stays_silent` - the player gate, with a
  `CrateCollisions` delivery guard proving the stimulus fired.
- `crates_are_spaced_for_distinct_pickups` - pins >= 5x pickup radius apart.
- `every_nova_sfx_key_has_a_file` - widened to cover all 12 keys (was 5).

All new tests green; workspace `cargo check` clean; `cargo fmt --check` clean.
Full suite deferred to CI per project convention.
