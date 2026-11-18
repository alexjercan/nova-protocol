# Job pay table, before and after

Rewards in cr. Each cell is min / median / max.

## Method

A throwaway integration test in `crates/nova_authoring/tests/` (deleted after
the run), the same shape as the measure in `MAP.md` section J:

- Base part snapshot (`repo_ship_part_packs(&["base"])`), `build_items()` and
  the real `station_job_offers`.
- Per advancement: 3 mixes x 5 cluster sets (none and each single type) x 400
  seeds x 3 slots = 18,000 offers.
- The field holds 4 wrecks and 3 hostiles.
- Wrecks: a pool of 480 (`generate_wreck` + `ship_stock`, 4 roles x 3
  civilizations x 40 seeds, at the poster's advancement).
- Hostiles: a pool of 480 intact hulls (`generate_ship`, the same 4 roles x 3
  civilizations x 40 seeds). A hull's `weapons` is the count of turret, torpedo
  bay and railgun sections in its design. Civilian and industrial hulls count 0.
  Pool weapons, min / median / max: 0 / 1 / 2 at advancement 0.0, 0 / 1 / 4 at
  0.15, 0 / 4 / 10 at 0.5.
- Field `leg_m`, the new input: three bodies in four stand 2,000 to 16,000 m
  from the poster (its own cluster) and one in four 30,000 to 55,000 m
  (another cluster), drawn evenly and rounded to 100 m. This follows the
  computed geometry in `MAP.md` section B. Measured: min 2,000, median 11,400
  (wrecks) and 11,500 (hostiles), max 55,000.

The Escort rows under After come from a second run, made when an escort's
leg became 10 to 16 km. It was measured, not corrected by arithmetic:

- A throwaway unit test in `crates/nova_gameplay/src/jobs.rs` (deleted after
  the run) over the real `station_job_offers`, with that file's test catalog
  and test field. An escort's reward reads no item and no field body, so
  these do not move it.
- The same 3 mixes x 5 cluster sets x 400 seeds x 3 slots per advancement, on
  the station `sector_1_0_0_cluster_0_0_0_station_0`: 2,930 escorts. The
  first run's station id was not recorded and its run gave 2,905.
- Check of the second run against the first: with each escort's leg drawn
  by the first run's rule (6 to 16 km), the same test gives 807 / 1,319 /
  1,799 at advancement 0.0, 921 / 1,480 / 2,003 at 0.15 and 2,687 / 3,828 /
  4,879 at 0.5. The first run gave 813 / 1,314 / 1,781, 927 / 1,474 / 1,985
  and 2,763 / 3,836 / 4,861.
- The median takes the upper middle of an even count. p90 is the value at
  index `9n / 10` of the sorted rewards.

## Before

Copied from `MAP.md` section J.

| Kind | adv 0.0 | adv 0.15 | adv 0.5 |
|---|---|---|---|
| Procure | 62 / 284 / 5,050 | same | same |
| Recover | 64 / 454 / 8,210 | 76 / 870 / 19,490 | 234 / 7,538 / 139,730 |
| Escort | 364 / 978 / 1,587 | same | 532 / 1,218 / 1,887 |
| Distress | 300 / 500 / 700 | same | same |
| Supply | 318 / 750 / 2,140 | same | same |
| Neutralize | 500 / 700 / 900 | same | same |

Recover p90: 1,464 at 0.0; 4,742 at 0.15; 34,230 at 0.5.

## After

| Kind | adv 0.0 | adv 0.15 | adv 0.5 |
|---|---|---|---|
| Procure | 62 / 294 / 5,050 | same | same |
| Recover | 212 / 670 / 2,627 | 269 / 942 / 4,369 | 513 / 2,684 / 5,720 |
| Escort | 930 / 1,373 / 1,810 | 1,080 / 1,544 / 2,014 | 2,930 / 3,912 / 4,890 |
| Distress | 430 / 690 / 930 | 544 / 850 / 1,134 | 810 / 1,220 / 1,610 |
| Supply | 460 / 890 / 2,250 | 574 / 1,050 / 2,418 | 840 / 1,450 / 2,810 |
| Neutralize | 413 / 1,073 / 2,788 | 522 / 1,409 / 3,895 | 776 / 2,868 / 7,082 |

p90:

| Kind | adv 0.0 | adv 0.15 | adv 0.5 |
|---|---|---|---|
| Procure | 1,450 | 1,450 | 1,450 |
| Recover | 1,667 | 2,296 | 4,450 |
| Escort | 1,618 | 1,816 | 4,588 |
| Distress | 870 | 1,056 | 1,510 |
| Supply | 1,420 | 1,572 | 1,990 |
| Neutralize | 2,030 | 2,653 | 4,790 |

### Term medians

Each cell is the median of that term alone over the kind's offers, so a row
does not add up to the median total. Base includes the fee (median 150).
`leg` is the median `leg_m`; `weapons` is the median `threat`.

Advancement 0.0:

| Kind | offers | base | goods | range | threat | leg m | weapons |
|---|---|---|---|---|---|---|---|
| Procure | 2,970 | 150 | 96 | 0 | 0 | 0 | 0 |
| Recover | 3,425 | 250 | 40 | 342 | 0 | 11,400 | 0 |
| Escort | 2,930 | 350 | 317 | 360 | 300 | 12,000 | 1 |
| Distress | 2,860 | 350 | 0 | 360 | 0 | 12,000 | 0 |
| Supply | 2,940 | 350 | 168 | 360 | 0 | 12,000 | 0 |
| Neutralize | 2,900 | 450 | 0 | 345 | 300 | 11,500 | 1 |

Advancement 0.15:

| Kind | offers | base | goods | range | threat | leg m | weapons |
|---|---|---|---|---|---|---|---|
| Procure | 2,970 | 150 | 96 | 0 | 0 | 0 | 0 |
| Recover | 3,425 | 280 | 120 | 445 | 0 | 11,400 | 0 |
| Escort | 2,930 | 410 | 317 | 468 | 300 | 12,000 | 1 |
| Distress | 2,860 | 410 | 0 | 468 | 0 | 12,000 | 0 |
| Supply | 2,940 | 410 | 168 | 468 | 0 | 12,000 | 0 |
| Neutralize | 2,900 | 540 | 0 | 449 | 300 | 11,500 | 1 |

Advancement 0.5:

| Kind | offers | base | goods | range | threat | leg m | weapons |
|---|---|---|---|---|---|---|---|
| Procure | 2,970 | 150 | 96 | 0 | 0 | 0 | 0 |
| Recover | 3,425 | 350 | 1,440 | 684 | 0 | 11,400 | 0 |
| Escort | 2,930 | 550 | 317 | 720 | 2,700 | 12,000 | 9 |
| Distress | 2,860 | 550 | 0 | 720 | 0 | 12,000 | 0 |
| Supply | 2,940 | 550 | 168 | 720 | 0 | 12,000 | 0 |
| Neutralize | 2,900 | 750 | 0 | 690 | 1,200 | 11,500 | 4 |

### Detail

- Escort by raiders. Advancement 0.0 and 0.15 post 1 raider with 1 weapon.
  Advancement 0.5 posts 2 raiders (6 weapons): 2,930 / 3,438 / 3,937, or
  3 raiders (9 weapons): 3,889 / 4,360 / 4,890.
- Distress and Supply legs: min 6,000, max 16,000 m, median 12,000 m. Escort
  legs: min 10,000, max 16,000 m, median 12,000 m. The legs of a range are
  drawn evenly, and the median of an even count takes the upper middle.
- Neutralize weapons: 0 / 1 / 2 at 0.0, 0 / 1 / 4 at 0.15, 0 / 4 / 10 at 0.5.
  An unarmed hull pays base and range alone: the minimum of each column.
- Recover's largest reward drops from 139,730 to 5,720 at advancement 0.5:
  the goods term stops at 2,000 cr.
- Procure's rule did not change: the goods at ask x 3/2 plus the fee. Its
  min and max are the same. Its median moves from 284 to 294. Not verified:
  the cause is taken to be the re-created test, whose station id and field
  picks differ from the first run's.

## Limits

- Field legs and the hostile role mix are synthetic. The four hostile roles
  weigh the same; a real civilization weighs its roles.
- Real cells mix cluster types; the test weights the five sets equally. The
  three mixes weigh the same.
- No play time was measured.
