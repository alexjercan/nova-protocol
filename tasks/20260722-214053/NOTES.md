# The Ledger - campaign-wide pace-map + weak-spot brief

Diagnostic-first (owner clarification 2026-07-22: pace-map derived from the
scripts; owner replays the result at Finish). This is the shared reference the
sibling tasks (20260722-214058 pacing, -214105 ch3 depth, -214110 ch4 endings,
-214115 look) author against. Grounded in the shipped RON at bundle v1.5.0
(`webmods/the-ledger/ledger_ch*.content.ron`). No scenario edits in this task.

Method: counted the pacing primitives per file (`OnStart`/`OnUpdate`/`OnEnter`/
`OnDestroyed` handlers, `Objective`, `StoryMessage`, `SpawnScenarioObject`,
`Outcome`, `NextScenario`, `scenario_elapsed`, `engage_delay`, `dwell`) and read
the opening + transition + ending handlers to trace the beat wiring.

## Headline findings

1. **`dwell` is unused across the ENTIRE mod (0 refs in all five files).** Not a
   single StoryMessage sets a per-line hold. Comms lines land and clear at the
   engine default; the beat sheet's "space consecutive lines with the scenario
   clock + dwell" is not applied anywhere.
2. **Three of five chapters have ZERO scenario-clock pacing** (`scenario_elapsed`
   refs: ch1=0, ch3=0, ch4=0). They are purely position/kill-gated - beat N
   completes and beat N+1's objective + spawn fire the same frame. This is the
   exact pre-fix Shakedown shape the owner rejected on 2026-07-21
   ("goes one after the other too fast").
3. **ch2/ch2b are the only chapters with any clock pacing** - and only ONE gate
   each (`scenario_elapsed=1`: a single one-shot teach line). Telegraphed spawns
   (`engage_delay: 8.0`) exist in ch2, ch2b AND ch3's optional ambush - but NOT
   on ch4's climactic Auditor (see #4).
4. **ch4's Auditor - the campaign's climactic fight - has NO telegraph**
   (`engage_delay=0` in ch4). The biggest gunship in the story materializes in
   the same handler frame as the choice, with no warning line and no arrival
   grace.
5. **Every chapter opens with a frame-0 dump**: `OnStart` posts the intro
   StoryMessage AND the first objective(s) AND spawns in one handler - the
   "objective shares a frame with conversation" anti-pattern the mainline pins
   against. Worst is ch1 (1 message + 4 objectives + 8 spawns at OnStart).

## Per-chapter pace-map

### ch1 - Dead Weight (1497 lines; OnStart + OnUpdate + 4 OnEnter + 1 OnDestroyed)
- Primitives: Objective=5(3 distinct + 2 updates), StoryMessage=5,
  SpawnScenarioObject=8, Outcome=2, NextScenario=3, scenario_elapsed=0,
  engage_delay=0, dwell=0.
- Opening (`OnStart`, line 22): Foreman Okono line (line 34) + FOUR objectives
  back-to-back (`obj_ch1_recap`:38, `obj_crate_1`:443, `obj_crate_2`:451,
  `obj_crate_3`:459) + all 8 spawns, one frame. No clock, no conversation.
- Body: three quota-crate pickups (`OnEnter`, ~1318/1347/1376) each do
  `crates`+1 + `ObjectiveComplete` + one Okono line, instantly. An `OnUpdate`
  (~1407) flips `act` and announces the unlisted 4th ping in a single beat, no
  breathe. Final `OnEnter` (~1439) = Victory -> `ledger_ch2_claim_jumpers`.
- **Weak spots**: (a) frame-0 objective dump; (b) no opening conversation - the
  campaign's FIRST impression is a wall of three objectives; (c) the "4th ping"
  reveal, the chapter's hook, gets one un-paced beat; (d) no dwell on any Okono
  line. No hostiles, so 0 engage_delay is correct here.
- **Target rhythm**: ~30-40s Okono opening conversation before the first
  objective; lazy-post the first crate objective on hand-off; breathers between
  the three pickups; give the 4th-ping reveal a dedicated announce->breathe beat.

