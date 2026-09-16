# Simplify authored content boundaries

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.14.0, refactor, content, modding

## Goal

Reduce `Content` to stable game-domain content. Move engine policy out of the
mod format. Simplify the remaining style schema without removing useful visual
control.

This is a breaking content-format change. Do not preserve compatibility with
the current unreleased formats.

## Claim

`Content` currently mixes durable game objects with engine lookup tables,
generator tuning, and HUD presentation presets. Impact audio, narrative
channels, and WFC grammars do not need independent mod-level identities.
`ShipStyleConfig` remains useful content, but its raw scatter and density
controls expose too much implementation detail.

## Evidence

- `Content` routes every variant through the common merge and registry path:
  `crates/nova_modding/src/lib.rs:76`.
- Impact rows use open material strings even though damage kinds are closed:
  `crates/nova_gameplay/src/impact_sound.rs:67`.
- Five asteroid impact rows select the same rock sound:
  `assets/base/impacts/base.content.ron`.
- Narrative channels only carry presentation fields:
  `crates/nova_gameplay/src/narrative_channel.rs:52`.
- All 22 current base narrative cues name the `comms` channel:
  `assets/base/scenarios/tutorial.content.ron`.
- The editor already owns WFC section selection, zone overrides, role
  selection, and grid growth: `crates/nova_editor/src/generate.rs:153` and
  `crates/nova_editor/src/generate.rs:257`.
- `wfc_arena` already modifies the shared grammar for its own rules:
  `examples/playable/wfc_arena.rs:852`.
- `ScatterRule` exposes 14 interacting controls:
  `crates/nova_ship/src/sections/skin_style.rs:224`.
- The shipped styles contain 54 fixtures and hundreds of explicit scatter
  values in `crates/nova_authoring/src/base_content/styles.rs`.
- Sections use density `1.0`, while shell plates use `0.25`:
  `crates/nova_ship/src/sections/base_section.rs:567` and
  `crates/nova_ship/src/sections/shell_skin.rs:96`.

## Decided content boundary

The resulting externally tagged content enum is:

```rust
pub enum Content {
    Section(Box<SectionConfig>),
    Scenario(ScenarioConfig),
    Campaign(CampaignConfig),
    Style(ShipStyleConfig),
    Ship(ShipDesignPrototype),
}
```

Remove `Content::Impact`, `Content::Channel`, and `Content::Grammar`, including
their merge arms, registries, lint walks, generated files, builders, and tests.

## Impact audio

Impact audio is engine-owned. Both axes are closed enums. Keep `DamageType` and
replace open material strings with a closed target classification:

```rust
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactSurface {
    Hull,
    Rock,
}
```

Use an engine-owned sound bank:

```rust
#[derive(Resource)]
pub struct ImpactSounds {
    pub kinetic_hull: Handle<AudioSource>,
    pub kinetic_rock: Handle<AudioSource>,
    pub pierce: Handle<AudioSource>,
    pub explosive: Handle<AudioSource>,
}
```

Resolution is exhaustive:

```rust
match (damage, surface) {
    (DamageType::Kinetic, ImpactSurface::Rock) => kinetic_rock,
    (DamageType::Kinetic, ImpactSurface::Hull) => kinetic_hull,
    (DamageType::Pierce, _) => pierce,
    (DamageType::Explosive, _) => explosive,
}
```

- Load the four fixed sound paths in the engine plugin.
- Remove `ImpactSoundConfig`, `GameImpacts`, and the impact content file.
- Remove `BaseSectionConfig::material`; sections are always `Hull`.
- Asteroids and planets always carry `ImpactSurface::Rock`.
- Keep asteroid visual kinds separate from impact classification.
- Rename asteroid configuration field `material` to `kind`.
- Mods may override sound assets but cannot add impact categories.

## Narrative cue presentation

Remove channel IDs, `NarrativeChannelConfig`, `GameChannels`, channel content,
channel lookup, channel lint, and fallback behavior.

Put one optional visual accent directly on each cue:

```rust
pub struct NarrativeCueActionConfig {
    pub speaker: String,
    pub text: String,
    pub dwell: Option<f32>,
    pub icon: Option<AssetRef<Image>>,
    #[serde(default = "default_comms_accent")]
    pub accent: Color,
}
```

- The default accent is the current comms blue.
- Speaker text and the fallback icon use the accent.
- The border uses the accent with the standard border alpha.
- Body text uses the accent lifted toward white.
- Every card uses the same standard dark background.
- Card opacity depends only on normal display and fade behavior.
- Remove channel tags and per-channel signal strength.
- `StoryLine` carries the resolved accent color directly.

Ordinary cue:

```ron
NarrativeCue((
    speaker: "Alpha",
    text: "Strip it clean.",
))
```

Custom accent:

```ron
NarrativeCue((
    speaker: "Meridian Control",
    text: "Unknown vessel, alter course.",
    accent: LinearRgba((
        red: 1.0,
        green: 0.55,
        blue: 0.2,
        alpha: 1.0,
    )),
))
```

## WFC ownership

WFC policy is editor-owned code. Do not add WFC metadata to prototype or inline
sections. A section describes geometry and capability; suitability for one
generator is generator policy.

- Remove serialized grammar catalogs, grammar IDs, and `GameGrammars`.
- Move remaining algorithm-input types from `nova_ship` to `nova_wfc`.
- Rename `Grammar*` types to `Wfc*`; they are not authored game content.
- Keep an algorithm input such as `WfcPlan` with grid, vacuum, keel, and part
  data.
- The editor constructs its plan from code defaults and current UI rows.
- The editor continues to list every merged section.
- Mod sections appear automatically and start unticked.
- The editor owns section selection, zones, role selection, and grid growth.
- `wfc_arena` and other examples construct their own plans and may diverge from
  the editor without changing content schemas.

