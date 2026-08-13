# Link-point integrity notes

## Accepted design

### Ownership

`nova_ship` owns the ship-specific structural authoring and derivation:

- `LinkPoint`
- `SectionLinkPoints`
- `BaseSectionConfig::link_points`
- pure link-point mate derivation
- ship `ConnectedTo` construction

`nova_gameplay` remains the generic integrity engine. It owns:

- `ConnectedTo`
- `IntegrityRoot`
- integrity lifecycle and graph consumption
- lone non-ship body initialization

Move the ship graph observer and its ship-specific tests from
`nova_gameplay::integrity::glue` into `nova_ship`. Scenario lint, NOVA OS, and
the future editor snapping path consume the public link-point model from
`nova_ship`.

Why:

- Preserves the intended dependency direction: `nova_ship -> nova_gameplay`.
- Keeps ship authoring vocabulary out of generic gameplay.
- Gives runtime, lint, NOVA OS, and editor one model owner.
- Corrects the existing boundary leak where gameplay integrity glue knows ship
  markers and section layout.

### Link-point schema and tolerances

```rust
pub struct LinkPoint {
    pub id: String,
    pub position: Vec3,
    pub normal: Vec3,
}
```

- Positions and normals are section-local.
- IDs are required and unique within one section prototype.
- IDs identify sockets for diagnostics and UI. They do not define mating
  compatibility.
- Authored normals must be finite, nonzero, and unit length within `1e-4`.
- Runtime normalizes valid normals again as a numerical safeguard.
- Mate tolerances are fixed public constants, not authored fields:
  - `LINK_POINT_POSITION_EPSILON = 1e-3`
  - `LINK_POINT_NORMAL_MIN_DOT = 0.999`
- Two transformed points mate when their distance is at most the position
  epsilon and `dot(a.normal, -b.normal)` is at least the normal threshold.
- Built-in cube point IDs use `positive_x`, `negative_x`, `positive_y`,
  `negative_y`, `positive_z`, and `negative_z`.

### Derivation identity

The shared derivation returns socket-level mates, not only section edges:

```rust
pub struct LinkPointRef {
    pub section_index: usize,
    pub link_point_index: usize,
}

pub struct LinkPointMate {
    pub a: LinkPointRef,
    pub b: LinkPointRef,
}
```

- `section_index` addresses the section in the derivation input ship.
- `link_point_index` addresses the point in that section's `link_points` list.
- These indices exist only for one derivation call. They are not authored or
  persisted.
- Runtime maps section indices to entities. Lint maps them to authored section
  IDs. NOVA OS and the future editor can resolve the exact socket positions.
- Consumers deduplicate socket mates into normalized section edges before
  building `ConnectedTo`. Multiple independent socket mates between the same
  two large sections still produce one structural section edge.

### Invalid graph behavior

Derivation is all-or-nothing:

```rust
Result<Vec<LinkPointMate>, Vec<LinkPointGraphError>>
```

- Any local validation, ambiguity, or graph-connectivity error invalidates the
  whole ship graph.
- Runtime logs every error and inserts an empty `ConnectedTo` list on every
  section.
- Runtime never publishes valid partial mates and never falls back to distance
  or AABB adjacency.
- Normal authored scenarios do not reach this fallback. Error-level content
  findings make `on_load_scenario` refuse the scenario before teardown or
  spawn.
- The empty-graph path exists for direct ECS/programmatic spawns, tests, and
  validation gaps. It keeps malformed behavior deterministic without hiding an
  authoring error behind a plausible partial graph.

### Explicit integrity structure initialization

Structures declare their integrity roles instead of relying on a generic
`ColliderOf` observer:

- `SpaceshipRootMarker` requires `IntegrityRoot`.
- `base_section` inserts an initial empty `ConnectedTo` node.
- `AsteroidMarker` requires `IntegrityRoot`.
- The asteroid collider child inserts an empty `ConnectedTo` node.
- The ship mate observer replaces initial section lists with the derived graph.
- A single section and a lone asteroid naturally retain an empty list.
- Remove gameplay's implicit graph/root initialization. Other destructible
  structures must explicitly declare their roots and nodes.

`IntegrityRoot` identifies the entity that owns one destructible structure; it
is not itself a graph node. Section and asteroid collider children carry
`ConnectedTo`. Both `SpaceshipRootMarker` and `AsteroidMarker` require
`IntegrityRoot`, so callers cannot create partially initialized roots.

`ColliderOf` remains the ship graph timing seam because Avian adds it after all
section children exist. It no longer creates generic integrity structures.

### Empty default and unit-cube points

- `BaseSectionConfig::link_points` is serde-defaulted and omitted when empty.
- Empty means no sockets. There is no implicit unit-cube behavior.
- `unit_cube_link_points()` returns six explicitly authored face-center sockets
  at `+/-0.5` with outward axis normals.
- Built-in cube constructors use that helper. Generated RON contains the six
  points.
- Socket positions are independent of collider bounds. Existing cut-cube
  prototypes with a `0.8` collider still use `+/-0.5` sockets to preserve the
  one-unit structural grid.
- Both `base_section` and `preview_section` snapshot authored points as
  `SectionLinkPoints`. Only live sections receive `ConnectedTo`; previews remain
  outside integrity while exposing sockets for future editor snapping.
- Old third-party multi-section content without points fails connectivity lint.
  A single-section structure can validly have no points.

