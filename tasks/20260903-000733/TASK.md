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
