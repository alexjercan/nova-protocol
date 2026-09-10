# Pages 16-18: the return and the bill

The user requested the next pages. The opening episode now has artwork for all
eighteen pages. It remains explicitly `draft`, pending complete-episode review.
Review `/story/season-1/episode-1/#page-16` in the normal website reader.

No commit, tag, push, deployment, or server startup was made in this batch. The
wider season and its writing acceptance checks remain unfinished.

## Sources and continuity

[Page 16](../../web/src/comics/season-1/episode-1/pages/page-16.ts),
[page 17](../../web/src/comics/season-1/episode-1/pages/page-17.ts), and
[page 18](../../web/src/comics/season-1/episode-1/pages/page-18.ts) keep the original
actions, dialogue, and purposes. The complete script remains fifty panels and
730 spoken words. [Homecoming art](../../web/src/comics/season-1/episode-1/art/homecoming.py)
owns eight named scenes, not another script or an episode export wrapper.

- **16a:** Ivo questions the refusal; Samir refers to the received record; Nadia
  asks for that record instead of asserting a conspiracy. Owen remains on the
  same padded support, held by the same kind of closed rigid clamps. The
  restrained first-aid case stays nearby, with no new treatment or diagnosis.
- **16b:** Nadia's thanks and Jonah's answer remain quiet. The small pressure
  window shows the coast, not an instant arrival or an approaching threat.
- **17a:** Elena welcomes the arrivals inside Baikal after transfer. Samir
  controls Owen's frame. This is a practical welcome, not a new station docking
  arrangement, medical centre, gravity specification, or full-cast portrait.
- **17b:** The same covered replacement sits in its service cradle beside the
  page-two pump and pipe anchors. Rina and Leila prepare to remove the covers;
  the line remains labelled isolated. No alternate assembly model is drawn.
- **17c:** An explicit `LATER` label precedes the completed checks and return to
  service. Ebro still waits with its load pending. The closed-system status does
  not invent a new repair mechanism, show an instant installation, or make the
  missing production appear immediately.
- **18a:** The exact page-three window artwork returns with the new conversation.
  Mara is reported, not given a face or a Foundation cutaway. Ebro remains in view.
- **18b:** Elena accepts the recoverable cost without blaming the crew. This is
  lost margin and rescheduling, not the later destruction of Clearwell.
- **18c:** The same borrowed mug reaches the worktop within Elena's reach. The
  original ceramic colors, band, handle, and Jonah's hand remain unchanged.
  Ordinary work continues beyond the glass; no pirate or victory gesture ends
  the episode.

All faces use their original frontal identities and expressions. The return
reuses the accepted supported pose rather than implying Owen no longer needs
care. The illustration package, ship models, assembly, mug, and public lore art
remain unchanged.

## Small shared-prop extraction

The two closed clamps in
[transfer.py](../../web/src/comics/season-1/episode-1/art/transfer.py) now have one
`receiving_clamps(placement, anchors)` helper. Page 14 and the return use it. The
extraction preserves page 14's exact raw artwork and page render; it does not
change the handover, add equipment, or select a hardware standard.

The first visual pass put the isolated-line console over Rina's face. It was
moved above the replacement. The final framing also leaves Jonah's collar in
the quiet return shot and separates the pending-load label from the console.
No accepted earlier artwork or lettering diagnostics were changed.

## Verification

New evidence is in [proof/homecoming/](proof/homecoming/), with
[build-homecoming.cjs](proof/build-homecoming.cjs),
[inspect-homecoming.mjs](proof/inspect-homecoming.mjs), and
[check-homecoming.py](proof/check-homecoming.py). Older proof is not overwritten.

- Targeted TS formatting, lint, comic DSL, and catalog checks pass.
- Eight [ending tests](../../web/src/comics/season-1/episode-1/art/test_homecoming.py),
  six transfer tests, eight docking tests, and forty shared illustration tests
  pass. These are drawing and continuity checks, not medicine or engineering.
- Root and project-prefix desktop and narrow-screen readers pass: 36 clean
  measured page layouts per prefix, eighteen native page exports, and fifty
  raw/lettered panel scenes. All first fifteen page exports are pixel-identical
  to the preceding batch.
- Existing transfer contact and face-crop checks still pass after helper reuse.
  Modals, focus, wrapping, history, hash reload, and unsafe SVG rejection remain
  checked. The narrow screen retains its fitted landscape overview.
- The real released-only root and prefix builds still show an empty Story
  archive. The now fully illustrated draft has no public route, art, dialogue,
  or source-map content. Completeness does not override publication metadata.
- All fifty raw scenes regenerate byte for byte. All 42 earlier raw scenes,
  protected earlier sources, public lore art, and historical proof remain exact.
- The full TS script has eighteen illustrated pages, fifty panels, and 730
  spoken words. No action, speaker, dialogue id, or page purpose changed here.

Browser inspection uses CDP pipes and intercepted static files. Recorded owned
browser PIDs have exited. No full website CI, Rust checks, or live publishing
was repeated for these private illustrations. No game changelog entry is needed.
Unrelated task changes, including the story task's priority, remain unstaged and
untouched.

Next is a complete-episode review before any requested commit or publication.
Independent comic release and bootstrap authorization remain separate steps.
