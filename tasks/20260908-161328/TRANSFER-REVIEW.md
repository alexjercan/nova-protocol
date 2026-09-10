# Pages 14-15: supported transfer and separation

The user approved continuing with a padded stretcher and straps. These two new
pages bring the draft episode to fifteen of eighteen illustrated pages. Review
`/story/season-1/episode-1/#page-14` in the normal website reader.

The new body poses and transfer staging await visual review before reuse for the
return. Pages 16-18 remain scripted. No commit, tag, push, publication, or server
startup was made in this batch.

## Sources and staging

The [page-14 script](../../web/src/comics/season-1/episode-1/pages/page-14.ts) and
[page-15 script](../../web/src/comics/season-1/episode-1/pages/page-15.ts) own all
words and placement. [Transfer art](../../web/src/comics/season-1/episode-1/art/transfer.py)
owns six named scenes, the padded stretcher, body straps, temporary receiving
clamps, and local pressure-passage views. It uses the existing opposed-heading
arrangement for separation, including the secured spare and stranded Gantry.

- **14a:** After seal and pressure checks, Rina enters Gantry and stops at a
  visible handhold. Ivo answers her question. Owen is alert, supported, and
  restrained; he still contributes to the work. His exact injury is not selected.
- **14b:** The stretcher comes feet first toward Kaveri. Rina controls its near
  end while holding a fixed rail. Ivo controls the head end from the passage.
  Both grip the frame, not Owen's armpits. The body, straps, rails, and hands have
  room in the drawing; this is not a dimensioned fit or medical assessment.
- **14c:** Samir closes two temporary rigid frame clamps on a receiving rail.
  These are rigid attachments, not loose foot-end tethers that leave Owen free
  to swing after the guides release.
  Rina and Ivo retain their grips while he does so. Their foreground sleeves
  continue to the crop instead of becoming detached hands. No medical bed,
  monitor, treatment, or permanent passenger layout is introduced.
- **15a:** Nadia crosses last with a restrained case for the crew's working
  records. It has no readable dossier or conspiracy label. Her report precedes
  Jonah's acknowledgement in the visual reading order as well as the script.
- **15b:** Rina checks the clear transfer side. Leila closes and checks the
  passage. Owen remains with Samir rather than joining a group portrait.
- **15c:** Kaveri withdraws along the same connection axis. The connector remains
  attached to Kaveri. Both hulls and the replacement remain unchanged. Gantry
  does not explode; no thrust, pursuer, pirate, or new failure closes the scene.

The only action-text change selects the approved support equipment in panel
14a. All spoken words, page purposes, and other actions remain exact. The episode
still has eighteen pages, fifty panels, and 730 spoken words.

## Shared drawing work

[Portrait helpers](../../scripts/nova_illustration/portraits.py) add a supported
Owen body, extended Rina/Ivo guide bodies, and separate frame-gripping arms and
hands. They retain the original forward-facing heads, features, and palette.
Owen's body is foreshortened from the foot end; no flat head is rotated into a
profile. Equipment stays with the episode rather than becoming part of his
identity. All earlier portrait functions remain byte-for-byte unchanged.

The first visual pass exposed cropped-off guide bodies and the reversed
Nadia/Jonah balloon order. The final staging continues the guide bodies to a real
crop or passage boundary and restores the authored dialogue sequence. Rina's
initial stopping grip is now visible; the first support strap is not hidden
below the panel edge. Speech tails leave those faces clear.

## Checks and evidence

New evidence lives in [proof/transfer/](proof/transfer/). Harnesses:
[build-transfer.cjs](proof/build-transfer.cjs),
[inspect-transfer.mjs](proof/inspect-transfer.mjs), and
[check-transfer.py](proof/check-transfer.py).

- Targeted TS formatting, lint, comic DSL, and catalog checks pass.
- Forty shared illustration tests pass, including three new pose tests. Six
  [transfer tests](../../web/src/comics/season-1/episode-1/art/test_transfer.py)
  and the existing eight docking tests pass. Public lore exporter checks pass.
- Root and project-prefix desktop and narrow-screen readers pass: thirty clean
  measured page layouts per prefix, fifteen native page exports, and 42
  raw/lettered panel scenes. All first thirteen page exports are pixel-identical
  to the preceding batch.
- Browser checks measure all five active transfer grips against their frame or
  latch anchors. Rina's stopping grip is inside the first panel. Ivo's face
  remains entirely inside the passage crop. Modals, focus, wrapping, history,
  hash reload, and unsafe SVG rejection remain checked.
- Real released-only root and prefix builds still show an empty Story archive.
  Draft routes, dialogue, new poses, and artwork are absent. The episode remains
  `draft`; illustrating it does not publish it.
- All 42 raw scenes regenerate byte for byte. Protected prior episode sources,
  public lore artwork, and historical proof remain exact. The old portrait
  source remains an exact prefix of the extended module.

The browser uses CDP pipes and intercepted static files, not a server. Recorded
owned browser PIDs have exited. No full website CI, Rust checks, or live release
checks were repeated. This private art needs no game changelog entry. Unrelated
task changes and the story task's priority remain unstaged and untouched.

Next are the survivors' return, Baikal homecoming, restored processing, and the
recoverable penalty with the mug callback. The wider season remains unfinished.
