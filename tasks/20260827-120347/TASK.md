# The command console: reach into the game by name

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.13.0,console,input,tooling

Split out of `20260820-174148`, which shipped named input and the process
channel in v0.12.0. This task adds one curated command language shared by an
in-game shell and that channel.

## Goal

Add a game-level Command shell to the existing NOVA OS CRT terminal emulator.
It can inspect the game, change player settings, and run a small documented set
of armed cheats. The process channel invokes the same parser and dispatcher.

This is not arbitrary `EventActionConfig` execution. Scenario actions remain an
authoring and implementation vocabulary. A new scenario action does not become
a command unless it is deliberately added to the command catalog.

## One emulator, two shells

The CRT is the terminal emulator. It owns the existing presentation and input
behavior:

- CRT casing, glass, animation, phosphor styling, sound, and pointer forwarding;
- scrollback, prompt, caret, completion ghost, and staged welcome rows;
- character input, Backspace, Delete, Home, End, history, Tab completion, and
  PageUp/PageDown scrolling;
- pause, cursor, open, and close behavior.

It hosts two shell languages:

- **NOVA OS**: the existing ship-related commands and apps. It still requires a
  player ship and opens with Tab.
- **Commands**: game-level inspection, settings, and cheats. It is available in
  the main menu, editor, gameplay, pause surfaces, and the CRT itself. It does
  not require a player ship.

Do not build a second console-looking overlay or duplicate the terminal editor.
Generalize the existing terminal into a shared emulator with shell-specific
registries, prompts, completion, welcome content, history, and scrollback.
Switching shells keeps the CRT open and the game paused; it does not replay the
open animation. Each shell restores its own transcript and history.

The NOVA OS prompt gains `commands`, which enters the Command shell. A user in a
NOVA OS app returns to its prompt first. Opening the Command shell directly uses
`Shift+Semicolon` (`:`) while no text field owns input. Inside the CRT or a
focused editor field, `:` remains text rather than a global shortcut.

## Command shell presentation

Use the existing CRT layout. The left header is:

```text
NOVA OS v0.13.0 // COMMANDS
```

Read the version from `nova_info::APP_VERSION`; never hard-code `v0.13.0`.
The right status is:

```text
CHEATS: OFF     LINK: LOCAL     FPS:  60
```

After arming, `CHEATS: ON` is amber. FPS uses the existing real-time diagnostic
path because virtual time is paused.

The first Command-shell entry reveals an introduction with the same timing and
skip-on-input behavior as the NOVA OS welcome block:

```text
NOVA OS v0.13.0 // COMMANDS
POST ......... command shell / ok
CORE ......... local game runtime / attached
REGISTRY ..... 25 commands / ready
WORLD ........ shakedown_run / paused
CHEATS ....... disabled / run clean
Hint: type `help` and press Enter.
```

The registry count is computed. The WORLD row describes the live context, for
example `main menu / idle`, `ship editor / paused`, a scenario id, or
`no scenario / idle`. After arming, the final row reads
`CHEATS ....... enabled / run marked`. `clear` restores this shell's current
introduction. The prompt is:

```text
cmd> _
```

## Pause and surface behavior

Opening the CRT in the Command shell pauses gameplay and physics. This is the
same fairness rule as NOVA OS: entering and reading commands must not let the
world run away from the player.

Preserve the underlying surface. Closing Commands returns to the main menu,
editor, gameplay, pause menu, outcome, or NOVA OS state that was underneath it.
Do not destroy or reboot a NOVA OS session merely because Commands was opened
from it. The CRT owns the freeze; switching shells does not unpause and pause it
again. Coordinate pause ownership rather than relying on the current
unconditional `unpause_clocks` exit hook.

The shell owns keyboard input and Escape while open. A close must not also
activate the pause menu or an underlying editor action. Commands are unavailable
while the world is in an incomplete loading transition.

## Public vocabulary

