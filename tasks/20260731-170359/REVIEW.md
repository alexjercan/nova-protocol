# Review: KISS pass on nova_menu - split lib.rs, cut comment fluff

- TASK: 20260731-170359
- BRANCH: refactor/nova-menu-split

Worktree: `/home/alex/.cache/sprouts/nova-protocol/refactor/nova-menu-split`

## Round 1

- REVIEWER: out-of-context `general-purpose` subagent (id ab2aa89ac97a915d7)
- VERDICT: REQUEST_CHANGES

Prompted with the task ID, worktree, dimensions and record format only. The
primary re-ran every check and independently re-derived findings 1, 2, 3, 4, 5,
6 and 7 by reading the cited lines and recounting against
`git show master:crates/nova_menu/src/lib.rs`.

### Checks

| Check | Result |
|-|-|
| `cargo check --workspace --all-targets` | exit 0; only the 4 pre-existing `nova_gameplay` warnings |
| `cargo fmt --check` | clean |
| `cargo test -p nova_menu --lib` | 76 passed, 0 failed |
| DoD 3 HUID grep | 1 hit, the deliberate `flex_shrink` NOTE |
| DoD 4 `wc -l` | largest 875 (`mods.rs`); matches the NOTES table |

The reviewer independently confirmed the behavior-preservation claims: the
`Plugin::build` body is byte-identical to master modulo comments, the
item-name multisets match, the `#[test]` multiset is 76/76 identical, and the
28 base-only non-comment lines are all import fragments or rustfmt re-wraps.
Public surface is `NovaMenuPlugin` + `prelude` only. No BLOCKER.

Every finding is on the comment axis - specifically, damage the regex
provenance-strip did to prose that the repair pass missed.

### Findings

**MAJOR**

1. `crates/nova_menu/src/mods.rs:36` - regex damage survived the repair pass.
   Master read `an inert placeholder until task 20260715-142916 wires it to the
   portal client`; the stripped text reads `an inert placeholder untilwires it
   to the portal client`. A glued word in live rustdoc, and it falsifies the
   close-out's "all six repaired by hand". **Change:** restate without the
   dangling pointer - Explore has landed, so say what is true now.

**MINOR**

2. `crates/nova_menu/src/mods.rs:471` - `DepStatus`'s rustdoc opens with
   `spawn_mod_details_header`'s paragraph before its own sentence. Third
   instance of the orphan-doc class the pass already fixed twice
   (`tests/support.rs` `app()`, `mods.rs` `mod_dep_graph`) and enumerated in
   NOTES.md. Missed and unlisted. **Change:** drop the stray first sentence;
   add the site to NOTES.md "Defects the pass uncovered".

3. `crates/nova_menu/src/widgets.rs:33` - `button()`'s rustdoc opens with two
   sentences describing the per-frame colour-polling system this same pass
   deleted the reference to in `lib.rs`. Same orphan class; the rewrap merged
   it into one paragraph with the real doc, hiding the seam. **Change:** delete
   the first two sentences; the real doc starts at `The main-menu / pause /
   outcome button:`.

4. `crates/nova_menu/src/pause.rs:117` - the strip left `Since ... , so ...`,
   which is not a sentence. **Change:** `The outcome now drives the pause itself
   and ESC is inert over it, so this is a defensive guard rather than the normal
   path.`

5. `crates/nova_menu/src/mods.rs:78` - `the Explore task adds its
   Install/Uninstall/Update buttons` now dangles: the ID that identified "the
   Explore task" is gone, and Explore has landed. **Change:** restate as fact -
   `the Explore tab spawns its ... buttons into this same container`.

**NIT**

6. Rewrap split hyphenated code spans across lines, so rustdoc renders a
   spurious space: `tests/settings.rs:146` (`pin-each-caller-not- just-shared-
   core`), `tests/outcome.rs:274` (`probe-the-adversarial- variant`),
   `tests/scenarios.rs:20` (`bevy-ui-scroll-input-clamps-stored- offset`).
   **Change:** move the whole token to the next line.

7. `tasks/20260731-170359/NOTES.md` - "Task-HUID provenance clauses, 55 sites"
   is not reproducible. Master's `lib.rs` + `settings_store.rs` carry 68 HUID
   occurrences across 67 comment lines (63 + 4), of which 67 were removed and
   one kept. **Change:** correct the number.

8. `crates/nova_menu/src/menu_ui.rs:142`, `:204`, `:390` - each comment lost its
   subject with the provenance sentence, leaving a dangling participle over a
   `commands.spawn`. **Change:** fold the subject back in.

### Verdict

REQUEST_CHANGES. One MAJOR (finding 1), four MINOR, three NIT - all comment
text, none touching executable code. The structural half and behavior
preservation need no rework.

### Pending manual items

- DoD 6 - owner skims the diff and agrees no behavior changed. Supporting
  evidence in NOTES.md and in the reviewer's independent re-derivation above.

### Round 1 responses

All eight findings accepted; no pushback.

| # | Severity | Fix |
|-|-|-|
| 1 | MAJOR | `mods.rs:36` - dangling pointer dropped entirely; the doc now says `the portal browser (`portal.rs`)`, which is true today. |
| 2 | MINOR | `mods.rs` - `DepStatus`'s stray opening paragraph removed; site added to NOTES.md "Defects the pass uncovered". |
| 3 | MINOR | `widgets.rs` - `button()`'s two orphan sentences removed; site added to the same list. |
| 4 | MINOR | `pause.rs:117` - reworded to `The outcome now drives the pause itself ... so this is a defensive guard`. |
| 5 | MINOR | `mods.rs:78` - restated as fact: `the Explore tab spawns its ... buttons`. |
| 6 | NIT | Three hyphenated code spans moved whole onto their own line. |
| 7 | NIT | NOTES.md corrected: 67 of the base's 68 HUID occurrences, over 67 comment lines (63 in `lib.rs`, 4 in `settings_store.rs`). |
| 8 | NIT | `menu_ui.rs` - the three comments got their subject back. |

Findings 2 and 3 raised the orphan-docstring count from two to four, so NOTES.md
now names the class rather than listing two incidents: a docstring describing
the item ABOVE it compiles and tests clean, so only reading catches it.

Prompted by finding 1, the ad-hoc glued-word grep was replaced with a word-level
multiset diff of every comment in the crate against master. It reports no lost
or invented token beyond the deliberate deletions - the check that would have
caught `untilwires` the first time. Recorded in the close-out reflection.

Re-verified after the fixes: `cargo check --workspace --all-targets` exit 0,
`cargo fmt --check` clean, `cargo test -p nova_menu --lib` 76 passed, DoD 3 grep
still one hit, largest file still 875 lines.

## Round 2

- REVIEWER: primary agent (in-session; round-1 fixes are comment text only, each
  re-read at its cited line)
- VERDICT: APPROVE

Round 1's eight findings are all comment text; each fix was re-read at the cited
line. No fix regressed anything, and no new finding. The behavior-preservation
and structure conclusions from round 1 stand unchanged - the executable diff was
not touched between rounds.

### Pending manual items

- DoD 6 - owner skims the diff and agrees no behavior changed.
