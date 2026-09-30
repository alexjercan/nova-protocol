# M3 Sol medium headless free-play evidence

- Revision: `16da6afbc`, clean `item-loop-m3` sprout before runs. Seed 7. Pi model `openai-codex/gpt-6-sol`, thinking `medium`; `pi --list-models` listed it. Both runs used `nix develop --command cargo run --features debug bench play`, no `--record`, `--ui quiet`. Runs were sequential. No gameplay or fixture files changed.
- Fixture check: `docking_warship_tender.content.ron` authors the catalog `block_line_warship` with `{PdcRound: 6000, RailSlug: 20, Torpedo: 12}` and the neutral damaged frame tender with `inventory: {}`. `range.content.ron` authors the `block_gunship` with `{PdcRound: 2000}` and an unarmed hostile derelict with `inventory: {}`. Both loaded and produced world observations; no required-inventory load blocker. These are unscored free-mode scenes (`outcome: none`, 0 objectives), so grade the stated goals from world views, not `finish` or exit 0.

## Run 1: real warship/tender capture - PASS

Plan: 6000 ticks, 40 acts, 420 s deadline; read docking page, lock Frame Tender, align and approach via RCS, capture without damage. Stop on capture, unsafe collision or budget. Proof: docking gates in `audit.jsonl` and `score.json.end.docking`, player and tender health.

```sh
cd /home/alex/.cache/sprouts/nova-protocol/item-loop-m3
nix develop --command cargo run --features debug bench play crates/nova_bench/scenarios/docking_warship_tender.content.ron --agent pi --model openai-codex/gpt-6-sol --thinking medium --seed 7 --ticks 6000 --turns 40 --deadline 420 --goal 'Free mode. Read the docking page. Lock Frame Tender, approach its port with RCS at low speed and dock Line Warship to Frame Tender. Pass only when me.docking.docked is true and connection names tender with no damage. Stop on capture, unsafe collision or budget; report observed docking gates.' --out /tmp/nova-m3-sol-bench-111-dock-16da6afbc --ui quiet
```

Exit 0; 2513 ticks, 9 turns, 75.6 s wall, score-reported model cost $0.1156. At tick 2511 the observed nearest collar pair was `starboard_collar`/`port_collar`, gap 9.2 m (limit 10), facing 4.0 deg (limit 15), relative speed 1.5 m/s (limit 5), spin 0.0 deg/s (limit 5), `eligible: true`. At tick 2513, `me.docking.docked: true`, connection `ship: tender` with those ports. Player HP stayed 28750/28750; tender finished 9990/9990; score damage 0, ammo 0, bad lines 0. `game.log` records `on_docking_connection_request` and `track_docking_transitions ... captured`. No game, fixture, agent, or harness failure observed.

Artifacts: `/tmp/nova-m3-sol-bench-111-dock-16da6afbc/{score.json,audit.jsonl,game.log,agent.log}` and `/tmp/nova-m3-sol-bench-111-dock-console.log`. Pi `agent.log` is empty; pi tool/text/usage events reside in the audit. Raw audit retained.

## Run 2: range PDC ammunition and target damage - PASS for observation, NOT a kill

Plan: 4800 ticks, 30 acts, 420 s deadline; read targeting/weapons/fighting pages, lock Derelict Hauler, hold about 1-2 km, fire PDCs while on target. Stop on world-observed ammo use and target damage, unsafe range or budget. Proof: successive `channel_in` magazine, contact health/range and player health, plus score ammo/damage. Destruction was optional.

```sh
cd /home/alex/.cache/sprouts/nova-protocol/item-loop-m3
nix develop --command cargo run --features debug bench play crates/nova_bench/scenarios/range.content.ron --agent pi --model openai-codex/gpt-6-sol --thinking medium --seed 7 --ticks 4800 --turns 30 --deadline 420 --goal 'Free mode. Read targeting, weapons and fighting pages. Lock Derelict Hauler, approach and hold a safe firing range of roughly 1-2 km, then fire PDCs while on target. Pass when world views show ammunition rounds spent and Derelict Hauler health reduced (destruction optional), with no player damage. Stop and finish on observed damage and ammo use, unsafe range or budget; report measured before/after values.' --out /tmp/nova-m3-sol-bench-111-range-16da6afbc --ui quiet
```

Exit 0; 1350 ticks, 14 turns, 86.4 s wall, score-reported model cost $0.2150. `channel_in` at tick 1064: lock `derelict`, range 1660.8 m, target HP 10380, six PDC magazines 500 each, player HP 10260. Tick 1154: all six magazines 349. Tick 1334: target HP 9900, magazines refilled to 500 (idle reload). Tick 1349/1350: target HP 9900, range 1659.1 m, magazines 474 each, player HP 10260. Score counts 1062 ammo spent across decreases, 0 damage, 0 kills, 0 bad lines. The initial 10380-to-9900 HP loss is world-observed, not inferred from the pilot report. Magazine refill means final magazine subtraction is not total expenditure; this play does not establish reserve conservation because the bench view/score does not expose inventory stacks. Derelict remains alive, as expected for the early stop. No game, fixture, agent, or harness failure observed.

