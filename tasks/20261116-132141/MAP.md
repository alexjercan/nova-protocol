# MAP: job rewards and the payout moment

Read-only code map of `master` at `de8cbaa3e`. Paths are from the repo root.
"Measured" means a throwaway integration test printed the number from the real
base catalog (section J). "Computed" means arithmetic on cited constants.
"Not verified" means the claim rests on a grep hit or a doc comment only.

Two corrections to `TASK.md` first:

- The hand-in code is not at `crates/nova_interface/src/jobs.rs:2116-2124`.
  It is at `crates/nova_interface/src/jobs.rs:1795-1876`.
- The hand-in writes no pane line. The `line` string is built and dropped
  (`crates/nova_interface/src/jobs.rs:1853-1873`). `JobsRuntime` has no text
  field (`crates/nova_interface/src/jobs.rs:168-177`). The one trace is a click
  (`crates/nova_interface/src/jobs.rs:1875`).

## A. Reward inputs

The draw is `station_job_offers(seed, ids, clusters, advancement, items, mix,
field, standing, assured) -> Vec<Job>`
(`crates/nova_gameplay/src/jobs.rs:2042-2190`).

What the draw holds:

- `JobWreck { id, name, stacks }` (`crates/nova_gameplay/src/jobs.rs:1929-1936`).
- `JobHull { id, name }` (`crates/nova_gameplay/src/jobs.rs:1941-1946`).
- `JobField { wrecks, hostiles }` (`crates/nova_gameplay/src/jobs.rs:1952-1957`).
- No position, no section count, no role, no weapon count, no size class.
- Each id is a `GeneratedBodyId` with a public `origin: SectorCoord`
  (`crates/nova_events/src/generated.rs:189-197`). The slot `JobId` carries the
  station id. So the draw can read the target cell and the poster cell today.
  That gives a cell-to-cell distance in 32 km steps with no new input. The pane
  already prints that distance (`crates/nova_interface/src/jobs.rs:318-329`).
- `advancement` is the poster's civilization, not the target's
  (`crates/nova_world_base/src/jobs.rs:445`, `:143-152`).

Reward arms today:

| Kind | Formula | Evidence |
|---|---|---|
| Procure | goods at ask x 3/2 + fee | `crates/nova_gameplay/src/jobs.rs:2234-2244` |
| Recover | whole stack at ask x 2 + fee | `crates/nova_gameplay/src/jobs.rs:2131-2148` |
| Escort | cargo / 2 + 150 x raiders + fee | `crates/nova_gameplay/src/jobs.rs:2248-2261` |
| Distress | 200 + 2 x fee | `crates/nova_gameplay/src/jobs.rs:2264-2271` |
| Supply | 200 + goods x 3/2 + 2 x fee | `crates/nova_gameplay/src/jobs.rs:2313-2337` |
| Neutralize | 400 + 2 x fee | `crates/nova_gameplay/src/jobs.rs:2152-2161` |

Fee is 50 to 250 cr in steps of 50 (`crates/nova_gameplay/src/jobs.rs:118-120`,
`:2219`). Other constants: `crates/nova_gameplay/src/jobs.rs:115-144`.

Callers of `station_job_offers`:

- Production: one. `post_station_boards`
  (`crates/nova_world_base/src/jobs.rs:446-456`). It holds the world seed, the
  cell's cluster types, the poster's advancement, the items, the start role's
  mix (`:309-315`), the field from `job_field` (`:444`, `:607-661`), the
  standing offers and the starter hull fit.
- Its station query has no transform (`crates/nova_world_base/src/jobs.rs:264-275`).
  Its wreck query has id, name, hold, parent (`:285-292`). Its hull query has
  id, name, allegiance, parent (`:293-301`). No transform. No sections.
- `PlayerHull` in the same system does hold the player's transform
  (`crates/nova_world_base/src/jobs.rs:158-222`).
- Tests: `crates/nova_gameplay/src/jobs.rs:2825`, `:3599`, `:3850`, `:3935`,
  `:4012`, `:4045`; `crates/nova_world_base/src/save/tests.rs:2242`, `:2483`.

Jobs that set `reward_cr` another way:

- Visit jobs. `VisitJobType::Procure { item, count, reward_cr }`
  (`crates/nova_gameplay/src/trade_profile.rs:99-109`) is copied to the `Job`
  as authored (`crates/nova_world_base/src/station_visits.rs:641-683`). Base
  values are literals: 180, 150, 220, 200, 160, 220 cr
  (`crates/nova_authoring/src/base_content/trade_profiles.rs:55-59`, `:79-114`).
- Scenario jobs. `PostJob` carries an authored `reward_cr`
  (`crates/nova_scenario/src/actions/jobs.rs:35-51`, `:158-171`). Lint refuses
  zero (`crates/nova_scenario/src/lint/scenario.rs:1475`). No base scenario
  uses `PostJob` (grep of `assets/**/*.ron` and
  `crates/nova_authoring/src`: no hit).
- The fitted offer. `fitted_posting` re-prices slot 0 at the starter station
  (`crates/nova_gameplay/src/jobs.rs:2171-2188`, `:2346-2379`).
