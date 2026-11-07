# Torpedo juice O1-O9: implementation design

Design of 2026-11-05, at master 018cdb96c (F1 landed separately). The owner approved it as D1-D9
and it is delivered; TASK.md holds the decisions, deviations and final state. Rendered summary:
JUICE-OPTIONS.html, section "Implementation design".

Only three live paths deal damage: rounds (`nova_gameplay/src/rounds.rs:962,1120`), contact
(`integrity/core.rs:257-330`), blasts (`damage.rs:362-389`). Classifying by source is complete.

## Shared seams (land first)

### S1. Who zeroed the health (dud vs PDC)

```rust
// nova_gameplay/src/integrity/health.rs (HealthZeroMarker at :53)
/// The source of the hit that zeroed this node. None = unknown.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct HealthZeroedBy(pub Option<Entity>);
// HealthZeroMarker gains #[require(HealthZeroedBy)]; on_damage (:120-155) inserts
// (HealthZeroMarker, HealthZeroedBy(damage.source)). First killer wins.

// nova_ship/src/sections/torpedo_section/mod.rs (:1032 unit struct -> data)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TorpedoDeathType { Contact, Damage }
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct TorpedoShotDownMarker { pub death: TorpedoDeathType }
#[derive(Event, Clone, Copy, Debug)]
pub struct TorpedoKilled { pub torpedo: Entity, pub at: Vec3, pub death: TorpedoDeathType, pub targeted: bool }
/// Pure. Contact iff the source is alive, has ColliderOf, and has neither ProjectileDamage nor NovaBlast.
pub(super) fn classify_torpedo_death(source_alive: bool, source_is_body: bool, source_is_weapon: bool) -> TorpedoDeathType
```

`on_torpedo_body_destroyed` (`bay.rs:60-78`) classifies and inserts the marker once.
`despawn_shot_down_torpedoes` (`bay.rs:84-92`) triggers `TorpedoKilled` before `try_despawn`.
Rejected: last-hit observer (nested flush order), cause parameter on `apply_damage` (touches
every weapon), a new `DamageType` (resistance rules are gameplay).

### S2. Quiet torpedo destroys

`on_destroy_juice` (`juice.rs:488-525`) throws orange sparks and 0.16 trauma on every torpedo
section death, dud or PDC. New `QuietDestroyMarker` in `juice.rs`; `spawn_torpedo` puts it on the
controller and thruster sections; `on_destroy_juice` skips them.

### S3. Defeat credit (O6a, O6c)

New `torpedo_section/kill_credit.rs`, registered on all builds. `TorpedoBlastCredit { owner, at_secs }`
stamped on a ship root by a `HealthApplyDamage` observer when the source is a torpedo `NovaBlast`
(pattern of `threat.rs:287-313`). At `Add<DefeatedMarker>` (fallback: root `IntegrityDestroyMarker`),
if the owner is the player and the credit is within `KILL_CREDIT_WINDOW` (0.5 s), trigger
`PlayerKillConfirmed { at }` (in `nova_gameplay/src/juice.rs`) and remove the credit.

### S4. Per-batch reload

`SectionReloadLanded { entity, rounds }` replaces `SectionReloadComplete` (`ammo.rs:496-511`),
triggered by `tick_section_reload` (`:423-477`) whenever a batch loads. `SectionReloadSound`
replaces `RailgunSectionReloadSound` (`railgun_section/mod.rs:57,355`; `firing.rs:270-274`).

### S5. Player torpedo ETA (O4, O5)

New `torpedo_section/eta.rs`:
`pub fn torpedo_time_to_impact(p, v, target_p, target_v) -> Option<f32>` (range / closing speed,
None when opening); `Resource PlayerTorpedoEta(Option<TorpedoEta { torpedo, seconds }>)`, the
minimum over the player's torpedoes whose target body matches the player's `CombatLock`
(`targeting/state.rs:106`), excluding shot-down and cold torpedoes. Runs in the Update chain
(`mod.rs:998-1017`) after `torpedo_sync_system`; HUD and audio already run later.

## Per option

