# v0.16.0 former DRAFT news media provenance

This is the archived provenance for the local preview, not the frozen release
media. The preview copies were removed after a clean v0.16.0 candidate at
`d3c4cf8f6` produced seven frozen loops, nine stills and a thumbnail on
isolated Xvfb. The live post now points to those frozen assets. The manifest
at `web/src/assets/loops/manifest.txt` names the loop source; still hashes are
in `/tmp/nova-news-0160-stills.sha256`. The verified preview backup is in
`/tmp/nova-news-0160-final-dTrzHE/draft.tar`.

- Captured on: 2026-10-26 (file times are local, UTC+2), except start-hulls
  and world-setup on 2026-10-27.
- Source revision: `10632d375` for the four pane stills and `bf25f9d08` for
  start-hulls (`revision` file in each run directory). World-setup was
  captured on `dca35b070` with the uncommitted producer change that clicks
  `Not now` and `Don't show again` (diff in
  `/tmp/nova-ws-dismiss/producer-at-capture.diff`; only the module doc was
  rewrapped after capture). The armed lead was
  captured from the producer later committed as `dca35b070` (only rustdoc
  changed after capture). Other runs did not record a revision and remain
  development proof, not final release-candidate media.
- Former per-figure DRAFT caveats (resolved by final recapture):
  release-lead now flies an eligible armed Military picket but sends the
  accept command directly instead of clicking the Jobs pane; world-setup
  shows the 0.15.0 footer; Load shows VERSION 0.15.0;
  start-hulls predates the release candidate.
- Thumbnail: DRAFT only, at `web/src/assets/draft-news-0160/thumb-news-0.16.0.png`
  (1280x720, sha256 `5b61109f259ce39aea70a9908006804ee40d797f23783b4e1c543652c4422a81`).
  It is a byte copy of the inspected frame
  `/tmp/nova-lead-military/frames/sec_012.png`, which
  equals the 12th `fps=1` frame (about 11.5 s) of the lead loop source
  `/tmp/nova-lead-military/news-0160-release-lead-military.webm` (sha256
  `557250ec2e9ac41e25575c49c59f84f528b15157fd74f48231614c296615fe13`), MAE 0.
  At card size the station ring, picket, target bracket and hauler are clear;
  the JOB-1 Escort tag is only an orange strip and its text is not readable.
  The frozen `web/src/assets/thumb-news-0.16.0.png` is not made. Cut it from
  the final lead after final recapture.

## Loops (`web/src/assets/draft-news-0160/loops/`)

| figure | source | sha256 | run / proof |
|---|---|---|---|
| `news-0160-release-lead.webm` | `/tmp/nova-lead-military/news-0160-release-lead-military.webm` | `557250ec2e9ac41e25575c49c59f84f528b15157fd74f48231614c296615fe13` | seed 2 Military armed-fit asserted; 15.37 s VP9 1280x720; isolated-Xvfb frames inspected in `/tmp/nova-lead-military/frames/` |
| `news-0160-world-setup.webm` | `/tmp/nova-ws-dismiss/cap/news-0160-world-setup.webm` | `72271a5bb20db0e4727facfc1c96a78897a3d0ea059405516a960d08ec65dad3` | isolated Xvfb :96, lavapipe, `cycle complete, no panic`; 287 frames, 9.57 s VP9 1280x720; both corner cards dismissed by real clicks and asserted hidden; `fps=1` frames in `/tmp/nova-ws-dismiss/after/` show no card; footer still shows 0.15.0 |
| `news-0160-station-orbit.webm` | `/tmp/nova-station-orbit/r7/cap/news-0160-station-orbit.webm` | `fcc32cf11d7c8c3a8684573d87cb1cd5f1d59a73558494008e77272cda336154` | r7 higher-camera recapture, 14.11 s, `cycle complete, no panic` |
| `news-0160-refit-catalogue.webm` | `/tmp/nova-refit-0160/capture2/shots/news-0160-refit-catalogue.webm` | `c8e93b7b9e58d3350f17b2a67988ba01641e06d0152926d55ca91da12ac22f5d` | seed 23, 11.11 s, `cycle complete, no panic` |
| `news-0160-escort-ambush.webm` | `/tmp/nova-escort-0160/run5/capture/news-0160-escort-ambush.webm` | `09e92a7bf2a5084a6486811856274c307649e448d79dd47e37f05999f293f119` | seed 2, run5, 17.24 s; inspected `/tmp/nova-escort-0160/inspect/run5/ambush-grid.png` |
| `news-0160-escort-done.webm` | `/tmp/nova-escort-0160/run5/capture/news-0160-escort-done.webm` | `dda16df513cbd7062751e5f03e0d8f9defd1409d9ef168be1dd3d7575df51fee` | same run5, 9.87 s; inspected `/tmp/nova-escort-0160/inspect/run5/done-grid.png` |
| `news-0160-chart-view.webm` | `target/news-0160-shots/news-0160-chart-view.webm` | `0538052a04e7f83bd28c01bbd0e8c40aadb26b3c70bf24a0385b358e0115e094` | seed 10, 12.77 s, 36-frame dwell CRF 10; same bytes as `target/news-0160-logs/chart/dwell36-crf10.webm` |

