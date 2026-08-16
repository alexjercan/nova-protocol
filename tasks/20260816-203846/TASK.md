# Greeble catalog example: every piece, named, orbited

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.11.0,example,art,skin

## Goal

Follow-up 3 of the greeble spike (tasks/20260816-194637/GREEBLES.md section 6,
approved by the owner). Parallel to everything else.

A greeble catalog example: every fixture model the styles own, stood in named
rows grouped by style, parts-preview treatment (idle orbit, the fleet capture
idiom), so a piece can be judged as an OBJECT before placement rules ever
touch it. The spike's section 6 spec is the contract.

## Done when

- section 6's definition of done in GREEBLES.md
- one command renders the whole catalog with names; a capture lands in this
  folder

## Closure

Landed 2026-08-16, lane greeble-catalog. `cargo run --example greeble_catalog
--features debug`. 27 fixtures across 5 styles resolved at runtime from the
merged GameStyles (industrial 7, armoured 4, civilian 5, salvage 7,
placeholder 4) - nothing hand-listed, a mod style joins with zero edits.
Arrows select, Enter turntable-focuses, L snaps rows, C toggles pedestals,
G shows the unit cell; per-fixture report line prints id, model, collider,
health and the full placement rule. Wall capture in this folder.

All models load; the only flat-colour pieces are the four intentionally
magenta placeholders. The wall makes the spike's variety gap visible: almost
the only colour across 27 pieces is industrial yellow and salvage rust.
