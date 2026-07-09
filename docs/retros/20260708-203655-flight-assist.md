# Retro: Spaceship handling / flight assist (velocity-command FCS)

- TASK: 20260708-203655
- BRANCH: feature/flight-feel-overhaul (squash-merged as 52b582d)
- REVIEW ROUNDS: 1 (APPROVE with 2 MINORs + 1 NIT; all addressed before merge)

See `tasks/20260708-203655/TASK.md` for what shipped and
`docs/2026-07-09-flight-assist.md` for the design. This retro is about how the
working went.

## What went well

- **Spike-first, and the spike read code, not vibes.** The whole control path
  (camera -> PD rotation, binary thruster input, AI's manual flip-and-burn)
  was mapped before any design talk, so the spike's options were real and the
  two open design calls (assisted default, 6DOF-vs-playability) resolved
  themselves against evidence. Plan and implementation then went through with
  zero direction changes.
- **Reading the dependency's source before writing prevented the one landmine.**
  avian's `Forces` query data was inspected field by field before the FCS was
  designed, so the `Write<LinearVelocity>` conflict was known in advance and
  the compute/apply split (mirroring the repo's own PD-controller pattern) was
  designed in, not discovered by a B0001 panic. Workspace compiled and all 15
  new tests passed on the first run.
- **The compounded lessons held.** Observer/App-level tests shipped with the
  feature (the audio->juice gap did not recur a third time, so no AGENTS.md
  promotion needed); the physics test harness was reused instead of rebuilt
  (`integrity/test_support` made `pub(crate)`); warm `CARGO_TARGET_DIR` and
  background runs kept builds at seconds; every feel constant got a written
  justification and an explicit "do not trust until playtested" note.

## What went wrong

- **R1.1 (unregistered reflect type) - the register-the-tree lesson was
  applied at module scope, not diff scope.** The new flight module registered
  its whole tree conscientiously, but the one reflected component added to a
  *neighboring* module (`ControllerSectionRcsMagnitude` in the controller
  section) was forgotten. Root cause: the juice lesson was internalized as
  "register your new module's types", when the real rule is "every
  `derive(Reflect)` the diff adds gets a `register_type` in the same diff,
  wherever it lives".
- **R1.2 (dead brake key in FA-off) - inputs were designed per mode, not as a
  matrix.** Brake semantics were carefully worked out for assisted mode; what
  X does in Newtonian mode was simply never asked. Root cause: no
  input-by-mode enumeration during design. Any modal control scheme should
  get a two-minute grid pass - every input times every mode, each cell either
  defined or explicitly "no-op, documented".
- **Small process wrinkle: REVIEW.md responses were drafted with a
  placeholder commit hash before the fix commit existed** and needed a sed
  afterwards. Write findings first, commit fixes, then fill responses.

## What to improve next time

- Diff-scope the reflect rule: before review, grep the diff for
  `derive(Reflect)` and check each addition against a `register_type`.
- For modal input schemes, write the input x mode matrix into the design note
  and let the review check cells, not vibes.
- Same-session self-review has a blind-spot pattern (PR #54's flash
  attenuation was caught only by an external reviewer). When a branch becomes
  a GitHub PR, treat automated/external review comments as a real second
  round, not noise - they have paid off twice now.

## Action items

- [x] All review findings addressed on-branch before merge.
- [ ] Playtest retune is already scheduled as task 20260709-095043 (rotation
  slew, camera weight, feel constants) - the defaults shipped here are
  reasoned but unflown.