## Stills (`web/src/assets/draft-news-0160/`)

| figure | source | sha256 | run / proof |
|---|---|---|---|
| `news-0160-start-hulls.png` | `/tmp/nova-start-hulls/r1/cap/news-0160-start-hulls.png` | `5ce86e350ef82bfe375cff9b8afc8480d8f3af739e0fa05c12d60f93ded52b64` | 2026-10-27 rerun at `bf25f9d08` (clean producer), `cycle complete, no panic`; 3x2 grid in a 360 x 160 m frame; six hulls, bows left, intake door visible on each; margins about 50/67 px left/right and 180/145 px top/bottom. Pixel diff against the old `ac426a33` copy is noise only (mean 0.03/255), so the old framing was not stale. Logged cell extents under-measure some hulls (interceptor logs 50 m, draws about 80 m), cause unverified; no overlap results |
| `news-0160-station-archetypes.png` | `/tmp/nova-0160-gen/station-r2/capture/news-0160-station-archetypes.png` | `32b3c8b1108eda2f628b6c9850f3c9aca0221caed94f4320f0aac76644c6021c` | seed 9, `cycle complete, no panic` |
| `news-0160-station-pane.png` | `/tmp/nova-news-panes/news_0160_station_pane/seed23-r3/shots/news-0160-station-pane.png` | `e52d693ce4806935e489e65bd71ab6938d5498c6201227179404bcad8b0eb836` | seed 23, exit 0 |
| `news-0160-jobs-pane.png` | `/tmp/nova-news-panes/news_0160_jobs_pane/seed10-gated/shots/news-0160-jobs-pane.png` | `515b25a39daa5d4eaa22a20deab6ed4db64e703bd48b3b0449e01ad02ca22fcb` | seed 10 gated run, exit 0 |
| `news-0160-market-inspector.png` | `/tmp/nova-news-panes/news_0160_market_inspector/seed7-r3/shots/news-0160-market-inspector.png` | `1079aec5a4400c9f564f4a6a102f17368068ef1a686b541bbc39f82df93fc9dd` | seed 7 r3, exit 0 |
| `news-0160-stances-block.png` | `/tmp/nova-news-panes/news_0160_stances_block/after-2/shots/news-0160-stances-block.png` | `7326c1abd4afd5ffe1b62c700103edecdc2ea5d8e9f29ed1a0af5ee98588875e` | seed 2, after the hint-wrap fix, exit 0 |
| `news-0160-load-screen.png` | `target/news-0160-shots/news_0160_load_screen/news-0160-load-screen.png` | `9da21ca97529ba66828b2fe76ae5638efd0ef5f840e4c713f5b713ff6116cc61` | seed 34; VERSION shows 0.15.0 |
| `news-0160-generated-scale.png` | `/tmp/nova-0160-gen/scale-r1/capture/news-0160-generated-scale.png` | `cca2f20c5c635cf2f0163fa75d5b8cb4e5adcb87f52ff6353b7dee8e51b83fb2` | seed 10 r1, `cycle complete, no panic`; not the `scale-spike` variant |
| `news-0160-settings-presets.png` | `target/news-0160-shots/news_0160_settings_presets/news-0160-settings-presets.png` | `a953db9babc173e7179b6ae1121154eebb228e73a2aab041a7ea201c4d9cb9fd` | Graphics Custom and levers |

## Rejected sources

- `/tmp/newsharness/**` and `/tmp/fakeout/**`: synthetic packager stubs.
- `target/news-0160-shots/news_0160_chart_view/news-0160-chart-view.webm`: the
  CRF 34 encode, which erased small digits.
- `target/news-0160-shots-world-setup-lane/news-0160-world-setup.webm`
  (sha256 `299ff763...`): the New pilot and field note cards stand in every
  frame (`/tmp/nova-ws-dismiss/before/`).
- Earlier runs superseded by the chosen ones: ineligible Industrial lead v1-v4, orbit r5/r6, refit
  `capture`, escort run3, jobs `seed10`, market `seed7-r2`, stances `v2-2` and
  `diag-2` (before the hint-wrap fix), scale `scale-spike`.
