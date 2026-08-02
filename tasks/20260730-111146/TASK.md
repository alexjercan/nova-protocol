# Refresh the tutorial against the current UI and automated captures

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.10.0, docs, web
- KIND: STORY
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955
- DEPENDS ON: 20260724-082856

## Story

Refresh the whole tutorial against current v0.10.0 flight/UI behavior and the
new reproducible capture set. The known retired objective-panel wording is the
starting defect, not the full scope of the audit.

## Steps

- [ ] Replay the tutorial path through its showcase automation and compare each
      instruction, key, named widget, and screenshot with observed behavior.
- [ ] Replace retired objective-panel language with the objective chip/stack and
      fix any other stale HUD, radar, autopilot, menu, or control references.
- [ ] Replace tutorial figures through the shared capture manifest; keep prose
      and screenshots aligned to the same checkpoints.
- [ ] Open the rendered page at desktop and narrow widths. Verify instructional
      order, captions, crops, alt text, and links.

## Definition of Done

- No tutorial instruction names the retired objective panel.
  (cmd: `! rg -n -i "objective panel" web/src/tutorial.html`)
- Every tutorial screenshot is current and has a declared showcase producer.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --check`)
- Tutorial markup resolves every refreshed image and link.
  (test: `tutorial_render_has_no_broken_assets`)
- The full tutorial matches the replayed player path at desktop and narrow
  widths. (manual: follow the rendered tutorial against the automated run)

## Notes

- Known stale line: `web/src/tutorial.html:85` before implementation.
- Objective presentation changed in `20260724-134312` and `20260729-211200`.
