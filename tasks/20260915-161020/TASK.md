# Replace section modifiers with root capabilities and typed design patches

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.14.0, refactor, content, ship, editor

## Goal

Replace controller-owned verbs and closed section modifications with:

- explicit capabilities on the spaceship root;
- physical controller sections that provide attitude authority only;
- reusable ship designs made from placed sections;
- typed, curated patches for prototype section instances; and
- patch-aware editor inspection and saving.

This is an intentional breaking content and code refactor. There are no external
users or external mods to preserve. Delete the old model. Do not add aliases,
legacy deserializers, fallback readers, migrations at load, deprecated variants,
or any other backward-compatibility machinery. Regenerate and update all shipped
content in this repository.

## Decisions

- Use a compiler-assisted refactor: delete rejected types and fields, add the
  accepted structs, then follow compiler errors through every consumer.
- A spaceship root owns software decisions, capability state, targeting state,
  autopilot state, RCS state, alarms, and future docking policy.
- A controller section is attitude hardware only. It contributes attitude
  sensing, steering lag, torque, and rotation command execution.
- Multiple controllers stack attitude authority. They do not union software
  capabilities.
- Losing all controllers prevents commanded rotation and disables or disengages
  attitude-dependent maneuvers. It does not remove configured LOCK, RCS, or
  point-defense capabilities.
- `CombatLock` is root runtime state. `lock_enabled` is the root capability that
  permits lock operations.
- Use `ShipDesign` terminology for the reusable assembled section design. Do not
  call it a hull because `Hull` is already a section kind.
- `SpaceshipConfig` remains the scenario object configuration.
- Complete capabilities live directly on `SpaceshipConfig`. They are not a
  patch. The all-enabled value is the default and is omitted from RON.
- Section patches belong to the `Prototype` design source. Inline designs are
  already complete and do not accept patches.
- Patches are a curated Update surface, not generic serialized-value merging.
- Patch gameplay and editor-facing values. Do not patch art, colliders, sockets,
  animations, section kind, section source, or turret joint topology.
- Position and rotation are patchable.
- Use domain enums for meaningful modes. Use `Option<Option<T>>` for ordinary
  nullable patch fields where that is simpler.
- Turret fire rate stays per muzzle. Add stable muzzle IDs and patch only muzzle
  gameplay data, not the joint tree.
- Root feedback audio and alarms live in the ship design presentation config.

## Proposed core structs

Names are accepted for this refactor unless implementation reveals a concrete
collision.

```rust
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct ShipCapabilities {
    pub stop_enabled: bool,
    pub goto_enabled: bool,
    pub orbit_enabled: bool,
    pub lock_enabled: bool,
    pub rcs_enabled: bool,
    pub point_defense_enabled: bool,
}
```

`Default` sets every field to `true`. Serde omits `capabilities` when the whole
value is default and omits enabled fields inside a non-default value.

```rust
pub struct SpaceshipConfig {
    pub design: ShipDesignSource,
    pub controller: SpaceshipController,
    pub allegiance: Option<Allegiance>,
    pub capabilities: ShipCapabilities,
}

pub enum ShipDesignSource {
    Inline(ShipDesign),
    Prototype {
        id: ShipDesignId,
        section_patches:
            Option<BTreeMap<SectionId, SpaceshipSectionConfigPatch>>,
    },
}

pub type ShipDesignId = String;

pub struct ShipDesign {
    pub sections: Vec<SpaceshipSectionConfig>,
    pub integrity: ShipIntegrityConfig,
    pub presentation: ShipPresentationConfig,
}

pub struct ShipIntegrityConfig {
    pub collapse_threshold: Option<f32>,
}

pub struct ShipPresentationConfig {
    pub skin: bool,
    pub style: Option<String>,
    pub collapse_sound: Option<AssetRef<AudioSource>>,
    pub lock_on_sound: Option<AssetRef<AudioSource>>,
    pub lock_off_sound: Option<AssetRef<AudioSource>>,
    pub radar_deny_sound: Option<AssetRef<AudioSource>>,
    pub radar_retarget_sound: Option<AssetRef<AudioSource>>,
    pub safety_on_sound: Option<AssetRef<AudioSource>>,
    pub warn_lock_sound: Option<AssetRef<AudioSource>>,
    pub ammo_dry_sound: Option<AssetRef<AudioSource>>,
    pub warn_hull_sound: Option<AssetRef<AudioSource>>,
    pub warn_hull_fraction: f32,
    pub rcs_loop_sound: Option<AssetRef<AudioSource>>,
}
```

Replace the current ambiguous catalog vocabulary:

```text
ShipHull    -> ShipDesign
ShipConfig  -> ShipDesignPrototype
ShipSource  -> ShipDesignSource
GameShips   -> GameShipDesigns
ShipId      -> ShipDesignId
```

