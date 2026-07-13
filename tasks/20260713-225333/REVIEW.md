# Review: trim the tutorial to first-scenario onboarding (20260713-225333)

- DATE: 20260713-225333
- VERDICT: APPROVE (round 1)

Reviewed the tutorial trim against the task Goal.

- The onboarding content is intact: intro, main-menu figure, game-loop/menu
  overview, the full Shakedown Run beat-by-beat (Parts 1-4 with figures), and
  the closing.
- The four reference sections + tables (Flight controls, Targeting and camera,
  Weapons, Interface) are removed. Nothing is lost: that reference already lives
  in the wiki Keybinds page (built this cycle), and the tutorial now points
  there.
- The wiki pointers link only to built, navigable pages (Keybinds, Ship
  sections, wiki index) - no links to coming-soon stubs, so no 404s.
- The blockquote wording that said "every verb on this page" is fixed to "every
  verb in order" (the verbs are no longer listed on the page).
- Title / meta description / prose__meta reworded to read as a first-hour guide.
- `npm run ci` green; built tutorial has 0 `.controls` tables and the wiki
  pointer present.

No findings. APPROVE.
