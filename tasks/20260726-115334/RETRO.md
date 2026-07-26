# Retro: NOVA OS app runtime

- TASK: 20260726-115334
- BRANCH: feat/nova-os-app-runtime
- REVIEW ROUNDS: 1 (APPROVE, out-of-context; MINOR findings fixed in place)

## What went well

- The two load-bearing forks were pulled to the owner at the plan gate (app-as-plugin
  trait objects; context-sensitive Escape) and recorded in DECISION.md, so
  implementation had no mid-flow surprises on shape - and the owner's Escape
  redirect ("Esc closes the app first") was absorbed before any code existed.
- Keeping app mode from ever touching the scrollback made "exit restores the
  terminal" fall out for free: exit is a one-line mode flip, and the scrollback
  entities are simply revealed again. The whole restore contract needed no save.
- The prior drawer task's lesson (split Tab caught by a sibling gamepad test) paid
  off: I handled the context-sensitive Escape in ONE system keyed on state rather
  than two cooperating readers, which the out-of-context reviewer specifically
  tried and failed to break.
- The out-of-context review earned its keep: it found a real launch-keystroke bleed
  that every headless test I wrote missed because my sample app was not
  Enter-sensitive. An Enter-exit test app made the bug reproducible and pinned the
  fix.

## What went wrong

- The DoD's verify command was malformed - `cargo test -p nova_gameplay drawer
  terminal` passes two positional filters, which `cargo test` rejects. It only
  surfaced at verify time; the "cmd" proof had never been run when it was written
  into the epic plan. Corrected to `... -- drawer terminal`.
- I shipped the app-word "first-class" claim in NOTES before it was true: the
  inline completion ghost and the did-you-mean suggester still looked only at the
  builtin table. The reviewer caught the contradiction. Fixed both, plus threaded
  `app_commands` through `parse_command`/`nearest_command`.
- One self-inflicted compile stumble: I matched on `(*pause.get(), active_mode)`,
  moving a non-`Copy` `PauseStates` out of a shared ref (E0507). Compare the state
  by `==` instead of moving it into a match tuple.

## What to improve next time

- When a mode/registry becomes a source of valid command words, sweep EVERY reader
  of the old command table in the same change (parse, complete, ghost, help,
  did-you-mean, parse-status) - not just the submit path. A grep for the const
  table name (`TERMINAL_COMMANDS`) lists them; I missed two.
- When writing a test app/fixture to exercise a runtime, make it sensitive to the
  exact input the runtime routes (here: Enter), or the test proves nothing about
  input bleed. A fixture that ignores the key under test is a silent pass.

## Action items

- Added two lessons to LESSONS.md:
  - `context-key-handled-in-one-owner` (x1): interpret a context-sensitive key
    (Escape closing an app vs the drawer) in ONE system branched on state, never
    two readers cooperating over `ButtonInput`/event edges - a single read cannot
    race itself. Generalizes the Tab-split lesson.
  - `validate-proof-command-shape-at-plan-time` (x1): a `cmd:` proof written into
    a plan is unrun until verify; check its shape (arity/flags) when authoring -
    `cargo test <a> <b>` takes ONE positional filter and rejects the second
    (`-- <a> <b>` for libtest multi-filter).
- No follow-up code tasks. This runtime unblocks the `map` app (`20260724-102320`)
  and the `ship viewer` stretch (`20260726-115339`); both register a
  `NovaOsAppRuntime` into `NovaOsAppRegistry` and spawn into the body slot.
