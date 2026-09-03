# Review the unpushed v0.13.0 work: console, railgun, settings, meters, polish

- STATUS: OPEN
- PRIORITY: 20
- TAGS: v0.13.0,review


Review the unpushed commits on `master` (`origin/master..HEAD`, 45 commits,
HEAD `340d54ad`) in five logical groups, one `nova-review` pass per group,
without the play lanes. The owner already ran the play and performance
measurement passes; this session judges code quality and potential bugs.

## Groups

| Group | Commits | Code lines |
|-|-|-|
| commands | `fe92322a` | ~6.7k |
| railgun | `afa27e08`, `65d9d0e7`, `e1e5bc73`, `532c1ef8`, `f9c8aa56` | ~5.7k |
| settings | `e920c49e`, `e9c9e3c6` | ~1.6k |
| meters | `fe92322a..540f5834`, plus `f3952cf8`, `40c068d2` | ~11k |
| polish | `b6aa4289`, `9c6fa3d7`, `5287cdf5`, `718ebfd2`, `2f5a8c75`, `8882ec39` | ~4.4k |

Task-record-only commits (`61f67a82`, `0791c1fc`, `bc7b7065`, `fec6e441`,
`437bd965`, `beb6b73c`, `e65b5e07`, `3384e919`, `d8053c78`, `0957bbc1`,
`540f5834`) and the merge `340d54ad` are not reviewed.

## Lanes

Craft, performance (static, no measurement slot), correctness, contracts.
Red team and feel skipped on the owner's instruction.

## Plan

1. Dispatch the four lanes per group; adjudicate in the main session.
2. Record every adjudicated finding in `REVIEW.md`, tagged `easy` or
   `decision`. No fixes in this session (owner's decision, 2026-09-03).
   Done: 56 findings (1 blocker, 16 major, 39 minor), seven decisions
   collected at the end of the review.
3. Owner picks the findings to act on; fixes are a separate step.

## Log

- 2026-09-03 00:10 Dispatched 20 lanes. Eleven completed: craft x5
  (commands, railgun, settings, meters, polish), performance x5, contracts
  (settings). Nine died on a session rate limit (HTTP 429, resets 04:00
  Europe/Bucharest): correctness x5, contracts (commands, railgun, meters,
  polish). Lane reports are in the session scratchpad; the adjudicated
  findings go in `REVIEW.md`.
- 2026-09-03 00:40 The dead lanes' transcripts survived with tool calls and
  results but redacted reasoning and no report. Rebuilt each as a history file
  (commands run + result excerpts) and re-ran the nine lanes three at a time,
  each seeded with its predecessor's history and its group's finished sibling
  reports. Batch 1: meters, commands, settings correctness. Batch 2: railgun,
  polish correctness + meters contracts. Batch 3: commands, railgun, polish
  contracts.
- 2026-09-03 02:30 All nine retried lanes reported. Adjudicated in the main
  session: dropped nothing as ungrounded, merged the duplicate changelog
  finding (two lanes), demoted the lazy wake build from MAJOR to a
  decision, re-derived every BLOCKER and MAJOR claim in the tree before it
  reached the verdict (see `REVIEW.md`, "Verified"). Verdict REQUEST
  CHANGES: 1 blocker (portal mod versions unchanged over x10 content), 16
  major, 39 minor. One contradiction resolved during adjudication: the
  cockpit loop cuts under a hold that the code says pauses its clock
  because `with_game_plugins` apps never carry the menu plugin that
  installs the hold.
- 2026-09-03 09:00 Fixed the commands group (R1.1-R1.19) on `master` at the
  owner's direction, together with two live bugs the owner reported: `:`
  left `cmd>` the active shell for the rest of the run, so Tab reopened the
  command shell instead of NOVA OS, and Escape out of a shell opened over
  flight unpaused whatever it had covered. Both share one cause - neither
  entry point NAMED the shell it wanted - and both are now driven by the new
  `system_command_shell` range (R1.16). Owner's answers to the decisions are
  recorded at the end of `REVIEW.md`; R1.32 is still open.
- 2026-09-03 09:30 Fixed the railgun group (R1.20-R1.30). The rake's arming
  mark now lives in the round's own flight clock, which is what the step
  boundary was losing; the wake is handed the segment its slug died on; the
  first lance of a session no longer builds its graphs on the frame it fires.
  R1.26 stands as the owner decided: two instances per shot, revisit only on a
  measured spike.
- 2026-09-03 10:15 Fixed the settings group (R1.31-R1.39). The store now
  carries its own root (`SettingsStoreRoot`, `None` = the platform store), so
  no test moves the process-wide `NOVA_CONFIG_ROOT` and the flake it caused is
  0 of 60 where it was 183 of 300; the store's systems moved out of the panel
  module into `settings_store.rs`, which had been importing them back. The
  bench and `perf_web` now declare their stores inert, and the rebind range
  asserts that gate instead of asserting its vacuous consequence.
  `system_headless_drag` grew a MOUSE beat that reads the slider, the resource
  and the rig's live `Scale`. R1.32's load/save split stays open; R1.39 is
  accepted with a NOTE, as the owner asked.
- 2026-09-03 10:55 Fixed the meters group (R1.40-R1.49). The blocker first:
  The Ledger 1.27.0, Gauntlet Run 1.11.0 and the example mod 1.2.0 republish
  the x10 content under versions the portal's string compare will offer as an
  Update, each with its own changelog entry (the-ledger's missing 1.26.0 line
  included). Hand-mirrored numbers now derive from the constants they were
  copies of - `BEACON_LOCK_SIGNATURE` and `AI_TORPEDO_MAX_RANGE` are public
  and typed, the shakedown pins read `GravitySettings` and `TargetingSettings`
  - and the console's two speed-cap readouts share one `cap_label`. The
  inspector no longer labels an unknown wrapper "m". Per the owner's answer to
  R1.49, weapon reach, muzzle speed and blast radius are derived into a
  `#[require]`d engine-figures component per weapon kind, refreshed only when
  the config changes, and the scripted camera pose keeps its every-frame write
  while losing its every-frame derive. R1.44's two example constants are left
  in world units on the finding's own rule: they are stage geometry.
- 2026-09-03 11:15 Fixed the polish group (R1.50-R1.56), which closes the
  review's list. The arena's replay line names every hull it fielded: the
  lobby stopped overwriting the stream head with its mint cursor, and the
  closing line prints `--seed <head>` plus one `--ship team:style:seed` per
  slot (verified live - re-running the printed line fields the same 151/204
  matchup). The run-level deadline counts `Time<Real>`, held by a test that
  pauses the game clock, and the docs now name the clock both timers read and
  the one place a loop capture makes them frames. `click_named` holds its aim
  instead of re-issuing it, the script is an `Arc<[Step]>`, and the four
  hand-spelled clicks across the fleet are `click_named` beats waiting on what
  the click was FOR. The three NOVA OS predicates live in `nova_debug::harness`
  once; `REACT_SECS`/`LOAD_SECS` are the fleet's own deadlines.
  Two defects the fleet runs turned up on the way, both fixed here and both
  recorded in REVIEW.md: R1.49's own regression left `ScriptedCameraTransform`
  on a camera whose pose was removed, so no script could aim it again
  (`screenshot_editor` stalled); and `system_field_controls` compared a
  meters-declared row step against the probe's world-unit pose.
