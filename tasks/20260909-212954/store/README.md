# Steam store page: the upload sheet

Everything that goes on the page, in the order it uploads, with the copy in a
form you can paste. Nothing here needs a render to rebuild it: the files beside
this document are the files that upload.

The page itself does not exist yet. Steamworks onboarding and the Steam Direct
credit are open, and the app ID is what unlocks every upload below.

Two working documents sit outside this folder, because neither is an upload
and both carry their own images. The build sheet holds every asset, the audit
against the Zukowski checklist, and the evidence behind each decision:
<https://claude.ai/code/artifact/28e31b1f-cdf0-45fa-b141-af88fa11973e>. The
replica of the finished page, which is the one to show people, is at
<https://claude.ai/code/artifact/44f16874-2cad-49cc-b641-cdea5a8838fd>. Both
are built from `projects/nova-v014-store/` in the content-machine repository.
Every field either of them carries is written out below, so this document
stands on its own if the links do not.

## Upload order

1. `trailer.mp4`. 68.5 s, 1920x1080, H.264 / AAC. Set it to lead the carousel.
2. `screenshots/`, all eleven, in the numbered order. `01-gal-combat-wide.png`
   leads, because the first image is the one a tag-search reader gives about a
   second to. The first four are the hover tooltip and are one each of fight,
   fly, build and HUD, so do not reorder them.
3. `loops/`, five GIFs, into the app's Extras image area. The description body
   references them by the tokens below.
4. `capsules/`, one file per slot. See the table.
5. The copy fields, below.

Mark these seven screenshots as suitable for all ages: 02, 03, 06, 07, 08, 09,
11. The other four show weapons firing or a hull breaking up.

## Short description

    Design a warship section by section, fly it under real physics, and watch every hit take it apart.

104 characters. Confirm the live counter before saving.

## Description

Steam's description field is BBCode. The five `{STEAM_APP_IMAGE}` tokens
resolve once each GIF is uploaded to the app's Extras area under that exact
filename.

    Design a warship section by section, fly it under real physics, and watch every hit take it apart.

    [h2]Build modular ships[/h2]
    Choose parts from a visual gallery, stack sections together, add procedurally generated details, and test the finished hull before taking it into a fight. Four hull styles - industrial, armoured, civilian and salvage - change what the same ship looks like.
    [img]{STEAM_APP_IMAGE}/extras/build.gif[/img]

    [h2]Fly what you build[/h2]
    Every section affects the ship. Thrusters provide acceleration, flight computers control handling, and mass determines how the hull moves under Newtonian physics. The flight computer also flies for you: GOTO runs a burn-and-flip transfer to whatever you have locked, ORBIT parks the hull around a gravity well, and STOP kills your velocity.
    [img]{STEAM_APP_IMAGE}/extras/fly.gif[/img]

    [h2]Combat[/h2]
    Launch torpedoes at long range, charge up a railgun shot for a mid range heavy hit, or use point defence cannons to engage in CQB or take down incoming ordnance. Lock a ship, then pick a section inside it: your guns target the piece you chose, and the HUD draws the bracket inside the bracket.
    [img]{STEAM_APP_IMAGE}/extras/combat.gif[/img]

    [h2]Damage[/h2]
    When a ship takes damage you can actually see it, cracks start to appear, machinery sparks, and sections break apart. A ship stays alive as long as it has enough sections left.
    [img]{STEAM_APP_IMAGE}/extras/damage.gif[/img]

    [h2]Cockpit[/h2]
    The game offers a diegetic HUD that updates based on what you are doing. The UI shows you the relevant things when you need them.
    [img]{STEAM_APP_IMAGE}/extras/cockpit.gif[/img]

    [h2]Features[/h2]
    [list]
    [*]A modular ship editor: pick sections from a parts catalog, stack them, clad them, and switch the whole hull between four styles with one toggle.
    [*]Newtonian flight, where every section you added is mass the ship has to move, and thrusters and flight computers are the only things that move it.
    [*]A diegetic HUD that changes with what you are doing, and a flight computer you order around with GOTO, ORBIT and STOP.
    [*]Three weapon classes: long-range torpedoes, a charged spinal railgun, and point defence cannons that shoot down incoming ordnance.
    [*]Section-by-section damage: plating spalls, machinery sparks, sections sever, and a ship keeps flying as long as enough of it is left.
    [*]Ships and scenarios are plain-text RON files, and the whole source is public on GitHub.
    [*]Single player. Windows, macOS and Linux.
    [/list]

    [h2]Where to play it now[/h2]
    The current build is a free demo on itch.io, and the source is public on GitHub. This page is for a future Steam release; no date and no price are announced.

