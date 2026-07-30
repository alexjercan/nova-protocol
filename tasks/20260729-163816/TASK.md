# Objective read-notification stack: demo-2 top-centre chips, retire the status-bar hint

- STATUS: CLOSED
- PRIORITY: 79
- TAGS: v0.9.0,feature,ui,hud,feedback
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

Owner playtest of 20260728-175747 (2026-07-29): "objectives still look the
same, not like in the reference HTML". Correct - that task added the POP and
BREATH to the existing top-right hint, but the hint is a count, not the
reference's objective chip. Demo 2 (`examples/ui/hud_rework_poc.html:246`)
shows a top-centre amber bordered chip carrying the objective itself:
`<div class="chip obj">&#9670; SALVAGE WRECK <span class="u">// 2.1 km</span></div>`,
which pops on posting and then breathes.

Today the game splits that across three things and lands none of them: the
top-right hint (star + active COUNT + TAB keycap, flat, parented into the bcs
status bar), the diegetic reveal card (the text, but transient - it tucks into
the hint after ~3.2s), and the world-anchored objective marker chips
(`objective_markers.rs` - diamond + label + live range + breath, but riding the
target entity, and only for objectives that declare an `ObjectiveMarkerTarget`).
After the card tucks away, nothing on the flight HUD says WHAT the objective is.

This task builds the reference's chip as a READ-NOTIFICATION STACK and folds
the hint into it, retiring the status-bar hint. See DECISION.md for the fork
and the owner's call.

## Steps

- [x] New `hud/objective_stack.rs`: a fixed top-CENTRE column of amber chips,
      one per active-and-UNREAD objective, newest first, in the demo's `.obj`
      language (`chip_paint(ChipTone::Amber)` from nova_ui, diamond glyph,
      objective label, dim `// <range>` suffix using `nova_ui::units` when the
      objective has a marker target to measure to - omit the suffix when it
      does not). Chrome tier. Stacks: several postings coexist as rows.
- [x] Read-notification lifecycle per chip: posted -> UNREAD (pop 1.16 for
      1.2s via `HudEmphasis`, then the slow 2.4s breath) -> READ, on EITHER a
      dwell elapsing OR the player opening NOVA OS (`PauseStates::NovaOs`) ->
      fades out and leaves the stack. A read chip never returns; a CHANGE to
      its objective (text edit, completion) re-posts it as unread. Opening
      NOVA OS marks EVERY currently shown chip read at once (you just read
      them all).
- [x] The TAB affordance rides the stack, not the status bar: the keycap sits
      on the stack (once, not per chip) and leaves with it. No standing cue in
      idle cruise - that is the point of the read model.
- [x] Retire the status-bar hint (`hud/objective_hint.rs`): the count + star +
      TAB block leaves `StatusBarRootMarker` entirely, so the bar is fps +
      version again. Re-home what the hint owned:
      - `NovaOsTabAnchor` (the reveal card's tuck target) is published from
        the STACK's screen rect, so the card still tucks into the thing that
        then pops. Reuse `ObjectiveRevealTucked` (20260728-175747) as the pop
        trigger.
      - the breath + pop the hint just gained move onto the stack chips.
      - the `Status`-tier / NOVA-OS-exempt reasoning that applied to the hint
        as a bar child no longer applies; the stack is ordinary flight chrome
        and hides with the rest of the HUD.
- [x] Reconcile with the world-anchored `objective_markers` chips: they keep
      the "go HERE" job (clamp + chevron on the target). Decide and RECORD
      whether a marker-target objective shows its label in both places at once
      or the stack chip drops its range suffix while its marker is on screen -
      the two carry the same text and must not read as a duplicate bug.
- [x] Docs sweep (keep-docs-in-sync): wiki `hud.md` (the objective paragraph +
      the "what is on screen, and when" list), `web/src/index.html` if the
      objective wording there goes stale, CHANGELOG [Unreleased]. Also update
      20260724-134312's and 20260724-161545's live-doc claims ONLY where they
      are live surfaces - the task records themselves are history, leave them.

## Definition of Done

1. test: App-driven - a posting spawns a chip carrying the objective's LABEL
   (not a count); two postings stack as two chips; a chip pops then settles to
   the breath on virtual-time advance.
2. test: the read model both ways - a chip leaves after its dwell, AND opening
   NOVA OS (`PauseStates::NovaOs`) marks every shown chip read so the stack is
   empty on close; a re-post after a change makes it unread again.
3. test: the status bar no longer parents an objective block
   (cmd: `grep -rn "StatusBarRootMarker" crates/nova_gameplay/src/hud` shows no
   objective site), and `NovaOsTabAnchor` is published from the stack (the
   reveal's existing tuck test still passes against the new anchor source).
4. cmd: `cargo run -p nova_probe -- run playable` passes.
5. manual: owner playtest - the objective reads like demo 2's chip, several
   objectives stack, and the notification clears itself by time or by opening
   NOVA OS.

## Notes

- Follow-up to 20260728-175747 (contextual HUD) from the owner's playtest;
  supersedes the presentation half of 20260724-134312 (minimalist top-right
  hint) and 20260724-161545 (hint becomes a status-bar item). Those tasks stay
  CLOSED as history; DECISION.md records the supersede.
- The stack must stack politely with the top-centre scenario readout strip
  (`hud/readout.rs`, `top: 16px`, grows down) - a time-trial run timer and an
  objective posting are both top-centre.
- `HudEmphasis` / `HudSituations` / `HudContextGate` from 20260728-175747 are
  the mechanisms to build on; do not add a second emphasis or gate mechanism.

## Implementation notes (2026-07-29)

Full record: `NOTES.md`. Deviations worth reading as decisions, not misses:

- No `// <range>` suffix: an `Objective` has no link to a world entity
  (`ObjectiveMarkerTarget` carries a free-form label, no objective id), so
  there is nothing to measure to. Range stays the marker chip's job - which
  also settles the "does the label appear twice" step: different strings.
- A COMPLETED objective drops its chip rather than re-posting it (the plan said
  re-post): it is gone from `GameObjectives`, so the chip would be stale text,
  and completion already ghosts green via `objective_feedback`.
- The stack sits at 96 px, not demo 2's 58 px - measured: 58 lands on the
  scenario readout strip's run timer. A 2+ readout scenario can still reach it;
  the durable fix (one shared top-centre column) is recorded at the constant.
- The diamond is a rotated bordered square node, not `\u{25c6}` (the shipped
  font renders that as tofu) - the same trick `objective_markers` uses.
- Also fixed here: `LOCK_COMPOSED_FIRING_PEAK` from 20260728-175747 warned
  `never used` in the non-test build; it is now `#[cfg(test)]`.