Representative internal input:

```rust
pub struct WfcPlan {
    pub grid: WfcGrid,
    pub vacuum: WfcVacuum,
    pub keel: WfcKeel,
    pub parts: Vec<WfcPart>,
}

pub struct WfcPart {
    pub prototype: String,
    pub weight: f32,
    pub aim: Option<WfcAim>,
    pub zone: Option<WfcZone>,
}
```

These fields may remain algorithm-specific because they are code-owned rather
than exposed as mod content.

## Simplified ship styles

Keep `Style` as content. Replace the surface list with an explicit palette and
replace raw scatter controls with a small authored vocabulary.

```rust
pub struct ShipStyleConfig {
    pub id: String,
    pub name: String,
    pub palette: StylePalette,
    pub fixtures: Vec<StyleFixtureConfig>,
}

pub struct StylePalette {
    pub top: SurfaceFinish,
    pub wall: SurfaceFinish,
}

pub struct SurfaceFinish {
    pub color: Color,
    pub roughness: f32,
    pub metallic: f32,
}
```

`top` is the shell face exposed to space and normally visible. `wall` is the
shell side exposed at slopes, gaps, and edges. The hidden `floor` against ship
structure keeps an engine-defined finish.

Fixtures remain ordered by priority. The first eligible fixture claims a plate:

```rust
pub struct StyleFixtureConfig {
    pub id: String,
    pub model: AssetRef<WorldAsset>,
    pub health: f32,
    pub collider: Vec3,
    pub placement: FixturePlacement,
}

pub struct FixturePlacement {
    pub region: FixtureRegion,
    pub density: FixtureDensity,
    pub orientation: FixtureOrientation,
}

pub enum FixtureRegion {
    Panel,
    Deck,
    Flank,
    Edge,
    HighGround,
    NearFitting,
    Anywhere,
}

pub enum FixtureDensity {
    Rare,
    Sparse,
    Regular,
    Dense,
    Every,
}

pub enum FixtureOrientation {
    Free,
    Along,
    Outward,
}
```

- Regions express visual intent rather than plate-reading implementation.
- Density values expand internally to filters, stride, chance, and patch rules.
- Orientation controls fixture rotation on the chosen plate.
- Keep fixture health authored because it controls when that fixture is shot
  off and cannot be inferred reliably from its volume.
- Keep collider dimensions authored because headless simulation does not load
  the model from which bounds could be calculated.
- Remove authored fixture physics density.
- Use engine-owned `DECOR_DENSITY = 0.25`, matching shell cladding.

Representative migrated style:

```ron
Style((
    id: "industrial",
    name: "Industrial",
    palette: (
        top: (
            color: LinearRgba((
                red: 0.2,
                green: 0.19,
                blue: 0.168,
                alpha: 1.0,
            )),
            roughness: 0.9,
            metallic: 0.15,
        ),
        wall: (
            color: LinearRgba((
                red: 0.03,
                green: 0.028,
                blue: 0.024,
                alpha: 1.0,
            )),
            roughness: 0.95,
            metallic: 0.1,
        ),
    ),
    fixtures: [
        (
            id: "industrial_stack",
            model: "self://gltf/greebles/industrial_stack.glb#Scene0",
            health: 10.0,
            collider: (0.18, 0.28, 0.18),
            placement: (
                region: HighGround,
                density: Sparse,
                orientation: Free,
            ),
        ),
    ],
))
```

## Blast radius

- `nova_modding`: content enum, serialization, parser tests.
- `nova_assets`: merge outcome, resources, load gates, reference rewriting.
- `nova_authoring`: builders, generated file inventory, parity tests, lint.
- `nova_gameplay`: impact surface and sound bank, narrative presentation.
- `nova_scenario`: asteroid field rename, cue schema, lint and synchronization.
- `nova_ship`: section material removal, style schema, private scatter mapping,
  fixture density, impact audio consumer.
- `nova_wfc`: ownership and naming of WFC input types.
- `nova_editor`: editor-owned WFC plan and style previews.
- `nova_hud`: accent-derived comms rendering.
- Examples, test fixtures, docs, generated RON, and changelog.

## Implementation order

1. Add the simplified style types and private placement expansion.
2. Migrate Rust style builders, regenerate RON, and inspect generated output.
3. Move WFC input ownership and make the editor/examples construct plans.
4. Replace narrative channels with cue accents.
5. Replace impact content with closed engine types and fixed sounds.
6. Remove obsolete content variants, registries, generated files, and lint.
7. Rename asteroid `material` to `kind` and regenerate scenario content.
8. Update affected documentation and add one breaking changelog entry.

## Verification

- Add focused unit tests for every placement region, density, and orientation
  expansion.
- Preserve deterministic fixture placement and priority behavior.
- Assert fixture health is authored and fixture density is always `0.25`.
- Assert default narrative cues render with the current comms blue.
- Assert a custom accent drives speaker, icon, border, and body derivation.
- Assert every `(DamageType, ImpactSurface)` pair resolves explicitly.
- Assert sections resolve to `Hull` and asteroids/planets resolve to `Rock`.
- Test editor generation with base sections and an otherwise unknown mod
  section; the mod section must appear unticked and be selectable.
- Run affected content generation and parity tests.
- Run affected content lint and load-gate tests.
- Run focused `nova_wfc`, editor generation, impact audio, comms panel, skin
  scatter, and skin spawn tests.
- Inspect regenerated style, scenario, and bundle RON rather than relying only
  on test exit status.
- Use a rendered editor or example run for final style appearance; headless
  tests do not prove palette or fixture placement appearance.
