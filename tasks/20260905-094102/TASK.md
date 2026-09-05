# Write the story of Nova Protocol

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.13.0,content,scenario

## Workspace

- `STORY.md`: the full story document. Source of truth. World, characters,
  locations, ships, names, timeline, both comics written out, the game
  mapping, and the rules of the world.
- `design.html`: the design document with the who-is-who tables, the system
  map, the timeline, and the storyboard draft of both comics. Published as
  the artifact "Off the Roster":
  https://claude.ai/code/artifact/4af44c85-e778-494d-80f7-5933e640add3
- `REVIEW01.md`: the owner's review of the first story version. Folded into
  `STORY.md` on 2026-09-05.
- `GAMEPLAY_PLAN.md`: what the game needs to play the story, scenario by
  scenario, with the consolidated vocabulary inventory and the release cadence.
- `ACT01.md`: the script of act one. Full dialogue and descriptions, one
  scene per page, with the reader checklist, the cast and the continuity facts.
  The comic pages and the first game scenarios are written from it.
- `art/`: HD panel candidates for The Strike and their generator. The
  production art moved to `art/comics/`.
- The comic: `web/src/comics/nova-protocol/` (manifest, cast, nine pages),
  read at `/story/nova-protocol/`. Its panel art is drawn by
  `art/comics/nova_protocol.py` on the shared vocabulary in `art/comics/crt.py`.

## Log

### 2026-09-05: Story backbone and design document

The first draft was rejected as dull and its branch deleted on 2026-09-05.
The owner and the agent rebuilt the story in discussion. Only the comic
engine from that draft landed on master, and only its story-craft research
survives, in task `20260815-231945`. None of its story or notes do.

Owner decisions taken into `STORY.md`:

- Saturn system. EWI is greed with a monopoly. Groups: Earth, Earth Fleet, EWI,
  Kestrel, the Kites (Scavengers to everyone else).
- The Shelter was Kestrel's largest station. The charge was aimed at it and
  staged as an ice accident. Only the Board and Meridian's commander of that
  day knew. That commander is Calloway, now the regional desk.
- Pell ran Meridian's boat deck, disobeyed, took a tender, went dark, and is
  believed dead. Survivors who did not join him were bought by EWI.
- Halloran's "would have gone with Pell" is character, not knowledge. She
  believes he died a hero. Okoro knew him and recognises his voice.
- The captain came from Earth weeks after the incident, only Cutter One's
  captain. The crew is bonded. The player enters now.
- Kestrel folded within weeks. The installation on the rock took years. The
  Fleet ship was stolen weeks ago. The opening site is a recent victim's junk,
  not the Shelter.
- Cutter One survives because the strike was a demonstration and Severance
  must keep moving. Skiffs do the small work.
- The crew fights: PDCs on a revived tender, skiffs first, Calloway last.
- The prologue is a separate comic. It shows the staged order. Only Pell's
  survival stays a surprise.
- A bad ending at the midpoint: trust Calloway, hand over the module, fly the
  GOTO, get blasted. Comic and game follow Pell's side. No fork at the
  argument.
- Names: EWI survey words, Kestrel birds, Fleet virtues, Kites rename what they
  take with company words. Azimuth is the second carrier. The main comic and
  the crew's ship are both Nova Protocol: section nine of the EWI contract, the
  clause that closes a lost site's roster, named at beat 3.
- Finale: Parallax and Severance destroy each other, the crew plays the module,
  Vigilant arrives late and takes the credit, EWI blames Calloway alone and
  praises the crew. The crew flees before the account is written and flies Nova
  Protocol to the Roost. No choice in the comic or the game.
- Midpoint: Severance does not appear. Parallax fires to disable, the crew
  loses him in the junk, and the skiff Rotation brings the word to the Roost.
  Pell first appears at the Roost, on his own ground.

Open: none. No chapter split yet.

