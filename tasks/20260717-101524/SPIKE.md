# Spike: sound ownership - which sounds are section/mod content vs engine UI, and the SoundBank split

- DATE: 20260717-101524
- STATUS: RECOMMENDED
- TAGS: spike, audio, modding, v0.7.0

## Question

Where is the ownership boundary for each of Nova's 16 sounds - section/mod
CONTENT (authorable, shipped by the base mod, referenceable via
`self://`/`dep://base`) vs engine/UI ASSET (loaded directly from `assets/`,
never mod content)? And what code structure replaces the single flat
`SoundBank<NovaSfx>` so the two kinds are structurally distinct instead of one
plopped-together bank? A good answer: a per-sound ownership table, a target
architecture, a migration order, seeded tasks.

## Context

Task 20260717-002228 gave the turret an authorable
`fire_sound: Option<AssetRef<AudioSource>>` (snapshotted unresolved at spawn,
resolved by the fire-cue observer, bank fallback) and moved ALL 16 wavs under
`assets/base/sounds/` + base `resources`. User feedback (2026-07-17): that
overshoots - UI/HUD sounds are game chrome like `icons/` (deliberately kept at
the asset root in the Option A migration), NOT base-mod content. The mod
boundary should be per-section/per-object sounds ("each turret kind can have
its own sounds"), and the code should stop treating all sounds as one flat
bank.

Current wiring: `SoundBank<NovaSfx>` (16 keys) inserted by
`nova_assets::register_sounds` (`load_paths`, `base/sounds/<name>.wav`); cues
fire from `nova_gameplay/src/audio.rs` (observers + polls),
`nova_gameplay/src/hud/objective_feedback.rs` (objectives),
`nova_menu` (menu_select/ui_toggle), and
`nova_scenario/src/objects/salvage.rs:176` (salvage pickup).

Grounding facts verified in this spike (file:line):

- `ControllerSectionConfig` exists and already carries a
  `render_mesh: Option<AssetRef<WorldAsset>>` (controller_section.rs:20-33) -
  a natural host for radar/lock/safety sound fields. Lock is a computer-gated
  capability (`ship_grants_verb`, targeting.rs:974-1020), so the radar/lock
  cue family conceptually belongs to the controller ("the ship's computer").
- The radar/lock messages carry NO entity - just `combat: bool`
  (targeting.rs:277-310); they are player-scoped, so the cue reads the player
  ship's `ControllerSectionMarker` child (one query).
- Torpedo projectiles carry `TorpedoSectionPartOf` /
  `TorpedoSectionSpawnerEntity` back-refs (torpedo_section/render.rs:476,
  projectile.rs:68) - the launch cue can reach its section like the turret
  cue reaches its turret via `TurretSectionPartOf`.
- `AsteroidConfig` already has `texture: AssetRef<Image>`
  (objects/asteroid.rs:23-27) - asteroids are content with AssetRefs;
  impact/destroy sound fields attach the same way.
- `SalvageCrateConfig` exists (objects/salvage.rs:44) and the pickup cue plays
  in the same file (:176) with the crate entity in hand - a
  `pickup_sound` field is trivial to route.
- The thruster hum is ONE global loop entity whose volume is a max over ships
  (audio.rs `ensure_thruster_loop`/`compute_thruster_hum_volume`); making its
  SOUND per-section means one loop entity per distinct resolved handle, with
  the same per-ship attenuation math feeding each group.
- `HealthApplyDamage` carries the target entity + `source: Option<Entity>` -
  the impact cue knows what was hit (and usually the shooter).

## Ownership table (the decision)

Owner "section/content" means: an `Option<AssetRef<AudioSource>>` field on the
config, authored in content, shipped under `assets/base/sounds/` + base
`resources`, mod-overridable via `self://`/`dep://`. Owner "UI" means: loaded
directly from root `assets/sounds/`, never mod content (the `icons/`
precedent).

| Sound | Cue site | Owner | Host field |
| --- | --- | --- | --- |
| turret_fire | Add<TurretBulletProjectileMarker> | turret section (LANDED 20260717-002228) | `TurretSectionConfig::fire_sound` |
| dry_fire | play_dry_fire_cue poll | turret section | `TurretSectionConfig::dry_fire_sound` |
| torpedo_launch | Add<TorpedoProjectileMarker> | torpedo section | `TorpedoSectionConfig::launch_sound` |
| thruster_loop | global hum entity | thruster section | `ThrusterSectionConfig::loop_sound` |
| lock_on | RadarLockAcquired msg | controller section | `ControllerSectionConfig::lock_on_sound` |
| lock_off | LockClearedToast msg | controller section | `ControllerSectionConfig::lock_off_sound` |
| radar_deny | RadarDenied msg | controller section | `ControllerSectionConfig::radar_deny_sound` |
| radar_retarget | RadarRetargeted msg | controller section | `ControllerSectionConfig::radar_retarget_sound` |
| safety_on | WeaponsHot edge | controller section (recommend; see open Qs) | `ControllerSectionConfig::safety_on_sound` |
| impact | On<HealthApplyDamage> | per-TARGET content | `BaseSectionConfig::impact_sound` + `AsteroidConfig::impact_sound` |
| explosion | On<Add, IntegrityDestroyMarker> | per-TARGET content | `BaseSectionConfig::destroy_sound` + `AsteroidConfig::destroy_sound` + `TorpedoSectionConfig::detonation_sound` |
| salvage_pickup | salvage.rs pickup | crate content | `SalvageCrateConfig::pickup_sound` |
| objective_new | hud/objective_feedback.rs | UI (assets root) | UiSfx bank key |
| objective_complete | hud/objective_feedback.rs | UI (assets root) | UiSfx bank key |
| menu_select | nova_menu On<Activate> | UI (assets root) | UiSfx bank key |
| ui_toggle | nova_menu ESC toggle | UI (assets root) | UiSfx bank key |

The impact/material insight: the user asked about different impact sounds per
material (bullet x material). Per-TARGET sounds already deliver per-material
variety - the target IS the material (a rock asteroid, a hull section, a
reinforced section can each author a different `impact_sound`), with no
combinatorial matrix. A projectile-side modifier (AP round sounds different on
the same hull) is a deferred extension, not part of this family.

## Options considered

- **A. UI bank + content-authored world sounds (recommended).** `NovaSfx`
  splits: a small `UiSfx` bank (4 keys: menu_select, ui_toggle, objective_new,
  objective_complete) loaded from root `assets/sounds/`; every world/gameplay
  sound becomes an authorable AssetRef field per the table, resolved at cue
  time (the landed fire_sound pattern). During migration a transitional
  `WorldSfx` bank (12 keys, `base/sounds/` paths) keeps un-migrated cues
  working; each family task deletes its keys; the last one deletes the bank.
  End state: no world sound exists outside content. Pros: the boundary is
  structural (a bank key IS engine chrome; an AssetRef field IS content); mods
  re-skin per section kind, which is the user's ask. Cons: most work; needs a
  fallback policy per cue (below).
- **B. Flat bank as permanent fallback + per-content overrides.** Keep the 16-key
  bank; add AssetRef overrides family by family (fire_sound stays the template).
  Pros: least change, never silent. Cons: exactly the "plop everything in the
  bank" the user objects to - the bank still hardcodes base-mod file paths
  outside the content pipeline, UI and world sounds stay one blob, and dead
  fallback keys accumulate as overrides land.
- **C. Content-driven global sound registry.** A new Content kind ("sound map")
  the base mod declares; the bank is populated from the merge. Pros: mods could
  re-skin global cues wholesale without touching sections. Cons: new content-type
  machinery for a need nobody has voiced; per-section ownership (the actual ask)
  still has to be built on top; C can be added later if wanted.
- **Do nothing.** Keeps all 16 under base/ including menu clicks - rejected by
  user feedback that motivated this spike.

## Recommendation

Option A, migrated family-by-family, foundation first:

1. **Bank split (foundation, task 20260717-101615).** `UiSfx` (4 keys) loaded
   from root `assets/sounds/` (move the 4 wavs BACK out of `assets/base/sounds/`
   and OUT of base `resources`); transitional `WorldSfx` bank for the remaining
   12 from `base/sounds/`. nova_menu + objective_feedback repoint to `UiSfx`.
   After this task the ownership boundary is structural; everything else is
   incremental.
2. **Weapon-section one-shots (20260717-101624).** `dry_fire_sound` on turret,
   `launch_sound` on torpedo bay - both have entity back-refs at the cue site;
   pure fire_sound-pattern application. Delete their WorldSfx keys.
3. **Controller sounds (20260717-101633).** lock_on/lock_off/radar_deny/
   radar_retarget/safety_on as `ControllerSectionConfig` fields; cues look up
   the player ship's controller section (messages are player-scoped). Delete
   their WorldSfx keys.
4. **Per-target impact/destroy (20260717-101641).** `impact_sound` +
   `destroy_sound` on `BaseSectionConfig` (all section kinds) and
   `AsteroidConfig`; `detonation_sound` on `TorpedoSectionConfig` (snapshotted
   onto the projectile). Observers read the snapshot from the target/destroyed
   entity; throttling/attenuation unchanged. Delete impact/explosion keys.
5. **Thruster loop (20260717-101650).** `loop_sound` on
   `ThrusterSectionConfig`; the single global loop becomes one loop entity per
   distinct resolved handle (normally 1), same per-ship max-volume math per
   group. The menu-ambience backdrop plays a live scenario, so the hum there
   must keep working. Delete the thruster key.
6. **Salvage pickup (20260717-101659).** `pickup_sound` on
   `SalvageCrateConfig`; the cue site already holds the crate entity. Deletes
   the last WorldSfx key -> delete the WorldSfx bank; only UiSfx remains.

Fallback policy: once a family migrates, its cue is authored-or-silent -
gen_content authors every base default (as it already does for fire_sound), so
the shipped game is audibly identical; a mod section that omits a sound is
silent by authored choice (mirrors how content owns its art). No hidden global
default survives - that hidden default is the "plop" being removed. Volume
constants stay code-side per cue for now (see open Qs).

Sequencing note: tasks 2-6 are independent of each other but all depend on 1.
Each is a small, reviewable /flow cycle extending a landed pattern; the risky
one is 5 (behavioral refactor of a continuous cue) and the fiddly one is 4
(most cue sites and target kinds).

## Open questions

- **safety_on placement**: recommended controller (the weapons computer
  re-engaging its safety); could argue it is player-UI chrome. Costs one field
  either way; confirm at /plan of 101633.
- **Authorable per-sound volume** (`fire_sound_volume` etc.): deferred - cue
  volume constants stay in audio.rs. Becomes attractive once mods ship loud
  sounds; revisit as its own small task if it comes up.
- **Projectile x material impact matrix**: deferred; per-target sounds cover
  per-material variety without a matrix.
- **salvage_pickup**: user flagged as "interesting question"; recommended mod
  side (crates are scenario content with a config). If it lands feeling wrong,
  reverting to a UiSfx key is a one-file change.
- Related open task 20260708-224303 (integration test for SFX event->sound
  wiring, p20) presumes the flat bank - re-audit/fold it once the family lands.

## Next steps

- tatr 20260717-101615 (p34): split the sound bank - UI in assets/, world behind the base mod
- tatr 20260717-101624 (p31): weapon-section one-shots (turret dry_fire, torpedo launch)
- tatr 20260717-101633 (p30): controller section sounds (lock/radar/safety)
- tatr 20260717-101641 (p28): per-target impact + destroy sounds
- tatr 20260717-101650 (p26): thruster loop as a section sound
- tatr 20260717-101659 (p24): salvage pickup sound on the crate config

## Fix record

(Each implementing task appends a line here as it lands.)

- 20260717-002228 (pre-spike): turret `fire_sound` AssetRef exemplar landed;
  all 16 wavs currently under `assets/base/sounds/` - the foundation task
  re-splits them per the table above.
- 20260717-101615 (foundation): UiSfx (4 keys, root `assets/sounds/`) /
  WorldSfx (12 keys, `base/sounds/`, transitional) bank split landed; the 4 UI
  wavs moved back to the root and out of base `resources`;
  `load_world_sfx_bank` is the one place the world path convention lives. The
  five family tasks now shrink WorldSfx to deletion.
- 20260717-101624 (weapon one-shots): turret `dry_fire_sound` + torpedo bay
  `launch_sound` landed as authored AssetRefs (snapshotted unresolved,
  resolved by the cue observers), and `fire_sound` flipped to
  authored-or-silent; WorldSfx dropped TurretFire/TorpedoLaunch/DryFire
  (12 -> 9 keys). Base content authors all three, so shipped audio unchanged.
- 20260717-101633 (controller sounds): the five radar/lock/safety cues became
  `ControllerSectionConfig` fields snapshotted into one
  `ControllerSectionSounds` component; the cues resolve the PLAYER ship's
  controller (messages carry no entity) and drain unconditionally. WorldSfx
  9 -> 4 (ThrusterLoop, Explosion, Impact, SalvagePickup remain). safety_on
  placed on the controller per the spike recommendation.