The catalog record may retain ID and display-name metadata around a
`ShipDesign`, but it must use design/prototype names and must not be confused
with scenario `SpaceshipConfig`.

## Proposed section patch structs

```rust
pub struct SpaceshipSectionConfigPatch {
    pub position: Option<Vec3>,
    pub rotation: Option<Quat>,
    pub config: SectionConfigPatch,
}

pub struct SectionConfigPatch {
    pub health: Option<f32>,
    pub kind: Option<SectionKindPatch>,
}

pub enum SectionKindPatch {
    Hull(HullSectionConfigPatch),
    Thruster(ThrusterSectionConfigPatch),
    Controller(ControllerSectionConfigPatch),
    Turret(TurretSectionConfigPatch),
    Torpedo(TorpedoSectionConfigPatch),
    Railgun(RailgunSectionConfigPatch),
}

pub struct HullSectionConfigPatch;

pub struct ThrusterSectionConfigPatch {
    pub magnitude: Option<f32>,
}

pub struct ControllerSectionConfigPatch {
    pub steering_lag: Option<f32>,
    pub max_torque: Option<f32>,
}
```

A kind patch must match the resolved prototype kind. A mismatch is a content
lint error. A patch cannot change `SectionKind`.

Use domain modes for weapon magazines and reload:

```rust
pub enum AmmoCapacity {
    Unlimited,
    Limited(u32),
}

pub enum ReloadConfig {
    Disabled,
    Batch(SectionReloadConfig),
}
```

Use these domain types in the complete weapon configs too, replacing the current
`Option<u32>` and `Option<SectionReloadConfig>` fields.

```rust
pub struct MuzzleConfig {
    pub id: String,
    pub fire_rate: f32,
    pub muzzle_effect: Option<AssetRef<EffectAsset>>,
}

pub struct MuzzleConfigPatch {
    pub fire_rate: Option<f32>,
}

pub struct TurretSectionConfigPatch {
    pub muzzle_speed: Option<MetersPerSecond>,
    pub projectile_lifetime: Option<f32>,
    pub bullet_damage: Option<f32>,
    pub bullet_kind: Option<DamageType>,
    pub ammunition: Option<AmmoCapacity>,
    pub reload: Option<ReloadConfig>,
    pub muzzles: Option<BTreeMap<String, MuzzleConfigPatch>>,
}

pub struct TorpedoSectionConfigPatch {
    pub fire_rate: Option<f32>,
    pub spawner_speed: Option<MetersPerSecond>,
    pub projectile_lifetime: Option<f32>,
    pub arm_time: Option<f32>,
    pub arm_distance: Option<Meters>,
    pub nav_constant: Option<f32>,
    pub blast_radius: Option<Meters>,
    pub blast_damage: Option<f32>,
    pub ammunition: Option<AmmoCapacity>,
    pub reload: Option<ReloadConfig>,
}

pub struct RailgunSectionConfigPatch {
    pub charge_seconds: Option<f32>,
    pub slug_speed: Option<MetersPerSecond>,
    pub slug_damage: Option<f32>,
    pub slug_power: Option<f32>,
    pub rake_radius: Option<Option<Meters>>,
    pub slug_lifetime: Option<f32>,
    pub recoil_impulse: Option<f32>,
    pub ammunition: Option<AmmoCapacity>,
    pub reload: Option<ReloadConfig>,
}
```

The field list follows the curated scenario editor in
`crates/nova_editor/src/inspect.rs`. Keep the patch API small when complete
section configs gain art or implementation fields.

## Patch resolution

Resolve once before preload, lint, preview, balance, or runtime spawn:

```text
resolve ShipDesignSource
  -> clone the prototype or use the inline design
  -> for Prototype, find each target by section ID
  -> apply position and rotation updates
  -> apply common SectionConfigPatch fields
  -> match and apply the SectionKindPatch
  -> locate turret muzzles by stable muzzle ID and patch fire_rate
  -> lint the complete resolved design
  -> preload final asset references
  -> spawn final components
```

Rules:

- A missing patch field preserves the prototype value.
- A scalar `Some(value)` replaces the value.
- For an ordinary nullable field, outer `None` preserves, `Some(Some(value))`
  sets, and `Some(None)` clears.
- Domain enums represent meaningful modes instead of nullable values.
- Unknown section and muzzle IDs are lint errors.
- Duplicate muzzle IDs are lint errors.
- Do not merge vectors by index.
- Do not expose patches for link points, animations, joint trees, meshes,
  colliders, effects, or sounds.

## Runtime capability actions

Remove the generic controller-verb action. Add explicit root actions with `id`
and `enabled` fields:

```text
SetShipCapabilityStop
SetShipCapabilityGoto
SetShipCapabilityOrbit
SetShipCapabilityLock
SetShipCapabilityRcs
SetShipCapabilityPointDefense
```

The action implementations may share a private helper. The authored variants
remain explicit and update one `ShipCapabilities` bool on one scoped root.

