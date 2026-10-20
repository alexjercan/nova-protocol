# Epic: located jobs on the Map

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.16.0,epic,jobs

## User facts
- Stage 4 of the game expansion. Owner direction 2026-10-12: stations and ships post jobs; jobs live on the Map; accept and hand in only while docked; held jobs stay visible undocked. Depends on the world-first strand's open-world events and addressable generated bodies.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Jobs:` title prefix and the `jobs` tag.
1. Job board on a station and job state in the save.
2. Job types: recover, escort, distress call, neutralize a named hull.

## Done when
- A player takes a job at one station, completes it in the world, hands it in at the same or another station, and the job survives a reload mid-way.

## Children
- `20261012-100023` Jobs: job board on a station and job state in the save
- `20261012-100024` Jobs: recover, escort, distress call and neutralize a named hull

## Owner feedback 2026-10-16
- Jobs could live in the "Station" tab of the HUD instead of (or as well as) the Map.
- Jobs should fit the start role the player chose: an industrial start gets carry-items and mine-something jobs; a bounty hunter start gets destroy-an-enemy-or-neutral-ship and scavenge-a-dead-civilization-ship jobs; pick by proximity.

## Owner feedback 2026-10-17 (UI, ideas only, nothing implemented)
The Station pane's job lists (`crates/nova_interface/src/station/jobs.rs`) are
text: a title, a "Reward N cr, M min left" line, a red refusal sentence and a
button per row. Ideas, cheapest first, all on widgets `nova_ui` already has:
1. One-line rows with aligned columns, as the Inventory table aligns Item and
   Qty: kind glyph | title | reward | target sector or distance | time left |
   action. Add a `JobType` icon set to `icons.rs` beside the section family
   icons (Bring, Recover, Escort, Answer, Neutralize) so a board scans by
   shape, not by reading.
2. Time left as a shrinking `slider_track` bar or a `badge` that turns Amber
   under a threshold, not a sentence. An expired offer greys and collapses to
   one line instead of a red refusal.
3. Refusals as a `badge` with the sentence in a `Tooltip`, not red prose under
   every row. A row that cannot act shows a greyed button with the tooltip.
4. Held-job progress as `have / need` with a meter for Bring and Recover, a
   `DONE` badge once the world marks the job done, a `HAND IN HERE` badge when
   the docked station is the target.
5. Select-and-inspect, as the Inventory pane does: click a row and a
   right-hand inspector shows `Job::task()`, the poster, the target, the
   hand-in station and a `Show on Map` button that opens the chart on the
   `JOB-n` mark. Rows stay one line; the prose moves to the inspector.
6. Filter chips over the board (All | Bring | Recover | Escort | Answer |
   Neutralize) with `segmented`, and a `FITS YOUR ROLE` badge on the jobs the
   start role weights, so an industrial start sees its carry jobs first.
7. Track one held job: a `Track` toggle on the row posts an objective chip on
   the HUD (`nova_hud::objective_stack`) with the target and the distance, so
   the job is visible in flight without opening the interface.
8. Hand-in feedback: the 2.5 s note line is easy to miss; a toast with the
   credit delta and the new balance, in the Inventory trade note style.
9. Empty state: "No jobs posted, next posting in N min" from `JobPostings`
   instead of hiding the whole block.

## Review 2026-10-17 (unpushed range to 0763909fe, read at HEAD)
- likely bug: `escort_on_arrival` (`crates/nova_world_base/src/jobs.rs:736-749`)
  drops the job on a hauler loss even after `mark_done`; raiders wait at 60%
  of the leg inside the 6 km leash, so they follow the hauler to the mark and
  kill it after arrival. The reward is lost, the board re-offers it, and a kill
  on the Hand in frame panics `apply_station_job_commands`
  (`station/jobs.rs:484-486`). Fix: gate the drop on `!jobs.is_done`.
- likely bug: only Escort has a drop path. A held Recover whose wreck dies, a
  held genuine Distress whose caller dies, or any held job whose poster turns
  derelict is refused `NotDone` or `WrongStation` forever and pins its slot
  (`jobs.rs:204`), so the slot never renews (`jobs.rs:651-701,762-793`).
