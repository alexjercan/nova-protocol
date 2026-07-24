# Notes: diegetic objective reveal

Design notes for task 20260721-211520. Mechanism detail lives in the module doc
(`crates/nova_gameplay/src/hud/objective_reveal.rs`); this file records the
cross-cutting decisions.

## does-the-old-element-survive: the gold posting flash is REMOVED

A fresh objective posting used to spawn a small **gold ghost line** in the ghost
column (task 20260717-163033). The owner asked for the big diegetic moment
instead, and confirmed at the plan gate (2026-07-24) that the gold flash should
be REPLACED, not kept alongside - otherwise a new objective would double-animate
(a gold ghost AND a big card). So:

- `objective_feedback.rs` now routes each ADDITION to
  `objective_reveal::spawn_objective_reveal` and no longer spawns a gold ghost.
- COMPLETIONS are unchanged: they still ghost green in the ghost column.
- Three `objective_feedback` tests that asserted the gold posting flash were
  updated to the new behavior (no gold ghost on a posting; the posting reveals
  instead; same-id message swaps add no second reveal).

One detection point is preserved: the reveal spawn is triggered from
`objective_feedback`'s existing single `GameObjectives` diff, so there is no
second change-detector to drift.

## Placement: the screen_indicator pattern, not UiTransform.translation

The card's screen position rides `Node.left/top` (logical px) and `UiTransform`
carries only scale + rotation - the same split `screen_indicator.rs` uses to move
a UI node to a projected screen point. This maps the tuck target
(`DrawerTabAnchor.rect`, already in screen px) directly and sidesteps any
`GlobalTransform`-vs-`UiTransform` coordinate ambiguity (whether a node's global
transform includes its UiTransform). `reuse-known-good-stack`: copied the proven
in-repo placement idiom.

## Clock: default Res<Time> is correct here

Unlike the drawer slide (which plays while the sim is PAUSED and so needs
`Time<Real>`), the objective reveal plays during normal flight (Unpaused), so the
default `Res<Time>` (= `Time<Virtual>`) is right. If the drawer is opened
mid-reveal the sim freezes and the reveal freezes with it - acceptable.

## Anchor fallback

`DrawerTabAnchor.rect` is `None` until the drawer tab handle has laid out. An
objective posted in that window fades in place at the base cockpit position
rather than tucking - a graceful degradation, not a panic. In practice the handle
lays out within a frame or two of the player spawning, well before objectives
post.

## Pacing is upstream (out of scope)

WHEN objectives post (so a reveal never lands mid-fight) is authored scenario
pacing - task 20260721-211506 (CLOSED) sets that pattern. This task only animates
the reveal when a posting happens.
