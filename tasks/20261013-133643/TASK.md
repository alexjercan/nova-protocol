# Greebles: plating reshapes to fit the greeble placed on it

- STATUS: OPEN
- PRIORITY: 94
- TAGS: v0.16.0,skin

## User facts
- Owner feedback 2026-10-13: small square greebles (vents etc.) sometimes land on "pointy" plating. "would be nice if we can somehow detect that a greeble is being placed somewhere and we make the pointy thing adapt to the greeble that will be placed on top, so the plating will basically take into account what/where it is placed on (like in terms of sections it is placed on and where) and also what greeble goes on top of it, so it can actually accommodate for that shape to make sense".
- The greeble follows the plate's axis; an angled plate turns the greeble and the plate makes room for its size (see the placement research task).

## Agent findings
- Depends on the placement language research task (same strand). Not yet surveyed.

- Owner decision 2026-10-14: the greeble placement language is accepted (`tasks/20261013-133640/RESEARCH.md`, "Accepted language"). Start the solver slice now: the surface graph module and the pure seeded solver with unit tests, as in section 7 of that note, using the accepted language rather than the candidate C keywords. No compatibility with today's greeble placement, and no save migration (nothing is released).

- Solver slice design 2026-10-14 (agent). No wiring, no style or format change.
  - `crates/nova_ship/src/sections/skin_surface.rs`: `skin_surface(&SkinStructure, &[SkinPlate]) -> SkinSurface`.
    Cell `i` is plate `i`. `SurfaceCell { key: u64, links: [Option<SurfaceLink>; 4] }`, slots in order up, right,
    down, left; `up = link_point_up(out)`, `right = up x out`, so every face reads clockwise from outside.
    `SurfaceLink { cell: usize, slot: u8 }` names the neighbour and its slot that faces back. Neighbour per slot
    `a`, in the derivation's order: structure in `c + a` that `walls` -> plate in `c + n` facing `-a`, else plate
    in `c + a + n` facing `n`; other structure -> none; plate in `c + a` -> that plate; exit pocket -> none; else
    plate in `c - n + a` facing `a`. The back slot is the neighbour slot that points most toward this plate's
    edge midpoint. One-sided links are made mutual when the far slot is free, mutual links first, then plate and
    slot order; a far slot held by a third plate leaves a border. No slot contexts: the solver reads none.
    `key` hashes the plate cell and out face, so a draw is local to the plate. `shell_skin::walls` becomes
    `pub(super)`.
  - `crates/nova_ship/src/sections/greeble_solver.rs`: serde types `GreebleRules { link_types, greebles }`,
    `GreebleLinkConfig { id: GreebleLinkDesignId, connects }`, `GreebleConfig { id: GreebleDesignId,
    fixture: Option<String>, weight, rotate: GreebleRotateType (All|Half|None), link_points:
    GreebleLinkPoints { up, right, down, left }, near: Vec<GreebleNearConfig { greebles, radius, factor }>,
    max_count: Option<u32> }`.
  - `GreebleRules::compile(&self) -> Result<CompiledGreebleRules, Vec<GreebleRulesError>>`: unknown and duplicate
    ids, asymmetric `connects`, weight and factor ranges, a missing `fixture: None` greeble, and a factor-0 term
    whose owner or target a link type can force. A link type forces when its `connects` holds no link type of a
    `fixture: None` greeble. Rotations expand to variants; duplicate variants drop; a greeble's weight splits
    over its variants.
  - `solve_greebles(&SkinSurface, &CompiledGreebleRules, seed: u64) -> GreebleSolution { cells:
    Vec<Option<GreeblePlacement { greeble, turns }>>, restarts, fallback }`. Bitset domains, AC-3 over the slot
    links, min domain size with a `Fnv64(seed, key, attempt, purpose)` tie-break, weighted draw by the same hash,
    soft `near` factors on placed cells within graph radius, factor-0 bans both ways on placement, `max_count`
    bans at the cap. Up to 8 restarts, then one solve without forcing greebles and a `warn!`; a cell that still
    contradicts there is `None`. `greeble_seed(&SkinStructure, style: &str) -> u64`.
  - Call graph (later wiring): `derive_skin -> skin_surface -> solve_greebles(compile(style), greeble_seed)`.
    Today: tests only.

## Delivery gate
- After the research note is accepted: propose how a plate learns which greeble sits on it and reshapes (flattened pad, footprint), show before/after frames, then implement. Regenerate base content.

## Verification
- Before/after screenshots of hulls where a greeble sat on a spike; asserted check that every placed greeble lands on a plate face that fits its footprint.

## Done when
- No greeble floats on or clips a pointy plate on the reviewed hulls; the owner accepts the look.