The same copy, without the markup, is in `../PITCH.md`.

The feature list is not decoration. Valve commonly rejects a page whose
description carries no feature list.

## Tags, in order

    Space Sim, Building, Spaceships, Physics, Destruction, Space, Sandbox,
    Sci-fi, Simulation, Combat, Moddable, Singleplayer

The first five are the hover tooltip and the "More like this" neighbourhood,
so the order matters more than the list does. Harvested off Cosmoteer and
Space Engineers, not guessed. Destruction stays although neither anchor carries
it: it is the differentiator.

Do not add Open Source. It names a licence rather than a sub-genre, and it
appears on neither anchor. The source being public belongs in the description
and the links, where it is a reason to trust the page.

## The other fields

| Field | Value |
| --- | --- |
| Release date | Coming Soon wording. No date, no price. |
| Genres | Simulation, Action. Leave Indie out: Steam seeds the tag cloud from the genres, and the community adds Indie anyway. |
| Supported languages | English, interface and subtitles. Nothing else is translated. |
| Platforms | Windows, macOS, Linux. |
| Mature content survey | No. Answer the free-text box with: this game depicts ships being destroyed in combat; no humans are depicted and no human casualties are shown. |
| Website link | The project site, which already links the itch.io demo and the public GitHub source. |
| Developer / publisher | Owner decision, still open. Must match the Steamworks account name exactly. |

### System requirements

Placeholder, and it must not ship this way. Take the numbers off the actual
v0.14.0 release builds rather than estimating them.

    Minimum
      OS: Windows 10 64-bit, macOS 12, or a 64-bit Linux distribution
      Processor: to be measured
      Memory: to be measured
      Graphics: Vulkan or Metal capable GPU
      Storage: to be measured

## Capsules

Set C, the vista set: a hull, a belt and a world. Every plate is stamped with
the `e-boot-tag` mark, which is the boot line over BUILD . FLY . FIGHT .
SALVAGE.

| File | Size | Slot |
| --- | --- | --- |
| `capsules/header-capsule.png` | 920x430 | Above the fold on the store page. |
| `capsules/small-capsule.png` | 462x174 | Steam generates 184x69 and 120x45 from this. It carries the bare mark, because nothing under the word survives the reduction. |
| `capsules/main-capsule.png` | 1232x706 | Front page, curator lists, sale rows. |
| `capsules/vertical-capsule.png` | 748x896 | Front-page features, 5:6. |
| `capsules/page-background.png` | 1438x810 | Ambient, sits under the page. No logo, no frame. |
| `capsules/library-capsule.png` | 600x900 | The grid tile in a player's library. |
| `capsules/library-header.png` | 920x430 | The header inside the library. |
| `capsules/library-hero.png` | 3840x1240 | No text at all, which Steam requires. Only the centre 860x380 is safe. |
| `capsules/library-logo.png` | 1280x212 | Logotype only, transparent, sits over the hero. |

`logo/logo-e-boot-tag.png` and `logo/logo-e-boot-bare.png` are the masters at
1280x480 with their margins intact. `capsules/library-logo.png` is the tagged
one trimmed to its own ink, and it is the file that uploads.

## What gets a page rejected

Each of these is a rule the store review enforces, or a limit that silently
truncates.

- Screenshots must be gameplay. No marketing text, no concept art, no logos in
  any carousel image. The capsules exist for that job.
- No debug status bar. It carries the build hash and the rig's frame rate.
  Nine shipped stills once went out wearing both.
- 5 MB per image. The largest file here is the 3.85 MB library hero.
- 15 MB for the description field, which counts only what the description body
  loads. That is the five GIFs, at 8.23 MB. Screenshots and capsules upload as
  store assets and spend none of it.
- List the features. See above.

## Not settled

- Steamworks onboarding, and one Steam Direct credit at 100 USD. Everything
  above waits on the app ID.
- Developer and publisher name.
- The system requirement numbers.
- Show the replica page to people who do not know the game before submitting.
  Ask what game it reminds them of and what they think you do in it. Do not ask
  whether they would buy it; that answer is always "ya kinda" and it is worth
  nothing. If the answer to the first question is neither anchor, the tags or
  the first four screenshots are wrong.
- Ask the community to seed the right tags. Worth a line in the v0.14.0 release
  notes and on the itch.io page. It only works once the page is public.
