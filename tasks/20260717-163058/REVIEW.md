# Review: Beat-sheet pass (lint arms + content compliance + graces + wiki)

- TASK: 20260717-163058
- BRANCH: content/beat-sheet-pass

## Round 1

- VERDICT: APPROVE

Verified by running in the worktree (not taken on trust):

- `cargo test -p nova_scenario --features serde`: 115 lib tests + 1 e2e
  green, including the new `beat_sheet_arms_warn` (which asserts exact
  issue counts and message substrings, so deleting either arm would turn
  its expected 1 issue into 0 and fail the test).
- `cargo test -p nova_assets`: all green, including content_ron_parity
  2/2, ledger_ch2_encounter 12/12, broadside_assault 11/11,
  content_lint_gate 2/2, webmods_validation (strict-RON load of the
  edited ledger files), balance_audit_gate.
- Mutation check of the new arm through the real bin: restored master's
  ledger_ch1.content.ron into the worktree and ran
  `cargo run -p nova_assets --bin content_lint` - the old dead line was
  flagged ("a StoryMessage beside an Outcome is never read..."); with the
  branch content restored the lint is clean except the pre-existing acked
  ch4 dual-spawn warning. So the arms fire on real violations and stay
  quiet on the shipped tree, exactly as claimed.
- `cargo run -p nova_assets --bin gen_content` then `git status`: no
  drift - builders and generated .ron are in sync (the corvette() change
  correctly touches only broadside.content.ron x2 + shakedown_run x1;
  corvette() is not used by broadside_gunship).
- `cargo run -p nova_assets --bin balance_audit`: 0 errors, 0 warnings,
  2 acked - "graces do not move spawns" holds.
- NOT verified locally: "workspace --all-targets green" (standing
  instruction: full suite runs on CI only).

Spot-checks that came back clean:

- The dwell-fixture commit (9700d19c) isolates rather than weakens: the
  three dwell lines moved from one handler into three one-line handlers,
  and the test still asserts exactly the one out-of-range-dwell warning
  for 120.0. Nothing about the dwell clamp assertion changed.
- The clock-gated teach beats (ch2a/ch2b): `teach_sent` is seeded to 0 in
  OnStart (comment even explains why - an undefined gate fails closed),
  the OnUpdate handler gates on `scenario_elapsed > 8`, `teach_sent == 0`
  and `act == 1`, and sets the flag before the line. One-shot holds; the
  act gate correctly suppresses the teach line once the wave is broken.
  `act` is seeded to 1 in OnStart in both files.
- All seven folds carry the original line's content into the Outcome
  message (see R1.1 for the trimmed clauses); none were silently deleted.
- The grace map matches the claim: 8.0 on ch2a magpie_1/magpie_2, ch2b
  both heavies, ch3 both nav-ambush magpies; 5.0 on both broadside
  corvettes (via the builder) and the shakedown scavenger (via the
  builder). Ch4's Auditor is untouched (both branches AI(()) hot, acked).
  Ch2's third AI is the friendly dray_mule (correctly ungraced); ch1's
  lone AI is the Neutral scout (no fight in ch1).
- EventActionConfig has no nested action containers, so the lint's flat
  scan over `event.actions` cannot miss hidden StoryMessages.
- Wiki section matches the engine: dwell clamp [3, 30] agrees with the
  lint's COMMS_DWELL range, `scenario_elapsed` / `engage_delay` /
  `delay` / `auto_advance_secs` all exist with the documented semantics,
  and "content_lint warns" is true (verified above).
- "Every fight gets a lead-in" holds where it changed: ch2/ch2b announce
  in OnStart with ~600u spawns + 8s grace; ch3's nav ambush fires its
  warning line in the same handler as its graced spawns.
- Bundle 1.3.0 -> 1.4.0; the encounter test asserts "bumped past 1.0.0"
  (deliberately not an exact pin, per its own comment), so no test was
  edited to make the bump pass.
- The 9 -> 0 violation count is arithmetically honest: 7 dead lines
  (ch1, ch2a, ch2b, ch3, ch4 x2, example) + 2 double openings
  (ch2a/ch2b OnStart). TASK.md says "six dead lines" in the ledger step
  because the seventh (example arena) is base content - NOTES.md counts
  all seven. Consistent.

Findings:

- [ ] R1.1 (MINOR) webmods/the-ledger/ledger_ch1.content.ron:435,
  ledger_ch4.content.ron:351,380 - "the writing survives" (NOTES.md) is
  slightly overstated: three clauses were dropped in the folds, not
  moved. Ch1 lost "Bring it home slow, Kestrel."; ch4's SOLD ending lost
  "Try to look surprised when the yard asks how."; ch4's BURNED ending
  lost "I'll even pay." The rewrites are competent (first-person Okono
  voice does not fit the neutral banner), but the SOLD punchline in
  particular is strong writing and works in banner voice as an
  imperative: e.g. "...a manifest lists a box that never existed. Try to
  look surprised when the yard asks how." Suggest restoring that one (and
  optionally the other two), or amending NOTES.md to state the trims
  honestly instead of "the writing survives".
  - Response: split the difference per the suggestion - SOLD's punchline restored to the banner (it lands in banner voice); ch1's "Bring it home slow" and BURNED's "I'll even pay." stay trimmed as deliberate (Okono's second/first-person comms voice, wrong register for an impersonal overlay) and NOTES.md now states the trims explicitly instead of claiming a lossless move.

- [ ] R1.2 (NIT) CHANGELOG.md:23 - "every fight now announces itself and
  its attackers fly in on a readable arrival grace before going hot ...
  only the finale's Auditor keeps its by-design hot entrance" is a shade
  too absolute: the broadside gunship carries no engage_delay either -
  its readability comes from the ~720u far spawn, not a grace. Consider
  softening to "...on a readable arrival - far spawns or an explicit
  engage grace - ..." or naming the gunship alongside the Auditor.
  - Response: reworded - far spawns keep an ungraced entrance (the broadside gunship reads via its ~720u approach), the Auditor keeps its by-design hot drop.
