# Page 13: the short connection

The user approved the preceding direction and chose the short rigid sealed
connector. Page 13 is now illustrated. The episode remains draft, with thirteen
of eighteen pages illustrated. No new commit, tag, push, deployment, or server
startup was made in this batch.

Review `/story/season-1/episode-1/#page-13` in the normal website reader. The
[source page](../../web/src/comics/season-1/episode-1/pages/page-13.ts) owns all
words and placement. [Docking art](../../web/src/comics/season-1/episode-1/art/docking.py)
owns three named scenes and this episode's connecting prop. It imports the
existing shared hulls and renderer; it does not copy or change their models.

## Staging

- The separated view shows both hull identities, opposite headings, the intact
  secured replacement, and the connector already attached to Kaveri.
- Leila and Tomas use their original frontal likenesses and expressions. The
  small console supplies no automatic clearance verdict or invented flight UI.
- The connected inset closes the same gap, then changes only the framing and
  magnification. One elevated camera and one shared surface-ordering pass render
  the whole arrangement. No independently rotated flat ship images are pasted
  together. The tighter crop keeps the adjacent stowed arm out of the inset.
- The hollow sleeve joins the existing side collars. Both original hatches stay
  closed while the checks begin. Pressure and seal checks precede page 14's
  opening. Gantry's existing stranded condition remains unchanged; neither ship
  shows thrust. Kaveri-relative drawing placement is not powered Gantry motion.
- The only action-text change records the approved hardware choice in panel
  13c. Dialogue, page purposes, and every other action remain exact. Pages 14-18
  remain scripted, with no support equipment or injury detail selected for Owen.

## Drawing geometry

[Eight local tests](../../web/src/comics/season-1/episode-1/art/test_docking.py)
check the retained collar anchors, opposing headings, rigid placement, unchanged
hulls/damage/load, fixed sleeve during closure, hull separation, hollow bore,
surrounding plating clearance, closed hatches, and invalid-gap rejection.

Every pair of opposing hull component boxes is separated on a fixed axis, or
separated along the closing axis with a gap that increases on withdrawal. This
conservative check covers the entire drawn closure, including the secured load.
Hull faces within the sleeve's axial span are clipped to that span and checked
against the shell and bore. Existing hatch fittings fit inside the bore; adjacent
plating and fixtures remain outside the sleeve.

These are unscaled illustration checks, not CAD certification, physical
measurements, a docking standard, pressure-system validation, or proof that any
specific patient support fits. Interior access and assisted-transfer staging
remain page-14 work. The first numerical closure assertion needed a small
floating-point tolerance for arithmetic order; the source-hull comparison stays
exact, and no geometry was changed to satisfy that assertion.

## Verification

Evidence is new in [proof/docking/](proof/docking/), not written into prior
batches. The build, browser, and source-check harnesses are
[build-docking.cjs](proof/build-docking.cjs),
[inspect-docking.mjs](proof/inspect-docking.mjs), and
[check-docking.py](proof/check-docking.py).

- Targeted TS formatting, lint, comic DSL, and catalog tests pass.
- All eight local drawing tests and 37 shared illustration tests pass.
- Desktop and narrow-screen readers pass at root and project-prefix routes:
  26 clean measured page layouts per prefix, thirteen native page exports, and
  36 raw/lettered panel scenes. All first twelve page exports are pixel-identical
  to the preceding batch. Modals, focus, history, hash reload, wrapping, and safe
  SVG rejection remain checked.
- Real released-only root and prefix builds still contain an empty Story
  archive, with no draft routes, art, or dialogue. Metadata remains `draft`.
- All 36 raw scenes regenerate byte for byte. Protected prior sources, public
  lore art, and historical proof remain exact. The full script remains eighteen
  pages, fifty panels, and 730 spoken words.
- Browser inspection uses CDP pipes and intercepted static files. One run met a
  Chrome navigation cancellation during an intercepted request. The new harness
  records that specific cancellation instead of retrying a closed request; all
  other interception errors and browser exceptions still fail the checks. The
  failed run is retained in `browser-navigation-retry.txt`; subsequent runs pass.

No full website CI, Rust checks, or live publishing was repeated for this private
illustration batch. No game changelog entry is needed. Existing unrelated task
changes, including the story task's priority, remain unstaged and untouched.

## Next decision

Page 14 needs Owen's temporary transfer support. A padded stretcher with straps
would keep his body supported during the controlled handover; a support harness
would leave more movement for Rina and Ivo to manage. The stretcher is recommended,
not selected or drawn. Neither choice establishes his exact injury, treatment,
Samir's medical qualifications, or a permanent passenger layout.