| Option | Owner | Entry | New items | Budget | Proof |
| --- | --- | --- | --- | --- | --- |
| O1 launch light | `render.rs` near :639 | `On<TorpedoLaunched>` | `on_torpedo_launch_light`; `LightFlash` cold blue, ~2e4 lm, ~10 u, ~0.12 s | transient-light cap (6/3/0) | ECS: one light per launch, none at cap 0 |
| O2 ignition | `ship_audio/combat.rs`; `render.rs` | `On<TorpedoIgnited>` | `on_torpedo_ignited_play_sfx`, `ThrottleKey::TorpedoIgnition`; `TorpedoExhaustMarker`, `TorpedoIgnitionFlare`, `ignition_flare_scale(age)`, `tick_torpedo_ignition_flare` (cone Transform scale, plume bucket clamps 0..1) | flare: particles; sound always | unit: scale >1 in (0,0.15), ==1 after |
| O3 trail | new `trail.rs` | `On<TorpedoIgnited>` inserts `TorpedoTrailEmitter` | `TorpedoTrailSegment`, `emit_torpedo_trail`, `fade_torpedo_trail`, `trail_width`; world-space unparented segments with `TempEntity`, shared mesh + one additive material | `GraphicsBudget.trail_seconds` 1.5/0.75/0 | ECS: no segment outlives its life; none on Low |
| O4 TTI | `nova_hud/src/torpedo_target.rs` | `Res<PlayerTorpedoEta>` | `TorpedoTargetReadoutLine::TimeToImpact`, `tti_line(s)` "TTI 4.2 s" | none | unit: TTI math + format; ECS: hidden with no torpedo |
| O5 closing tone | `ship_audio/cues.rs` | `Res<PlayerTorpedoEta>` | `play_torpedo_closing_tone`, `closing_tick_interval(tti)` ~ (0.08+0.11*tti).clamp(0.08,1), silent above ~8 s | none | unit: interval non-increasing; ECS: ticks stop when ETA None |
| O6a kill stinger | `ship_audio/combat.rs` | `On<PlayerKillConfirmed>` | `on_player_kill_play_stinger` | none | ECS: one event for a player-torpedo kill; none for PDC-only, hostile blast, spawned derelict |
| O6b shock shell | new `death.rs` | `On<Add, NovaBlast>` + `TorpedoSectionPartOf` | `TorpedoBlastShell`, `shell_scale(age, radius)` ease-out to blast radius in ~0.3 s; shared sphere, ~8 bucketed alpha materials | particles | unit: scale == radius at 0.3 s |
| O6c trauma floor | `nova_gameplay/src/juice.rs` | `On<PlayerKillConfirmed>` | `ShakeSettings.player_kill_floor` ~0.20, trauma = max(current, floor) | shake | ECS: floor reached beyond far_distance with shake on; unchanged off |
| O7 dud | `death.rs`, `combat.rs` | `On<TorpedoKilled>` Contact | grey non-emissive puff ~0.6 s; `dud_sound`; `ThrottleKey::TorpedoDeath(IVec3)` | visual: particles; sound always | ECS: contact kill -> one Contact event; unit: classify |
| O8 intercept | `death.rs`, `combat.rs` | `On<TorpedoKilled>` Damage (all torpedoes) | small emissive pop ~0.12 s + few `ImpactSparks`; `intercept_sound` | same | ECS: round/blast kill -> Damage event |
| O9 rack clunk | `ammo.rs`, `combat.rs:109-133` | `On<SectionReloadLanded>` | S4; bay gets `SectionReloadSound(config.reload_sound)` | none | ECS: 6-round rack refilling 1/batch from 4 fires 2 events |

## New content fields (all optional, serde default, not breaking)

| Struct | Field | Base value | Builder |
| --- | --- | --- | --- |
| `TorpedoSectionConfig` | `ignition_sound` | `torpedo_ignite.wav` (licensed) | `nova_authoring/.../sections/torpedo_bay.rs` |
| same | `dud_sound` | `torpedo_dud.wav` (licensed) | same |
| same | `intercept_sound` | `torpedo_intercept.wav` (licensed) | same |
| same | `reload_sound` | `torpedo_rack.wav` (licensed) | same |
| `ShipPresentationConfig` | `torpedo_closing_sound` | `torpedo_closing.wav` (gen-ui-sfx.py) | `nova_authoring/.../ships/mod.rs:242-256` |
| same | `kill_confirm_sound` | `kill_confirm.wav` (gen-ui-sfx.py) | same |