### ch2 - Claim Jumpers (2155 lines; OnStart + 2 OnUpdate + 4 OnDestroyed)
- Primitives: Objective=3, StoryMessage=4, Spawn=12, Outcome=3, NextScenario=3,
  scenario_elapsed=1, engage_delay=2, dwell=0.
- Opening (`OnStart`, line 27): Okono line + 3 objectives + both wave-one Magpie
  spawns (`engage_delay: 8.0`), one frame.
- Only clock beat: a single `scenario_elapsed`-gated teach line (`teach_sent`).
  Two `OnDestroyed` kill-counters (+1 `kills`, one Okono line each). Win at
  `kills==2` -> `ledger_ch2b_the_heavies` (checkpoint). Two Defeat retries.
- **Weak spots**: frame-0 objective dump; win->next-scenario handoff has no
  breather beyond `linger: true`; no dwell. Fight geometry/telegraph is GOOD
  (this is the well-tested fair fight - see `ledger_ch2_encounter.rs`).
- **Target rhythm**: short opening conversation; split the 3-objective dump;
  breather between the `kills==2` victory and the checkpoint handoff. DO NOT
  touch spawn geometry (the fairness rig pins ranges/bearings/cover).

### ch2b - The Heavies (2065 lines) - near-mirror of ch2
- Primitives: Objective=3, StoryMessage=4, Spawn=12, Outcome=3, NextScenario=3,
  scenario_elapsed=1, engage_delay=2, dwell=0.
- Same shape: OnStart dump + heavy spawns (`engage_delay: 8.0`), one teach gate,
  two kill counters, win at `kills==2` -> `ledger_ch3_quiet_channel`, two Defeat
  retries.
- **Weak spots / target**: identical to ch2 - split the opening dump, add a
  short opener, breather before the handoff, keep geometry.

### ch3 - The Quiet Channel (1230 lines; OnStart + 4 OnEnter + 1 OnDestroyed) - THINNEST
- Primitives: Objective=2, StoryMessage=4, Spawn=7, Outcome=2, NextScenario=2,
  scenario_elapsed=0, engage_delay=2, dwell=0.
- Structure: ONE linear position-gated act. `OnStart` (line 18): Vesh line + 2
  objectives + player + 4 beacons (NAV-1/2/3, YARD) + `ScatterObjects` debris
  (26 rocks, decorative). Then a strict gate line: `OnEnter nav_1` (gate==1) ->
  set 2 + one Vesh line; `OnEnter nav_2` (gate==2) -> set 3 + one Vesh line +
  spawn two Magpies (`engage_delay: 8.0`, the ONE optional fight); `OnEnter
  nav_3` (gate==3) -> set 4 + one line; `OnEnter vesh_yard` (gate==4) -> Victory
  -> `ledger_ch4_the_buyer`.
- **Why thinnest**: a 4-beacon corridor of one-line comm beats + a single
  optional ambush, zero clock pacing, and the 26-rock debris field is pure
  decoration (no beat references it). Every beat is "reach beacon -> one Vesh
  line -> bump `gate`", instantly.
- **Target rhythm (task -214105)**: (a) clock-paced opening act (Vesh briefs,
  Kestrel answers, "go dark") before NAV-1 arms, first objective lazy-posted;
  (b) breathers - stamp `beat_gate` at each gate transition, land the next Vesh
  line a fixed beat later; (c) a SECOND distinct encounter (staggered contact at
  NAV-3 with a different loadout+telegraph, or a debris-pinch hazard) keeping the
  optional-fight contract; (d) make the debris load-bearing for one beat.

### ch4 - The Buyer (1856 lines; OnStart + 2 OnEnter + 3 OnDestroyed) - CONVERGENT ENDINGS + UNTELEGRAPHED CLIMAX
- Primitives: Objective=3, StoryMessage=3, Spawn=6, Outcome=3, NextScenario=1,
  scenario_elapsed=0, engage_delay=0, dwell=0.
- Vars: `act` (1 choice, 2 fight, 3 done), `choice` (0/1 sold/2 burned).
- `OnStart` (line 21): Vesh line + objective + player + two hulks + `handoff_berth`
  (beacon trigger, line 359) + `burn_buoy` (beacon, line 374).
