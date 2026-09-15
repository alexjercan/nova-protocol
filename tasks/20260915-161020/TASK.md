# Replace section modifiers with root capabilities and typed design patches

- STATUS: CLOSED
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
- Section patches belong to every `Prototype` reference, at both levels. A
  prototype ship design accepts `section_patches` keyed by section ID. A
  prototype SECTION reference accepts one `patch`. Inline designs and inline
  sections are already complete and accept none.
- Patch layers apply outward and the outer layer wins: the catalog section
  prototype, then the design's own section patch, then the spawn's
  `section_patches`. This is the precedence the spawn modifications already
  have (`crates/nova_scenario/src/objects/spaceship.rs:407`).
- Patches are a curated Update surface, not generic serialized-value merging.
- Every patch SLOT is a value, never an `Option`. A patch struct defaults to
  all-inherit and a patch map defaults to empty, both meaning "no change", and
  serde omits them. `Option` appears only INSIDE a patch, on a scalar or enum
  field, where it carries the inherit-or-set meaning. Absent and empty would
  otherwise mean the same thing, which is the repo's test for whether `Option`
  earns its place.
- Patch gameplay and editor-facing values. Do not patch art, colliders, sockets,
  animations, section kind, section source, or turret joint topology.
- Position and rotation are patchable.
- Use domain enums for meaningful modes. Use `Option<Option<T>>` for ordinary
  nullable patch fields where that is simpler.
- Turret fire rate stays per muzzle. Add stable muzzle IDs and patch only muzzle
  gameplay data, not the joint tree.
- Root feedback audio and alarms live in the ship design presentation config.
- This REVERSES the v0.6.0 modification model (`tasks/20260714-113411`), which
  chose an open enum plus one component and observer per variant so a new
  delta needed no central match. Typed patches cost a field per patchable
  value and will drift from the complete configs. We take that cost: an open
  enum cannot tell the inspector which fields are patchable, so it cannot show
  inherited, overridden, and reset state, and the observer model already pays
  for its openness in accidental complexity (two identical observers for one
  `SetAmmo` at `crates/nova_scenario/src/objects/modification.rs:140` and
  `:160`, hand-accumulated `WithheldVerbs` in `insert_all`). Changing our mind
  here is deliberate, not drift.
- Dropping `SensorsDark` is a player-visible BEHAVIOR change, not a rename: a
  ship that loses every controller keeps its radar and contacts. It follows
  from the decision that a controller is an attitude sensor and not the ship's
  brain. It needs its own changelog line, not just the format-break line.

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
        section_patches: BTreeMap<SectionId, SpaceshipSectionConfigPatch>,
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

Rust names only. The authored content kind stays `Ship(..)` in RON and `"ship"`
on the wire (`crates/nova_modding/src/lib.rs:97`, `:130`). Renaming that would
break every mod file for no gain, and it is the one thing the sweep must not
touch.

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
```

A prototype SECTION reference carries its own patch, so a design can tune a
catalog section without inlining it. The ledger's ships need this: 22 sections
in `webmods/the-ledger/ledger_ships.content.ron` are `source: Prototype(..)`
with `modifications: [SetHealth(..)]` INSIDE the ship design, where the
spawn-level `section_patches` map cannot reach them. Without it those sections
must be inlined whole, which loses exactly the prototype inheritance this task
adds.

```rust
pub enum SectionSource {
    Inline(SectionConfig),
    Prototype {
        id: SectionId,
        patch: SectionConfigPatch,
    },
}
```

Position and rotation are authored fields of `SpaceshipSectionConfig`, so this
patch is `SectionConfigPatch` and not `SpaceshipSectionConfigPatch`.

Authored form. A patched section:

```ron
source: Prototype(
    id: "cargoa_engine_starboard",
    patch: (
        health: Some(90.0),
    ),
),
```

An unpatched one omits the slot entirely:

```ron
source: Prototype(
    id: "hull_light",
),
```

```rust

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
    pub muzzles: BTreeMap<String, MuzzleConfigPatch>,
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
  -> for each section, resolve SectionSource
       -> clone the section prototype or use the inline config
       -> apply the section reference's own patch
  -> for a Prototype design, find each target by section ID
  -> apply position and rotation updates
  -> apply common SectionConfigPatch fields
  -> match and apply the SectionKindPatch
  -> locate turret muzzles by stable muzzle ID and patch fire_rate
  -> lint the complete resolved design
  -> preload final asset references
  -> spawn final components
