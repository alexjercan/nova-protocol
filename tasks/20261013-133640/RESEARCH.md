# Research: greeble placement as a seeded solve on the hull surface

Task: `tasks/20261013-133640/TASK.md`. Consumer: `tasks/20261013-133643/TASK.md`
(plating reshapes to fit the greeble). Research only. No committed code changed.
Diagrams: `RESEARCH.html` in this folder (cell graph across a fold, socket
matching, solver loop).

This note replaces the rejected note of 0a46d2f95. The owner's direction
(TASK.md, 2026-10-14): greebles get link-point-like sockets, the outer shell is
a 2D surface, a deterministic seeded WFC-style solver places greebles and empty
cells, rules may look past direct neighbours, and the language is a technical
solver configuration.

Labels: **[code]** read from the source at a0fd0baf3. **[proto]** measured with
the throwaway prototype in section 8. **[web]** checked on the web during this
research. **[knowledge]** from memory, not checked. **[unverified]** nobody
measured it.

## Accepted language (owner, 2026-10-14)

This section overrides the candidate RON in section 3. The owner rejected the
plate filters and socket keywords there (`on`, `falls`, `out`, `clear`,
`within`, `count`) and any attempt to keep today's look. Ships may look
different after the change.

- A greeble has four link points, `up`, `right`, `down`, `left`, in the cell
  frame. Each names a link type.
- Link types are ids declared by the style, each with `connects`: the link
  types it may face. Lint requires `connects` to be symmetric and every id to
  be known.
- Empty is a greeble with `fixture: None`. It takes part in the solve.
- `rotate: All | Half | None` and `weight` are required.
- `near: [(greebles, radius, factor)]` is optional (absent = no range rules).
  `radius` is graph distance in cells. Factor 0 is a hard ban; any other
  factor scales the weight.
- `max_count` is optional (absent = no cap).
- The solver reads no plate geometry: no edge, corner, normal or seat
  filters. The shell is a graph with one cell per plate. A side with no
  neighbour accepts any link type.