`STORY.md` was then expanded from the beat outline into the full document,
with sections for the world, characters, locations and ships. The character
and location sections add texture that the outline did not carry (Halloran
flew Kestrel's Harrier, Brandt taught Okoro after Pell, Solberg came out of
the Shelter on Pell's tender). The owner may strike any of it.

### 2026-09-05: Panel style candidates

The owner accepted the design page but not the style of its storyboard
thumbnails. The comic panels should follow the campaign portraits in
`art/portrait-candidates/`: the CRT frame and the phosphor palette. A first
pixel-block set was too coarse to read and was replaced by HD vector panels
at 1920x1080. `art/` in this task holds five candidates for The Strike
(beat 2) and the generator that draws them, for the owner to pick from or
give feedback on. The design page keeps its line-art boards for now.

Next: owner review of `design.html` and `STORY.md`, then split into cutscene
scenarios and gameplay scenarios, then write the prologue and main comics.

### 2026-09-05: Gameplay scope and feature cadence

[`GAMEPLAY_PLAN.md`](GAMEPLAY_PLAN.md) records the owner review of the first
campaign-system assessment. v0.13.0 publishes the story on the website and
ships First Shift and Second Shift as proofs of concept. It does not add saves,
docking, fitting, radio simulation, custom factions, or an open world.

Later campaign work proceeds one scenario and one enabling feature slice at a
time. Ship upgrades use authored hull stages and scripted installation. Story
resources remain variables with safe data-driven HUD meters. Campaign
continuity starts with a Wesnoth-style scenario-boundary journal, not a saved
ECS world. The plan also inventories the events, filters, actions, queries,
objects, narration changes, and skip-safe cinematic contract expected by the
story.

### 2026-09-05: Story review 02

[`REVIEW02.md`](REVIEW02.md) checks the revised story against Review 01. Most of
the original issues are resolved. It records nine remaining questions, with
the life-support timeline and Calloway bad ending resolved during review.

Cutter One now needs only stored breathing capacity for short shifts. Bringing
the tender's regenerative life-support plant online ends the forty-hour clock
before the two-day trip to the Roost. In the bad ending, the crew hands the
recorder to Parallax before Calloway sends them toward Datum and kills the
witnesses.

### 2026-09-05: Review 01 folded into the story

`REVIEW01.md` found the backbone sound and the joints stated, not caused. Three
rounds of annotated messages settled the answers, now in `STORY.md`:

- Pell is revenge, not theory. He reads section nine over the guard channel
  before he fires and knows it was wrong. He learns of the archive at the
  Roost, when Okoro plays it in his village.
- The skiffs chase for real. They learn who the crew are when Parallax fires
  on the tender they were chasing. Rotation then follows, holds fire, and
  gives the heading.
- The captain believes in rotation. Calloway offers it by name at the
  midpoint. The company offers it again at the end to a channel nobody
  answers.
- Halloran speaks to the sixty, not to Pell. Solberg names her signing and
  answers for the sixty: no Azimuth. Pell cannot fly Severance alone and takes
  Datum because it still gives him Calloway.
- Datum: Nova goes in first under Meridian's still-open code, kills turret
  control, docks. The captain holds the dock while Halloran plays the module
  inside. Calloway turns Parallax on his own relay tower. Okoro gives the
  boat-deck call, Pell tells his deck to obey and drives Severance in. Most
  of the sixty leave in the boats.
- Recorder rules: append-only, certification-bound, beacons when unseated.
  Every recorder at Saturn keeps the broadcast. Azimuth holds station.
- Scale: a full-length first campaign that ends at Datum. Three game-only
  Roost missions and a two-part Datum join the beats-to-scenarios table.

The design page still shows the pre-review story. It is updated when the
owner asks.

### 2026-09-05: Gameplay plan revised per scenario

`GAMEPLAY_PLAN.md` was rewritten around the reviewed story: the scope
decisions stay, the engine's vocabulary was verified against
`crates/nova_scenario`, every scenario lists what exists and what is new, and
the inventory tags each event, filter, action, query, object, HUD widget and
catalog item with the scenario that first needs it. The cadence keeps v0.13.0
as publish-and-prove and re-cuts increments A to E to the reviewed beats.

### 2026-09-05: The prologue drawn as a comic

The story ships as one comic per campaign in the site's archive, so the
prologue is the first chapter block of the Nova Protocol comic, with its own
cover and a closing frame that jumps four years, instead of a separate comic.
"Four years ago" and "now" still never share a page.

Nine pages: cover, For the ice, The charge, For the Shelter, Roster closed,
Nobody searched, Tenders away, The plaque, Four years later. The chapter is
`planned`: the comic tells it, the game does not yet play it.

The panels are drawn in the CRT style the owner accepted for The Strike. The
generator moved out of this task into `art/comics/`: `crt.py` holds the
palette, the screen ground, the ships, the Shelter and its wreck, the faces,
consoles and the boat bay; `nova_protocol.py` composes the fifteen panels and
writes them into `web/src/assets/story/nova-protocol/`. The comic engine gained
one thing for it: a `crt` panel variant that draws the panel as a screen in a
bezel with status bars, so the art stops at the screen and survives the
reader's crop.

Verified with `npm run ci` and headless screenshots of every page at 1400x900
and four pages at 420x820.

### 2026-09-05: Review 02 folded into the story

The owner accepted the agent's answers to the nine questions in `REVIEW02.md`
and they are now in `STORY.md` and `GAMEPLAY_PLAN.md`:

- The sixty knew Meridian was the target. Meridian was the ship that did it;
  Azimuth is the Shelter again with them in Calloway's seat. They refuse in
  the open, Rotation's pilot first, and Solberg gives the refusal its words.
  A few stay with Pell, and a few is not a crew.
- The module tells Pell the Shelter was ordered, which he never knew. It turns
  his revenge from the ship to the man. He wants Calloway to hear it, and he
  puts Severance in front of the tower for the people in it, not the record.
- The Board's Earth desk orders Azimuth on to the site after Datum. Its
  captain answers once, "Azimuth holds station," and not again.
- Datum's credential is Cutter One's transponder, a Meridian work boat on the
  open roster, riding in Nova Protocol's cockpit. The recorder is evidence
  only.
- Certified recorders sign every entry; the relay carries the signed record
  and every receiving recorder appends it, so every copy verifies.
- Cutter One is a short-shift boat with stored air. The forty-hour clock ends
  when the tender's regenerative life-support plant runs on the salvaged cell
  bank. The salvage chapter escalates in that order.
- The bad ending passes the module to a Parallax picket by canister, then the
  GOTO, then the shot.
- Small-ship radio reaches only a big antenna pointed at it, and Datum's
  always is. The crew can reach the desk; the desk owns the relay.

The comic stays as shipped: the owner accepted it as a proof of concept, with
small text alignment issues noted for later.

### 2026-09-05: Act one drafted from the game, then scripted

Six pages were drafted from First Shift's lines (the release, the handling
card, the donut, Demir's challenge, the beacon) with fourteen new panels. The
owner rejected the draft: too split for a reader who does not know the story,
and too close to a scenario that is itself a proof of concept and will be
rewritten. Decision: the story and the game stay apart. The game takes events
and names from the story, not lines, and the story carries no tutorial.

Act one is now written first as a script, `ACT01.md`: eight pages with the
full dialogue, the reader checklist, the cast and the continuity facts. The
draft's presentation decisions survive into it: the captain is a helmet seen
from behind, Halloran, Okoro and Demir have faces matched to the game's
portraits, the reader gets the full reading as a recorder transcript marked
recovered later, and the reader sees the hull in the moonlet's shadow one page
before the crew does.

The drafted pages, the manifest and the generator vocabulary (Cutter One, the
warship, skiffs, barges, the cabin, three faces) are committed as a proof of
concept, labelled so in the manifest, the closing frame and the changelog. The
pages are replaced when act one is redrawn.

### 2026-09-05: The vocabulary, and scenario one rebuilt on it

The owner asked for the actions and mod capabilities to be inventoried first,
then refactored, then extended - with no backwards compatibility, because the
campaign and The Ledger are both throwaway proofs. `VOCABULARY.md` is the
inventory, taken at `3af5d051`.

The refactor: a `scenario_actions!` TABLE in
`crates/nova_scenario/src/actions/registry.rs` now generates the enum arm, the
dispatch, the tag, the RON name, the menu label, the minted-id stem, the
injection class and the reflected payload accessor from one row. Adding an
action was fifteen edits, nine of them exhaustive matches over the same enum in
two crates; it is now the row, the payload struct, the editor's stock value,
the lint and the docs. `step_chain` and `step_chain_mut` are the single
accessors every nesting reader goes through, so a second nesting arm cannot be
honoured by one reader and missed by the others.

What the story asked for, and now exists:

- `Cinematic` - a beat chain the player may leave. It reports
  `OnCinematicFinished` on every path out and `OnCinematicSkipped` first on a
  skip; a `Cinematic` filter matches an ending by scene key. `CancelCinematic`
  ends one from the scenario. A skip cancels the cursor and never replays the
  beats at speed.
- `NarrativeCue` replaces `StoryMessage`, and every line authors a `channel`:
  `Comms`, `Crew` or `Guard`. Only `Guard` is tagged and drawn weak, because
  what the tag marks is a line the ship was not sent.
- `PlaySound` - one authored cue on the `Interface` or `Hull` route, for the
  sound a scene needs and nothing in the world produces.
- `SetCamera` and `SetCameraAnchor` take a `blend`, so a shot can be a move.

Scenario one was then rebuilt from `STORY.md` rather than from the old code.
Second Shift was scrapped: the campaign is one chapter, `first_shift`, named
"An Ordinary Shift". The strike is two scenes - `strike_approach` (skippable,
75 s of a ship getting closer) and `strike_salvo` (not skippable, 25 s, the
chapter's point). One skip cancels the whole strike, because the skip handler
advances the beat past the gate that would start the salvo, and both endings
share `strike_aftermath()` so they leave the identical camera.

The story material went into the shift's existing dead air: the plaque and the
banner at launch, the first guard-channel fragment in the RCS briefing,
junk ownership at the crate hand-off, section nine on the transit legs, the
rotation argument over the orbit, the second fragment on the run home, and the
clause itself during the strike. The guard voice speaks three times in authored
order - two rehearsals, then the sentence - and a test holds that.

Nested `Sequence` cursors are NOT cancelled by a cinematic skip, so mid-leg
dialogue had to move inline into the cinematic chain. The second helm leg
measures 33 s live and carries 23 s of lines, which is why the challenge moved
from 18 s to 14 s.

Verified: `cargo check --workspace --all-targets` clean; nova_authoring lib,
nova_scenario lib and the affected nova_assets integration tests green;
`content lint` 0 errors, 0 warnings; both strike examples RUN headless under
Xvfb with no panics - the approach plays its two helm legs, the guard clause
and the lock warning, and the salvo plays its torpedoes, its paired railguns
and the wreck shot. The web suite could not run in this worktree (no
`web/node_modules`); the two dependency-free tests pass and every anchor added
to the docs resolves.

The Ledger needed nothing but the rename, which it already carries; it lints
and maps clean. A copy installed under `~/.local/share/nova-protocol/mods/`
predates the rename and refuses to load, which is the migration working.
