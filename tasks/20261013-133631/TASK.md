# Skin: plating forms no spikes; iterate with owner review

- STATUS: OPEN
- PRIORITY: 97
- TAGS: v0.16.0,skin

## User facts
- Owner feedback 2026-10-13: "we will need to improve a bit the skin plating right now I don't like that it forms "spikes" would like to not have that happen anymore need more iteration and review on this feature".
- Visible on the new station designs (ring, spire, yard; screenshots from screenshot_station_showcase) and on ship hulls.

## Agent findings
- Skin is built in the shell skin code (`shell_skin.rs`; faces without sockets are not clad). Not yet surveyed.

- Survey 2026-10-13 (no code landed; frames in the session scratchpad `133631/`): the style never shapes plates; four rules make the spikes. (1) rim drop: a corner touching any open cell drops to the floor (`crates/nova_ship/src/sections/shell_skin.rs:403-411`); (2) a raised edge midpoint between two floor corners, the main spike cause, about 45% of ring plates (`shell_skin.rs:418-427`); (3) the plate centre is the mean of its edge heights and an all-floor plate gets a stud lift to half a cell (`shell_shape.rs:271-273`, `:508`); (4) a socket in the plate plane lifts a corner to a full cell, one-corner climbs make 10 m points (`shell_skin.rs:456-459`). Candidates prototyped and reverted: slab (no rim drop), softrim (quarter drop, rejected), bevel (pillow tops, rejected), edge-chamfer (flat plate at half height, chamfer only on open edges, no climbs). Saved wrecks store plate shapes (`frozen.rs:68`).

- Owner direction 2026-10-13: a hybrid of slab and edge-chamfer ("if we have a step we need to use a slab, if we don't we use a chamfer ... slabs and adding chamfers to smooth them out"; bevel kept straight). Hybrid prototype (reverted, diff `prototype-hybrid.diff` in the scratchpad `133631/`): flat plates at the plate level, one straight chamfer per lowered edge, no midpoint spikes, centre lift or point climbs; fits the 8-digit ShellShape with new digit meaning (frozen wreck plates need a re-derive or a breaking entry). All pyramids gone, decoration kept. Open problems: chamfers on two faces of a convex corner do not meet (teeth on box edges), a raised slab beside a lower plate leaves triangular gaps, and a slab chamfered on both sides reads as a gable without a minimum flat top.

## Delivery gate
- Survey which skin rule makes the spikes (with screenshots of named hulls), show before/after frames and the proposed rule change to the owner before landing. This feature needs owner review rounds, not one pass.

## Verification
- Before/after lavapipe screenshots of the same hulls (at least one station, one small ship, one large ship) with no spike plates; owner review.

## Done when
- The owner accepts the skin on the reviewed hulls.
