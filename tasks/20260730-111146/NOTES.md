# Notes: Refresh the tutorial against the current UI and automated captures

## What changes

Before: `web/src/tutorial.html` (288 lines) walks the Shakedown Run in four
parts. All four figures are still `figure__placeholder` blocks ("Screenshot
needed") - even though the four images they name (`tutorial-menu.png`,
`tutorial-radar-lock.png`, `tutorial-orbit.png`, `tutorial-combat-lock.png`)
ALREADY EXIST in `web/src/assets/` and have declared producers in
`gen-web-screenshots.py`. The prose also still says "the objective panel just
states the goal" (line ~85), naming a surface replaced by the objective
chip/stack in `20260724-134312` / `20260729-211200`.

After: the four placeholders become real `<img>` figures against current
captures, and every instruction, key, widget name and screenshot has been
replayed against the actual game before it ships.

## Surfaces

| File | Why |
| --- | --- |
| `web/src/tutorial.html` | The page: prose, four figure blocks, captions, links. |
| `web/src/assets/tutorial-*.png` | The four figures - present, currently unreferenced by any `<img>`. |
| `scripts/gen-web-screenshots.py` | Declares the producers (`screenshot_ui`, `screenshot_combat`, `screenshot_orbit`). |
| `examples/screenshots/screenshot_{ui,combat,orbit}.rs` | Where a re-framing fix lands. |
| `examples/gameplay/` (Shakedown path) | The automation that replays the tutorial path to check the prose. |
| `crates/nova_gameplay/src/hud/` | Current objective chip/stack, HUD and radar naming to check the prose against. |

## Data and interfaces

No new interfaces. The page-level change is markup:

```html
<!-- placeholder block -> real figure -->
<figure class="figure">
  <img class="figure__img" src="assets/tutorial-menu.png"
       alt="The main menu with its live ambient backdrop and the New Game / Sandbox / Settings / Exit options." />
  <figcaption class="figure__caption">...</figcaption>
</figure>
```

The placeholder note text is already written as alt text in all four blocks -
it moves into `alt`, so nothing has to be re-authored.

## Sketches

Illustrative only.

```diff
-                    the story; the objective panel just states the goal. She
+                    the story; the objective chip just states the goal. She
```

Verification loop per part: run the Shakedown path under the gameplay
automation, watch the beat, read the paragraph, fix whichever is wrong.

## Shape

```
examples/gameplay (Shakedown walk)   examples/screenshots (tutorial-*)
        |  replay: keys, HUD, beats           |  capture
        v                                     v
   tutorial.html prose  <--- compare --->  web/src/assets/tutorial-*.png
        |                                     ^
        |  <img> + alt + caption              | gen-web-screenshots.py --report
        v                                     |
   rendered page (desktop + narrow) ----------+
```

## Consequences and open questions

- Cost: small in code, real in attention. The value is in actually walking the
  tutorial against the running game, not in the markup edit.
- The four existing captures may predate the current HUD; if they show
  pre-v0.10.0 chrome they must be re-captured, not shipped. Deciding that needs
  eyes on the images next to a live run.
- Depends on `20260724-082856` for the manifest/`--check` and the refreshed
  capture set; the prose audit does not depend on it and could start earlier.
- Open: whether the tutorial should gain a fifth figure for the objective
  chip/stack, since the prose change is about a surface no current figure
  shows.
- Open: whether the narrow-width layout needs a different crop; the figures are
  16:9 `object-fit: cover`, so a tall crop may cut the HUD element the caption
  points at.

## Accepted prose audit

Owner confirmed on 2026-08-13 that progressive figure enhancement works and
that the rendered tutorial images and desktop/narrow presentation look good.
Keep the placeholder markup: `site.ts` replaces it with an image after the
asset loads.

Update only stale prose and figure alt text:

- List the current New Game, Sandbox, Scenarios, Mods, Settings, and native Exit
  menu entries.
- Describe the functional volume, graphics, controls, and UI-skin settings.
- Include Settings in the pause menu.
- State that manual burn or RCS takes control from autopilot; camera and
  targeting input do not.
- Use current comms-stack and objective-notification terminology.
- Describe staged Ctrl clearing: combat lock, then nav lock.
- Say Shakedown teaches core flight and targeting verbs, not every game verb.

The twelve steps, keys, objective order, orbit hold, combat flow, and Broadside
continuation remain current.