## Controller and root ownership

Delete capability state and feedback configuration from controller sections.
Keep these controller fields:

```rust
pub struct ControllerSectionConfig {
    pub steering_lag: f32,
    pub max_torque: f32,
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    pub render_mesh_transform: Option<RenderMeshTransform>,
}
```

Keep the controller's runtime PD, stack tuning, target synchronization, and
rotation input. Move radar, safety, warning, ammo, hull, and RCS sounds to the
resolved root presentation state.

Replace capability checks with direct root bool reads. Keep physical checks
separate:

- rotation requires at least one live controller;
- main-drive maneuvers require usable drives;
- attitude-dependent autopilot disengages when attitude authority reaches zero;
- RCS remains a root COM impulse and reads `rcs_enabled`;
- targeting reads `lock_enabled`;
- point defense reads `point_defense_enabled`.

Stop deriving `SensorsDark` from controller survival. Sensor range and contacts
already live on roots. A future damageable sensor system needs its own physical
section; attitude controllers must not stand in for it.

## Editor integration

Inline sections continue to edit their complete config directly. A prototype
instance edits `SpaceshipSectionConfigPatch`.

The inspector must track:

```text
prototype value
current patch value
resolved displayed value
```

Behavior:

- inherited fields show the resolved prototype value;
- editing an inherited field creates that patch field;
- overridden fields show a small override marker;
- overridden fields offer Reset to inherited;
- reset removes the patch field instead of copying the current prototype value;
- a later prototype change reaches unpatched fields only.

Give shipped muzzle configs stable IDs such as `main`, `left`, and `right`.
Show twin-PDC rows with identity:

```text
Left Muzzle
  Fire Rate  50 /s

Right Muzzle
  Fire Rate  50 /s
```

Persist muzzle edits by ID, never by the current joint-child index. Share the
patchability schema and validation with the editor rather than duplicating the
field boundary in UI code.

## Delete first

Start the compiler-assisted refactor by deleting these old concepts rather than
adapting them:

- `FlightVerb`;
- `WithheldVerbs`;
- `LiveFlightComputers` as a capability query;
- `SectionModification` and every variant;
- modification marker components and apply-on-add observers;
- `ShipSectionModification`;
- `SpaceshipModifications`;
- `SpaceshipSectionConfig::modifications`;
- `SpaceshipConfig::modifications`;
- `SetControllerVerbActionConfig` and its event variant;
- controller-owned root feedback fields and components;
- old ship/hull catalog names listed above.

Add the accepted structs next. Then use compiler errors to update callers,
builders, tests, imports, preludes, actions, editor state, lint, preload, reports,
and generated content. Do not temporarily support both models.

## Migration and blast radius

Expected areas:

- `nova_ship`: controller config and systems, capability readers, autopilot,
  RCS, targeting, point defense, hints, audio, signatures, sensor darkness,
  weapon ammo configs, muzzle configs, section patch types and resolver seams.
- `nova_scenario`: design catalog/source types, spaceship spawn, action enum and
  actions, preload, lint, scenario object reflection and serialization.
- `nova_authoring`: ship and scenario builders, balance audit, tutorial gates,
  muzzle IDs, generated base content and parity tests.
- `nova_editor`: document model, prototype resolution, inspector rows, override
  state/reset, twin muzzle labels, save/load, preview, readout and probes.
- `nova_hud`, `nova_os_ui`, `nova_probe`, and examples: capability names,
  observations, snapshots, status and test fixtures.
- Documentation and changelog: mark the content format replacement as breaking.

Search every old type and runtime ID. Change Rust builders first. Regenerate
`assets/base/**/*.content.ron`; never hand-edit generated content.

## Verification

Add assertions that fail against the old model:

- two controllers stack attitude but never aggregate capabilities;
- losing one controller reduces attitude authority only;
- losing all controllers prevents rotation and disengages attitude-dependent
  autopilot;
- LOCK, RCS, and point-defense configuration survives controller loss;
- each explicit capability action changes one root and one bool;
- omitted capability config resolves all current capabilities enabled;
- section patch omission inherits the prototype value;
- editing and reset create and remove the patch field;
- position and rotation patches affect the resolved placement;
- wrong-kind and unknown-section patches fail lint;
- unknown and duplicate muzzle IDs fail lint;
- a twin PDC patches left and right rates by ID, not joint index;
- ammo domain modes preserve unlimited, limited, disabled reload, and batch
  reload behavior;
- runtime spawn, preload, balance, editor preview, and lint consume the same
  resolved design;
- generated RON is unchanged after a second generation pass.

Run affected crate tests and content generation/lint only. Add an asserted
editor scenario or probe for inherited, overridden, and reset fields. Inspect
its editor rows and saved output. Do not claim appearance from headless tests.
Do not run full workspace tests or Clippy unless requested.