- The base open world grants 50 cr over the first hand-in, with a comms line
  (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:46`,
  `:144-161`).

## B. Where each job's world leg is decided

Bodies are placed at accept, not at the draw. `spawn_job_bodies` observes
`JobAccepted` (`crates/nova_world_base/src/jobs.rs:685-934`).

| Kind | Leg | Known in the pure draw? | Evidence |
|---|---|---|---|
| Procure | none; work is the hand-in | yes (zero) | `crates/nova_gameplay/src/jobs.rs:1301-1309` |
| Escort | hauler 1,000 m off the poster, mark 10,000 m out, raiders at 7,500 m along | constant, but it lives in `nova_world_base` | `crates/nova_world_base/src/jobs.rs:102-133`, `:835-861`, `:1006-1015` |
| Distress | caller at 10,000 m | same | `crates/nova_world_base/src/jobs.rs:897-931` |
| Supply | caller at 10,000 m | same | `crates/nova_world_base/src/jobs.rs:897-931` |
| Recover | an existing wreck in the 27-cell cube | cell distance only (`id.origin`) | `crates/nova_world_base/src/jobs.rs:585-661` |
| Neutralize | an existing Enemy hull in the cube | cell distance only (`id.origin`) | `crates/nova_world_base/src/jobs.rs:585-661` |

- `JOB_LEG = Meters(10_000.0)` (`crates/nova_world_base/src/jobs.rs:102`). Three
  kinds share it. Their distance does not vary today.
- A cell is 32,000 m (`crates/nova_world_base/src/lib.rs:200`). The cube is
  3 x 3 x 3 cells. Cell centres stand 0, 32, 45 or 55 km apart (computed).
- In-cell positions: a station's two traffic hulls stand 2,000 to 4,000 m from
  it (`crates/nova_world_base/src/clusters.rs:242-251`). A cluster reaches at
  most 8,000 m from its anchor (`:132`). Clusters sit on a 50 km lattice with
  30 km least gap (`:106`, `:123`). So a located target is 2 to 16 km away in
  the poster's cluster, or 30 km and more in another one (computed, not
  measured).
- Exact metres need the world crate. The live target has a `GlobalTransform`;
  a frozen one has no entity (`crates/nova_world_base/src/jobs.rs:1502-1508`).
- Leg direction is seeded by job id (`crates/nova_world_base/src/jobs.rs:960-1001`).

What decides a Neutralize target's hull:

- The target is a streamed sector ship. `plan_ship` picks its role from the
  civilization's role weights and sizes it `ShipSizeType::Advancement`
  (`crates/nova_world_base/src/sector_ships.rs:189-352`, `:230`, `:260`).
- Its side is the civilization's stance (`crates/nova_world_base/src/sector_ships.rs:333`).
  A hostile is any intact, not neutralized, `Allegiance::Enemy` ship
  (`crates/nova_world_base/src/jobs.rs:607-661`). So an unarmed Civilian or
  Industrial hull can be the bounty.
- Weapon slots by role: Civilian and Industrial 0; Scavenger `1 + round(adv x 3)`;
  Armored `1 + round(adv x 8)` (`crates/nova_world_base/src/ship_layout.rs:1239-1246`).
  The function is private to that file.

Existing threat or class measures:

- `ShipSizeType` (`Advancement` or `Budget { sections, turrets }`) is in
  `nova_gameplay` (`crates/nova_gameplay/src/jobs.rs:190-207`).
- `ESCORT_RAIDER_BANDS` maps poster advancement to raider count and budget
  (`crates/nova_gameplay/src/jobs.rs:155-186`, `:1480-1522`). It is the only
  threat table the draw reads.
- `ShipRoleType` is in `nova_gameplay` (`crates/nova_gameplay/src/market.rs:63-72`).
  A live ship carries no role component. The role shows only in its `Name`
  (`crates/nova_world/src/streaming.rs:505-541`). `SectorShip.role` exists at
  generation (`crates/nova_world/src/generation.rs:122-150`).
- `crates/nova_authoring/src/balance.rs:1-55` audits authored scenario ships.
  It does not read generated hulls. Not verified past the header.

Measured hull sizes (J test, 120 hulls per row):

| Hull | adv 0.0 sections min/med/max | adv 0.0 turrets | adv 0.5 sections | adv 0.5 weapons |
|---|---|---|---|---|
| Escort raider (Budget) | 22 / 23 / 36 | 1 | 48 / 76 / 108 | 3 turrets |
| Scavenger (Advancement) | 22 / 27 / 155 | 0 to 2 | 105 / 255 / 512 | 1 to 6 turrets, 0 to 4 torpedo bays |
| Armored (Advancement) | 40 / 86 / 149 | 0 to 2 | 481 / 918 / 1,749 | 2 to 8 turrets, 1 to 6 bays, 0 to 2 railguns |
| Civilian | 27 / 32 / 89 | 0 | 108 / 262 / 646 | 0 |
| Industrial | 38 / 73 / 104 | 0 | 350 / 694 / 1,462 | 0 |

The start ship has 27 sections and one turret
(`crates/nova_gameplay/src/jobs.rs:151-154`).

## C. Persistence

- `Job { id, kind, target, reward_cr }` is `deny_unknown_fields`
  (`crates/nova_gameplay/src/jobs.rs:1168-1180`). So are `JobType`
  (`:981-1048`), `SlotPosting` (`:367-381`), `JobPostings` (`:452-462`) and
  `Jobs` (`:1659-1665`).
- Both are saved: `WorldSaveState.jobs` and `.postings`
  (`crates/nova_world_base/src/save/mod.rs:318-321`). Capture and restore copy
  them whole (`crates/nova_world_base/src/save/session.rs:373-374`, `:566-567`).
- Offers are saved, not re-drawn. A slot's `offer: Job` is in the save. Only a
  renewing slot calls the draw (`crates/nova_world_base/src/jobs.rs:414-456`).
  The wiki says a Load shows the same offers (`web/src/wiki/interface.md:277`).
- A held job on Load keeps the reward it was drawn with. `Jobs::validate` and
  `Job::validate` run on Load and refuse a zero reward
  (`crates/nova_gameplay/src/jobs.rs:1338-1342`, `:1871-1875`).
- A new field on `Job` or `JobType` makes an older save fail to parse. The
  format is strict and nothing migrates
  (`crates/nova_world_base/src/save/mod.rs:18-24`, `:104-116`).
- `WORLD_SAVE_VERSION = "v0.18.0"` and it is set once per release
  (`crates/nova_world_base/src/save/mod.rs:117`).
- The header also carries a catalog digest. A save of another catalog is
  refused (`crates/nova_world_base/src/save/mod.rs:123-136`, `:2115-2146`). The
  digest covers every content item (`crates/nova_assets/src/merge.rs:1002-1036`).
- Base content already changed since v0.18.0 (`git diff --stat v0.18.0..HEAD
  -- assets/base`: 10 files). `CHANGELOG.md:23` already says a v0.18.0 save no
  longer loads. So a `Job` layout change costs no released save. It does break
  development saves made this cycle.
- Reward constants are Rust, not content. A constant change alone does not
  move the digest.

## D. Jobs pane UI

File: `crates/nova_interface/src/jobs.rs`. Module doc: `:1-58`.

Structure: two lists and an inspector. The whole pane is rebuilt when
`JobsView` differs (`:509-802`, rebuild at `:761-801`).

- Board list: `board_panel` (`:1193-1228`), rows by `posted_row` (`:1232-1327`).
  Columns: icon, Job, Reward, Time left, verbs (`:1211`). The reward is
  `spawn_credit_figure(.., row.reward_cr, TextRole::Value, UiColorType::Accent, ())`
  (`:1280-1289`).
- Held list: `held_panel` (`:1388-1425`), rows by `held_row` (`:1430-1491`).
  Columns: icon, Job, Progress, Hand-in, verbs (`:1408`). A held row shows no
  reward. `HeldRow` has no reward field (`:446-458`).
- Inspector: `inspector_panel` (`:1494-1546`). Rows: Task, Target, Hand in,
  Reward (`:1524-1529`), Time left, Progress, Show on Map (`:1541`).
- View structs: `PostedRow` (`:425-442`), `HeldRow` (`:446-458`),
  `InspectorView` (`:468-482`), `JobsView` (`:486-497`).
- Column widths: `REWARD_PX = 90` and others (`:805-826`).

Layout-kit builders in use:

| Builder | Defined at |
|---|---|
| `text`, `text_toned`, `TextRole` | `crates/nova_ui/src/widget/layout.rs:91-134` |
| `VerbGate` | `crates/nova_ui/src/widget/layout.rs:195-205` |
| `verb_row`, `RowAlign` | `crates/nova_ui/src/widget/layout.rs:285-295` |
| `control_row` | `crates/nova_ui/src/widget/layout.rs:312` |
| `spawn_kv`, `spawn_kv_slot` | `crates/nova_ui/src/widget/layout.rs:325-339` |
| `spawn_block` | `crates/nova_ui/src/widget/layout.rs:461` |
| `tag` | `crates/nova_ui/src/widget/layout.rs:485` |
| `tool_list_row` | `crates/nova_ui/src/widget/list_row.rs:50` |
| `ButtonSpec::control`, `button` | `crates/nova_ui/src/widget/button.rs:402-509` |
| `spawn_credit_figure` | `crates/nova_interface/src/pane.rs:573-613` |
| `gated_verb` | `crates/nova_interface/src/pane.rs:943-950` |
| `cr_digits`, `cr_text` | `crates/nova_gameplay/src/inventory.rs:305-320` |

Range, bearing, location shown today:

- Inspector Target row only: name, cell coordinate and a cell-to-cell distance,
  "this sector" or "<n> km" (`:318-349`, `:551`, `:734`). No bearing. No metres.
- Board and held rows show no range.
- The Map chart panel shows the reward in its Bearing field and a route note
  (`crates/nova_interface/src/map/chart.rs:1821-1835`). `job_cell` is the
  target's `origin` (`crates/nova_interface/src/map/chart.rs:186-191`).
- In flight the target wears a chip with the job code, verb and distance
  (`crates/nova_world_base/src/jobs.rs:1502-1513`, `web/src/wiki/interface.md:294`).

Hand-in flow (`:1795-1876`):

1. `plan_job_hand_in` (`:1821`; rule at `crates/nova_gameplay/src/jobs.rs:2599-2650`).
2. Refusal: `debug!` and `runtime.cue.raise(ActionOutcomeType::Refused)` (`:1823-1827`).
3. Goods leave the hold (`:1829-1834`).
4. Credits move on both sides with cause `JobReward` (`:1841-1846`).
5. `jobs.complete` then `commands.trigger(JobHandedIn { job })` (`:1847-1848`).
6. `info!` log (`:1849-1852`).
7. Dead `line` string (`:1853-1873`). Stance merit and `StanceChanged` (`:1858-1868`).
8. `runtime.cue.raise(ActionOutcomeType::Done)` (`:1875`).

Sound: `play_jobs_cue` (`:1676-1682`) calls `play_outcome_cue`. Done plays
`UiSfx::MenuSelect` in `UiClick`; Refused plays `UiSfx::EditorDeny` in
`UiError` (`crates/nova_interface/src/pane.rs:878-901`). `ActionCue` holds one
pending cue per frame and states the rule "The pane shows no result text"
(`crates/nova_interface/src/pane.rs:903-938`).

Events: `JobAccepted`, `JobDone`, `JobHandedIn`, `JobDropped`
(`crates/nova_gameplay/src/jobs.rs:1549-1577`). The scenario bridge copies
`reward_cr` into `OnJobHandedIn` (`crates/nova_scenario/src/loader/bridges.rs:38`,
`crates/nova_events/src/lib.rs:914`; grep only).

Refusals: `JobHandInRefusalType` in check order NotDocked, WrongStation,
ShortItems, NotDone, StationCannotPay, CreditOverflow
(`crates/nova_gameplay/src/jobs.rs:2549-2577`). The pane maps them to a gate;
`StationCannotPay` is `VerbGate::Tagged("NO FUNDS")` (`:377-388`).

Systems: `(apply_job_commands, update_jobs_pane, play_jobs_cue).chain()` in
`JobsPaneSystems` (`:1970-1991`).

A prototype exists. The UI gallery example shows a hand-in toast, "+N cr" over
"balance N cr", bottom right, 4 s (`examples/playable/ui_gallery.rs:682-693`,
`examples/playable/ui_gallery/chrome.rs:522-560`). No crate has a toast widget
(grep of `crates/` for a UI toast: no hit; not read file by file).

## E. HUD notice and audio

Notice API: `pub struct HudNotice(pub String);`, a `Message`
(`crates/nova_hud/src/hud_notice.rs:95-96`). One slot, no queue; a new text
replaces the shown one (`:38-40`). Dwell 4 s (`:73`), fade 0.4 s (`:77`).

Full call example (`crates/nova_hud/src/hud_notice.rs:319-335`, `:399-410`):

```rust
fn post_rescue_notice(
    mut ended: MessageReader<RescueCallEnded>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut notices: MessageWriter<HudNotice>,
) {
    for ended in ended.read() {
        if !q_player.contains(ended.ship) {
            continue;
        }
        if let Some(line) = rescue_end_line(&ended.end) {
            debug!("hud_notice: {line}");
            notices.write(HudNotice(line));
        }
    }
}
// rescue_end_line: format!("SOS ANSWERED: PAID {}", cr_text(*paid_cr).to_uppercase())
```

- The job-done notice is an observer: `post_job_done_notice(done: On<JobDone>, ..)`
  writes `"{code} done: hand in at {station}"` (`:517-531`). The module doc
  says it names no credits: "payment stays at the hand-in" (`:9-10`).
- Wiring: `crates/nova_hud/src/hud_notice.rs:204-241`.
- The notice has one fixed cue. `show_hud_notice` always plays
  `UiSfx::HudNotice` in `SoundCategoryType::Notice` (`:601-643`, cue at `:635-641`).
  A caller cannot pick a cue.
- Dwell runs on `Time<Virtual>` (`:602`). Paint is amber (`:647-673`).

Two facts block a notice at the hand-in:

- The Tab interface hides every HUD root that is not `HudInterfaceExempt`
  (`crates/nova_hud/src/lib.rs:495-545`). The notice box is a plain HUD root
  (`crates/nova_hud/src/lib.rs:387`).
- The Tab interface holds virtual time and physics
  (`crates/nova_interface/src/pane.rs:1620-1662`). A notice posted at the
  hand-in plays its cue at once, then shows for its full 4 s only after the
  pane shuts (computed from the two facts above; not run).

So a count-up or flash in the pane must run on `Time<Real>`. The crate already
uses real time (`crates/nova_interface/src/ship/scene.rs:791`,
`crates/nova_interface/src/terminal/shell.rs:143`).

Cues:

- `pub enum UiSfx` has 24 keys (`crates/nova_sound/src/lib.rs:143-204`). No
  payout, credit or coin cue exists. Nearest: `ObjectiveComplete`, `NovaOsOk`
  ("a soft confirmation beep"), `MenuSelect`.
- `UI_SFX_FILES: [(UiSfx, &str); 24]` maps keys to `assets/sounds/<name>.wav`
  (`crates/nova_sound/src/lib.rs:209-235`). Test `every_ui_sfx_key_has_a_file`
  (`:329`).
- Levels: `UiClick` -14 dB, `UiConfirm` -9, `UiError` -9, `Notice` -8,
  `Objective` -5 (`crates/nova_sound/src/category.rs:205-210`). `UiConfirm` is
  "a command or an operation that succeeded" (`:74-95`).
- Play call: `commands.trigger(PlaySound::ui(bank.get(cue), category))`
  (`crates/nova_sound/src/play.rs:71-80`, `crates/nova_interface/src/pane.rs:888-901`).
- Sounds are generated WAV files. `scripts/gen-ui-sfx.py` has one function per
  cue (`:106-386`) and a `CUES` table (`:409-435`). `assets/sounds/README.md`
  documents the run command and has a file table.

To add a cue: add a function and a `CUES` row in `scripts/gen-ui-sfx.py`; run
`nix develop --command python3 scripts/gen-ui-sfx.py --only <name>`; add a
`UiSfx` variant and a `UI_SFX_FILES` row and raise the array length; add the
README row; pick a `SoundCategoryType`. Content lint levels every sound file
(`crates/nova_authoring/Cargo.toml:33-35`).

Settings and themes:

- Volume: `InterfaceVolume` and `MasterVolume`; `HarnessMute` silences harness
  runs (`crates/nova_sound/src/bus.rs:38-94`, `crates/nova_menu/src/settings.rs:1922-1936`).
- `ButtonCueType::Owned` means the handler plays the cue; job buttons use it
  (`crates/nova_ui/src/widget/button.rs:50-71`, `crates/nova_interface/src/jobs.rs:1083-1103`).
- UI themes name no sound (`assets/base/ui_themes/base.content.ron`: grep for
  sfx, sound, cue gives 0 lines).

## F. Credits display

| Place | What | Evidence |
|---|---|---|
| Tab pane head | player balance, every pane | `crates/nova_interface/src/pane.rs:784-794` |
| Jobs board row | reward | `crates/nova_interface/src/jobs.rs:1280-1289` |
| Jobs inspector | reward | `crates/nova_interface/src/jobs.rs:1524-1529` |
| Map chart panel | held job reward | `crates/nova_interface/src/map/chart.rs:1826-1829` |
| Inventory pane | partner credits only; prices | `crates/nova_interface/src/inventory/app.rs:529-533`, `:1172` |
| Station pane | card prices | `crates/nova_interface/src/station/mod.rs:232-253` |

- The HUD draws no balance (`ShipCredits`: no use in `crates/nova_hud`).
- `show_pane_balance` reads `ShipCredits` each frame and writes `cr_digits`
  when the text differs (`crates/nova_interface/src/pane.rs:617-651`). No
  change detection. No animation. It runs with the pane shut too. It is in the
  `InterfaceSystems` chain (`crates/nova_interface/src/lib.rs:198`).
- No count-up, tween or animated number widget exists (grep of `nova_ui`,
  `nova_interface`, `nova_hud` for count_up, CountUp, Tween, AnimatedNumber:
  no hit).
- Marker and name for tests: `InterfacePaneBalance`, `INTERFACE_PANE_BALANCE`
  (`crates/nova_interface/src/pane.rs:617-620`).

## G. Station balances

- Station credits are `lerp((10_000, 40_000), advancement)`
  (`crates/nova_world_base/src/station_layout.rs:142-144`, `:595`). Measured:
  10,000 at 0.0; 14,500 at 0.15; 17,500 at 0.25; 25,000 at 0.5.
- No refill exists. Restock on thaw moves items only
  (`crates/nova_world_base/src/economy.rs:27-70`). Frozen ships keep their
  balance (`crates/nova_world/src/tests/frozen.rs:853`, `:957`).
- A station gains credits only from the player: trades, rebuild, swap, add,
  ship buy, escort hire, fine, craft fee, offer refresh
  (`crates/nova_events/src/lib.rs:969-1004`; call sites:
  `crates/nova_interface/src/inventory/app.rs:2445-2446`,
  `crates/nova_interface/src/ship/rebuild.rs:671-672`,
  `crates/nova_interface/src/ship/refit.rs:241-242`, `:571-572`,
  `crates/nova_interface/src/station/shipyard.rs:407-408`,
  `crates/nova_interface/src/station/escorts.rs:428-429`,
  `crates/nova_interface/src/station/settle.rs:117-118`,
  `crates/nova_ship/src/sections/craft_job.rs:286-295`). Visiting traders also
  trade with it (`crates/nova_world_base/src/station_visits.rs:605-640`).
- The hand-in is refused when the station is short:
  `partner.credits_cr.checked_sub(job.reward_cr)`
  (`crates/nova_gameplay/src/jobs.rs:2633-2638`). The pane shows `NO FUNDS`
  (`crates/nova_interface/src/jobs.rs:385`). The accept does not check the
  station balance: `JobAcceptRefusalType` has no such case
  (`crates/nova_gameplay/src/jobs.rs:2382-2397`). So a player can finish a job
  the poster cannot pay.

Refusal numbers (computed from J):

- Start station, 10,000 cr. Median non-Recover reward is 500 to 1,000 cr. With
  no player spending the station pays about 10 to 20 hand-ins, then refuses.
- At 3x rewards that is about 4 to 7 hand-ins. A board posts 3 offers per
  20 minutes (`assets/base/market/base.content.ron:26`). One board cycle would
  nearly empty a start station.
- Recover already breaks the cap. At advancement 0.5 the measured Recover
  median is 7,538 cr, p75 18,730, p90 34,230, against 25,000 cr. Between 10 %
  and 25 % of those offers cannot be paid today. At 0.15: p90 4,742, max
  19,490, against 14,500.
- 2x to 4x on top makes `NO FUNDS` common unless stations refill or rewards
  are capped by the poster's balance.

## H. Economy anchors

| Anchor | Value | Evidence |
|---|---|---|
| Start credits | industrial 2,200; scavenger 1,800; transport 2,000; bounty hunter 1,600; military 1,000; pirate 1,800 | `assets/base/roles/base.content.ron:9`, `:32`, `:55`, `:78`, `:101`, `:125` |
| Start kit worth | 3,000 cr unarmed; 2,300 to 2,500 armed; plus 3 repair kits (360) | `crates/nova_authoring/src/base_content/start_roles.rs:4-13` |
| Ship buy | sum of section recipes at ask | `crates/nova_ship/src/sections/hull_buy.rs:110-120` |
| Ship buy, measured adv 0.0 | Scavenger median 5,360; Civilian 5,480; Industrial 11,340; Armored 13,500 | J test |
| Ship buy, measured adv 0.5 | Scavenger median 51,380; Armored 170,710 | J test |
| Escort hire | 3/2 of the hull | `crates/nova_ship/src/sections/escort_hire.rs:21`, `:82-84` |
| Station buy | 2 x (hull + stock + credits) | `crates/nova_ship/src/sections/station_buy.rs:56-82` |
| Station buy, measured | adv 0.0: 48,256 to 60,824 (median 54,536); adv 0.15: median 70,232; adv 0.5: 104,376 to 395,468 (median 197,352) | J test |
| Rebuild one section | recipe credits + parts at ask: light hull 120; thruster 260; reinforced hull 400; PDC turret 480; basic controller 540 (computed) | `assets/base/sections/base.content.ron:8-15`, `:110-116`; `crates/nova_gameplay/src/inventory.rs:1688-1714` |
| Rescue fee | 200 cr + rebuilt recipes; `RESCUE_FEE_CR = DISTRESS_BASE_CR` | `crates/nova_gameplay/src/rescue.rs:32`, `:122-150` |
| Fine | 1,000 to 10,000 cr by advancement | `crates/nova_world_base/src/stance.rs:55` |
| Offer refresh fee | 100 cr | `crates/nova_gameplay/src/jobs.rs:2657` |
| Item ask range | 4 cr (stone ore) to 400 cr (flight core, torpedo) per unit | `crates/nova_authoring/src/base_content/items.rs:63-323` |
| Mined ore | iron ore 10 kg, ask 16, bid 12; a hull section holds 100 kg, so 120 cr at bid per section (computed) | `crates/nova_authoring/src/base_content/items.rs:117-124`; `crates/nova_scenario/src/objects/ship_design.rs:413` |
| Price factors | clamp 0.25 to 3.0; raw 0.6 in asteroid cells, 1.5 near planets | `crates/nova_authoring/src/base_content/market.rs:14-16`, `:47-51` |
| Trade profit of a Procure | half the goods at ask plus the fee, if bought at reference ask (computed) | `crates/nova_gameplay/src/jobs.rs:2234-2244` |
| Offer window | 1,200 s; escort 3,600 s; visit extras 1,200 s | `crates/nova_authoring/src/base_content/market.rs:38-44`; `crates/nova_gameplay/src/jobs.rs:112` |
| Restock | 1,800 s | `crates/nova_authoring/src/base_content/market.rs:38` |
| Role goals | six hand-ins of the dominant kind, then one owned station | `crates/nova_authoring/src/base_content/start_roles.rs:15-26`; `crates/nova_world_base/src/goal.rs:4-8` |

Reading: a median job pays 500 to 1,000 cr. The cheapest hull costs about
5,000. A start station costs about 54,000. That is about 55 to 110 median
hand-ins for the role goal's first line (computed).

Not measured: a typical trade-run profit and a full mined hold in play.

## I. What asserts or quotes reward values

Unit tests that pin formulas or numbers:

- `crates/nova_gameplay/src/jobs.rs:2872` board determinism; Procure reward
  inside goods to goods + 250 (`:2911-2913`).
- `:2947` per-kind lower bounds: Neutralize `>= 400 + 100` (`:3003`), Escort
  (`:3020`), Supply (`:3038-3039`), Recover (`:2987`).
- `:3049` `a_distress_call_pays_its_base_and_twice_the_fee` (`:3053`).
- `:3193` `a_supply_job_pays_its_base_the_goods_at_ask_plus_half_and_twice_the_fee`.
- `:3272` raider count per band. `:3297`
  `an_escort_pays_150_cr_per_raider_over_half_its_cargo_and_the_fee`
  (literals 500 + 150 + 50).
- `:3421` hand-in plan and `StationCannotPay` (`:3483-3505`).
- `:3828` a renewed slot keeps the standing offers' rewards (`:3864-3865`).
- `crates/nova_interface/src/jobs.rs:2156`
  `a_hand_in_pays_completes_the_job_and_clicks_once`: balance 2,146 and cue
  list `[UiSfx::MenuSelect]` (`:2166-2169`). A new payout cue breaks it.
- `crates/nova_hud/src/hud_notice.rs:813`, `:873` job fixtures.
- `crates/nova_world_base/src/save/tests.rs:2099`, `:2388` `Job` literals;
  `:2242`, `:2483` draws.
- `crates/nova_authoring/src/base_content/items.rs:333` pins the test item
  fixture to the base catalog.
- Fixtures with a literal `reward_cr` (struct shape only):
  `crates/nova_world_base/src/goal.rs:247`,
  `crates/nova_gameplay/src/open_objectives.rs:101`,
  `crates/nova_interface/src/map/tests.rs` (1 use).

Examples that judge rewards or credits (all relative to `job.reward_cr`, none
pins a formula):

- `examples/systems/system_station_jobs.rs:395-400`, `:861` read the reward
  text; `:1008` asserts `station_cr >= job.reward_cr`; `:1042-1047` assert both
  balances after the hand-in, with the 50 cr bonus.
- `examples/systems/shared/located_jobs.rs:2192`, `:2221-2226` the same for
  located kinds.
- `examples/playable/job_board_sandbox.rs:841-865`, `:1001`, `:1022` read
  "<reward> cr" in the inspector and on the Map.
- `examples/systems/system_job_ammo_economics.rs:405`, `:490-500`, `:827-838`
  ledger reward against ammo cost.
- `examples/systems/system_scenario_jobs.rs:613`, `:673`, `:823`.
- `examples/playable/scenario_vocabulary_sandbox.rs:279`, `:691`, `:746`.
- `examples/systems/system_role_goals.rs:287`,
  `examples/screenshots/news_0180_events_blocks.rs:105` literals.

Docs that quote formulas or numbers:

- `web/src/wiki/interface.md:281-286` jobs table. Line 283 says each raider
  adds 150 cr. Line 286 says the reward covers the goods at ask plus half.
- `web/src/wiki/interface.md:292` hand-in text.
- `web/src/create/stations.md:600-605` all six formulas; `:659-660` hand-in.
- `web/src/create/trade-profiles.md:30`, `:54`, `:90` visit rewards.
- `web/src/create/events.md:66-69`, `web/src/create/actions.md:1619-1642`
  carry `reward_cr` as a field.
- `web/src/news/0.16.0.md:301-316` is released news; leave it.
- `CHANGELOG.md:16-68` `[Unreleased]`: no entry names a job reward value.
  Line 21 quotes the 200 cr SOS bill, which is `DISTRESS_BASE_CR`.
- Editor: `crates/nova_editor/src/inspect.rs:1185`,
  `crates/nova_editor/src/problems.rs:576`.

## J. The current pay table

Measured with a throwaway integration test in `crates/nova_authoring/tests/`
(deleted; `git status` is clean). It used the base part snapshot
(`repo_ship_part_packs(&["base"])`), `build_items()`, and the real
`station_job_offers`. Per advancement: 3 mixes x 5 cluster sets (none and each
single type) x 400 seeds x 3 slots = 18,000 offers. The field held 4 wrecks
from a pool of 480 (`generate_wreck` + `ship_stock`, 4 roles x 3 civilizations
x 40 seeds, at the poster's advancement) and 3 hostiles.

Rewards in cr, min / median / max:

| Kind | adv 0.0 | adv 0.15 | adv 0.5 |
|---|---|---|---|
| Procure | 62 / 284 / 5,050 | same | same |
| Recover | 64 / 454 / 8,210 | 76 / 870 / 19,490 | 234 / 7,538 / 139,730 |
| Escort | 364 / 978 / 1,587 | same | 532 / 1,218 / 1,887 |
| Distress | 300 / 500 / 700 | same | same |
| Supply | 318 / 750 / 2,140 | same | same |
| Neutralize | 500 / 700 / 900 | same | same |

Detail:

- Procure by cell: asteroid, rock and planet cells 62 / 226-236 / 442;
  derelict field 110 / 870 / 5,050; no cluster 68 / 630 / 4,950.
- Escort by raiders: 1 raider 364 / 978 / 1,587; 2 raiders 532 / 1,143 / 1,732;
  3 raiders 664 / 1,248 / 1,887. At 0.25 the count is 1 or 2.
- Recover p90: 1,464 at 0.0; 4,742 at 0.15; 34,230 at 0.5.
- The first four rings read at most 0.15
  (`crates/nova_world_base/src/civilizations.rs:57-61`). Advancement 0.5 is
  near 2,000 km out. So 0.0 to 0.15 is the start game.

Limits of the measure:

- Recover depends on the wrecks in a real cube. The test pool is synthetic.
  A wreck's civilization can differ from the poster's.
- Real cells mix cluster types; the test weights the five sets equally.
- The three mixes are weighted equally. Mix weights:
  `crates/nova_gameplay/src/jobs.rs:949-955`.

Rough work per kind (computed from constants; no play time was measured):

| Kind | Work | Fixed timers |
|---|---|---|
| Procure | buy, mine or already hold the stack; hand in | none |
| Recover | fly 2 to 55 km, dock the wreck, take the stack, undock, fly back; the player keeps the stack (`crates/nova_gameplay/src/jobs.rs:2619-2630`) | none |
| Escort | meet the hauler at 1 km, 9 km at 100 m/s, fight 1 to 3 raiders near 7.5 km, fly 10 km back | 90 s cruise (`crates/nova_world_base/src/jobs.rs:110`) |
| Distress | fly 10 km, dock, fly 10 km back | none |
| Supply | get the goods, fly 10 km, dock, fly back | none |
| Neutralize | fly 2 to 55 km, neutralize one hull, fly back | none |

The player's cruise speed was not found, so minutes per job are not given.

## Risks

1. Reward is drawn in four places and none shares a view struct: board row,
   inspector, Map chart panel, and the events (`crates/nova_interface/src/jobs.rs:1280`,
   `:1524`; `crates/nova_interface/src/map/chart.rs:1826`;
   `crates/nova_events/src/lib.rs:914`). A reward breakdown (base, distance,
   risk) has no home in `Job` today; `Job` holds one `reward_cr`.
2. Draw determinism. `SlotDraw` takes a fixed number of draws per slot, and "A
   new draw goes last, so the draws before it keep their numbers"
   (`crates/nova_gameplay/src/jobs.rs:2194-2226`). A new random input must go
   after `raiders`. Changing only the arithmetic keeps kinds and targets.
   Tests `:2872` and `:3828` hold this.
3. Standing offers are saved. A development save keeps old rewards on standing
   and held jobs until each slot renews (section C).
4. The fitted-offer path re-draws slot 0 and re-prices it
   (`crates/nova_gameplay/src/jobs.rs:2171-2188`, `:2346-2379`). A new formula
   must cover `fitted_posting` too, or the starter board pays by the old rule.
5. Distance is not in the draw's inputs. The cell distance is free from
   `id.origin`. Metres need a new `JobField` input from
   `crates/nova_world_base/src/jobs.rs:607-661`, where the queries have no
   transform. Escort, Distress and Supply have one constant leg, and the
   constant is in the world crate (`crates/nova_world_base/src/jobs.rs:102`).
   Scaling by distance does nothing for them unless the leg varies too. The
   leg direction and the "mark stays in the poster's cell" rule
   (`:960-1001`) limit how long a leg can be.
6. Risk is not in the draw's inputs. Neutralize pays a flat 500 to 900 for an
   unarmed 27-section hull or a 1,700-section warship (section B). The poster's
   advancement is the only proxy. A hull measure needs a new `JobHull` field
   and a role or section source on the live ship, which has none
   (`crates/nova_world/src/streaming.rs:505-541`).
7. Station balances. No refill, 10,000 cr at the start, refusal when short
   (section G). Recover already exceeds the balance at advancement 0.5.
8. Shared constants. `RESCUE_FEE_CR = DISTRESS_BASE_CR`
   (`crates/nova_gameplay/src/rescue.rs:32`) and `SUPPLY_BASE_CR =
   DISTRESS_BASE_CR` (`crates/nova_gameplay/src/jobs.rs:129`). Raising the
   distress base raises the SOS bill and breaks `CHANGELOG.md:21`.
9. Scenario and visit jobs carry authored rewards (section A). A global scale
   skips them. Base visit rewards (150 to 220 cr) then read low beside the
   board. Mods author `reward_cr`; scaling it at load would be a format break.
10. Tests that pin numbers (section I): the distress, supply and escort
    formula tests, the Neutralize lower bound, and the hand-in cue test.
11. The payout moment. The pane rule is "no result text"
    (`crates/nova_interface/src/pane.rs:903-907`), and `CHANGELOG.md:39`
    records the removal of result lines this cycle. The HUD notice is hidden
    and frozen under the pane, and it has one fixed cue (section E). The pane
    is rebuilt on every view change (`crates/nova_interface/src/jobs.rs:761-801`),
    so an animated row dies on the hand-in frame; the pane head balance
    (`crates/nova_interface/src/pane.rs:617-651`) survives rebuilds.
12. Examples read the reward as "<digits> cr" text and assert
    `station_cr >= job.reward_cr` (section I). A new reward layout or a station
    that cannot pay fails those walks.
13. Stale comment: `RAIDER_LATERAL` doc says "about 2.2 km of its 7 km"
    (`crates/nova_world_base/src/jobs.rs:115-118`); the leg is 9 km.
