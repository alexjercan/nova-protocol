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
