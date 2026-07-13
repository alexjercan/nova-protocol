# Retro: Fix WebGL2 fatal crash: inset render target view_formats

- TASK: 20260713-175415
- BRANCH: fix/webgl2-inset-view-formats (landed as da88be7)
- REVIEW ROUNDS: 1

## What went well

- Error-signature-first diagnosis: matching the browser log's exact error
  sequence (create_texture -> write_texture -> create_view) against Bevy's
  GpuImage upload path, then grepping for the workspace's one RTT site,
  root-caused the crash before touching any code. No mechanism theorizing.
- The lessons ledger paid its rent: commit-before-sabotage, the one-command
  landing chain with pwd first, and fail-first A/B evidence all applied
  cleanly; the landing was uneventful.
- Round 1 APPROVE with zero rework, because the review re-derived the
  load-bearing claims (upload path, default-view format fallback) from
  bevy_render source instead of trusting the implementer's summary.

## What went wrong

- tatr same-second collision, sixth occurrence: three chained `tatr new`
  calls produced one surviving task; two had to be recreated. Root cause:
  the ledger's known trap was read after task creation - /flow reads retros
  at the start of each task cycle, but /plan creates tasks before the first
  cycle begins, so the lesson arrived too late to help.
- Shared-checkout friction unique to background sessions: the isolation
  guard rejects Write in the main checkout, which /plan's "fill in TASK.md"
  step assumes; meanwhile a parallel session's broad commit swept up the
  freshly created task stubs. Adaptation that worked: author all task/doc
  content inside the first sprouted worktree and let it land with the
  squash; only `tatr new` stubs touch the shared checkout.
- The bug itself was a copied-pattern failure: the original inset work took
  Bevy's render_to_texture example verbatim without asking what wgpu
  capabilities the pattern implies, in a repo that ships WebGL2 as a
  first-class target. The example is simply not WebGL2-safe.

## What to improve next time

- Never chain `tatr new` invocations; sleep or create sequentially
  (ledger entry bumped to x6 - already in Pending promotions).
- When copying a rendering pattern - even from an upstream engine example -
  check the device capabilities it implies (downlevel flags, limits)
  against the weakest platform the project ships (here WebGL2). New ledger
  entry `copied-pattern-weakest-target`.
- Background sessions: plan content lives on the first task branch, not the
  shared checkout; expect parallel sessions to move master between any two
  commands and re-verify branch state before each landing.

## Action items

- [x] LESSONS.md: bump `tatr-same-second-collision` to x6
- [x] LESSONS.md: add `copied-pattern-weakest-target`
- [x] LESSONS.md: add `bg-session-authors-on-branch`
- [ ] User confirms the fix on the deployed web build (no WebGL2 context
      available in this environment)
