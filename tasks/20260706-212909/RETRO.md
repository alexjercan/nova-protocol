# Retro: Editor preview controller spams PD 'root not found' errors

- TASK: 20260706-212909
- BRANCH: fix/editor-preview-controller
- PR: #44 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260706-212909/TASK.md`. A log-noise bug whose fix hinged on picking the option that
was structural rather than suppressive.

## What went well

- Read both sides of the boundary before choosing. The error is in bcs, but the fix belongs in
  nova. Reading `update_controller_root_torque`'s exact query - `(PDController, ...,
  PDControllerTarget, ...)` - is what turned three vague fix options into one clean answer: remove
  `PDController` from the preview and it simply is not in the query. No state gating, no
  quiet-no-op, no cross-repo change.
- Rejected the tempting-but-fragile option. "Skip if the root has no `RigidBody`" looked minimal,
  but checking the scenario spawn showed the root's `RigidBody` is added by a *separate* observer,
  so that check would be timing-dependent and could silently break real ships. Verifying the
  spawn order before committing to a condition avoided a regression.
- Made the preview inert by construction, not by suppression. A render-only bundle with no
  `PDController` cannot enter the erroring query - the fix is provable, and the test asserts
  exactly that (neither component present), which is stronger than "no error was logged".
- Scoped honestly. The same "preview carries live behavior" shape affects turret/thruster/torpedo
  previews too; flagged as a follow-up rather than silently widening the change or pretending the
  preview is now fully inert.

## What went wrong

- Could not reproduce the exact spam headless - it needs a UI click to add a controller, and the
  editor has no autopilot. Landed on structural proof + a clean boot instead. Root cause: the
  editor is interaction-only with no headless harness, unlike the gated example scenes. Not
  fixable in this task, but it is why a class of editor bugs can only be argued, not demonstrated.
- One small review inaccuracy (called the torpedo warhead's live controller a "debug helper")
  caught only by grepping the call sites afterwards. Verifying the "what stays live" claim before
  writing it down would have avoided the correction.

## What to improve next time

- For a cross-crate error, read the failing system's exact query/signature first; the cleanest
  fix is usually "stop matching the query", which is often a one-component change on the data side.
- Before writing "the only remaining uses are X", grep them - do not describe call sites from
  memory.
- Editor behavior is currently unverifiable headless. If editor bugs keep coming (this is the
  second editor-preview issue), an autopilot/harness for the editor - even a minimal "enter
  Editor, trigger these Activates, assert no error logs" - would pay for itself.

## Action items

- [ ] Possible follow-up (new task if it bites): render-only preview variants for the other kind
      sections (turret/thruster/torpedo) so the whole editor preview is inert, not just the
      controller.
- [ ] Possible follow-up: a minimal editor autopilot/log-assertion harness so editor-preview
      regressions can be caught in CI.
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro).
