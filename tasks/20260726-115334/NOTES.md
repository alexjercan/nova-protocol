# Notes: NOVA OS app runtime

- TASK: 20260726-115334

## App lifecycle contract

The runtime lives in `crates/nova_gameplay/src/hud/drawer.rs` beside the terminal
it swallows. The terminal's `active_mode: TerminalMode` gained an `App { id }`
variant (id is `&'static str`, an app's stable launch word, so the mode stays
`Copy` and allocation-free). The lifecycle:

- Launch: submitting a registered app's launch word (resolved from the terminal's
  mirrored `app_commands`, checked before the static command table) pushes a
  `launching <id> ...` row and sets `active_mode = App { id }`. The scrollback and
  prompt are left untouched.
- Render: `sync_nova_os_app_ui` reconciles the on-screen surface with
  `active_mode` - it spawns one `NovaOsAppRoot` (chrome bar with title + close
  control, over the app's `spawn_body`) at content depth and hides the terminal
  content, or despawns the app root and reveals the terminal. It diff-guards on
  the running app id, so a drawer reopened onto a persisted app rebuilds it and a
  plain reopen keeps the terminal.
- Input ownership: `handle_terminal_keyboard` is already inert outside prompt
  mode, so the prompt never sees keys while an app runs. `handle_nova_os_app_keyboard`
  feeds each key to the app's `NovaOsAppRuntime::handle_key`, which can return
  `Exit`. It only processes keys on frames where the SAME app was already live
  last frame (tracked in a `Local`); every transition frame - the launch itself,
  an app switch, or a Tab that reopens the computer onto a persisted app - drops
  the event buffer, so the launching keystroke (e.g. the Enter that submitted
  `map`) never bleeds into the app it just opened (review finding, regression-pinned
  by `nova_os_launch_keystroke_does_not_bleed_into_the_app`).
- Exit: chrome close control (an `On<Activate>` observer) and Escape both call
  `NovaOsTerminal::exit_app`, which just flips the mode back to `Prompt`; the
  untouched scrollback is what "restores" the terminal.
- Teardown: `remove_drawer` -> `reset_session` already resets `active_mode` to
  `Prompt` and the scrollback to the welcome block, so a new player ship never
  inherits a stale app.

## App-as-plugin seam

Per `DECISION.md`, apps are trait objects: `NovaOsAppRuntime` (`id`, `title`,
`summary`, `spawn_body`, optional `handle_key`) registered into a
`NovaOsAppRegistry` resource as `Box<dyn ...>`. The drawer owns the generic parts
(mode, input gating, chrome, uniform exit); a future `map`/`ship viewer` registers
its own runtime and spawns arbitrary UI into the body slot without editing this
module. No production app registers yet - this task ships the runtime, and the
sample app is `#[cfg(test)]` only, so `NovaOsAppRegistry::register` is
`#[allow(dead_code)]` outside tests until the map task lands.

## Input decisions

Escape is context-sensitive (owner directive, overriding the spike's
"Escape stays drawer-close"): in app mode it exits the app, at the prompt it
closes the computer. The load-bearing subtlety is that Escape was read by both a
would-be app-exit handler and the drawer-close handler in the same frame, which
would exit the app AND close the drawer on one press. The fix is to interpret
Escape (and gamepad Start) in exactly ONE place - `close_drawer_from_menu_keys`,
which now branches on `active_mode` - so there is a single read and no ordering
race. `Ctrl+C`/`Ctrl+[` were dropped; `Ctrl+[` is the ANSI escape sequence for
Escape and would have reintroduced the same collision.

App launch words are mirrored into the terminal (`sync_nova_os_app_commands`) so
Tab completion, the inline completion ghost, parse-status colouring, `help` and
the did-you-mean suggestions all treat them as first-class commands (the ghost
and did-you-mean were completed in the review round), instead of the registry
being consulted from every terminal method. The mirror
reads through the `ResMut` `Deref` and only assigns when the set actually changed,
so it does not thrash `rebuild_terminal_ui`.

## Difficulties

- The task's DoD verify command was malformed: `cargo test -p nova_gameplay drawer
  terminal` - `cargo test` takes a single positional filter and rejects the
  second word. Corrected to `... -- drawer terminal` (both filters passed through
  to libtest). All terminal tests live under `hud::drawer::tests`, so `drawer`
  alone is already a superset; the `--` form just honours the stated intent. 54
  pass.
- `NovaOsAppRegistry::register` tripped `dead_code` because no production app
  registers yet; annotated as the future seam rather than deleted.

## Self-reflection

Handling one physical key in one system, keyed on state, is far more robust than
two systems trying to cooperate over `ButtonInput` edges - the same lesson the
prior drawer task learned splitting Tab. When a new binding is context-sensitive,
find the existing single owner of that key and branch there rather than adding a
second reader. Next time, sanity-check a plan's proof command for shape (arity,
flags) at plan time, not at verify time - a malformed `cmd:` proof is a silent
gate that only bites when run.
