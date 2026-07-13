# Retro: Bullet-type slot + ammo-readout color-coding

- TASK: 20260712-133349
- BRANCH: feature/bullet-type-slot (landed as 12de44e)
- REVIEW ROUNDS: 1 (APPROVE, zero findings)

What/why: TASK.md. Implementation write-up: tasks/20260712-133349/NOTES.md.
Process only here.

## What went well

- **Scoping down to the user's stated "foundation" kept the diff tight and got a
  zero-finding APPROVE.** The task title ("multi-type magazines, reload,
  switching") could have ballooned; the user's "one mag of one type, structured
  for growth" turned it into a small, reviewable seam (a runtime slot + HUD
  color) with everything else explicitly deferred. Stating the scope in the plan
  checkpoint up front meant the reviewer judged the diff against the right bar.
- **A prior lesson fired proactively.** Adding `&LoadedBullet` to the existing
  `shoot_spawn_projectile` query is exactly `required-component-in-shared-query`
  (a REQUIRED fetch narrows the query's membership). I recalled it before running
  anything and switched to `Option<&LoadedBullet>` + a config fallback, so the
  headless fire rigs (which spawn turrets by hand, no slot) kept firing. The
  lesson turned a would-be broken-rig debugging loop into a one-line choice.
- **The readout's own foresight paid off.** `drive_ammo_readouts` had a comment
  flagging itself as the single ammo-source read to change for per-bullet-type
  work; the color-coding landed exactly there.

## What went wrong

- Nothing of substance. Two mechanical fixups, both caught at compile time: the
  `bullet_kind` field broke the two full-literal turret configs (fixed; the
  check-all-targets habit surfaced them), and removing `LIT_COLOR`'s last
  non-test use left it dead (removed it, made the lit-pip test alpha-based).

## What to improve next time

- Keep doing the plan-checkpoint scope statement for tasks whose title is broader
  than the intended slice - it aligns the reviewer and prevents scope creep.

## Action items

- [x] Retro written; ledger bumped (required-component-in-shared-query x2, applied
  proactively this time).
- [ ] File + fix the PDC-one-shots-asteroids tuning (playtest feedback received
  mid-cycle; routed to its own task per the flow discipline). Numbers gathered:
  field asteroids 100 HP, better turret ~20/hit @ 100 rounds/s (~2000 DPS) so a
  short burst vaporizes them; salvage crates are sensors (not damageable).
