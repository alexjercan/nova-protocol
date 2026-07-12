# Review: Conveyance gold text readability

- TASK: 20260712-152340
- BRANCH: gold-text-readability

## Round 1

- VERDICT: APPROVE

Small focused diff (2 code files) reviewed against the playtest report
("gold and white make the text not readable"); the sweep-then-delete and
re-derive rules applied in-session:

- The cross-hue mix is fully retired: zero remaining references to
  EMPHASIS_LERP_MAX or `.mix(&OBJECTIVE_GOLD` in the workspace
  (grep-verified); emphasis_color is the single source of the emphasized
  color and is pure-gold by construction, pinned by a whole-wave hue
  test plus band-separation and sweep guards.
- The restore path is untouched and still covered by the app-driven
  gate tests (clear -> base, rig-despawn -> DIM), all green after the
  change - the alpha-band pulse still differs from both NAV_CYAN and
  DIM at every wave point, so no restore assert became vacuous.
- Re-derived: TextShadow (bevy_ui native) is consumed at render
  extraction like the other color components - no layout cost; a plate
  node would have added layout surface for no extra contrast, so the
  shadow is the right shape for the "consider a backdrop" plan clause.
- The marker label is out of the breath query by component removal (the
  breath marker no longer sits on the label node), not by a filter that
  could drift; the delivery-guarded test proves the glyphs still move
  while the label holds full gold + shadow.
- Availability semantics survive: dim band (0.3..0.5) sits strictly
  below the bright band (0.7..1.0), asserted per wave point.
- Checks: cargo check --workspace --all-targets clean (the ledger's
  check-all-targets lesson applied), fmt clean, keybind_hints 9 passed,
  objective_markers 5 passed.

No findings. MINOR observation, no action needed: the emphasized
unavailable row (gold 0.3..0.5) is slightly dimmer than the plain DIM
row (0.5); acceptable - the hue shift, not brightness, carries the
spotlight, and the playtest can retune the band constants.
