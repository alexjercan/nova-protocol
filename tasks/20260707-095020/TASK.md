# Mark promotion-eligible systems for bevy-common-systems

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.4.0, crates, refactor

Part of the v0.4.0 goal of identifying which nova systems are game-agnostic enough
to be copy-pasted into other games via bevy-common-systems (tier 2 -> external in the
crate-boundary policy in `docs/architecture.md`). This task is the *catalog + tagging*
pass; the actual cross-repo promotions are tracked by task 20260706-151804.

Goal: a single documented list of promotion-eligible modules with a short rationale
and a stable-API check for each, so promotion is a deliberate decision rather than an
ad-hoc one.

## Steps

- [ ] Sweep `nova_gameplay` (and `nova_scenario`/`nova_debug` where relevant) for
      modules that touch only generic Bevy/Avian + generic components (Health, Transform,
      physics) and no nova-specific config/assets.
- [ ] For each candidate, record: what it is, why it is game-agnostic, what still
      couples it to nova, and whether its API is stable enough to reuse. Start from the
      known candidates in task 20260706-151804 (hud/health, hud/objectives, hud/velocity
      + shaders, integrity/blast + collision-damage helpers) and extend.
- [ ] Add an in-code marker convention for eligible items (a short doc-comment tag like
      `// PROMOTE(bevy-common-systems): ...`) so `grep` surfaces the candidates.
- [ ] Write the catalog to `docs/` (a "promotion candidates" doc), cross-linking task
      20260706-151804 for the actual moves and 20260706-160503 (mesh slicer hardening,
      already external).
- [ ] Do NOT move code here - promotion is the follow-up. This task only decides and
      marks what is eligible.

## Notes

Reference the crate-boundary policy in `docs/architecture.md` and the existing
bevy-common-systems module layout in `~/personal/bevy-common-systems/src`.

Spike (decided catalog, Tier A-D + marker convention):
`tasks/20260708-110317/SPIKE.md`. The remaining steps of this
task (apply `PROMOTE(bevy-common-systems)` markers to the Tier A/B/C items and write
`docs/promotion-candidates.md`) execute that catalog. The Tier-B seam design is split out
to task 20260708-110449; the cross-repo moves stay under task 20260706-151804.

