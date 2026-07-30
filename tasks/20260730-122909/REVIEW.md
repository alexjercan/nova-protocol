# Review: Floating chip background covers only a corner of its label

- TASK: 20260730-122909
- BRANCH: fix/chip-full-background

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

The reviewer ran the DoD proofs itself (`cargo check --workspace --all-targets`
clean; `cargo test -p nova_gameplay --lib chip` 29 passed; the DoD 3 multiline
grep read by hand across all 7 matches; `cargo test --test examples_smoke
screenshots` 1 passed in 175s), re-derived the load-bearing engine claims
(`update_arrows` writes only `.rotation`/`Visibility`; `UiTransform::compute_affine`
adds the translation to the node centre without rotating it; `Percent(50)` on the
absolute chevron resolves against a symmetric padding box), and re-ran the
fail-first experiment by moving the `Text*` components back onto the chip entity
- the rig failed on the measured numbers, and `labels_show_label_and_distance`
and `label_stays_full_alpha_while_glyphs_breathe` failed with it. It also opened
`hud-nav-chips.png` and confirmed both pills back their labels.

Pending user check (not a finding): DoD 6 is a `manual:` owner playtest item -
"Owner sees a full background behind BEACON 1 in the Shakedown run".

- [x] R1.1 (MINOR) examples/screenshots/screenshot_combat.rs:183 - the
  `combat_range` scenario's `nav_beacon` visual radius was shrunk 2.0 -> 0.5,
  but that beacon is the SUBJECT of `tutorial-radar-lock.png` (packaged by
  `scripts/gen-web-screenshots.py` and served as the tutorial's radar figure),
  and the justifying comment is copy-pasted from the new chip beat and does not
  describe this site. Revert to `radius: 2.0`; the chip beat spawns its own
  0.5-radius beacon, so nothing in this task needs the scenario change.
  - Response: Confirmed and fixed - an unintended edit (the scripted
    search/replace that set the chip beat's radius matched BOTH `BeaconConfig`
    literals). Scenario beacon restored to `radius: 2.0` with no comment; the
    chip beat keeps 0.5 and its comment. Re-ran the capture and eyeballed
    `tutorial-radar-lock.png`: the orb fills the NAV lock brackets as before.
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/objective_markers.rs:547 -
  `the_objective_chips_diamond_sits_inside_the_pill` hand-rolls a descendant
  walk matching the display `Name` string, duplicating
  `chip_layout_rig::only_descendant_with` and coupling the test to a display
  string. Give the diamond its own marker component and use the helper.
  - Response: Fixed - added a private `ObjectiveMarkerDiamondMarker` on the
    diamond bundle and the test now calls
    `only_descendant_with::<ObjectiveMarkerDiamondMarker>`.
- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/chip_layout_rig.rs:239 -
  `assert_child_sits_in_the_pill` only checks the child's CENTRE is inside the
  content box, while DoD 5 and the doc comment claim the glyph "sits inside" the
  pill; a diamond half outside the fill would pass. Assert the whole rect.
  - Response: Fixed - asserts `content.contains(rect.min) && content.contains(rect.max)`
    on the layout box, with a comment noting that a `UiTransform` rotation spins
    the paint inside the box without changing it and that the resulting corner
    reach is covered by the chip's padding.
- [x] R1.4 (NIT) crates/nova_gameplay/src/hud/objective_markers.rs:65 (and
  crates/nova_gameplay/src/hud/beacon_chips.rs:54) - `ObjectiveMarkerChipLabelMarker`
  / `BeaconChipLabelMarker` now mark the chip CONTAINER while the new
  `*ChipTextMarker` marks the label, so both public prelude names say the
  opposite of what they identify. Rename to `*ChipNodeMarker`.
  - Response: Fixed - renamed to `ObjectiveMarkerChipNodeMarker` and
    `BeaconChipNodeMarker`. No users outside the two modules and their prelude
    re-exports.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

Re-review of the round-1 delta only. The reviewer confirmed all four round-1
findings resolved: the scenario beacon is back to `radius: 2.0` with the
copy-pasted comment gone (it re-opened `tutorial-radar-lock.png` and saw the orb
restored), the diamond marker replaces the `Name`-string walk, the whole-rect
containment holds without slack, and a repo-wide `ChipLabelMarker` grep finds no
code, wiki or CHANGELOG hit. It re-ran `cargo check --workspace --all-targets`
(clean), `cargo test -p nova_gameplay --lib chip` (29 passed) and the beacon and
objective-marker modules alone (4 + 8 passed) to confirm the rename did not
orphan the suppress/restore observer tests.

- [x] R2.1 (NIT) tasks/20260730-122909/DECISION.md:48 (and TASK.md:101) - the
  R1.4 rename left two stale `*ChipLabelMarker` references in the task record,
  so a future reader greps for a name that no longer exists. Update both.
  - Response: Addressed differently, deliberately. The flow skill treats the
    task trail as append-only history - "once written, a task record is not
    rewritten to match a later rename" - and both hits are planning text
    describing what was ABOUT to be built. Rewriting them would erase the fact
    that the rename happened in review. Instead the task's Outcome section now
    records the rename explicitly and names both new markers, so the grep
    problem is solved forward rather than by editing history.
- [x] R2.2 (NIT) crates/nova_gameplay/src/hud/chip_layout_rig.rs:241 - the
  rotation-margin comment says the corners reach "sqrt(2)/2 of the box past this
  bound"; that is the half-diagonal from the centre, not the overshoot past the
  edge, which is `side * (sqrt(2)-1)/2` (~1.7 px). The conclusion is unaffected.
  - Response: Fixed - the comment now states the overshoot as
    `side * (sqrt(2) - 1) / 2` (~1.7 px for the 8 px diamond) and names the 9/4
    px padding that absorbs it.
