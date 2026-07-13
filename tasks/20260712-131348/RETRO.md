# Retro: Diegetic per-weapon ammo readout

- TASK: 20260712-131348
- BRANCH: ammo-hud-diegetic
- REVIEW ROUNDS: 1 (APPROVE)

Process notes only; what/why/evidence live in
[TASK.md](../../tasks/20260712-131348/TASK.md), the substrate decision in the
[spike](../spikes/tasks/20260712-143113/SPIKE.md).

## What went well

- The spike paid for itself. The user's framing ("diegetic, on the weapon")
  reads as "must be a world-space 3D widget", and chasing that would have meant
  net-new infrastructure (billboarding, a fill material, 3D text). Writing the
  options down surfaced that the existing `screen_indicator` substrate already
  delivers "rides on the weapon, scales, hides when gone" and that occlusion
  (world-space's one edge) is actively harmful for a status readout. The whole
  feature then landed as a thin consumer of `turret_lead`'s reconcile pattern.
- One-round APPROVE. The plan cited the exact template files
  (`turret_lead.rs`, `screen_indicator.rs`) and current markers, so
  implementation was mechanical and matched conventions (tier, observers,
  PostUpdate slot) without guesswork.
- The review's independent re-derivation earned its keep: rather than assume
  the tier-hiding worked, I read `apply_hud_visibility` and confirmed the
  self-driven debug number (a grandchild in neither of that system's query
  sets) is never stomped - so no `HudSelfDrivenVisibility` opt-out was needed.
  A same-session review that only read the diff would have shipped that as an
  assumption.

## What went wrong

- Two compile errors from writing Bevy bundle code against a remembered API
  instead of the installed 0.19 surface: `TextFont { font_size: 9.0 }` (the
  field is now a `FontSize` enum; the idiom is `TextFont::from_font_size(..)`)
  and `BorderRadius::MAX` as a standalone bundle component (it is a *field* of
  `Node`, not a component). Root cause: both were written from a mental model
  of the API, not from an in-repo callsite. Cheap to fix (a check + one grep
  each), but avoidable - the repo already had `TextFont::from_font_size` and
  `border_radius: BorderRadius::MAX` callsites to copy.

## What to improve next time

- Before writing a new Bevy bundle/struct-literal, grep the repo for an
  existing use of each unfamiliar component/field and copy its exact shape.
  The API churns between 0.x releases; a 10-second grep beats two check
  round-trips.

## Action items

- [x] Ledger: `verify-bevy-api-at-callsite` (new), `spike-reuse-over-new-infra`
  (new, positive).
- No follow-up code tasks. Textured ring art and a distance-scaled offset are
  noted as optional polish in TASK.md, not worth a task until playtest asks.