There is no public `action` console command and no channel `Action` lane.
`EventActionConfig` remains the scenario enum and may be used behind a command.
The channel's reserved `action` key stays refused, but its stale task message is
replaced with a permanent error directing callers to `command`.

The channel carries the exact same command text as the CRT:

```json
{"tick":120,"command":"graphics low"}
{"tick":121,"command":"ammo infinite player on"}
```

Both front ends receive the same structured command result. Channel
acknowledgements report the command, class, and result, not its internal scenario
action.

Every registered command supplies one source of metadata for parsing, help,
completion, and documentation: name, usage, summary, class, and executor. Every
command is documented. Cheats have their own catalog section.

## Command classes

- **Utility** controls the shell or abandons one scenario to load another.
- **ReadOnly** observes state and never mutates it.
- **Setting** changes the same persisted player settings as the settings UI and
  never marks the run.
- **Cheat** changes the live world, requires arming, and is unavailable until
  the player deliberately runs `cheats enable`.

`cfg(debug)` is not a gate. The channel and shell vocabulary must not differ
between debug and release builds.

Arming marks the current run immediately and irreversibly. A fresh scenario is
a fresh run: `scenario load <id>` abandons the old attempt without assigning an
outcome or advancing campaign progress, then resets the cheat mark and arming.
Scenario creative-map classification remains separate: authored scenario
injection never accuses the player, but the linter computes and reports that
content classification.

## Initial command catalog

| Command | Class | Behavior |
| --- | --- | --- |
| `help` | Utility | Show basic usage. |
| `help <command>` | Utility | Show usage, class, arguments, and examples. |
| `commands [class]` | Utility | List all commands or one class. |
| `clear` | Utility | Restore the Command-shell introduction. |
| `close` | Utility | Close the CRT. |
| `scenario load <id>` | Utility | Abandon the current attempt and load a fresh scenario without an outcome or campaign advance. |
| `status` | ReadOnly | Show a compact run and world summary. |
| `scenario` | ReadOnly | Show the current scenario, state, and outcome. |
| `ships` | ReadOnly | List live ships by id. |
| `ship <id>` | ReadOnly | Inspect one ship. |
| `sections <ship-id>` | ReadOnly | List a ship's sections. |
| `section <id>` | ReadOnly | Inspect one section. |
| `objectives` | ReadOnly | List current objectives and completion state. |
| `variables` | ReadOnly | List scenario variables. |
| `variable <name>` | ReadOnly | Read one scenario variable. |
| `bindings` | ReadOnly | List registered input actions and current bindings. |
| `bindings <action>` | ReadOnly | Inspect one action's bindings and context. |
| `settings` | ReadOnly | Show all current settings. |
| `cheats status` | ReadOnly | Report arming and run-mark state. |
| `graphics [low\|medium\|high]` | Setting | With no value, print the current quality; with one, change it. |
| `volume [channel [0..1]]` | Setting | With no arguments, print all channels; with a channel, print it; with a value, change it. Channels are `master`, `music`, `world`, and `interface`. |
| `window [windowed\|borderless]` | Setting | With no value, print the current mode; with one, change it. |
| `bind <action> <source>` | Setting | Rebind through `InputBindings::rebind`. |
| `bind reset <action>` | Setting | Restore the registered default. |
| `cheats enable` | Cheat | Arm cheats and mark the current run. This command is the deliberate arming act and needs no prior arming. |
| `ammo infinite <ship-id> <on\|off>` | Cheat | Enable or disable unlimited ammunition. |
| `ammo refill <ship-id>` | Cheat | Refill every finite magazine on one ship. |
| `ammo refill section <section-id>` | Cheat | Refill one finite magazine. |
| `speed-cap <ship-id> <number\|off>` | Cheat | Change or remove a ship's manual speed cap. |

The setting grammar follows one rule: the command without a value reads the
current setting; adding a value changes it.

## Deliberately not exposed

