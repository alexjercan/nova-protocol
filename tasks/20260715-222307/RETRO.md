# Retro: preserve wiki drawer scroll

- TASK: 20260715-222307
- OUTCOME: shipped (landed b6836026); review APPROVE, no findings.

## What went well

- Small, standard fix (sessionStorage save/restore of the nav scrollTop), placed
  right where the sidebar is already booted - no new deps, no CSS.
- The verification was the highlight: instead of eyeballing (a screenshot cannot
  show a scroll-restore across a navigation), drove the REAL chromium with
  puppeteer-core (no browser download - it reuses ~/.nix-profile/bin/chromium)
  and asserted set/stored/afterNav/afterReload all == 320. The test fails without
  the code, so it is real evidence, not a vacuous pass.
- sessionStorage was the right lifetime call (survives same-tab nav + reload,
  clears on tab close) - documented the reasoning so a future reader does not
  "fix" it to localStorage.

## What went wrong

- Nothing. The one judgement was storage lifetime, settled up front.

## Lessons

- `puppeteer-core-over-system-chromium` (positive): for client BEHAVIOR a
  screenshot cannot verify (scroll restore, focus, storage, multi-step
  navigation), drive the existing chromium with `puppeteer-core`
  (`executablePath: ~/.nix-profile/bin/chromium`, `--no-sandbox`) - it installs
  without downloading a browser and gives a real, can-fail e2e. Recipe:
  `npm i puppeteer-core` in scratch, launch with the nix chromium path, script
  goto/evaluate/reload. Screenshots stay for layout; drive the browser for
  behavior. 20260715-222307.