- likely bug (narrow): job-spawned raiders and the trap caller are Enemy hulls
  under the poster's root, so `job_field` (`jobs.rs:315-326`) offers them as
  Neutralize targets on neighbouring boards; a renewal or hauler loss
  `try_despawn`s them without an event, so that Neutralize is never done.
  Fix: exclude every held job's `spawned_bodies()` from the field.
- nit: the board after a Load is not the board before it: `taken` is filtered
  before the draws and `swap_remove` reorders the pool
  (`nova_gameplay/src/jobs.rs:888-899,951,984`); the module docs promise the
  same board.
- nit: "Your jobs" only builds inside `if let Some(partner)`
  (`station/jobs.rs:301-329`), so held jobs hide at a derelict, owned or
  no-Jobs station; the docs say any station.
- nit: `ship_stock` `None` panics at accept (`jobs.rs:416-427`) where the
  sector path fails the sector instead; seed-dependent.
- nit: `tasks/20261012-100023/TASK.md` still says "Expiry: none" and "save
  format 12".
- Sound: save format 19 validation and the Load order, expiry race on click,
  hand-in moves and refusals, admission of a neutralized partner, 3 slots,
  role mix weights, 27-cell field, trap odds, raider count and leash,
  `TargetGone` and `NotDone`, `Jobs.completed: Vec<Job>`.

## Owner feedback 2026-10-17 (round 2, jobs UI and gating)
- Kind glyphs as text marks are not helpful. Use generated icons (the game
  draws icons as procedural masks in `crates/nova_interface/src/icons.rs`;
  add job-kind icons there).
- Keep every column. Time left is the meter only, no text and no badge. An
  expired offer is not shown at all. (Superseded 2026-10-20: the row shows
  `N min` beside a shorter meter, as the inspector does.)
- Drop the Track idea; nothing backs it.
- Held jobs must stay visible after undocking: a Jobs pane in the interface
  that works undocked, with the board only while docked.
- Drop the FITS YOUR ROLE badge: the player can buy or refit another ship,
  so the class is not a fixed trait.
- Gating instead: for the first jobs the stations near the start post only
  jobs the player's class can do; after a timer, or after an upgrade, every
  job opens; stations past a radius from the start post any job. The goal is
  an easy early progression from jobs.
- Agent suggestion (open): gate on the current hull's capability instead of
  the class and a timer: Bring needs hold mass for the stack, Recover needs
  a docking clamp, Neutralize needs a weapon, Escort needs a weapon, Answer
  needs a clamp; a starter station hides what the hull cannot do and a core
  station shows all with a greyed Accept and the reason. An upgrade opens
  jobs by itself, no timer or class table to tune. Near-start stations post
  small counts and close targets; farther stations post harder jobs.

## Owner idea 2026-10-17 (starting station)
- A new world starts docked: the generator finds the closest station of an
  allied civilization to the start point and moves the start there, so the
  player's ship begins docked at it. That station is "the starting station":
  only it posts the class-specific starter jobs; every other station posts
  normal jobs. See the wizard epic `20261012-100008` for the placement side.

## Owner feedback 2026-10-17 (round 3) and next up in the game
- Jobs get their own interface pane: undocked shows only the assigned jobs;
  docked adds the jobs available to take. The current job kinds are a proof
  of concept; the owner has plans for them, do not get attached.
