# Audit hardcoded scenario/mod references in source code

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: research, docs

Research task: find every place where source code references specific
scenario ids or specific mods, document them, and honestly answer "should
we be doing this?" for each pattern.

Prompted by three observations:

- The base scenarios are authored as .rs builders and converted to .ron in
  assets/base; confirm this is the case, how it happens, and that the game
  loads the .ron variant. Then judge: is this ok, or should we author .ron
  directly?
- "gauntlet" (a portal mod, not core content) is referenced in code and has
  dedicated tests in the core suite. That feels wrong for a mod, and the
  amount of testing feels like too much.
- nova_menu hardcodes which scenarios it uses (menu backdrop, new game
  start). Scenarios already carry a `hidden` flag; the menu roles could be
  data-driven flags too.

Deliverable: FINDINGS.md in this task folder with the full reference map
and an honest assessment per pattern.

## Notes

- 2026-07-16: audit complete, FINDINGS.md written.
- 2026-07-16: direction picked (user confirmed): keep .rs authoring for big
  scenarios; split generation into an explicit bin and make the parity test
  assert-only; remove the base demo scenario; remove deep mod-content
  behavior tests; decouple portal tests from named mods; menu backdrops
  become a `menu_backdrop` scenario flag with random pick (moddable), the
  new-game start becomes base-owned config (NOT moddable).
- Follow-up tasks created: 20260716-155816 (demo scenario removal),
  20260716-155823 (generator bin + assert-only parity), 20260716-155830
  (remove mod-content behavior tests), 20260716-155839 (decouple portal
  tests), 20260716-155849 (menu scenario roles).
- Research only; no code changed in this task. Reflection: the audit's
  grep-first approach worked well - the one non-obvious find was that the
  RON "converter" is a test that writes files on first run, which pure
  file-listing would not have revealed; reading the parity test paid off.
