# Conveyance gold text readability: no white mid-blends, steady label alpha

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.5.0,hud,polish,playtest

## Goal

Playtest feedback 2026-07-12 on the conveyance visuals (20260712-093831,
landed 63293fd): "the gold and white make the text not readable; the rest
is fine." Fix the readability of the gold text without changing the
approved design language. Two mechanisms in the shipped code plausibly
produce "gold and white" unreadability; address both:

- The hint-emphasis pulse mixes the row's base color toward gold
  (`base.mix(&OBJECTIVE_GOLD, lerp)` in keybind_hints.rs). For a LIT row
  the base is NAV_CYAN, and the cyan->gold RGB path passes through a
  pale washed near-white blend every cycle - low-contrast 12 px text.
- The objective marker chip breathes the LABEL TEXT's alpha down to 0.7
  (breathe_objective_markers in objective_markers.rs), thinning already
  small gold text over bright scene content (planetoid, skybox).

## Steps

- [x] Hint emphasis: replace the cross-hue mix with a single-hue gold
      pulse - the emphasized row's color stays OBJECTIVE_GOLD hue and
      pulses brightness/alpha only (e.g. available: alpha ~0.7..1.0;
      unavailable: a dim gold band so the spotlight-not-availability rule
      survives). No frame of the cycle may render a desaturated
      white-ish blend. Update the pulse tests (they assert != base and
      != full gold today; assert hue stays gold instead).
- [x] Objective marker chip: keep the label text at constant full alpha;
      carry the breath on the diamond glyph and chevron only. Consider a
      thin dark backdrop node behind the label (like a caption plate) if
      constant alpha alone is not enough - decide at /work by looking at
      contrast against the bright planetoid case.
- [x] Record the playtest verdict: append a dated note to spike
      20260712-140842's Open questions (gold accent approved in play;
      readability finding filed here; colorblind + marker-vs-reticle
      still open).
- [x] Verify: cargo fmt + check + the touched test suites
      (keybind_hints, objective_markers).

## Notes

- Playtest verdict (2026-07-12, user): conveyance visuals approved
  except gold text readability - this task is the only follow-up.
- Design spike: tasks/20260712-140842/SPIKE.md
- Follows: 20260712-093831 (landed 63293fd)
- Constraint: keep the four-hue language (gold = do this now); the fix is
  contrast/purity, not a color change.

## Close record

What changed: pulse_emphasized_rows renders pure OBJECTIVE_GOLD hue and
sweeps only alpha (available band 0.7..1.0, unavailable 0.3..0.5) - the
cyan->gold cross-mix and its washed near-white mid-blend are gone; the
marker chip label no longer breathes (constant full gold + a 1px black
TextShadow at 0.9 alpha for contrast), with the diamond and chevron
carrying the motion. Spike 20260712-140842 Open questions records the
playtest verdict. Tests: emphasized_rows_pulse_pure_gold_alpha_only
(hue equality across the whole wave + band separation + sweep guard),
label_stays_full_alpha_while_glyphs_breathe (steady label + shadow, with
a the-diamond-did-move delivery guard).

AS EXECUTED deltas from plan: the dark backdrop option resolved to
TextShadow (bevy_ui has it natively; a plate node would add layout
surface for no extra contrast). Verify used --all-targets per the fresh
check-all-targets-for-struct-field ledger lesson.

Difficulties: the first delivery-guard advance (period/4) landed exactly
on the wave crest where the alpha factor is 1.0 - indistinguishable from
the spawn color; period/8 fixed it. Same family as the exact-instant
pitfalls the ledger notes for wave-based asserts.

Self-reflection: the cross-hue mix was called out as pulsing "toward
white" in the ORIGINAL spike text and got converted to gold late in
design; the readability hazard of mid-blends was never re-examined after
that switch. When a color transition is replaced, re-ask what the
INTERMEDIATE frames look like, not just the endpoints.
