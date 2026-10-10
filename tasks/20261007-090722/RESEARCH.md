# v0.16.0 gameplay polish research (read-only)

This is the source audit at the base revision below, not a description of current
master. The post-audit delivery status and remaining proof gaps are in section 8.
Section 9 traces the remaining flows at `2febf1b49` and ranks new candidates.

Base: `master` at `cb9ab4c66`, after #110 (`b9f2b84f5`, frozen sectors) and
#111 (`b210fee0c`, saved worlds with Load). Static code and evidence audit
only. No cargo, game, probe, bench or GPU run. No code, content or docs
change. Claims are labelled **code-backed** (read in source at the cited
lines), **observed** (seen in a rendered frame or a recorded audit), or
**hypothesis** (not yet shown).

## 1. Stale feature matrix (`tasks/20261007-090751/TASK.md`)

Not edited. After #110/#111 these rows are wrong:

| Row | Matrix says | HEAD |
| --- | --- | --- |
| New Game | "there is no Load/Continue yet"; cites `world_setup.rs:171-230` | Create names a world (`crates/nova_menu/src/world_setup.rs:92-230`, `:318-364`). Load lists, refuses and deletes worlds (`crates/nova_menu/src/load_screen.rs`). Pause Retry reads "Load last save" (`crates/nova_menu/src/pause.rs:404-414`). Desktop only. |
| Sector streaming | "pristine revisit is explicit in `nova_world_base/src/lib.rs:27-30`" | A retired sector is frozen and comes back as left, and the world saves to disk (`crates/nova_world_base/src/lib.rs:27-34`). |
| Provisional order | item 2 "persist a chosen world" | Delivered; task `20261007-090756` is CLOSED. |
| Mining | "proved carve pulses but not ore pickup" | Still true for real New Game. `system_world_resume` injects a canister as disclosed fixture state (`examples/systems/system_world_resume.rs:4-13`). |

## 2. Journey inventory (real New Game)

