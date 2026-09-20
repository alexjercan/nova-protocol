# Release v0.14.0

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.14.0, release, meta

Meta task: ship the initial v0.14.0 GitHub release and website update. Store
publication follows the tag. The v0.13.0 flow (`20260831-145920`) is the
precedent.

Owner decision, 2026-09-19: cut the tag and generate the release binaries before
finishing the open release-media, store-presence, Steam and itch.io work. Those
tasks consume the tagged artifacts and remain open. The release proceeds with
one complete green CI run instead of the planned three-repeat probe gate.

The mdBook repair run then failed only `lesson_flight_well`: on the two-core CI
host, its braking sheet closed after the retro burn ended and tripped the
fixture's explicit visual assertion. The preceding `master` run was green and
the intervening commit changed only mdBook packaging. This host-rate fixture
failure is deferred past binary generation under the same owner decision; it is
not treated as evidence that the tagged gameplay changed.

## News loop TODOs

Create these loops for `web/src/news/0.14.0.md`. Add each release-owned alias
to `scripts/capture-web-media.sh`, capture it through the pipeline, and inspect
the packaged WebM before checking the item.

- [ ] `news-0140-release-lead.webm`: the Gantry docking rescue from approach
  through capture and joined drift.
- [ ] `news-0140-newtonian-burn.webm`: one uninterrupted main-drive burn
  through the removed 500 m/s ceiling.
- [ ] `news-0140-ai-flight.webm`: a skiff and carrier holding hull-relative
  standoff and jinking in the moving target frame.
- [ ] `news-0140-docking-approach.webm`: the docking sight resolving distance,
  alignment and closing speed before DOCK lights.
- [ ] `news-0140-useful-job.webm`: the workship clearing its lane and finding
  the damaged Gantry under the first rescue comms.
- [ ] `news-0140-handbook.webm`: open a lesson, play its demonstration, and
  launch its practice range.
- [ ] `news-0140-editor-patches.webm`: tune one twin-turret muzzle, show the
  patch mark, then reset the field to inheritance.
- [ ] `news-0140-sensor-scale.webm`: compare skiff and carrier returns, then
  break both lock slots with cover.
- [ ] `news-0140-torpedo-envelope.webm`: compare arming distance from a skiff
  and a carrier, measured from each hull's skin.
- [ ] `news-0140-live-themes.webm`: switch Phosphor, Hardware and an inherited
  mod theme while the visible interface repaints in place.
- [ ] `news-0140-carrier-collapse.webm`: show hull-wide fire, one collapse
  impulse and size-scaled wreck separation.
- [ ] `news-0140-four-v-four.webm`: record the matched busy fight with visible
  GPU chips and the crater ceiling identified without claiming timing from the
  video.

## Gate

- Every pre-tag `v0.14.0` task is CLOSED or explicitly cut with the cut
  recorded on the task. The store-page and itch.io publication tasks remain
  open because they consume the tagged release.
- Full correctness probe green three times in a row on master, content
  lint, Rust checks, and web CI all pass. A flaky range is a defect, not a
  re-run.
- The gameplay feedback ledger (`20260909-213118`) has a disposition on
  every row.
- CHANGELOG.md `[Unreleased]` reviewed whole against the changelog rules:
  baseline is v0.13.0, one entry per released change, no intra-release fix
  notes, grouped by subsystem. Bugs found and fixed inside this cycle that
  never shipped get no entry; bugs that shipped in v0.13.0 or earlier go
  under Fixes.
- Documentation (/wiki, /create, /dev) matches shipped behavior; the wiki
  stills re-shot where a fix changed what they show.
- Anything deferred out of v0.14.0 lands in the backlog, recorded on its
  task.
- Tag, build, verify the web artifact, publish the site, then hand the
  builds to the distribution task (`20260824-130004`) for itch.io.