Do not add these in the initial catalog:

- arbitrary serialized `EventActionConfig` values;
- `win`, `lose`, or any direct `Outcome` command;
- scenario-variable writes;
- arbitrary spawn or despawn;
- allegiance changes;
- forced weapon launches;
- controller-verb changes.

They can be considered individually later. Their existence as scenario actions
is not an argument for exposing them.

## Internal action work

`ammo infinite` replaces the `infinite_ammo` field on
`SpaceshipController::Player`. No shipped scenario enables that field: the six
base content files and the example mod pass `false`; only debug-feature examples
use `true`.

The replacement must operate on live section entities. Enabling unlimited ammo
removes the finite `SectionAmmo` behavior; disabling it must restore a coherent
magazine and reload state rather than inventing accidental state. Settle and
record the exact restoration rule before implementation. Port examples away
from the controller field.

`ammo refill` should also use a typed internal scenario action so authored
training or story content can request the same operation. `speed-cap` maps to
`SetSpeedCap`. Class every internal action that injects world state for the
creative-map lint; recompute the catalog count after adding ammo actions rather
than preserving the old 8-of-26 count.

The RUN mark and SCENARIO classification remain different subjects:

- shell/channel cheat command: requires arming and marks the current run;
- scenario/mod action: does not mark the player, but may classify the scenario
  as a creative map;
- input command: remains in the existing input lane and goes through gameplay
  rules;
- Utility, ReadOnly, and Setting commands: never mark the run.

## Implementation sequence

1. Extract or generalize the CRT emulator state and presentation without
   changing the existing NOVA OS behavior.
2. Add shell selection, separate histories/scrollback, the Commands header and
   welcome block, direct `:` opening, and the NOVA OS `commands` switch.
3. Coordinate pause ownership and prove every supported underlying surface is
   restored after close.
4. Add the command metadata registry, parser, help, completion, structured
   results, and the Utility/ReadOnly commands.
5. Add Setting commands through the live persisted setting resources.
6. Add run arming and marking, then the bounded Cheat catalog and internal ammo
   actions.
7. Replace the channel's reserved `command` refusal with dispatch through the
   same parser. Keep `action` permanently refused.
8. Update the command reference, scenario action reference, player/developer
   documentation, changelog, and task proof.

## Open decisions to settle before implementation

- Exact Escape/`exit` behavior when Commands was entered from NOVA OS rather
  than opened directly.
- The state retained when `ammo infinite` is turned off: restore the exact
  suspended magazine/reload state, restore authored capacity full, or another
  explicit rule.
- Crate ownership for the shared emulator and high-level command dispatcher;
  preserve the dependency direction and do not make low-level NOVA OS logic
  depend on gameplay, scenario, and menu crates.

## Done when

- The same command text and result contract works through the CRT and channel.
- The CRT switches between NOVA OS and Commands without duplicated editing code,
  lost shell state, animation replay, or a clock unpause.
- Commands opens and closes over every supported surface and restores it.
- Help and completion derive from the command registry and cover every command.
- Read-only output identifies unknown and ambiguous runtime ids clearly.
- Setting commands update the live resources, settings UI, and persisted store.
- Cheats are refused before arming; arming marks the run one-way; a fresh
  scenario resets the mark.
- Unlimited ammo can be enabled and disabled on live ships without corrupting
  magazine or reload state, and the controller config field is gone.
- Creative-map lint classification covers every authored injection action
  without classifying bookkeeping and presentation actions as injection.
- Focused unit tests and driven CRT/channel ranges pass through `nix develop`.
- The CRT presentation, command output, settings persistence, run mark, and
  representative channel acknowledgements are inspected with commit-bound proof
  kept under this task.

## Depends on

`20260820-174148` landed the transport, input registry, snapshots, and reserved
`action`/`command` keys in v0.12.0. Scheduled into v0.13.0 in the 2026-08-31
planning round.
