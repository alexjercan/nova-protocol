# Retro: Contextual HUD - show-by-relevance, grow-in-use, On/Cinematic

- TASK: 20260728-175747
- BRANCH: feat/hud-contextual-emphasis
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES with 1 MAJOR + 2 MINOR + 2 NIT, all
  addressed; round 2 APPROVE with 1 NIT, taken)

## What went well

- **The demo was again a real spec, and its `reflect()` was the ruleset.**
  Reading demo 2's JavaScript as the authority - not the prose summary of it -
  gave the exact situations, scale values and durations, and made "did I
  implement the rule?" a diff against 20 lines of JS rather than a judgement
  call. Both HUD tasks in this pair were cheap for the same reason.
- **Sensing the situations ONCE paid for itself immediately.** `HudSituations`
  meant every widget driver is two lines and the ruleset is readable in one
  file. The alternative - each widget querying `Autopilot`/`WeaponsHot`/... for
  itself - would have put "what counts as firing?" in six places, and the
  safety-plus-trigger subtlety in exactly one of them.
- **Riding the existing enforcement instead of building a second one.** The
  contextual gate is enforced inside `apply_hud_visibility`, which already ran
  in the one slot where a projected indicator's every-frame
  `Visibility::Visible` can be overruled. A separate gate system would have
  looked simpler and lost a race with the projection - the kind of bug that
  only shows up on screen.
- **The out-of-context reviewer re-derived rather than trusted.** Round 2 did
  not take the `resolve_chain` refactor on its word: it reconstructed the old
  and new visited sets, then mutated each half separately to prove which test
  pins which. That is the level at which a refactor of enforcement code should
  be checked, and it is exactly what an in-session review would have skipped.

## What went wrong

- **The doc sweep was symbol-scoped, so marketing prose survived it (R1.1).**
  I grepped `HudVisibility::All|Minimal|None` and "All / Minimal / None" and
  fixed every hit - wiki, keybinds, README, dev wiki, CHANGELOG - and called
  the step done. The landing page describes the same feature in plain English
  ("from full chrome to instruments-only to a clean screen") and names no
  symbol, so it matched nothing. Root cause: I swept for the NAME of the thing
  I renamed instead of for the CONCEPT I changed. The public page is the most
  visible surface in the repo and it was the one surface left stale.
- **Two comments outlived the vocabulary they used (R1.2, R1.4).** Same root
  cause one layer down: `cycle_hud_visibility`'s "three states are at most two
  presses away" and the screenshot examples' "full chrome"/"instrument tier"
  are prose about the levels, not references to them, so the symbol sweep
  missed them too. One miss, three findings.
- **A deliberate composition was recorded in prose but not in code (R1.3).**
  The lock readout is a child of the reticle, so the two emphases multiply to
  1.2544 while firing. I decided that was right, wrote it in NOTES.md and left
  the code silent about it - which means a future reader debugging "why is the
  readout jumping" starts from 1.12 and a retro file they will not find. The
  fix (a named constant plus a test that also fails on a re-parent) took five
  minutes and should have been part of the original decision, not a review
  response.
- **The visual evidence had a hole I only found while hunting for it.** No
  harnessed example fires the player's guns - every example ship is
  `infinite_ammo: true` and the scripted walks kill via `HealthApplyDamage` -
  so the ammo gauges cannot be seen APPEARING anywhere in the example set. I
  did find the honest half (a finite-ammo ship in idle cruise with no gauges,
  via `menu_newgame` into `shakedown_run`) but only after three failed
  attempts. Worth knowing before planning the next weapons-facing visual.

## What to improve next time

- When a rename retires a CONCEPT the docs describe in words (a level, a mode,
  a tier), sweep for the concept's words too - here, `grep -rni "minimal\|full
  chrome\|instruments-only\|three levels" web/ README.md` - and always include
  `web/src/index.html` and `web/src/tutorial.html` by name, since they sell
  features in prose that names no symbol.
- When a decision produces a NUMBER a playtest might complain about, put the
  number in the code as a named constant with a test, not only in NOTES.md.
  The retro file is not where a future debugger looks first.
- Before promising a visual check on a weapons/ammo behaviour, check whether
  any example actually has finite ammo and fires. If none does, say so in the
  DoD at plan time and lean on the App-driven pin plus the owner playtest,
  rather than discovering it during verification.

## Action items

- [x] Ledger: bumped `sweep-a-rename-where-the-name-is-spoken` to x2 with the
      concept-words sharpening (its first occurrence, 20260728-175742, was the
      cross-crate symbol case; this one is the no-symbol-at-all case).
- [x] Ledger: added `ui-node-rebuilt-per-frame-needs-age-seeded-state`.
- [ ] Open for the owner's playtest (DoD 4, carried in TASK.md): the always-on
      allegiance triangles, and whether the 1.2544 composed lock motion while
      firing reads as too busy.
- No follow-up code tasks: nothing was deferred out of this branch.