| Step | Entry and control | Preconditions | Result UI and error policy |
| --- | --- | --- | --- |
| Create | Menu New Game, name and seed fields, Create | Name 1-32 of `[A-Za-z0-9 _-]` (`crates/nova_world_base/src/save/mod.rs:845-864`); seed u32 (`world_setup.rs:42`) | Inline field error, Create disabled (`world_setup.rs:236-296`); taken name refused by Create (`:318-364`). Desktop only; web shows a note (`:184-193`). |
| Spawn | Open-world bootstrap: player at origin, no objective, no outcome (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:1-8,36-53`) | - | Stock 12 HullPlate, 6000 PdcRound, 20 RailSlug, 12 Torpedo, 2000 cr (`open_world.rs:69-81`). Bindings: PDC LMB, railgun R, torpedo F, mining V (`:86-123`). |
| Save | Automatic on sector crossing and on leave (`crates/nova_world_base/src/save/session.rs:44-49,284-305`) | Sector edge 32 km (`crates/nova_world_base/src/lib.rs:139`) | Status line "World saved" etc. Owner decisions D1-D5 in `tasks/20261007-090756/GATE.md:46-90`. Death writes nothing (`GATE.md:22`). |
| Load | Menu Load row; pause "Load last save" | Lock, format, content digest (wiki table in `web/src/wiki/getting-started.md`, "A refused Load") | Refusal reason on the row; Load last save failure goes to the FAILED TO START report (`crates/nova_menu/src/leave.rs:159-169`). |
| Travel | Ctrl lock, G GOTO, X STOP, O ORBIT, Z cancel | Lock dwell | Keybind dock: STOP GOTO ORBIT CANCEL RADAR COMPONENT RCS DOCK HELM only (`crates/nova_hud/src/keybind_dock.rs:87-97`). |
| Mine | Hold V on a travel-locked rock inside 100 m (lesson text `crates/nova_authoring/src/base_content/lessons.rs:471-489`) | Nose on the rock | Refusals are log-only (`crates/nova_scenario/src/mining.rs:488-490`). No HUD element names mining (`crates/nova_hud/src` has no mining reference). The key is listed in the Ship pane and Settings (`crates/nova_interface/src/ship/sections.rs:293`, `crates/nova_menu/src/settings.rs:1186`). |
| Pick up | Fly the top-deck intake door onto a canister (`lessons.rs:490-508`) | Free hold mass | Pickup sight line (`crates/nova_hud/src/pickup_sight.rs:175-227`); door opens on detection even if a full hold prevents pickup, with no capacity-specific cue. See candidate 2. |
| Trade, take | Dock (D) with a calm living generated ship or a derelict; TAB Inventory | Admission refuses Hostile/fighting (CHANGELOG 0.15.0) | Confirm moves all or nothing (`lessons.rs:1155-1173`). |
| Repair | TAB Ship pane, plates count, Repair | Section alive and not disabled; exact plate count (`crates/nova_gameplay/src/inventory.rs:793-823`) | Refusal text in the pane note (`crates/nova_interface/src/ship/app.rs:240-269`). A destroyed or disabled section can never be repaired (`inventory.rs:801-803`). |
| Death | Player root removed | - | See candidate 1. |

## 3. Evidence audit

| Evidence | Revision, date | Flow | Kind | What it shows |
| --- | --- | --- | --- | --- |
| `bench-runs/34dd96176/tutorial/pi-gpt-5.6-sol-medium-20260921-080202/score.json` | `34dd96176`, 2026-09-21 | Basic Training | Authored scenario, LLM pilot | Defeat at "live" (two drones); 9 of 10 objectives, 1720 damage, 0 kills. Pilot skill, not player friction. Pre-0.15.0. |
| `tasks/20260824-125933/tutorial-play.md` | `e6aea2d99`, 2026-09-07 | Basic Training | Authored scenario, LLM pilot transcript | Pilot prose reports waiting out the opening cinematic and an early STOP tap (`[74.1]`) that did not count before the objective went live (`[87.8]`). Pilot claims, not observed player friction. Historical; tutorial only. |
| `tasks/20260926-174806/m3-sol-bench-evidence.md` | `16da6afbc`, 2026-09-26 | Dock capture, PDC fire, jettison attempt | Bench fixtures, LLM pilot | Dock and fire observed in world state. Jettison and inventory not established; the bench then had no inventory view. HEAD now has one (`crates/nova_bench/src/observation.rs:81-176`). |
| `tasks/20260925-190131/TASK.md:68-90` and its PNGs | 2026-10-02 | New Game seed 4426 via real menu; controlled `world_encounters` | Real New Game (player placed, not flown) plus controlled example | Enemy fired on the player; three side markers rendered. Take after boarding by ECS assertion. Streamed Take/dock/boarding not verified. |
| `tasks/20261007-090756/p6-frames/*.png` | `655100c29`, 2026-10-09 | New Game via `system_world_resume` | Real menu path, disclosed fixture state | Status line, leave overlay, load screen render. The status-line frame shows a fresh spawn: empty space, three dock chips (STOP RADAR RCS), no objective. **Observed.** |
| `examples/systems/system_open_world.rs:1-25` | HEAD | New Game, fire every weapon, Retry, menu | Real menu path, autopilot | Session lifecycle. No death, mining, pickup or trade. |
| `crates/nova_bench/src/cli.rs:133-260` | HEAD | `bench play --session new-game` | Harness | A real New Game bench path exists. No retained New Game recording was found in `tasks/` or `bench-runs/` during this audit; this does not mean no earlier runs occurred. |
| `~/Videos/Recording/2026-10-09_*.mp4` | 2026-10-09 | - | Not Nova | Frames show a first-person interior from another project. Excluded. No `~/Videos/nova-bench-*` folder exists. |
| `tasks/20260926-132243` | CLOSED | Docking | - | Resolved per owner. Not reopened. |

No evidence at this audit shows a human player in New Game. No retained recording
of a complete New Game mine, pickup, sell, repair or death flow was found; earlier
New Game experiments do not establish those complete outcomes.

## 4. Ranked polish candidates

### 1. New Game death leaves a stripped world, and leaving then reports SAVE FAILED

- **Code-backed chain.**
  1. The bootstrap authors no defeat (`open_world.rs:36-53`); the outcome
     overlay only mirrors an authored outcome (`crates/nova_menu/src/outcome.rs:39-60`).
  2. Root removal swaps the camera to free WASD (`crates/nova_scenario/src/loader/lifecycle.rs:686-705`)
     and despawns the player HUD (`crates/nova_hud/src/lib.rs:600,705,767,847`).
  3. Zero players disarms the world (`crates/nova_world_base/src/lib.rs:252-260,328-332`);
     config removal runs `Cleanup` (`crates/nova_world/src/lib.rs:848-853`),
     which retires the sector roots.
  4. Esc, Back to Main Menu / Exit / window close requests a leave save
     (`leave.rs:66-77`). The session is spent on disarm
     (`session.rs:290-297`), so the save drops with
     `"the world ended; the last save is kept"` (`session.rs:325-331`) as a
     `Failed` status (`session.rs:189-192`). The overlay shows
     `SAVE FAILED` and `"{error}. The last save is kept."` (`leave.rs:193-197`),
     which repeats the sentence. Try again re-requests (`leave.rs:294-301`)
     and fails the same way; only Leave without saving exits.
  5. "Load last save" works: it writes nothing (`leave.rs:118-171`).
- **Observed:** nothing. **Hypothesis:** the player sees an empty sky with no
  HUD and no prompt until they find Esc. Needs a rendered frame.
- **No test** covers leaving after death (`crates/nova_menu/src/tests/leave.rs` test list).
- **Before:** silent void; the correct exit is labelled a failure.
  **After (options for the owner, none approved):**
  a. Author a Defeat outcome in the open-world bootstrap content, and make the
     outcome overlay's primary action "Load last save" in a saved world.
     Consequence: reuses the existing overlay; the overlay must learn saved
     worlds and web must keep reload-from-seed.
  b. Treat a dead player's leave as a no-save leave (no `SAVE FAILED`), and
     add only a prompt. Consequence: smaller; still two places own death UI.
  Recommendation: a, plus the dead-leave wording fix, because one modal then
  owns every end state.
- **Owning seam:** `nova_menu` (`outcome.rs`, `leave.rs`), `nova_world_base/src/save/session.rs`,
  open-world bootstrap builder in `nova_authoring` (regenerate base RON).
- **What dies:** the free-camera death state in New Game; `SAVE FAILED` after death.
- **May break:** `system_open_world` Retry beat; web open-world Retry;
  outcome-overlay queued-switch logic (`outcome.rs:44-56`); leave tests.
- **Fail loud:** an outcome with no defined action must not fall back to the
  menu silently; Load last save refusal stays the FAILED TO START report.
- **Proof proposal, not run:** first inspect existing evidence or use an
  existing executable with an isolated save root for a lethal New Game flow,
  and record its frame and leave message under this task folder. A disposable
  ECS test on `tests/leave.rs` would write code outside this folder and is
  **not authorized** during this read-only research. After an implementation
  gate, add a targeted test for the death/leave transition and save recovery.
- **Effort:** S to M (one to two days with proof).

### 2. The economy loop refuses silently, and New Game has no recorded run of it

- **Code-backed, silent refusals:**
  - Mining refuses for no lock, not an asteroid, barren rock, out of reach and
    off target (`crates/nova_scenario/src/mining.rs:160-175,504-540`). A refusal
    goes only to an `info!` log (`mining.rs:488-490`); the beam flash and the
    pulse sound both return early on a refusal (`mining.rs:799-801,838-840`).
    The player sees and hears nothing and cannot tell which rule failed.
  - Intake: `ready` needs trigger contact AND `free_g >= canister mass`
    (`crates/nova_ship/src/sections/cargo_intake_section.rs:574-595`). A
    canister too heavy for the hold stays out with no pickup event. The pickup
    sight selects pairs without consulting `ready`
    (`crates/nova_hud/src/pickup_sight.rs:175-227`), so its line does not
    indicate free room. Worse, a canister merely in detection range opens the
    door and triggers `CargoIntakeDoorMoved` regardless of hold capacity
    (`cargo_intake_section.rs:555-569`); this can give a misleading positive
    animation/audio cue before a capacity refusal. This is a code-backed
    outcome, not yet a rendered observation.
  - By contrast, inventory, trade and repair refusals are named on the pane
    note line (`crates/nova_interface/src/inventory/app.rs:1736-1748`,
    `crates/nova_interface/src/ship/app.rs:262-269`).
- **Existing policy:** the mining module explicitly documents refused pulses
  as silent and invisible (`mining.rs:16-23`). Surfacing reasons would change
  an intentional feedback policy, not repair an accidental missing effect.
- **Code-backed, discovery:** Lessons for DOCK, MINE, pickup, Take/Give,
  jettison and ship service carry `practice: None`
  (`crates/nova_authoring/src/base_content/lessons.rs:452-508,1097-1192`).
  Basic Training teaches flight and combat only, on another ship. The MINE
  lesson lists only `radar_hold` as its action (`lessons.rs:484`), so it shows
  no mining key. The flight HUD has no mining chip (`keybind_dock.rs:87-97`).
  Whether a docked ship trades is known only after docking and opening TAB
  (`inventory/app.rs:1095-1115`).
- **Hypothesis:** a first-time player holds V on a far or barren rock, gets
  nothing, and gives up. Not observed.
- **Before:** refusal is invisible. **After (direction only, not approved):**
  the existing `MiningPulse` refusal and the intake's room check reach the
  player through a defined feedback surface. Decide whether capacity should
  prevent door opening or retain the door cue with an explicit refusal. The
  owner picks the surface, rate limit, and feedback policy; none is approved.
- **Owning seam:** `nova_scenario::mining` (`MiningPulse` already carries the
  refusal, `mining.rs:491-494`), `nova_ship::sections::cargo_intake_section`
  (`CargoPickupPair` has one `ready` flag for two reasons), `nova_hud`.
- **What dies:** the log-only refusal path. **May break:** pickup-sight tests,
  mining audio tests, any probe reading `CargoPickupReadiness`. **Fail loud:**
  none new; refusals are valid runtime conditions.
- **Proof proposal, not run:** a bounded `nova-bench` New Game play with
  explicit world/gameplay seeds, goal: mine a rock, take a canister, dock and
  sell. Judge audit snapshots for canister and inventory/credit deltas
  (`observation.rs:81-176`), not pilot prose. For full-hold feedback, an ECS
  regression or isolated fixture would require a separate code-edit gate;
  source alone establishes the current door/readiness ordering.
- **Effort:** proof S (one bench run, lavapipe, LLM cost). Refusal cues S to M,
  with their own gate.

### 3. First minute in New Game has no orientation

- **Observed:** the status-line frame shows a fresh spawn with nothing in view
  and three chips (`tasks/20261007-090756/p6-frames/world_resume-status-line.png`).
- **Code-backed:** no objective (`open_world.rs:1-8`). The New Game profile
  names "derelicts" only, not traders or hostiles (`world_setup.rs:154-157`).
- **Hypothesis:** the player does not know where to go. Needs player
  observation; one frame cannot prove it.
- **Proof:** frames of a fresh spawn on three seeds through `loop_world_start`
  smoke mode, then owner judgement. Fix scope is a content or copy decision.
- **Effort:** proof S; fix S to M.

### 4. A crippled player has no recovery, and a crossing save can keep it

- **Code-backed:** the armed rule neutralizes a ship that loses every weapon
  or its last flight computer, with no player exclusion
  (`crates/nova_gameplay/src/integrity/neutralize.rs:1-13,118-135`); the marker
  is never removed (`:52-58`). The warship has three controllers and nine
  weapons (`crates/nova_authoring/src/base_content/ships/block.rs:423-478`).
  AI drops a neutralized target (`crates/nova_ship/src/input/ai/acquisition.rs:691-697`).
  Disabled sections cannot be repaired (`inventory.rs:801-803`). The save
  needs only a player marker (`session.rs:333-345`), and thaw restores
  `NeutralizedMarker` (`crates/nova_scenario/src/objects/spaceship.rs:1172-1173`).
- **Hypothesis:** a drifting crippled ship crosses a 32 km edge, the crossing
  save keeps it, and Load last save then resumes a ship that cannot fight or
  turn. Frequency unknown.
- **Decision class:** policy, not polish. Repairing a destroyed section is new
  behavior. Research and reproduce only.
- **Proof:** disposable ECS test: neutralize the player, cross a sector,
  reopen, assert `NeutralizedMarker` and disabled controllers.

### Minor, unranked

- **Superseded by `a4f244427`:** the name field now opens on `New World`
  (`crates/nova_menu/src/world_setup.rs:49-52,187`), so the empty-name state
  is gone. See section 9 for the taken-default consequence.
- **Code-backed:** repair refuses a plate count above need
  (`inventory.rs:816-818`) instead of clamping; the All button exists
  (`ship/app.rs:196-198`). Low impact.
- **Code-backed:** on web, pause Retry in the open world keeps the label
  "Retry" and reloads the bootstrap from the seed (`pause.rs:414-415,628-648`).
  This matches the shipped web policy (CHANGELOG `[Unreleased]`, Web &
  Platform). A copy question only.
- **Code-backed:** mined canisters carry `ScenarioScopedMarker`
  (`mining.rs:975-985`). Frozen sectors carry canisters since #110; no
  failure is known. Not a candidate.

## 5. Out of scope

- Save cadence (crossing and leave only) and "death writes nothing" are owner
  decisions (`GATE.md:14-22,71-78`). Mining progress lost to a death inside one
  sector is by design unless the owner reopens it.
- RON item identity (`20260930-100831`), gamepad and mobile, docking.

## 6. Limits

- No run of any kind. Every symptom above is unreproduced. An independent
  read-only review traced the death/save path, mining refusal, door/readiness
  split, Load label, saved neutralization marker and bench cargo observation.
  It found no blocker in candidates 1 or 2 and no rendered-player proof.
- Six read-only scouts ran. Their replies arrived after the first draft. Their
  claims were checked before use. Two were refuted:
  - "NeutralizedMarker is not restored on thaw": wrong;
    `crates/nova_scenario/src/objects/spaceship.rs:1172-1173` restores it.
  - "`world_resume_refusal.rs:178-197` shows a New Game Retry gap": wrong;
    the test proves that gap closed.
- Canisters have no expiry (`mining.rs:940-987`, `cargo_intake_section.rs`);
  scout claim, not refuted.
- Not traced at the base revision: hostile-encounter readability, player
  hit-direction feedback. Section 9 traces both at `2febf1b49`.

## 7. Bounded next proof plan (not executed)

1. P-A: no existing example kills a player in a saved native New Game.
   `examples/systems/system_open_world.rs:56-67` and the sandbox helper in
   `system_world_resume.rs:221-230` isolate world storage, but do not kill
   that player. `system_outcomes.rs:113` kills a player in an authored fixture
   without `WorldSaveSession`, so its primary action is scenario Retry, not
   `Load last save` (`crates/nova_menu/src/outcome.rs:98-107,176-190,223-230`).
   Recommend a separate, bounded `system_open_world_defeat` example using
   the real Create/Load route and an isolated save root, plus the production
   root-overkill method from `system_outcomes`; this needs a separate code
   gate and explicit authorization under the research-only task. Do not
   fold this into `system_open_world`, whose single subject is session
   lifecycle and weapon inputs. Render the death prompt and successful
   Load with authoritative save identity. No death-to-Load visual proof exists
   yet.
2. P-B: one bounded `nova-bench` New Game economy run with explicit seeds,
   isolated save root, budget and recorded audit/media; distinguish mining
   pulses, canister spawn, pickup and sale using authoritative deltas.
3. P-C: fresh-spawn rendered frames on three seeds and an owner/player review
   of clarity; absence of an objective alone does not prove confusion.

All runs need separate approval under the current research-only scope. Output
and findings would remain within this task folder; tests or fixture source
outside it require a distinct implementation authorization.

## 8. Post-audit status (2026-10-09)

The owner's later instruction authorized three separate polish fixes on master.
This does not turn the original source audit into player-observed evidence.

| Candidate | Local commit | Established proof | Still missing |
| --- | --- | --- | --- |
| Death recovery | `aab28e2d3`, writer-lock correction `acaa26f6e` | Focused save/menu tests and a deterministic blocked-writer regression (40/40 fixed; 10/10 old) | Rendered native death-to-Load flow and separate web seed-only check |
| Mining refusal | `d274c5e2d` | Focused implementation checks; cheated fixture recorded seven NoLock refusals and seven SFX sidecar voices | Perceptual listening, the other four refusal reasons, and accepted-pulse contrast |
| Capacity-aware intake | `78fb1a192` | Oversized-canister red test failed before the fix (`left: 1.0, right: 0.0`); 14 focused tests pass, including mixed-mass and ejection cases; final full/control pair passes unchanged judge and visually shows door motion | Perceptual listening and a flown full-hold approach without ejection |

The first-frame Load yaw fix is separate (`398fbc804`): a non-identity saved
pose now seeds the body and helm before thaw. Its regression failed before and
passed after the fix. A rendered non-identity Load transition is unverified.
These commits are local and were not pushed at this audit (still 9 commits
ahead of `origin/master` at `2febf1b49`).

The original candidate text above describes the pre-fix code at `cb9ab4c66`;
its claims that death has no recovery, mining refusals are silent, or an unfit
canister opens the door do not describe current master. The New Game economy
journey and first-minute usability still lack player-run evidence. No
performance comparison or general usability conclusion follows from these
focused fixes. Task-local scripted scenario captures (not New Game) have now run at
`398fbc804`: #79 NoLock logged seven refusals and seven matching spaced
`radar_deny.wav` voice entries (545 frames), with no accepted pulse; #80's
cheat-marked full/control pair (1533/1531 frames) retained one ejected
canister and unchanged HullPlate counts. With zero free mass the door-close
cue follows ejection by one tick; with 10,000 g free it follows after 713
ticks near the 40 m range edge. Both #80 rejudgments pass after correcting
a pre-spawn canister sample. The bench run trees under both proof folders
were deleted after the tasks closed. Only these summaries and
`tasks/20261009-184234/TASK.md` ("Proof and limits") remain; the run IDs and
the `inspect/` frame cited below cannot be reopened. `tasks/20261009-184226/proof/`
keeps only diffs and ignored check logs. The rear camera does **not** resolve
the door pose. A later free-look pair (`control-20261010T094427-456478`,
`full-20261010T095303-463592`) renders upright open door leaves and later
folded leaves, but the combined free-look/aim gesture also turns the ship;
its control closes after 115 ticks and fails the unchanged 300-tick judge
threshold. The corrected split-input pair (`control-20261010T101138-469984`,
`full-20261010T101706-474036`) keeps turn rate zero, passes the unchanged
judge (713 vs 1 tick until close), and renders visibly upright open leaves
versus folded leaves (deleted `inspect/matched-control-top-full-bottom.png`).
Section 9, candidate A, traces the camera/ship steering mechanism in source. Nobody
listened to the sound: sidecar voices and AAC levels prove a recorded cue,
not perceptual quality. These fixtures do not prove a
real New Game mine/pickup/sale flow or player usability.

A separate fixture risk surfaced during script preparation: the shipped
`crates/nova_bench/scenarios/docking_warship_tender.content.ron:60-62`
authors an empty mapping for a player `block_line_warship`, while
`crates/nova_scenario/src/objects/spaceship.rs:967-977` panics if its Mining
section lacks a key. This is source-backed, **not reproduced** in that bench
scenario. Do not silently modify that shipped fixture under the task-local
proof scope; reproduce, gate, then repair separately if confirmed.

## 9. Remaining flows at `2febf1b49` (2026-10-10)

Read-only source trace of travel/aim, inventory/trade/repair, combat and the
first minute. No cargo, game, probe, bench or GPU run. Four read-only scouts
mapped the flows; every claim below was rechecked in source by the main
worker. Labels as in the header.

### 9.1 Evidence since section 8

No new player-run or rendered evidence exists. The #79/#80 bench trees are
deleted (section 8). The newest rendered frames are still
`tasks/20261007-090756/p6-frames/*.png` (2026-10-09, fixture New Game) and
`tasks/20260925-190131/stream-seed4426-*.png` (2026-10-02, real New Game,
player placed). **Observed** in `stream-seed4426-enemy-vasfenda-armored-firing.png`:
a red marker on the locked enemy and inbound tracers, but no player hull or
section element on screen. No `bench-runs/` score and no `~/Videos/nova-bench-*`
folder records a New Game travel, trade, repair or combat flow.

### 9.2 Ranked candidates

**A. A camera mode switch leaves the hull-steering rate live (travel/aim, combat).**
- **Code-backed chain.**
  1. The rotate observer writes the mouse rate only into rigs that hold
     `SpaceshipRotationInputActiveMarker` (`crates/nova_ship/src/camera/mode.rs:132-172`).
  2. Free-look (Alt) or turret aim (RMB) moves the marker off the Normal rig
     and does not zero the Normal rig's `PointRotationInput`
     (`mode.rs:81-123`).
  3. `point_rotation_update_system` integrates EVERY `PointRotation` rig in
     `PostUpdate`, marker or not (`crates/nova_gameplay/src/transform/point_rotation.rs:53,79-87`).
  4. The hull PD target reads the Normal rig's output, gated only on
     `Without<Autopilot>` and `Without<RcsActive>` (`crates/nova_ship/src/input/player/intent.rs:20-52`).
  5. The rate is cleared only when the rotate action completes
     (`mode.rs:174-181`) or control is suspended (`input/player/control.rs:87`).
     That `Complete` fires on the first frame without mouse motion is
     inferred from the RCS precedent, not read in `bevy_enhanced_input` source.
- **Precedent:** RCS had the same stale-rate drift and was fixed by zeroing
  the rate (`mode.rs:162-167`, test `mode.rs:613-666`).
- **Related observation (artifact deleted, cause unproven):** the #80 combined
  free-look/aim gesture turned the hull and closed the door after 115 ticks;
  splitting the press from aim kept turn rate zero (section 8).
- **Hypothesis:** a player who presses Alt or RMB while the mouse moves keeps
  turning the ship at the last Normal-mode rate until the mouse stops for one
  frame. Turret aim in combat is the common case. Magnitude depends on mouse
  event cadence. Not reproduced.
- **Before (hypothesis, not reproduced):** free-look or aim started mid-motion
  may steer the hull. **After (direction, not approved):** the mode switch zeroes the outgoing
  rig's rate. The press-frame delta still lands on the Normal rig; whether
  the observer should route by the derived mode is a separate decision.
- **Blast radius:** `camera/mode.rs` only. May break nested-hold, seeding and
  pause tests in `mode.rs:253-611`; autopilot free-look reads the same rigs.
- **Proof:** disposable ECS repro on the `mode.rs:613` rig: Normal rig
  active with mouse motion, hold FreeLook with motion for N updates, assert
  the Normal `PointRotationOutput` stops changing after the switch frame.
  Needs a test gate. **Effort:** repro S (hours), fix S.

**B. The player cannot read their own integrity in flight (combat).**
- **Code-backed:** the code states there is no hull readout in the HUD; one
  falling-edge hull alarm is the only integrity instrument
  (`crates/nova_ship/src/ship_audio/cues.rs:270-276`). A hostile lock plays one
  alarm (`cues.rs:200-259`). No hit-direction or damage-flash code exists
  (no match under `crates/` for hit direction or damage flash). A
  neutralized player gets no self cue: every `NeutralizedMarker` HUD reader
  is about another ship (`crates/nova_hud/src/allegiance_markers.rs`,
  `target_inset.rs`). AI then drops the player as a target
  (`crates/nova_ship/src/input/ai/acquisition.rs:691-697`) and the open world
  handles only `OnDestroyed` (`open_world.rs:56-65`). The player drifts alive
  with no state and no outcome. This extends candidate 4.
- **Observed:** the 2026-10-02 enemy frame shows no player integrity element.
- **Hypothesis:** players do not know they are losing until the alarm or death.
- **Class:** feature work (new HUD element and a neutralized-player policy),
  not small polish. **Proof:** one rendered real-New-Game combat capture with
  hull deltas from the audit. **Effort:** proof S, fix M.
- **Doc debt, XS:** "Nothing repairs a hull today" (`cues.rs:264,773`) is
  stale; plate repair exists (`crates/nova_gameplay/src/inventory.rs:793-824`).

**C. Mining and TAB are not shown in flight, and New Game does not point at training (first minute).**
- **Code-backed:** the dock lists nine verbs, none for mining or the
  interface (`crates/nova_hud/src/keybind_dock.rs:87-97`). The open world
  authors only spawn and Defeat, no lesson or `HintEmphasis`
  (`open_world.rs:40-65`). The New Game modal names derelicts only
  (`world_setup.rs:160-163`); the Basic Training offer is a menu corner on a
  fresh install (`crates/nova_menu/src/menu_ui.rs:531`). Clusters sit on a 50 km
  lattice and a node may hold nothing (`crates/nova_world_base/src/clusters.rs:9-11,90,1223-1226`),
  so a spawn can face empty space.
- **Observed:** one fixture spawn frame is empty with three chips (STOP RADAR
  RCS).
- **Hypothesis:** a first-time player never finds V or TAB. Not observed.
- **After (options, none approved):** (a) a MINE chip lit when the lock is a
  mineable rock in reach, reusing the refusal checks
  (`crates/nova_scenario/src/mining.rs:503-532`); (b) an open-world
  `HintEmphasis` spotlight at start. (a) also states why mining is refused.
- **Proof:** P-C (section 7) frames on three seeds, then owner judgement.
  **Effort:** proof S, fix S to M.

**D. Refused DOCK, GOTO and ORBIT give no reason (travel, trade).**
- **Code-backed:** a refused dock is `debug!` only
  (`crates/nova_ship/src/sections/docking_section/connection.rs:193-196`);
  GOTO/STOP refusals are `debug!` only
  (`crates/nova_ship/src/input/player/flight_rig.rs:424-500`); a GOTO or ORBIT
  whose target or well is gone disengages with a log only
  (`crates/nova_ship/src/flight/autopilot.rs:651-656`). The dock hides a verb
  that cannot run (`keybind_dock.rs:8-13`, `input/player/hints.rs:271-283`),
  so this is the shipped policy, not an accident.
- **Hypothesis:** a player pressing D on a hostile or fighting ship cannot
  tell why nothing happens. Whether a sector retire despawns a GOTO target
  is not traced.
- **After (option):** reuse the mining refusal pattern (`d274c5e2d`) for a
  refused verb press. **Effort:** S. Ranked low because the hide is designed.

### 9.3 Checked, no candidate

- **Trade and repair refusals are named:** every transfer, trade and credit
  refusal and every repair refusal, `Destroyed` included, reaches a note line
  (`crates/nova_interface/src/inventory/app.rs:1711-1762`,
  `crates/nova_interface/src/ship/sections.rs:453-470`). Undock mid-trade
  re-resolves the pair and refuses `not docked` (`inventory/app.rs:1223-1253,1666-1668`).
  Trade status is learned only from the Buy/Sell row action after docking
  (`inventory/app.rs:1102-1115`); a pre-dock trader cue is feature work.
- **Human trigger and stance:** the fire gate reads `WeaponsHot` live each
  tick (`crates/nova_ship/src/sections/turret_section/firing.rs:140-158`), so
  the autopilot same-frame drop does not apply to a human press.
- **Empty trigger:** a player dry-fire click and gauge cue exist
  (`cues.rs:121-192`).
- **Taken default name:** a second New Game opens on `New World`, which is
  refused only on Create (`world_setup.rs:242-301` vs `:324-361`; test
  `crates/nova_menu/src/tests/world_setup.rs:221`). The owner chose "nothing
  numbers it" (`world_setup.rs:49-50`). Not a candidate.

### 9.4 Done-when status

Not met. The plan is code-backed, but no candidate has bounded visual or
player proof. Gaps:
1. P-A: rendered native death-to-Load flow; web seed-only check.
2. P-B: New Game mine, pickup and sale bench run with audit deltas.
3. P-C: fresh-spawn frames on three seeds (candidate C).
4. Candidate A: ECS repro, needs a test gate.
5. Candidate B: one rendered combat capture with hull deltas.
6. Perceptual listening for the #79 refusal and #80 door cues.
Each run needs a quiet host and separate approval. The task stays OPEN.