Artifacts: `/tmp/nova-m3-sol-bench-111-range-16da6afbc/{score.json,audit.jsonl,game.log,agent.log}` and `/tmp/nova-m3-sol-bench-111-range-console.log`. Pi `agent.log` is empty; pi events reside in the audit. Raw audit retained.

## Limits and cleanup

No replay: neither goal failed, and neither audit shows a meaningful failure to reproduce. No footage or visual claim: headless by design. Neither play proves authored reserve depletion, empty-stock refusal, transfer/conservation, or a kill. `--ui quiet` does not suppress audit/game logs. Both outputs remain outside the repository for parent review. Safe cleanup candidates *after* review are these two `/tmp/nova-m3-sol-bench-111-*` run directories and console logs (about 0.5 MiB total); do not delete raw audits yet. The sprout `target/` is about 6.4 GiB and shared with this sprout's build cache; do not clean or delete it without parent review. Filesystem had about 326 GiB free after the plays. These first two runs made no gameplay or fixture changes.

## Follow-up: navigation, attempted jettison, and gravity-well flight

All three follow-up plays used the same clean M3 revision `16da6afbc`, seed 7, `--agent pi --model openai-codex/gpt-6-sol --thinking medium`, `nix develop --command cargo run --features debug bench play`, and no `--record`. Their `audit.jsonl` `run_start` entries retain the full goal text and tick/turn/deadline budgets. These unscored free-mode fixtures have no victory objective; `finish` and exit 0 are not proof of any mechanic.

| Fixture and raw output | Budget | World-observed result | Limit |
| --- | --- | --- | --- |
| `range.content.ron`, `/tmp/nova-m3-sol-rich-range-16da6afbc-20260926-01/` | 5,400 ticks, 36 acts, 420 s | At tick 12 WORK MARK was 2,576.8 m away. At tick 897 the gunship was at `[-449.9, 98.9, -489.9]` m, about 672 m from its start; WORK MARK was 1,905.2 m away. Speed was 0.4 m/s after braking and HP remained 10,260/10,260. | The pilot opened/cycled the interface but could not observe cargo, canisters, or jettison confirmation. Two invalid `system.interface_next_tab` lines were refused; the pilot then used `interface.interface_next_tab`. No jettison or streamed sector was proved. |
| `docking_warship_tender.content.ron`, `/tmp/nova-m3-sol-rich-warship-16da6afbc-20260926-01/` | 5,400 ticks, 36 acts, 420 s | The undocked warship moved from `[0, 0, 0]` to `[2.8, 0.4, -1761.8]` m by tick 1479; tender range rose from 172.4 to 1,793.8 m. It stopped at 0.4 m/s; HP remained 28,750/28,750. | The fixture authors stocked ammo, but neither the condensed bench view nor the game log exposed inventory/canister changes. Interface attempts did not establish jettison. No sector materialization was observed. |
| `slingshot.content.ron`, `/tmp/nova-m3-sol-rich-slingshot-1lOaUt/` | 12,000 ticks, 80 acts, 900 s | The gravity well was absent at tick 481, `kestrel` at ticks 721-1931, and absent at tick 2051. EXIT range fell from 13,000 m at tick 1 to 311.8 m at tick 2351. At tick 3431 the ship was at `[691.8, 34.7, -21077.5]` m, with full 10,260 HP and outside the well. | It overshot EXIT: final range was 8,107.1 m. Late Fee remained at 0 m/s even after 45 game seconds; range grew from 19,212.7 m at tick 2831 to 23,587.7 m at tick 3431. Safe separation is observed, but active-pursuit evasion is not proved. |

The follow-up score files report 897/1479/3431 ticks, 11/14/15 acts, zero damage, zero sections lost, and no cheats for range/warship/slingshot respectively. Range has two bad lines; the others have none. The slingshot audit has no refusals. Read the `channel_in` records for the measured state; the pilot's `agent_report` is not the source of these results. Raw `score.json`, `audit.jsonl`, `game.log`, and empty pi `agent.log` files remain in each output directory. The slingshot directory also contains `bench.stdout`.

**Not established:** a successful inventory jettison, stock conservation, canister spawn, or generated-sector/chunk materialization. The bench's condensed world view does not expose inventory stacks or canister counts, and the two attempted inventory runs did not deliver a usable UI confirmation. These particular fixtures did not arm the base open-world stream: `NovaWorldBasePlugin::sync_open_world` requires a live `ScenarioRole::OpenWorld`, one player ship, and an `OpenWorldSession` seed (`crates/nova_world_base/src/lib.rs`). A loose `.ron` file is not inherently unable to stream; these specific plays have no sector-materialization evidence. Their authored rocks/planets are not generated sectors. For a future bench proof, use a New Game session or a purpose-built streamed fixture with world-state observation, and expose inventory/canister state or run a rendered UI flow for jettison. Do not substitute the M5 mining demo for an M3 jettison proof.

No source, fixture, or generated content was edited for these plays. Keep raw `/tmp` audits for review; do not clean the shared sprout target while it is in use. This report records headless world-state evidence only, not rendered UI behavior.
