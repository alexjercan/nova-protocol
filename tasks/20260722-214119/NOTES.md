# NOTES - 20260722-214119 Ledger close-out

Closes umbrella 20260722-212808. Landed the four content commits' close-out:
lint/audit, version bump, tests, catalog regen, doc sweep. LIVE publish/push +
over-the-wire native/web install is the OWNER's Finish step (out of scope here).

## Per-surface summary

- **the-ledger.bundle.ron**: `meta.version` 1.5.0 -> 1.6.0 (content rework =
  MINOR bump, per 20260718-231601). Header comment left as-is: it describes the
  four-chapter structure and the two-act ch2 (still accurate); it states no
  version or per-chapter ending summary, so nothing stale to fix.
- **CHANGELOG.md**: added `## 1.6.0` at the top, four bullets grounded in the
  diff - pacing pass (clock-paced openings, lazy-posted objectives, beat-spaced
  comms with dwell, deferred ch2/burn Victory overlay), ch3 deepened (opening
  act + per-gate breathers + NEW debris-pinch hazard, ambush still stands), ch4
  diverging endings (sell keeps the telegraphed Auditor for a payday; burn
  avoids it entirely - no fight, no payout; distinct terminal Victories; dead
  handler + stale ack removed), per-chapter skybox identity + SetSkybox accents.
- **README.md**: rewrote the ch3 line (nav drops + debris pinch + Magpies, both
  optional) and the ch4 line (real divergence: sell = break the Auditor for the
  payday; burn = slip the belt, no fight, no payout - "clear, and broke"). Old
  ch4 line ("sell it or burn it, then survive the Auditor") was FALSE for burn.
  Updated Authoring notes to add the pacing vocabulary (dwell holds, clock-paced
  briefings, lazy-posted objectives, beat_gate spacing) and the skybox vocabulary
  (deliberate starting cubemap + SetSkybox accents).
- **web/src/wiki/dev/guide-make-a-mod.md**: line 72 `The Ledger 1.0.0 -> 1.5.0`
  -> `1.0.0 -> 1.6.0`.
- **docs/news-0.8.0-the-ledger.md**: NEW durable cycle note (docs/ is ephemeral
  release-scratch, wiped at tag) for the v0.8.0 news writer - "The Ledger 1.6.0"
  section, 4 player-facing bullets (pacing, ch3 teeth, forking finale, per-chapter
  sky). Game-centric, no agentic-process angle.
- **crates/nova_assets/tests/ledger_ch2_encounter.rs**: NO change needed. The
  `the_bundle_ships_both_parts_and_the_bump` test asserts the version is a RANGE
  (`> [1,0,0]`), which 1.6.0 satisfies; no geometry pin touched by the bump.

## Lint / audit (Step 1)

`content lint --target the-ledger`:
`0 error(s), 0 warning(s), 0 finding(s), 5 scenario(s) balance-audited, 1 acked`.
The 1 ack is the ch4 Auditor close-spawn (301u) on the SELL branch only - the
burn branch no longer spawns the Auditor and its duplicate ack was pruned. Clean.

## Tests (Steps 3/5)

`cargo test -p nova_assets --test ledger_ch2_encounter --test ledger_ch3_channel
--test ledger_ch4_ending --test ledger_skybox --test gen_portal_gate` - EXIT 0:
- gen_portal_gate: 4 passed
- ledger_ch2_encounter: 12 passed
- ledger_ch3_channel: 9 passed
- ledger_ch4_ending: 10 passed
- ledger_skybox: 6 passed

## Portal catalog (Step 6) - LOCAL only, NOT pushed

`gen-portal.py --source webmods --shipped assets/mods.catalog.ron --out
$CLAUDE_JOB_DIR/tmp/ledger-catalog` - EXIT 0:
`portal: published 2 mod(s) ... the-ledger 1.6.0 (8 files, 432604 bytes)`.

catalog.json entry proof (from `entries[]`):
```
id: the-ledger
version: 1.6.0
meta.version: 1.6.0
icon: None          (bundle meta declares no icon - unchanged from prior releases)
screenshots: []     (bundle meta declares none - unchanged)
files: CHANGELOG.md, README.md, ledger_ch1..ch4 content.ron, ledger_ch2b, the-ledger.bundle.ron
       (8 files, each with size + sha256)
```
Per-version tree written at `the-ledger/1.6.0/` (all 8 files). The portal-level
icon/screenshots are empty because the mod's bundle meta declares none (this is
unchanged from 1.5.0, not a regression); per-scenario `thumbnail` fields live
inside the content RON. gen_portal_gate test green confirms the generator output
matches the gate.

LIVE publish/push + native/web over-the-wire install verification = OWNER's
Finish step (per GOAL.md landing scope). Not done here.

## Doc-sweep grep (Step 7)

`grep -rniE "ledger" web/src/wiki/ README.md CHANGELOG.md`:

| hit | stale? | action |
|-----|--------|--------|
| web/src/wiki/dev/development.md:578 (lessons ledger = LESSONS.md) | no | unrelated - "lessons ledger", not the mod. left. |
| web/src/wiki/dev/mod-portal.md:87 (in-repo id example `the-ledger`) | no | just an id example, no version/chapter/ending claim. left. |
| web/src/wiki/dev/guide-make-a-mod.md:72 (`The Ledger 1.0.0 -> 1.5.0`) | YES | FIXED -> `1.0.0 -> 1.6.0`. |
| web/src/wiki/dev/guide-make-a-mod.md:87 (`--target the-ledger` example) | no | lint invocation example, no stale fact. left. |
| CHANGELOG.md:45,86,88,90,101,102,121,142 (root repo changelog) | no | HISTORICAL release entries - append-only record of what shipped in PAST versions, describing the state at that release, not a live "current" claim. Editing them would falsify history. left. |

Additional grep for `ending|chapter|1.x.x|auditor|burn|sell|four` over the two
wiki dev pages surfaced no other live Ledger version/chapter/ending claim - the
only prose ending description was in the mod README (fixed) and the docs news
note (new). No `tasks/` files edited (append-only history).

## Reviewer-scrutiny points

- CHANGELOG/README/news prose is written from `git diff 803a4e0c~1..HEAD --
  webmods/the-ledger`, not from the task summary. Key claims verified against the
  RON: ch2/ch2b start `cubemap_alt.png` and have NO SetSkybox (sky unchanged by
  design); ch1 SetSkybox fires on the "fourth return / not on any manifest"
  reveal; ch3 SetSkybox fires at the pinch warning; ch4 SetSkybox fires on the
  sell/Auditor OnEnter (burn path has none). The burn ending truly spawns no
  Auditor (its whole spawn block + the choice==2 auditor-death handler were
  deleted); the sole surviving auditor-death handler is choice==1 (sell).
- The root CHANGELOG.md was deliberately NOT edited: those are historical
  per-release entries. Only the mod's own CHANGELOG and the live wiki version
  claim were updated.
- Catalog icon/screenshots empty is not a regression - the bundle meta has never
  declared them; the mod's visual identity is per-scenario cubemaps/thumbnails
  inside the content RON, which are unaffected by the portal manifest.