- The choice: `handoff_berth` OnEnter (~392, act==1) sets `choice=1`,`act=2`, one
  line, `obj_auditor`, and **spawns `auditor`** (id at 422, `cargob` gunship,
  `controller: AI(())`, no engage_delay). `burn_buoy` OnEnter (~1082, act==1)
  sets `choice=2`,`act=2`, one line, `obj_auditor`, and **spawns an identical
  `auditor`** (id at 1112).
- The endings: BOTH are `OnDestroyed(auditor)` (act==2), split only by `choice`
  guard: `choice==1` -> Outcome Victory "SOLD" (1796); `choice==2` -> Outcome
  Victory "BURNED" (1825). Defeat handler `OnDestroyed(player)` act<3 (~1833).
- **Weak spots**: (a) both endings converge - the Auditor fight is mandatory on
  both paths, the choice changes only flavor text (this is the owner's headline
  gripe); (b) the Auditor, the climactic spawn, has NO engage_delay and NO
  warning line - it materializes in the choice handler's frame; (c) both endings
  are the same terminal outcome kind (Victory), differing by message only.
- **Target (task -214110)**: BURN path drops the `auditor` spawn and lands its
  OWN terminal Outcome (no fight - "you slipped the belt"); SELL path keeps the
  Auditor but gets `engage_delay` + a warning line + a choice->fight breather;
  the two endings become DISTINCT terminal outcomes; the now-dead `choice==2`
  auditor-death handler is removed; the burn path advances `act` to 3 before any
  death window (outcome-is-last-write-wins-close-the-act). New rig
  `ledger_ch4_ending.rs` pins the divergence.

## Cross-cutting conventions to apply (from the beat sheet + Shakedown)

- One StoryMessage per handler; objectives post a beat AFTER their intro line
  (never at OnStart during a conversation). Enforced by `content lint --target
  the-ledger` arms (>1 StoryMessage/handler; StoryMessage+Outcome co-fire).
- Opening conversation idiom: seed an `open_step` counter, gate each line on
  `open_step == n && scenario_elapsed > t`, final step posts objective 1 and
  spawns its target (lazy). Breathers: `mark_clock(beat_gate, delay)` at each
  transition, gate the next line on `clock_past(beat_gate)`.
- Add per-line `dwell` where a line needs to breathe (clamp [3,30]s) - currently
  used nowhere in the mod.
- Every fight telegraphs: warning line + far spawn + `engage_delay` grace.
- Time-gated content needs a clock-pumping test path or walk tests silently
  stall (lesson from 20260721-211506).
- Probe changed scenarios against the REAL loader, not a synthetic rig
  (`probe-content-not-just-code`, `review-rig-can-false-green`): the loader
  fires OnStart before the first tick, so an OnStart clock read must be verified
  in production, not a rig that pre-seeds the clock.

## Owner playtest questions (for the Finish checkpoint)

1. **ch4 burn ending tone**: clean escape ("you slipped the belt, box gone,
   nobody left to collect") vs bittersweet ("you're clear but broke - no payout,
   the Kestrel limps home")? This decides the burn path's terminal outcome
   framing (task -214110).
2. **ch3 second encounter flavor**: a staggered second CONTACT at NAV-3 (a
   different, heavier loadout that telegraphs) vs a debris-PINCH hazard
   (invulnerable rocks tightening the lane, no new enemies)? Both keep the
   "fighting is optional" contract (task -214105).
3. **Opening conversation length**: Shakedown's opener is ~40s. Is that the
   right dwell for a portal-mod chapter, or should the Ledger openers be tighter
   (~20-25s) since a returning player has already met the cast? (tasks -214058,
   -214105).
4. **ch1 4th-ping reveal**: should the hook get a modal/`auto_advance` beat (a
   real "stinger") or stay an inline comms line with a breather? (task -214058).
5. **Difficulty**: the pace-map is structural only - no difficulty numbers were
   changed. After replay, does any chapter's fight feel too easy/hard now that
   the beats breathe? (feel/balance is the owner's call per the parent task.)

## Scope guard

Diagnostic only. The sibling tasks implement against this brief. Spawn geometry
in ch2/ch2b is OFF-LIMITS (the fairness rig owns it); the pacing layer is
additive (clock gates + comms + dwell + telegraphs), not a geometry change.
