# Review: Relocate input-prompt key glyphs to assets/ (Alt only) + credits

- TASK: 20260728-233707
- VERDICT: APPROVE
- ROUNDS: 1 (out-of-context reviewer)
- DATE: 2026-07-29

## Round 1 - APPROVE (no findings)

Out-of-context reviewer checked the branch `chore/relocate-input-glyphs`
(commit 59a2cd84) against all 6 Definition-of-Done items and scrutinized the
non-mechanical parts.

### DoD verification (all pass)

1. `assets/input-prompts/keyboard/Alt` = 99 files (98 PNG + 1 SVG);
   `git ls-files examples/ui/assets` empty.
2. `_Key_(Dark|White)\.png` in git = 0.
3. `grep examples/ui/assets` (excl tasks/node_modules/dist) = 0 live hits.
4. CREDITS.md "FREE Input Prompts" entry present;
   `credits/licenses/FREE-Input-Prompts_CC0-1.0.md` exists; Kenney link now
   points to the real `Kenney_Space_Kit_License.txt`.
5. Webpack copy rule change sound (`../assets/input-prompts` ->
   `nova-hud/assets/input-prompts`), consistent with deployed `assets/...`
   srcs under the `/nova-hud/` publicPath.
6. onRoute shim logic reviewed and correct (build + file:// eyeball run by
   the implementer, screenshot confirmed glyphs render).

### Scrutiny (no issues)

- onRoute path-rewrite shim: rewrites only when `!onRoute`, guards on the
  `assets/input-prompts/` prefix, prepends `../../`. All 8 glyph imgs carry
  `class="ki"` (7 cue/dock chips + the NOVA OS navbtn), so the single
  `img.ki` selector covers every glyph including the navbtn - the task text's
  separate `.navbtn img` mention is subsumed, no gap.
- Sweep complete: no stale `examples/ui/assets`, Dark/White, or `NOTICE.md`
  references anywhere in code/docs/web/scripts/workflows. Old broken
  `kenney-space-kit_CC0_1.0.md` link survives only in this task's own history.
- CREDITS entry: pack/author/v1.4/source links/CC0/dates/Alt-only/trademark
  note all present, provenance absorbed from the deleted NOTICE.md.
- License file: full authentic CC0 1.0 Universal legal code + provenance
  header, mirrors existing license files.
- No `dist/` or `node_modules/` artifacts committed; single well-scoped commit.
- `T_Crtl_Key_Alt.png` upstream typo preserved verbatim.

Verdict: APPROVE. Ship it.