Code-only: `GraphicsBudget.trail_seconds`, `ShakeSettings.player_kill_floor`. Rust API
replacements: `SectionReloadComplete` -> `SectionReloadLanded`, `RailgunSectionReloadSound` ->
`SectionReloadSound`, `TorpedoShotDownMarker` gains a field. New wavs go in the base bundle
resource list (`assets/base/base.bundle.ron:274-308`, check whether generated).

## Caller graph after (+ new, ~ changed)

```
TorpedoLaunched ├─> on_torpedo_launch_effect, on_torpedo_launch_play_sfx
                └+> on_torpedo_launch_light [light cap]                         O1
TorpedoIgnited  ├─> on_torpedo_ignition
                ├+> flare ──> tick_torpedo_ignition_flare [particles]           O2
                ├+> on_torpedo_ignited_play_sfx                                  O2
                └+> TorpedoTrailEmitter ──> emit/fade_torpedo_trail [trail_s]    O3
NovaBlast Add   ├─> insert_particle_effect
                └+> spawn_blast_shell [particles]                                O6b
HealthApplyDamage ├─> on_damage ~(HealthZeroMarker, HealthZeroedBy)              S1
                  └+> on_damage_stamp_torpedo_credit                             S3
HealthZeroMarker(torpedo child) ──> ~on_torpedo_body_destroyed (classify)
    ──> ~TorpedoShotDownMarker{death} ──> ~despawn_shot_down_torpedoes
          └+> TorpedoKilled ├+> death.rs dud puff / intercept pop [particles]   O7/O8
                            └+> on_torpedo_killed_play_sfx                       O7/O8
IntegrityDestroyMarker ──> ~on_destroy_juice (skips QuietDestroyMarker)          S2
DefeatedMarker Add ──> +confirm_kill ──> PlayerKillConfirmed ├+> stinger         O6a
                                                             └+> trauma floor    O6c
~tick_section_reload ──(every batch)──> SectionReloadLanded ──> clunk           O9
+update_player_torpedo_eta ──> PlayerTorpedoEta ├+> TTI line                     O4
                                                └+> closing tone                 O5
```

## Lanes (no two lanes edit one file at once)

| Lane | Files | Covers | After |
| --- | --- | --- | --- |
| A gameplay seams | `integrity/health.rs`, `juice.rs`, `audio/mixing.rs`, `settings.rs` | S1 part, S2, PlayerKillConfirmed, O6c, ThrottleKeys, trail_seconds | - |
| B torpedo core | `torpedo_section/{mod,bay}.rs`, new `eta.rs`, `kill_credit.rs`, `sections/ammo.rs`, `railgun_section/{mod,firing}.rs` | S1 rest, S3, S4, S5, config fields | A |
| C torpedo visuals | `render.rs`, new `death.rs`, `trail.rs` | O1, O2 flare, O3, O6b, O7/O8 visuals | B |
| D audio + presentation | `ship_audio/*`, `ship_design.rs`, `spaceship.rs` | all sounds | B |
| E content | authoring builders, `gen-ui-sfx.py`, wavs, bundle, regenerated RON | base values | D |
| F HUD | `torpedo_target.rs`, `nova_ui` units | O4 | B + owner commits torpedo_target.rs edits |
| H docs | sounds README, web/src/create/{sections,ships}.md, wiki torpedo-bay/hud/combat-weapons/settings, CHANGELOG | - | all |

## Open uncertainties

U1 quiet torpedo destroys drop today's sparks/trauma (look decision). U2 O1 tuning. U3 flare pivot.
U4 trail_seconds derived-only vs a menu lever. U5 trail entity count, floating origin, saves.
U6 duration formatter home. U7 TTI to target anchor, not skin (~0.3 s error on a capital).
U8 shell from inside the blast. U9 floor value; neutralize counts as kill. U10 PDC throttle window.
U11 dud = any Contact death vs untargeted only. U12 per-batch railgun clunk for mod magazines >1.
U13 one rack volume. U14 a contact torpedo may survive and bounce (verify on bench). U15 bundle
list generated or hand-kept. U16 duel scenario bay inherits sounds? U17 a contact source
despawned in the same flush classifies as Damage. U18 0.5 s credit window heuristic. U19 compare
target bodies, not entities.