### Lint prototype catalog

Scenario lint keeps a focused resolved snapshot rather than full section
configs:

```rust
pub struct KnownSection {
    pub mounts: bool,
    pub link_points: Vec<LinkPoint>,
}

pub struct KnownSections {
    entries: HashMap<String, KnownSection>,
}
```

- `KnownSections::from_configs` uses last-wins section-ID precedence in iterator
  order, matching cross-bundle content overlay.
- Static authoring lint supplies configs in `base -> dependencies -> owning
  bundle` order. The current `base -> owner -> dependencies` order must be
  corrected.
- Runtime lint receives the already merged unique catalog.
- Prototype graph lint resolves points and mount classification from one entry.
- Inline sections use their own `BaseSectionConfig::link_points`.
- The snapshot does not retain unrelated meshes, sounds, or weapon settings.

### Section transform validity

- Link points support arbitrary valid rotations; quarter turns are not required.
- Derivation rejects non-finite section positions and rotations, zero rotation
  quaternions, and quaternions whose length differs from one by more than
  `1e-4`.
- Runtime normalizes a valid quaternion again before transforming points.
- It never repairs a malformed graph rotation differently from the authored
  entity transform.
- Unmatched sockets are valid. Exterior and optional sockets do not need mates.

### Public API and ship integrity plugin

`nova_ship::sections::link_points` exposes:

- `LinkPoint`
- `SectionLinkPoints`
- `PlacedSectionLinkPoints`
- `LinkPointRef { section_index, link_point_index }`
- `LinkPointMate`
- `LinkPointGraphError`
- `derive_link_point_graph`
- `unit_cube_link_points`
- `LINK_POINT_POSITION_EPSILON`
- `LINK_POINT_NORMAL_MIN_DOT`

`ShipIntegrityPlugin` observes Avian `Add<ColliderOf>`, collects all live
sections under the spaceship root, derives mates, and publishes deterministic
symmetric `ConnectedTo` lists. Repeated collider events rebuild idempotently.

Move all ship-specific integrity glue from `nova_gameplay` into `nova_ship`:

- link-point graph construction;
- aggregate ship health;
- section disable behavior.

Delete `nova_gameplay::integrity::glue`. Generic gameplay retains the integrity
components and lifecycle that consume a declared graph.

No persistent mate component is needed. `ConnectedTo` remains runtime
structural truth. NOVA OS can re-derive socket-level mates from current section
transforms and `SectionLinkPoints`, avoiding stale references after section
destruction.

### Structured derivation errors

`LinkPointGraphError` has structured variants for:

- non-finite section position;
- non-finite section rotation;
- non-unit section rotation;
- empty link-point ID;
- duplicate link-point ID, with first and duplicate references;
- non-finite link-point position;
- non-finite link-point normal;
- zero link-point normal;
- non-unit link-point normal;
- ambiguous mate, with the socket and all candidate socket references;
- disconnected graph, with all connected components.

Errors carry input indices rather than copied authored values. Callers use the
original input to produce contextual names and values. Candidate and component
indices are sorted for deterministic diagnostics. Local validation collects all
independent errors before returning; mating does not run after local errors, and
connectivity does not run after ambiguity errors. Zero-section and one-section
inputs return `Ok`. The error type derives `Clone`, `Debug`, and `PartialEq`.

## Implementation notes

- Moving the ship adapter exposed a deferred-command race between root physical
  destruction and defeat-marker insertion. Both destruction and neutralization
  reactions now use `try_insert`; lifecycle ordering and exact-once event tests
  remain green.
- A Raid playtest exposed false disconnected-graph errors while Avian added
  `ColliderOf` one section at a time. The graph itself became valid when later
  bridge sections arrived, but each partial rebuild logged an error and
  temporarily published empty neighbors. Graph publication now runs from
  `Added<SectionLinkPoints>` once per affected root in `Update`, after the full
  section spawn command batch lands. A bridge-spawned-last regression pins this
  behavior.
- Existing cube prototypes explicitly serialize all six points. The example mod
  overrides `reinforced_hull_section`, so its override also had to repeat the
  sockets; complete prototype overlays do not inherit fields from base.
- NOVA OS derives a default-off mate overlay from live section snapshots. `G`
  toggles it. The overlay stores no persistent graph copy.
- The existing `screenshot_nova_os` harness was attempted twice on DISPLAY `:0`.
  The non-debug run is inert by design; the debug run spawned the range but did
  not advance its first harness step before the command timeout, so no rendered
  PNG was produced. Unit-level mesh derivation and the existing rendered ship
  app path are covered, but visual overlay inspection remains pending.

## Verification

- `nix develop --command cargo check`
- `nix develop --command cargo test --lib -p nova_ship` - 444 passed before the
  post-review spawn-batch regression; the new regression passes separately
- `nix develop --command cargo test --lib -p nova_scenario` - 162 passed before
  the added focused lint regression; that regression passes separately
- `nix develop --command cargo test --lib -p nova_authoring` - 46 passed
- `nix develop --command cargo test --lib -p nova_os_ui ship::` - 22 passed
- Built-in old-distance/new-link-point parity test passes
- `nix develop --command cargo run content -- lint` - 0 errors, 0 warnings
- `nix develop --command cargo fmt --check`
- `git diff --check`
- `cd web && npm run ci`
