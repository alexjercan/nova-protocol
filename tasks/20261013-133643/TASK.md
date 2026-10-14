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
- Solver slice landed 2026-10-14 (agent): 335045214 (surface graph), 8d51bde7a (rules, compile, solve). Not wired.
  Tests: 4 `skin_surface` and 8 `greeble_solver` unit tests pass (the RON test runs under `--features serde`).
- Base hull measurement 2026-10-14 (agent). Method: the research prototype's Python reader turned the
  working-tree base RON into each hull's cells, sockets and exits; the committed Rust code then ran `derive_skin`,
  `skin_surface` and `solve_greebles` with the "Accepted language" rule set, 500 seeds per hull, `cargo test`
  profile. Throwaway harness, not committed. Times are one run on a shared host, not a reference.

  | hull | cells | plates | links | border slots | solve us mean / max | graph us | restarts | mean empty / line / end / crane |
  |---|---|---|---|---|---|---|---|---|
  | mining_tug | 33 | 56 | 92 | 40 | 83 / 190 | 136 | 0 | 28.0 / 22.8 / 4.4 / 0.8 |
  | salvage_hauler | 91 | 90 | 162 | 36 | 151 / 330 | 146 | 0 | 49.7 / 31.6 / 7.4 / 1.3 |
  | light_freighter | 66 | 74 | 131 | 34 | 129 / 204 | 112 | 0 | 39.2 / 27.7 / 6.2 / 1.0 |
  | interceptor | 40 | 64 | 112 | 32 | 101 / 202 | 100 | 0 | 34.1 / 23.7 / 5.2 / 0.9 |
  | raider | 40 | 67 | 116 | 36 | 106 / 176 | 132 | 0 | 35.6 / 24.9 / 5.6 / 1.0 |
  | patrol_picket | 21 | 40 | 66 | 28 | 47 / 53 | 59 | 0 | 18.9 / 17.4 / 3.2 / 0.5 |
  | line_warship | 179 | 178 | 313 | 86 | 410 / 937 | 274 | 0 | 94.1 / 66.7 / 15.0 / 2.2 |
  | freighter_derelict | 37 | 67 | 122 | 24 | 101 / 199 | 105 | 0 | 36.3 / 23.9 / 5.9 / 0.8 |
  | debris_plate | 11 | 33 | 63 | 6 | 44 / 72 | 57 | 0 | 16.5 / 12.9 / 3.2 / 0.4 |
  | block_waystation | 19 | 42 | 81 | 6 | 64 / 114 | 72 | 0 | 22.6 / 15.3 / 3.6 / 0.5 |
  | ring_station | 177 | 206 | 284 | 256 | 524 / 946 | 392 | 0 | 90.6 / 95.9 / 17.1 / 2.4 |
  | spire_station | 217 | 122 | 172 | 144 | 203 / 293 | 199 | 0 | 49.8 / 58.4 / 12.5 / 1.3 |
  | yard_station | 190 | 122 | 182 | 124 | 214 / 401 | 161 | 0 | 58.5 / 51.6 / 10.1 / 1.7 |

  No solve restarted and none fell back. Station plate counts match the research table (42, 206, 122, 122).
- Language gaps found in this slice (agent, for owner review):
  - `max_count: 3` in the accepted RON does not parse: the repo's RON has no implicit `Some`, so it is
    `max_count: Some(3)`, as `fixture: Some(..)` already is.
  - The seed hashes the whole structure, so any edit re-rolls every plate. The research's 93-100% per-edit
    stability assumed per-plate keys under a seed that does not change with the structure. The per-plate key is
    in place; seeding from the style id alone would restore that stability.
  - Agent policy beyond the owner's lint list: the factor-0 check covers both ends of the term; a factor-0 term
    on a `fixture: None` greeble and a `max_count` on one are refused; at least one `fixture: None` greeble is
    required. A `max_count` on a forceable greeble is not refused and can still restart.
  - A greeble's `weight` is split over its distinct turns, so `rotate` does not change how often it appears.
  - `fixture` stays `Option<String>` because `StyleFixtureConfig.id` is a `String`; typing both is wiring work.
  - A side with no neighbour accepts any link type, so a line may run off a border (a `Bare` face, a fitting,
    an exit pocket) without an end piece.
  - Soft `near` factors see only cells already placed, so they depend on the collapse order.

- Owner decision 2026-10-14: the greeble seed comes from the style id alone, with the per-plate keys, so a ship edit only changes greebles near the edit. It does not hash the whole structure.

## Delivery gate
- After the research note is accepted: propose how a plate learns which greeble sits on it and reshapes (flattened pad, footprint), show before/after frames, then implement. Regenerate base content.

## Verification
- Before/after screenshots of hulls where a greeble sat on a spike; asserted check that every placed greeble lands on a plate face that fits its footprint.

## Done when
- No greeble floats on or clips a pointy plate on the reviewed hulls; the owner accepts the look.
