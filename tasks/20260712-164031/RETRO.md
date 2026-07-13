# Retro: Turret free-aim while holding CTRL

- TASK: 20260712-164031
- BRANCH: feature/turret-free-aim-ctrl (landed as 3eaca1b)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

What/why: TASK.md. Process only here.

## What went well

- **Planned against the real mechanism, not a guess.** The task was stepless and
  said "needs /plan". Reading `update_turret_target_input`'s actual three-tier
  feed (component -> ship-lock -> camera ray) made the fix obvious and one-line:
  force the ray tier while CTRL is held. No thrash.
- **Didn't invent a mode.** The task said "only in manual mode"; grepping first
  confirmed there is NO turret auto/manual mode enum - "manual mode" just means
  the player turret feed. So the change went where it belonged instead of adding
  a speculative mode toggle.
- **Anticipated the test-rig ripple.** Adding `Res<ButtonInput<KeyCode>>` to the
  system means every `run_system_once(update_turret_target_input)` test rig must
  provide that resource or panic. Init'd it in both rigs up front (default = no
  CTRL = unchanged), so no test broke.
- **Flagged the CTRL overlap honestly** (shared with the CTRL+scroll ship-lock
  cycle) in the task, code comment, and PR message rather than silently shipping
  it; the review confirmed it's a feel issue, not a bug (cycle writes lock state,
  feed reads it).

## What went wrong

- Nothing. The one thing to stay alert to: there are TWO same-named
  `update_turret_target_input` functions (player.rs for the player, ai.rs for AI
  turrets). I edited only the player one; the review double-checked the AI one was
  untouched. The name collision is a latent trip hazard when grepping.

## What to improve next time

- When adding a `Res<T>` (or any new param) to a system that tests drive via
  `run_system_once`, grep its callsites and init the resource in each rig in the
  same change - the failure is a runtime panic, not a compile error.

## Action items

- [x] Retro written; ledger note added (new-system-input ripple to run_system_once
  rigs, sibling of required-component-in-shared-query).
- [ ] Playtest the CTRL overlap feel (free-aim while cycling ship locks); if
  unwanted, a follow-up can move one gesture to another modifier.