- Next up (game changes this epic owns):
  - `InterfacePaneType::Jobs` in `crates/nova_interface/src/pane.rs`, always
    available; move the lists out of `station/jobs.rs` into a `jobs` pane:
    "Your jobs" undocked and docked, "Jobs posted here" docked only; the
    Station pane keeps the rest. Update `next_interface_pane`, the tab bar,
    the footer key hints, `wiki/interface.md` and `wiki/keybinds.md`.
  - Rows as the gallery shows: kind icon | title | reward | target sector and
    distance | time-left meter | Accept; expired offers are not listed; held
    rows carry a `have / need` meter, HAND IN HERE or "hand in at X", a greyed
    Hand in with the refusal as a tooltip; an inspector with the task text,
    poster, target, hand-in station, reward and Show on Map.
  - Job-kind icons as procedural masks in `crates/nova_interface/src/icons.rs`.
  - Starter-station gating: only the starting station (see the owner idea
    above and the wizard epic) posts gated jobs; rule to decide: class plus
    timer plus radius (owner) or hull capability (agent suggestion).
  - The three jobs review bugs (`20261017-202136`, `-202139`, `-202141`)
    land first.

## Decision 2026-10-18 (starter gating)
- Rule taken: the starting station's board (`StarterBoardMarker`) lists only
  the jobs the player's current hull can do, judged by `plan_job_hull_fit`
  in `crates/nova_gameplay/src/jobs.rs`: a Procure needs hold capacity for
  the stack, a Recover and a Distress need a live docking section, an
  Escort and a Neutralize need a live weapon section (turret, torpedo bay
  or railgun). Every other board lists every job and greys Accept with the
  hull's reason in a tooltip. No timer, no radius: the gate follows a refit
  at once and needs no saved state.
- Owner's alternative, not taken: class plus timer plus radius (the start
  role's class picks the kinds, a play-time window opens the rest, farther
  stations post harder jobs). It needs a clock and a radius in the save and
  gates a hull that was refitted to do the work. Reopen it if the kinds get
  the owner's planned rework and class matters more than hardware.

## Progress 2026-10-18 (Jobs pane)
- Claim: `InterfacePaneType::Jobs` is always on the tab bar; "Your jobs"
  shows docked and undocked, "Jobs posted here" docked only. The Station
  pane no longer lists jobs. Rows, inspector, Show on Map, kind icons and
  the starter gate follow the round 3 notes and
  `examples/playable/ui_gallery/jobs.rs`.
- Evidence: pane enum and tab order `crates/nova_interface/src/pane.rs:79`,
  `:100`; listing gate `crates/nova_interface/src/jobs.rs:371`
  (`board_listing`); pane build `jobs.rs:486`, board `:976`, held `:1053`,
  inspector `:1142`, Show on Map `:1269`, accept and hand-in `:1327`; hull
  gate `crates/nova_gameplay/src/jobs.rs:1135` (`plan_job_hull_fit`); chart
  hand-off `crates/nova_interface/src/map/chart.rs:348`
  (`MapRuntime::chart_show`); job-kind masks
  `crates/nova_interface/src/icons.rs:62` (`JobIconType`).
- Change: new `crates/nova_interface/src/jobs.rs` replaces the deleted
  `station/jobs.rs`; `JobAcceptCommand` and `JobHandInCommand` move with
  it. `StarterBoardMarker` is now the whole gate: the board draw is the same
  everywhere and the pane hides unfit jobs on that one board.
- Verification: `nova_gameplay` test
  `the_hull_gate_refuses_each_kind_the_hull_cannot_do_and_passes_a_fit_hull`;
  `nova_interface` tests
  `the_starter_board_hides_a_job_the_hull_cannot_do_and_a_core_board_greys_its_accept`,
  `a_held_row_measures_the_stack_in_the_hold_and_a_located_job_by_done`,
  `a_cell_distance_reads_this_sector_or_whole_kilometres_from_the_player_s_cell`,
  `the_next_tab_key_steps_through_jobs_always_and_station_only_while_docked`.
  `job_board_sandbox` with `NOVA_AUTOPILOT=1` walks starter board (2 of 3
  listed, the transport's escort hidden), select, accept, undock, held row,
  Show on Map (`JOB-1` selected, HELD JOB), redock, HAND IN HERE, hand-in;
  captures `sandbox-jobs-{starter,board,held,map,hand-in,done}.png` read.
- Gap: seed 10 has no second job-posting station in the window, so the
  greyed Accept on a non-starter board has unit proof only, no capture.