- Kept from the note: one cell per plate; seed hashed from structure plus
  style id; min-entropy choice with a hash tie-break; restart up to 8 times,
  then a logged fallback; mirror off; lint refuses a factor-0 `near` term on a
  greeble that a link type can force (a link type whose `connects` excludes
  the empty greeble's types).

```ron
greeble_rules: (
    link_types: [
        (id: "any",    connects: ["any", "empty"]),
        (id: "empty",  connects: ["any", "empty", "spaced"]),
        (id: "spaced", connects: ["empty"]),
        (id: "line",   connects: ["line"]),
    ],
    greebles: [
        (id: "empty", fixture: None, weight: 6.0, rotate: None,
         link_points: (up: "empty", right: "empty", down: "empty", left: "empty")),
        (id: "hazard_line", fixture: Some("industrial_hazard_band"), weight: 8.0, rotate: All,
         link_points: (up: "line", right: "any", down: "line", left: "any")),
        (id: "hazard_line_end", fixture: Some("industrial_hazard_band_end"), weight: 1.0, rotate: All,
         link_points: (up: "line", right: "any", down: "any", left: "any")),
        (id: "crane", fixture: Some("industrial_crane"), weight: 0.6, rotate: All,
         link_points: (up: "spaced", right: "spaced", down: "spaced", left: "spaced"),
         near: [(greebles: ["crane"], radius: 3, factor: 0.0)],
         max_count: 3),
    ],
)
```

## 0. Summary

- **Domain.** One solver cell per derived plate. A plate already is one quad
  of outer surface (`derive_skin`, `shell_skin.rs:263`). Each cell has 4 edge
  slots. Across each slot the derivation already knows what lies there; the
  graph keeps that answer instead of discarding it. The result is a quad graph
  with borders and some 3-valent corners, like an irregular Townscaper grid.
- **Language.** Three candidates: A, edge sockets plus a closed list of hard
  constraints (DeBroglie style); B, authored window patterns (overlapping-model
  style); C, edge sockets for radius 1 plus `near` range terms that are soft
  weights, or hard bans when the factor is 0.
- **Solver.** A small AC-3 + min-entropy solver with counter-based hashing. The
  prototype solves real hulls (42 to 206 plates) in 15 to 192 us per ship,
  release build. Over 13 hulls x 1000 seeds it needed 0 restarts. With one
  hard radius-2 rule that can hit a forced cell, 0 to 0.5% of solves needed
  one restart. The result is the same for the same seed. Removing one section
  keeps the tile on 93 to 100% of the plates the edit did not change.
- **Recommendation.** Candidate C. First slice: the surface graph and the
  solver as pure functions in `nova_ship` with unit tests, no wiring.

## 1. What exists today

### The decoration pipeline [code]

```
read_structure(placed)              shell_skin.rs:1053  cells, sockets, exits
derive_skin(structure)              shell_skin.rs:263   one SkinPlate per clad cell
read_plates(structure, plates)      skin_reading.rs:300 PlateReading per plate
scatter_decor(plates, readings, st) skin_decor.rs:129   <- replaced by the solve
decor_pose(plate, turns)            skin_decor.rs:530   bed on seat_normal, then yaw
spawn decor ChildOf(plate)          shell_skin.rs:979-985
```

- `FixturePlacement { region, density, orientation }` (`skin_style.rs:200`)
  expands into a private scatter rule. A plate takes at most one piece; the
  first fixture in the list wins (`skin_style.rs:219-236`). The share is a
  hash of cell, out face and fixture id (`skin_decor.rs:237`, `:498`).
- The module states "There is no RNG anywhere in this module"
  (`skin_decor.rs:11`). Decoration is a pure function of structure. The editor
  re-derives the whole skin and decoration on every structure change
  (`crates/nova_editor/src/skin.rs:74`, `:147`, `:182`). The live ship derives
  once, when sections are added to a clad root (`shell_skin.rs:886-1000`).
- A frozen wreck stores each plate's `ShellShape` and each greeble's model,
  pose and health (`frozen.rs:57-73`). It does not re-run placement on thaw.

### What dies and what stays

| dies | stays |
|---|---|
| `FixtureRegion`, `FixtureDensity`, `FixtureOrientation`, `FixturePlacement` (`skin_style.rs:121-205`) | `derive_skin`, `ShellShape`, `read_plates` facts |
| `ScatterRule`, `claims`, `fill_patches`, `on_lattice`, `turns_for` (`skin_decor.rs:237-479`) | `decor_pose`, `decor_body`, frozen decor records |
| fixture list order as priority (`skin_style.rs:229-236`) | `StyleFixtureConfig { id, model, health, collider }` minus `placement` |

### The solver the repo already has [code]

`crates/nova_wfc` collapses ship structure on a box grid.

- Simple tiled model. Sockets come from catalog link points
  (`nova_wfc/src/lib.rs:1-20`). Multi-cell parts are segment tiles with joint
  faces that accept only their partner segment (`tiles.rs:379-397`).
- Min-entropy cell choice with an RNG tie-break (`collapse.rs:780`),
  weighted draw (`collapse.rs:374`), AC-3 style propagation with an O(T^2)
  support scan per edge (`collapse.rs:338`).
- No backtracking. It never contradicts because `VACUUM` is compatible with
  every tile and is never struck (`collapse.rs:74-78`). This is the key trick
  for greebles too: the empty tile.
- Seeded by `StdRng::seed_from_u64` (`collapse.rs:315`). The `rand` docs do not
  promise `StdRng` output across `rand` versions [knowledge]. Ship looks must
  not change on a dependency bump, which is why the scatter uses `Fnv64`
  (`skin_decor.rs:483-497`).
- `nova_wfc` depends on `nova_ship` (`nova_wfc/Cargo.toml:17`), so the skin
  cannot call it. Reuse its patterns, not its code.

## 2. The surface model

### 2.1 Cells

**One cell per plate.** A plate stands in one empty cell and shows exactly one
out face (`facing`, `shell_skin.rs:678`). So the plates are already a partition
of the outer surface into unit quads. The number of cells is the plate count:
42 to 206 on the base hulls (2.5).

Not one cell per voxel face. At an inside angle one plate covers two voxel
faces: the deck plate rises to `FULL` against the wall
(`boundary_heights`, `shell_skin.rs:388`; `plate_level`, `:505`). A face-based
graph would have cells with no plate under them.

Not sub-cells yet. All 56 base greebles fit inside one plate: the largest
collider is `salvage_patch_plate` at 0.88 x 0.68 cells (generated RON,
`assets/base/styles/base.content.ron`). A 2x2 split per plate keeps quad
topology (subdividing quads gives quads; odd corners stay at the old corners),
so it can come later without changing the language. It quadruples the cell
count.

### 2.2 Edge slots and neighbours

Each cell has 4 edge slots. For slot `k`, `a` is the in-plane step across it,
`c` the plate cell and `n` its out direction. `boundary_heights` already
classifies the cell across each edge (`shell_skin.rs:388-457`, `open_edge`
`:458-501`). The graph uses the same branches and records the neighbour plate:

| derivation branch | context | neighbour plate |
|---|---|---|
| `walls(c + a)` (`shell_skin.rs:617`) | `Concave` | plate in `c + n` facing `-a` (the wall continues up) |
| same, wall one cell high | `StepUp` | plate in `c + a + n` facing `n` (top of the wall) |
| `c + a` is clad, same out | `Flat` | plate in `c + a` |
| `c + a` is clad, other out | `Twist` | plate in `c + a` (the one-plate-per-cell rule bent the surface) |
| open, plate in `c - n + a` faces `a` (`open_edge`) | `Convex` | plate in `c - n + a` |
| `ends_against` structure (`shell_skin.rs:604`) | `Stop` | none (fitting, or a `Bare` face) |
| `exit_pocket` (`shell_skin.rs:328`) | `Pocket` | none (muzzle, bell or bay mouth) |
| open space, no plate round the corner | `Rim` | none |

Two rules make the graph well formed:

- **Symmetrise.** If plate P names Q across slot k, Q's slot back to P is the
  slot whose direction points from Q toward P in Q's plane. If Q's own
  derivation named nobody there, the link is added (the far side reads
  `StepDown`). If Q's slot already names a third plate, P's slot becomes a
  border. [proto] counts: 0 to 45 one-sided links per hull before this step,
  0 to 15 conflicts, and at most 1 non-mutual slot after it (5 on one edited
  hull).
- **Priority** when several candidates exist: Concave, StepUp, Flat or Twist,
  then Convex. This keeps two sheets apart where voxels touch only on an edge,
  as the usual voxel-surface rule does [knowledge].

`Bare` sockets need no special case. A `Bare` face offers nothing to the skin
(`shell_skin.rs:1103`), so no plate stands there and its neighbours read
`Stop` or `Rim`.

### 2.3 The cell frame

Directional sockets need an "up" on each cell, and the frames must agree on
handedness. The obvious frame, `face_plane(out)` (`shell_skin.rs:352`), does
not: for `+Y` the axes are `(x, z)` and `x cross z = -y`, so the corner order
runs one way on `+Y` and the other way on `-Y`. A tile turned by `r` would read
mirrored on half the faces.

Proposal: `up = link_point_up(n)` (`link_points.rs:415`): ship `+Y` projected
onto the face, or `+Z` on faces that point along Y. `right = up x n`. Slots in
order `E (right), N (up), W, S`, counter-clockwise seen from outside on every
face. This is the convention sockets already use, so "tile north" means the
same as "socket up". The plate's own `rotation` is a proper rotation too, but
it is chosen to make the shape canonical (`plate_for`, `shell_skin.rs:635`), so
its axes follow the plate's edge pattern, not the ship. The solver output
converts the cell-frame turn into the plate frame for `decor_pose`.

Across an edge the solver needs the neighbour's slot index `m`, not a global
direction. A tile on cell P with turn `r` shows socket `sides[(k - r) mod 4]` on
slot `k`, and that socket meets the neighbour's socket on slot `m`. With one
consistent handedness, a socket read left to right on one side reads right to
left on the other, so chiral sockets need the usual mirrored pair (2.4, A).

### 2.4 What a cell carries

All of it is already computed by the derivation or the reading.

| fact | source | used for |
|---|---|---|
| `out` class: Up, Down, Side | `PlateFacing` (`skin_reading.rs:148`) | tile filter |
| `level`: Half, Full | `ShellShape.level` (`shell_shape.rs:194`) | tile filter |
| `falls`: 0 to 4, slots whose context is `Convex` or `Rim` | graph | tile filter (replaces relief words) |
| `seat` | `ShellShape::has_seat` (`shell_shape.rs:324`) | tile filter |
| `tilt` | `ShellShape::tilt` | tile filter |
| `fitting`: face steps to a fitting, saturating at 3 | `PlateReading.fitting` (`skin_reading.rs:81`) | tile filter, weights |
| per slot: context (2.2) | graph | socket filter |

`falls` replaces `PlateRelief`. It is the same count `relief_of` takes
(`skin_reading.rs:492`) without the class names. A filter says
`falls: (min: 2)`, not `HighGround`.

### 2.5 Measured topology [proto]

Graph built by a Python port of `cladding_cells`, `facing` and the edge
branches (diagonal stair ramps not ported). Plate counts match the table in
`docs/sections.md` for all five ships. The station counts differ from that
table, which predates the current station layouts (truss and solar sections
with `Bare` sockets, 86e971480). Link counts are per link, borders per slot.

| hull | plates | Flat | Convex | Concave | Step | Twist | border slots | 3-valent corners |
|---|---|---|---|---|---|---|---|---|
| `block_waystation` | 42 | 50 | 31 | 0 | 0 | 0 | 6 | 8 |
| `block_cutter` | 64 | 68 | 40 | 0 | 8 | 0 | 24 | 8 |
| `block_gunship` | 61 | 44 | 30 | 5 | 18 | 5 | 39 | 4 |
| `block_hauler` | 93 | 84 | 60 | 14 | 8 | 4 | 31 | 16 |
| `block_line_warship` | 171 | 179 | 66 | 7 | 38 | 9 | 85 | 1 |
| `ring_station` (177 sections) | 206 | 112 | 152 | 0 | 20 | 0 | 256 | 28 |
| `spire_station` (217 sections) | 122 | 120 | 52 | 0 | 0 | 0 | 144 | 0 |
| `yard_station` (190 sections) | 122 | 108 | 72 | 0 | 0 | 0 | 128 | 20 |

Readings:

- Convex folds are a third to over half of all links. Ships are thin, so a
  line rule that cannot cross a convex fold will stop at every hull corner.
- Stations are mostly border: truss and solar sections are `Bare`, and
  fittings stop the skin. A rule that needs a long run of plate finds little
  there.
- Most corners are 4-valent. 3-valent corners sit on convex box corners. One
  6-valent corner appeared on the hauler.

### 2.6 Irregular topology

The graph is a quad mesh with borders and extraordinary vertices. Every cell
has exactly 4 slots, so tiles with 4 sides work everywhere. What is irregular
is the vertex: 3, 4 or more cells can meet at a corner. Consequences:

- **Edge rules are always well defined.** They need a cell, a slot and the
  neighbour's slot.
- **Lines are well defined.** On a quad mesh the slot opposite the one you
  entered by is unique, so "continue straight" walks a quad strip. It turns
  over a fold by itself. This is how a line greeble follows a hull edge.
- **2x2 and 3x3 windows are not.** At a 3-valent corner a "2x2" window has 3
  cells. Windows must be defined by graph distance (a ring of radius r), or
  only at 4-valent corners, or per valence.
- **Prior art.** Townscaper runs on an irregular quad grid made by splitting
  a triangle-and-quad hex grid into quads and relaxing it; its modules are
  picked from the corner states of each quad [knowledge]. DeBroglie supports
  graph topologies built from mesh faces (`MeshTopologyBuilder`), but only the
  adjacent model works on them, not the overlapping model [web]. Both match
  this graph: adjacency on edges works, window patterns are the hard part.

## 3. The rule language

### 3.1 Shared by all candidates

- A style keeps its fixtures (`model`, `health`, `collider`) and loses
  `placement`. A new `greebles` block holds the solver configuration. Tiles
  name fixtures by id. One fixture may back several tiles (a line piece and its
  end).
- **Empty** is a tile with a weight. Its sides use the built-in socket
  `empty`.
- **Filter** `on:` takes the cell facts of 2.4:
  `(out: [Up, Side], level: [Half], falls: (min: 1, max: 1), seat: true, fitting: (max: 1))`.
  An omitted field does not filter. A filter is a hard unary rule: the tile is
  struck from the cell before the solve, as `nova_wfc` does for aim and zone
  (`aim_allowed`, `collapse.rs:67-79`).
- **Rotation** `rotate: All | Half | None`. `None` keeps tile `+Z` on the
  cell's `up`. `Half` allows 0 and 180 degrees. `All` allows 4 turns.
  Symmetric socket sets drop duplicate variants, as in mxgmn's symmetry
  classes [knowledge].
- **Mirror** `mirror: true` adds reflected variants. Off by default.
  Reflection is not a rotation (`shell_shape.rs:38` says the same for plates),
  and a mirrored greeble needs a mirrored model or a symmetric one. [proto]
  found the failure this prevents: a line end with an `out` side next to its
  line side can cap only one end of a run. Rotations cannot produce the other
  end, so propagation struck the line from every run (0.0 lines per ship).
- **Footprint** `footprint: (1, 1)` default. A `(1, n)` or `(2, 2)` tile
  compiles to segment tiles joined by private sockets that accept only the
  next segment, as `nova_wfc` does for multi-cell parts (`tiles.rs:383-388`).
  Segments may need all-`Flat` internal links.
- **Pins** come out of filters and sockets: a socket that only sits on a
  `Pocket` or `Stop` edge pins a tile beside fitting mouths; a `require` rule
  strikes `empty` from the cells a filter selects, so something must go there.
- **Seed.** The seed is a hash of the style id and the plate keys, not a
  stored number (4.1). Same structure and style give the same result.

The five greebles re-expressed below:

| greeble | today (`base.content.ron`) | intent in solver terms |
|---|---|---|
| `industrial_hazard_band` | `Edge`, `Every`, `Along` | a line on single-fall plates, along the fall edge, capped at both ends |
| `industrial_crane` | `Edge`, `Rare`, `Outward` | back side faces a `Convex` or `Rim` edge, empty ring, at least 4 apart, at most 3 |
| `industrial_stack` | `HighGround`, `Regular`, `Free` | `out: Up`, `falls >= 2`, at least 2 apart |
| `industrial_radiator` | `Panel`, `Dense`, `Along` | `falls: 0`, seat, empty ring, `rotate: Half` |
| `armoured_ammo_stripes` | `NearFitting`, `Every`, `Along` | one side on a `Pocket` edge, required there |

### 3.2 Candidate A: edge sockets + named constraints

**Model.** Simple tiled WFC. Every tile side carries a socket. Sockets pair
through an explicit symmetric table and may restrict which edge contexts they
sit on. Everything past radius 1 is a hard constraint from a closed list,
modelled on DeBroglie: `Separation`, `PairSeparation`, `Count`, `MaxRun`,
`Require` [web: DeBroglie constraint list].

- Sockets: `(id, pairs: [ids], edges: [contexts])`. Matching is by the table,
  not by equal names. A chiral socket is written as a pair `"x"` / `"x'"`
  listed in each other's `pairs`.
- Rotation and mirror: shared rules.
- Weights: one number per tile, plus `Weight(tiles, on: filter, factor)` to
  scale it on some cells.
- Larger windows: none directly. Separation covers spacing and rings; `MaxRun`
  covers line length; `Count` covers per-ship limits.
- Plate constraints: `on:` filters and socket `edges:`.

```ron
greebles: (
    empty: (weight: 6.0),
    sockets: [
        (id: "free",  pairs: ["free", "out", "empty"]),
        (id: "out",   pairs: ["free", "out", "empty"], edges: [Convex, Rim]),
        (id: "clear", pairs: ["empty"]),
        (id: "band",  pairs: ["band"], edges: [Flat]),
        (id: "mouth", pairs: ["empty"], edges: [Pocket]),
    ],
    tiles: [
        (id: "hazard_band", fixture: "industrial_hazard_band",
         sides: ("band", "out", "band", "free"), rotate: All, weight: 8.0,
         on: (falls: (min: 1, max: 1))),
        (id: "hazard_band_end", fixture: "industrial_hazard_band",
         sides: ("band", "free", "free", "free"), rotate: All, weight: 1.0,
         on: (falls: (min: 1, max: 2))),
        (id: "crane", fixture: "industrial_crane",
         sides: ("clear", "out", "clear", "clear"), rotate: All, weight: 0.6,
         on: (falls: (min: 1))),
        (id: "stack", fixture: "industrial_stack",
         sides: ("free", "free", "free", "free"), rotate: None, weight: 1.0,
         on: (out: [Up], falls: (min: 2))),
        (id: "radiator", fixture: "industrial_radiator",
         sides: ("clear", "clear", "clear", "clear"), rotate: Half, weight: 1.5,
         on: (falls: (max: 0), seat: true)),
    ],
    constraints: [
        Separation(tiles: ["crane"], distance: 4),
        Separation(tiles: ["stack"], distance: 2),
        Count(tiles: ["crane"], max: 3),
        MaxRun(tiles: ["hazard_band"], length: 8),
    ],
),
// armoured style, same shape:
(id: "ammo_stripes", fixture: "armoured_ammo_stripes",
 sides: ("free", "mouth", "free", "free"), rotate: All, weight: 1.0),
Require(tiles: ["ammo_stripes"], on: (fitting: (max: 1))),
```

- Pros: every rule is a named, documented thing. Lint can check each one. Maps
  one to one onto DeBroglie's set, which is proven in practice.
- Cons: every new idea needs a new constraint type in code. "Something changes
  when X is near" is not expressible except as a hard ban. Many hard rules
  raise the restart rate (4.2).

### 3.3 Candidate B: window patterns

**Model.** Overlapping-model style, but the patterns are authored, not learned
from a sample. Tiles carry no sockets. Rules say which tiles may or must
appear together inside a window: a pair across one edge, a ring of radius r
round a cell, or the cells round a 4-valent corner (2x2).

- "Sockets": none. Adjacency is `Pair` rules with a relative axis (`Along`,
  `Across`, `Any`) and an edge context. Default allow: two tiles with no rule
  may touch.
- Rotation: a tile has an `axis: None | Line | Facing`; rules speak about the
  axis relative to the edge.
- Weights: per tile, plus per-window `weight` on `Quad` patterns.
- Larger windows: native. `Ring(r)` windows, `Quad` patterns at 4-valent
  corners only.
- Plate constraints: `on:` filters; `Facing(tile, toward: contexts)`.

```ron
greebles: (
    empty: (weight: 6.0),
    tiles: [
        (id: "hazard_band", fixture: "industrial_hazard_band", axis: Line,
         weight: 8.0, on: (falls: (min: 1, max: 1))),
        (id: "crane", fixture: "industrial_crane", axis: Facing,
         weight: 0.6, on: (falls: (min: 1))),
        (id: "stack", fixture: "industrial_stack", axis: None,
         weight: 1.0, on: (out: [Up], falls: (min: 2))),
        (id: "radiator", fixture: "industrial_radiator", axis: Line,
         weight: 1.5, on: (falls: (max: 0), seat: true)),
    ],
    rules: [
        Pair(a: "hazard_band", b: "hazard_band", across: [Flat], axes: Along),
        Line(tile: "hazard_band", min: 2, max: 8),        // continuation + ends
        Facing(tile: "crane", toward: [Convex, Rim]),
        Ring(centre: "crane", radius: 1, others: Only(["empty"])),
        Ring(centre: "crane", radius: 3, others: Not(["crane"])),
        Ring(centre: "stack", radius: 1, others: Not(["stack"])),
        Ring(centre: "radiator", radius: 1, others: Only(["empty"])),
        Quad(cells: [["radiator", "radiator"], ["radiator", "radiator"]], weight: 0.0),
        Count(tile: "crane", max: 3),
    ],
),
// armoured:
Ring(centre: "ammo_stripes", radius: 1, others: Not(["ammo_stripes"]), on_edge: [Pocket]),
```

- Pros: speaks the owner's words directly ("2x2", "within range"). One
  mechanism for near and far.
- Cons: windows are n-ary constraints. AC-3 handles binary constraints; rings
  and quads need their own propagators. `Quad` is undefined at 3-valent
  corners (2.6), which are on every convex box corner. Default-allow pairs make
  it hard to see why two tiles never touch. The true overlapping model learns
  patterns from an example; doing that would need an authored sample hull,
  which is a tool, not a format [knowledge: mxgmn overlapping model].

### 3.4 Candidate C: edge sockets + range terms

**Model.** A's sockets for radius 1, so AC-3 sees every local rule. Every rule
past radius 1 is one `near` term on a tile: which tiles or plate facts, within
what graph distance, and a factor. Factor 0 is a hard ban, applied when the
other tile is placed (forward checking). Any other factor scales the tile's
weight at draw time. A per-tile `count` caps the total.

- Sockets: as A, `(id, pairs, edges)`.
- Rotation and mirror: shared rules.
- Weights: per tile, scaled by every `near` term that matches already-placed
  cells or plate facts.
- Larger windows: `near` with `within: r` is a ring of radius r in the graph;
  `tiles` or `fitting` say what to look for. Line continuation and ends are
  sockets (as A). Required margin is a `clear` socket (radius 1) or a factor-0
  term (radius > 1).
- Plate constraints: `on:` filters and socket `edges:`.
- Lint rule: a factor-0 term may not target a tile that a socket can force
  (a tile with a socket whose `pairs` exclude `empty`). Such a ban can empty a
  forced cell; [proto] measured 0.5% restarts with min-entropy and up to 37%
  with a fixed order (4.2). Without it, [proto] measured 0 restarts.

```ron
greebles: (
    empty: (weight: 6.0),
    sockets: [
        (id: "free",  pairs: ["free", "out", "empty"]),
        (id: "out",   pairs: ["free", "out", "empty"], edges: [Convex, Rim]),
        (id: "clear", pairs: ["empty"]),
        (id: "band",  pairs: ["band"], edges: [Flat]),
        (id: "mouth", pairs: ["empty"], edges: [Pocket]),
    ],
    tiles: [
        (id: "hazard_band", fixture: "industrial_hazard_band",
         sides: ("band", "out", "band", "free"), rotate: All, weight: 8.0,
         on: (falls: (min: 1, max: 1))),
        (id: "hazard_band_end", fixture: "industrial_hazard_band",
         sides: ("band", "free", "free", "free"), rotate: All, weight: 1.0,
         on: (falls: (min: 1, max: 2))),
        (id: "crane", fixture: "industrial_crane",
         sides: ("clear", "out", "clear", "clear"), rotate: All, weight: 0.6,
         on: (falls: (min: 1)),
         near: [
             (tiles: ["crane"], within: 3, factor: 0.0),
             (tiles: ["hazard_band_end"], within: 1, factor: 3.0),
         ],
         count: (max: 3)),
        (id: "stack", fixture: "industrial_stack",
         sides: ("free", "free", "free", "free"), rotate: None, weight: 1.0,
         on: (out: [Up], falls: (min: 2)),
         near: [(tiles: ["stack"], within: 1, factor: 0.0)]),
        (id: "radiator", fixture: "industrial_radiator",
         sides: ("clear", "clear", "clear", "clear"), rotate: Half, weight: 1.5,
         on: (falls: (max: 0), seat: true),
         near: [(fitting: 2, factor: 0.5)]),
    ],
),
// armoured:
(id: "ammo_stripes", fixture: "armoured_ammo_stripes",
 sides: ("free", "mouth", "free", "free"), rotate: All, weight: 1.0,
 require: true),            // strikes `empty` where this tile fits
```

- Pros: two mechanisms only (sockets, `near`). Soft terms never contradict.
  "If a certain greeble is in range something changes" is a weight change, not
  a new constraint type. The lint rule above makes the hard part provably
  local.
- Cons: soft terms depend on collapse order: a term can only see cells already
  placed. With a fixed order this is stable; it is still less transparent than
  a hard rule. Counts are enforced by banning after the cap, so a hull can end
  below a minimum (no `min` without backtracking).

### 3.5 Side by side

| | A sockets + constraints | B windows | C sockets + `near` |
|---|---|---|---|
| radius-1 rules | sockets (AC-3) | `Pair`, `Ring(1)` (n-ary) | sockets (AC-3) |
| past radius 1 | closed hard list | `Ring(r)`, `Quad` | `near` factor, 0 = hard |
| 3-valent corners | no issue | `Quad` undefined | no issue |
| contradiction risk | grows with each hard rule | highest (n-ary) | none without forced-tile bans (lint) |
| new rule kinds need code | yes | rarely | rarely |
| maps to | DeBroglie, ghx_proc_gen | overlapping model | `nova_wfc` + weight fields |

## 4. The solver

### 4.1 Determinism

- **Cell order**: the plate order of `derive_skin`, sorted by cell
  (`shell_skin.rs:263-271`). Variant order: tile order, then turn.
- **Randomness**: no RNG stream. Each draw is a counter-based hash:
  `Fnv64(cell, out, style id, attempt, purpose)`, the same family as
  `cell_hash` (`skin_decor.rs:498`). A per-cell value does not depend on how
  many draws came before it, so an edit far away does not shift it.
- **Tie-break**: min-entropy picks the cell with fewest variants; ties go to
  the lowest per-cell hash, then the lowest index. No hash map iteration.
- **Weights**: summed in variant order, so float rounding is the same every
  run. Integer weights would remove the question entirely [unverified need].
- **Not bevy_rand.** Gameplay is seeded through `bevy_rand`, but the skin must
  stay a pure function of structure: the editor preview must equal the flown
  ship, and nothing is saved. A per-design seed field could be added later;
  it would be content and save data.
- [proto] Re-solving every 37th seed (28 per hull) gave identical results on
  all 13 hulls, both orders.

### 4.2 Contradictions

Sources, from the prototype:

1. **Radius-1 rules** are sockets. AC-3 sees them. With `empty` compatible with
   every non-forcing socket, no contradiction appeared in 13 hulls x 1000
   seeds.
2. **Bans past radius 1** (factor 0, `Separation`, `Count`) are applied when a
   tile is placed. They only remove the banned tile. They cannot empty a cell
   unless the cell was forced to that tile by a socket (a line or a segment).
3. **Global structure** that AC-3 cannot see: a chiral line end (3.1 Mirror)
   or a line that cannot terminate inside its run. Propagation strikes these
   tiles up front; they do not contradict, they silently vanish. Lint must
   catch them (a tile placeable on no reference hull is an error).

Policy (bounded):

- Restart with `attempt + 1` in the hash, at most 8 attempts.
- If all fail, solve once more with every forcing tile removed. Without them
  the solve has no forced cells, so bans cannot contradict. Log a warning with
  the ship and style. A player-built hull is a valid runtime condition, so this
  is a runtime fallback, not a hidden authoring error; reference hulls are
  checked at lint.
- Worst case: 9 solves x O(cells x variants x 4) propagation steps plus
  O(cells^2) for a naive min-entropy scan. At 206 cells that is under 2 ms
  even at 10x the prototype time.
- No backtracking. Model Synthesis "modifying in blocks" re-solves a failing
  region instead of the whole output [web: Merrell 2021 comparison]. It is the
  next step if restarts ever become common; on hulls of a few hundred cells a
  full restart is cheaper than the bookkeeping.

### 4.3 Performance [proto]

Release build, std only, bitmask domains, precomputed compatibility masks,
8 tiles / 23 variants, 1000 seeds per hull:

| hull | plates | min-entropy us/solve | fixed-order us/solve |
|---|---|---|---|
| `block_waystation` | 42 | 15 | 6 |
| `block_hauler` | 93 | 49 | 14 |
| `block_line_warship` | 171 | 137 | 26 |
| `ring_station` | 206 | 192 | 31 |
| `spire_station` | 122 | 76 | 18 |
| `yard_station` | 122 | 79 | 19 |

- A per-ship solve at spawn is cheap. First-party crates build at
  `opt-level = 1` in dev (`Cargo.toml:1310-1311`), so expect several times
  slower there [unverified].
- Cache per style, once at load: the compiled variant table and the
  compatibility masks, as `nova_wfc::TileSet` is built once and collapsed many
  times (`nova_wfc/src/lib.rs:228`). Do not cache per ship: the solve is
  cheaper than invalidation.
- The min-entropy scan is O(cells) per collapse. A bucket queue makes it
  O(1); not needed at these sizes.

### 4.4 Stability under edits

- **Live ship.** Solved once at spawn. A destroyed section takes its plates
  and their greebles (`shell_skin.rs:979-986`); nothing re-solves. How a live
  section add re-clads was not checked [unverified].
- **Editor.** Re-solves on every change (`nova_editor/src/skin.rs:74`). [proto]
  Removing one section, the share of plates present before and after that
  keep their tile and turn:

| edit | plates | min-entropy | fixed order |
|---|---|---|---|
| hauler, section 0 | 93 -> 91 | 95.5% | 97.9% |
| hauler, section 25 | 93 -> 92 | 93.3% | 95.4% |
| warship, section 0 | 171 -> 171 | 96.7% | 98.2% |
| warship, section 75 | 171 -> 172 | 93.1% | 97.5% |
| ring station, section 0 | 206 -> 205 | 97.4% | 98.9% |
| ring station, section 88 | 206 -> 206 | 100% | 100% |

  So 2 to 7% of untouched plates change per edit. The fixed order is more
  stable and faster, but restarts far more when forcing tiles meet bans
  (4.2). Hash-keyed draws are what keep this local; a single RNG stream would
  re-roll the whole hull on any edit.
- **No incremental solve.** Keeping the previous answer outside the edit would
  make the look depend on build history, and the flown ship would differ from
  a fresh solve of the same structure. The skin's contract is a pure function
  (`shell_skin.rs:1-14`).

### 4.5 Frozen wrecks

Placements are stored, not re-solved: a freeze keeps each greeble's model,
pose and health under its plate (`frozen.rs:57-73`). Thaw needs no solver, and
a solver change cannot alter an existing wreck. A non-frozen ship respawned
from its design re-solves and gets the same result for the same structure,
style and solver version. A solver change re-rolls those ships, as a scatter
change does today.

## 5. Interaction with task 133643 (plating fits the greeble)

The order changes from derive, read, scatter, spawn to:

```
derive_skin -> read_plates -> surface graph -> solve -> reshape plates -> spawn plates + greebles
```

The solver hands each placement to the reshape step:

```rust
pub struct GreeblePlacement {
    pub plate: usize,          // anchor plate, index into the plates
    pub tile: usize,           // compiled tile, names the fixture
    pub turns: u8,             // quarter turns in the PLATE frame, for decor_pose
    pub cells: Vec<usize>,     // every plate the footprint covers, anchor first
    pub pad: Vec2,             // fixture collider x, z: the flat area to provide
}
```

- The reshape flattens the plateau under `pad` on each covered plate. For a
  footprint over several plates, the shared edges hold the level so the pads
  meet.
- The solve reads pre-reshape facts only, and the reshape changes heights
  inside a plate, never an edge context. So there is no loop between them.
- A filter like `seat: true` may become unnecessary once every placed greeble
  gets a pad; keep it until 133643 shows the frames.
- Frozen plates store `ShellShape` (`frozen.rs:66`). A pad that changes the
  shape changes the stored plate. World saves are at format 8
  (`nova_world_base/src/save/mod.rs:75`), so that is a migration and a `**(breaking)**` entry, or the pad must be
  re-derivable from the stored decor.
- A multi-plate greeble is a child of its anchor plate (`shell_skin.rs:985`).
  If the anchor is shot off the piece goes; the other plates keep their pads.

## 6. Prior art

| source | what it does | lesson | label |
|---|---|---|---|
| WFC, simple tiled vs overlapping (mxgmn) | tiled: tiles + adjacency; overlapping: every NxN window is a copy from a sample | sockets for edges; windows need a sample or authored patterns | [knowledge] |
| Model Synthesis (Merrell 2007; comparison 2021) | same algorithm family; scanline order; modifies in blocks to cut failure rate | block re-solve is the fallback if restarts grow | [web] |
| DeBroglie (Boris the Brave) | adjacent and overlapping models; constraints: border, fixed tile, path, max consecutive, mirror, symmetry, count, separation, pair separation; backtracking optional | the constraint list for A; graph topology only with the adjacent model | [web] |
| Townscaper, Bad North (Stalberg) | irregular quad grid; modules from corner states; constraint solve over modules | an irregular quad graph is enough; corners are where irregularity lives | [knowledge] |
| marian42 infinite city | horizontal connectors flipped, not flipped or symmetric; vertical connectors with a rotation index; backtracking from history | chiral sockets as mirrored pairs; backtracking can undo late and far | [web] |
| WFC on meshes (DeBroglie `MeshTopologyBuilder`; Merrell's thesis on non-grid shapes) | cells are mesh faces, adjacency through shared edges | our plate graph is this case | [web] |
| AC-3 / AC-4 | AC-3 re-checks an arc when a neighbour changes; AC-4 keeps support counts for optimal time | AC-3 with bitmasks is enough for ~25 variants; ghx_proc_gen uses AC-4 | [knowledge], [web] for ghx |
| `nova_wfc` (this repo) | 3D simple tiled, empty always compatible, no backtracking | the contradiction-free pattern; joint sockets for multi-cell parts | [code] |

**Rust crates.**

- `ghx_proc_gen`: model synthesis / WFC with sockets, rotations, weights and
  AC-4. Cartesian 2D and 3D grids only; restarts from the beginning on a
  contradiction [web]. Our domain is a graph, so it does not fit.
- The `wfc` and `wave-function-collapse` crates are grid and image oriented
  [web: crates.io listing; not read in depth].
- Recommendation: write a small solver in `nova_ship` (about 400 lines in the
  prototype's shape). The domain is a graph, the variant count is small
  (bitmask fits in a u32 or u64), and the hash and order rules are ours.

## 7. Recommendation

**Candidate C, edge sockets plus `near` range terms.** It keeps every rule
that touches a neighbour in the socket table, where AC-3 sees it and the
prototype measured zero restarts on every base hull. It folds everything
farther away into one term whose factor is either a hard ban or a weight
change, which is exactly the owner's "if a certain greeble is in range maybe
something changes" without a new constraint type per idea. Its one source of
contradiction, a hard ban on a tile that a socket can force, is a static
property of the configuration, so lint can refuse it instead of the solver
backtracking. A has the same core but grows a constraint type per idea and
has no soft form. B speaks windows natively but needs n-ary propagators and
breaks at 3-valent corners, which sit on every convex box corner.

**Smallest first slice** (no behaviour change, no format change):

1. `crates/nova_ship/src/sections/skin_surface.rs`: build the surface graph
   from `SkinStructure` and `&[SkinPlate]` (cells, 4 slots, contexts, frame).
   Proof: unit tests on the existing `hull(...)` fixtures: a lone cube gives 6
   cells, 12 `Convex` links and 8 3-valent corners; an L shape gives a
   `Concave` or `StepUp` link; a `Bare` face gives `Stop` borders.
2. The solver as a pure function over the graph and a compiled tile set:
   sockets, `rotate`, `on`, weights, `near`, `count`, restart and fallback.
   Proof: same input twice gives the same placements; a line tile placed on a
   run continues to both ends; a factor-0 `near` term is never violated.
3. Next slice, after the owner reviews frames: the `greebles` block replaces
   `placement` in style content (`**(breaking)**`), all 5 base kits migrate,
   `FixturePlacement` and the scatter are deleted, and 133643 starts.

Agent defaults for owner review: seed from structure and style id (no saved
seed); one cell per plate (no sub-cells); restarts then a fallback without
forcing tiles; mirror off by default.

## 8. Prototype

Scratchpad only, not committed:
`/tmp/claude-1000/-home-alex-personal-nova-protocol/216d5d00-66e8-4958-a2e4-e285d6d0793a/scratchpad/133640-wfc/`.

- `ron.py`, `skin.py`: read the base sections, ships and stations RON and port
  `read_cells`, `cladding_cells`, `stands`, `facing` and the edge branches of
  `boundary_heights` to Python. Diagonal stair ramps are not ported.
- `graph.py`, `dump.py`: build the surface graph of 2.2 and dump it, plus three
  hulls with one section removed.
- `rs/main.rs`: std-only Rust solver, built with `rustc -O`. Bitmask domains,
  AC-3, min-entropy or fixed hash order, counter-based hash, restart up to 30.
- Tiles: empty, a line (`band`) and its end, a crane (`out` back side, `clear`
  sides, separation 4, max 3), a vent (`clear` sides), a stack (separation 2),
  a 1x2 radiator as two joined segments.

Results, 13 hulls x 1000 seeds each:

| run | restarts | unsolved | us/solve |
|---|---|---|---|
| min-entropy | 0 on every hull | 0 | 15 to 192 |
| fixed hash order | 0 on every hull | 0 | 6 to 31 |
| min-entropy + radius-2 ban on forcing tiles | 0 to 0.5% (max 2 attempts) | 0 | 18 to 204 |
| fixed order + same ban | 4.5% to 37% (max 8 attempts) | 0 | 10 to 49 |

Mean per ship on `block_line_warship`, min-entropy: 18.5 band, 19.4 band end,
2.9 crane, 3.0 stack, 3.5 radiators, 117.8 empty of 171 plates. Two prototype
findings are recorded above: the chiral line end (3.1) and the forced-cell
contradiction (4.2).

Do you accept candidate C (edge sockets for radius-1 rules, `near` range terms with a factor where 0 is a hard ban, `on` plate filters, one solver cell per plate) as the placement language task 133643 implements?