```

Rules:

- The two patch layers use one `SectionConfigPatch` type and one apply
  function. The spawn layer runs last and wins field by field.
- An omitted patch slot is the empty patch and changes nothing. Resolution
  never distinguishes absent from empty.
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

## Retiring FlightVerb

The enum has two jobs and only one of them is a capability. Both leave.

- `crates/nova_ship/src/input/player/hints.rs`: `FlightVerbHints` is already a
  struct of named fields (`stop`, `goto`, `orbit`, `cancel`, `radar`, `rcs`,
  `component_cycle`), so the enum appears only as the argument to
  `verb_granted(FlightVerb::Stop)` at `:149`. Each call becomes a root
  `ShipCapabilities` bool read. The struct shape does not change.
- `crates/nova_hud/src/situation.rs:32`: `HudSituations::maneuver` is MANEUVER
  identity - which dock chip is lit - not a capability. `maneuver_verb` at
  `:124` already derives it from `AutopilotAction`, mapping four actions onto
  three chips with `MatchVelocity -> None`.

Replace it with a HUD-side discriminant, three variants where the old enum had
six, owned by the layer whose policy it is:

```rust
pub enum ManeuverChip {
    Stop,
    Goto,
    Orbit,
}
```

`maneuver_verb` becomes `maneuver_chip` with the same match arms.
`crates/nova_hud/src/keybind_dock.rs:471` compares against it unchanged, and
`flight_status.rs:433` only asks `is_some()`.

Rejected: `maneuver: Option<AutopilotAction>` directly. It compiles -
`AutopilotAction` is `Copy + PartialEq` at `flight/state.rs:170` - but
`HudSituations` is compared WHOLE for change detection and for
`idle_cruise()`, so `Orbit { plan }` flipping `None -> Some(OrbitPlan)` on the
first engaged tick would dirty the HUD resource over a payload it does not
read, and `Goto { target: Entity }` would make every dock test fabricate an
entity it has no use for (8 call sites in `keybind_dock.rs` and
`flight_status.rs`).

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
- `nova_hud`, `nova_os_ui`, `nova_probe`: capability names, observations,
  snapshots, status and test fixtures, plus the `ManeuverChip` swap above
  (`situation.rs:32`, `:124`, `keybind_dock.rs:471`, `flight_status.rs:433`).
- `nova_wfc`: `place`, `hull_errors` and `TileSet::hull` are typed on
  `ShipHull`/`ShipSource` and build `SpaceshipSectionConfig` values
  (`crates/nova_wfc/src/check.rs:56`, `:260`, `src/lib.rs:299`).
- `nova_assets`: the mod merge inserts `GameShips`
  (`crates/nova_assets/src/merge.rs:420`).
- `nova_modding`: `Content::Ship(ShipConfig)` and the `"ship"` wire kind
  (`crates/nova_modding/src/lib.rs:97`, `:130`).
- 74 files under `examples/`.

Hand-written RON that the generator does NOT cover, each edited by hand:

- `assets/mods/example/example.content.ron`;
- `webmods/gauntlet/gauntlet.content.ron`;
- `webmods/the-ledger/*.content.ron` (8 files, including the 22 design-level
  section modifications above);
- `crates/nova_bench/scenarios/{hunt,arsenal,range,slingshot}.content.ron`.

The bench fixtures need care: editing them changes the fixture revision, so
performance sets measured against the old files stop being matched
comparisons. Re-baseline after the refactor lands, or state the break.

Docs that carry the old vocabulary:

- `docs/project-tour.md:36`, `docs/architecture.md:20`,
  `docs/concept-index.md:35,42`, `docs/sections.md:280`,
  `docs/automation-harness.md:87`;
- `web/src/create/ships.md:55,85,93`, `web/src/create/objects.md:216,243,364`,
  `web/src/create/actions.md:30,395,790`, `web/src/create/reference.md:46,81`,
  `web/src/wiki/sections/hull.md:70`, `web/src/docs-manifest.js:769`.

Changelog: mark the content format replacement **(breaking)**, and give the
`SensorsDark` removal its own entry.

Search every old type and runtime ID. Change Rust builders first. Regenerate
`assets/base/**/*.content.ron`; never hand-edit generated content.

Measured 2026-09-15: 133 Rust files and 667 references to the deleted names.

## Verification

Add assertions that fail against the old model:

- two controllers stack attitude but never aggregate capabilities;
- losing one controller reduces attitude authority only;
- losing all controllers prevents rotation and disengages attitude-dependent
  autopilot;
- LOCK, RCS, and point-defense configuration survives controller loss;
- each explicit capability action changes one root and one bool;
- the dock lights the engaged maneuver's own chip with no `FlightVerb` left in
  the tree, and an idle `HudSituations` still compares equal to its default;
- omitted capability config resolves all current capabilities enabled;
- section patch omission inherits the prototype value;
- a design's own section patch tunes a catalog section without inlining it,
  and a later catalog change still reaches that section's unpatched fields;
- a spawn `section_patches` entry and a design section patch on the same field
  resolve to the spawn's value;
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
