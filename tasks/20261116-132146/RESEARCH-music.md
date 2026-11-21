# Research: background music source and licence

Date: 2026-11-21. Scope: web research only. This is not legal advice.

Facts about the repo that drive the result:

- The repo is public. Code and original assets are MIT (`LICENSE`).
- `credits/CREDITS.md` ships with every build and lists third-party assets.
  Licence texts live in `credits/licenses/`.
- The game has no in-game credits screen. ("Credits" in the menu code is the
  in-game currency.)
- So every track that ships is also redistributed as a raw file in git.

## 1. Licence options

### 1.1 Creative Commons classes

| Class | Sold game | Raw file in public repo | Trailer on YouTube | Verdict |
|---|---|---|---|---|
| CC0 / public domain | Yes | Yes | Yes. A third party can still register a CC0 track in Content ID by error. | Best |
| CC BY 4.0 | Yes, with credit | Yes, with credit | Yes, with credit in the video description | Good |
| CC BY 3.0 | Yes, with credit | Yes, with credit | Yes | Good, but see DRM note |
| CC BY-SA | Risky | Yes | A trailer is an adaptation, so the trailer must be BY-SA | Reject |
| NC (any) | No | n/a | n/a | Reject |
| ND (any) | Not usable if you loop, trim or sync | n/a | No | Reject |

Key licence lines (CC BY 4.0 legal code,
https://creativecommons.org/licenses/by/4.0/legalcode.en):

- Attribution items, 3(a)(1)(A): "identification of the creator(s)", "a
  copyright notice", "a notice that refers to this Public License", "a notice
  that refers to the disclaimer of warranties", "a URI or hyperlink to the
  Licensed Material to the extent reasonably practicable". 3(a)(1)(B) also
  requires you to say if you modified the work.
- Placement, 3(a)(2): "You may satisfy the conditions in Section 3(a)(1) in
  any reasonable manner based on the medium, means, and context". A credits
  file that ships with the build plus a store-page line is a normal reading.
- DRM, 2(a)(4): the licensor "waives and/or agrees not to assert any right
  ... to forbid You from making technical modifications necessary to
  circumvent Effective Technological Measures". 2(a)(5)(B): you may not
  impose terms or ETMs "if doing so restricts exercise of the Licensed Rights
  by any recipient". The repo copy is DRM-free, so a Steam build is safe.
- CC BY 3.0 is stricter on DRM. OpenGameArt says CC-BY means the work "may
  not be not used on distribution networks that use some form of Digital
  Rights Management" and lists Steam as "may or may not use DRM depending on
  how a developer chooses to package" (https://opengameart.org/content/faq).
  Rule: do not add the Steamworks DRM wrapper.

Why reject CC BY-SA (https://creativecommons.org/licenses/by-sa/4.0/legalcode.en):

- 1.1: "where the Licensed Material is a musical work, performance, or sound
  recording, Adapted Material is always produced where the Licensed Material
  is synched in timed relation with a moving image."
- 3(b): if you share Adapted Material, "The Adapter's License You apply must
  be a Creative Commons license with the same License Elements".
- Result: every trailer and devlog with the track must be BY-SA. Gameplay
  itself is a grey area. The risk is not worth it while CC0 and CC BY
  options exist.

Why reject NC and ND:

- NC: CC defines NonCommercial as "not primarily intended for or directed
  towards commercial advantage or monetary compensation". A sold game fails
  this.
- ND: no adapted material may be shared. Loop edits, trims, fades and trailer
  sync are adaptations.

### 1.2 Royalty-free libraries with their own licence

| Source | In sold game | Raw file in public repo | YouTube trailer / let's plays | Evidence |
|---|---|---|---|---|
| Pixabay Content License | Yes, inside a larger work | No (standalone distribution ban) | Some tracks are in Content ID | See below |
| Uppbeat | Only "online games or applications" per plan; unclear for a desktop Steam game | Not stated; assume no | Tracks are in Content ID; free plan needs a credit code | See below |
| Epidemic Sound | Game use steered to Enterprise plans | Not stated; assume no | Claims unless channel is safelisted and sub is active | See below |
| Artlist | Enterprise plan only | No | n/a | See below |
| Soundimage.org (Eric Matyas) | Yes, credit "in your actual game" | Not stated; ask the author | Not stated | See below |
| Kevin MacLeod (incompetech) | Yes, CC BY 4.0 | Yes (CC BY 4.0) | Pre-registered in Content ID; credit in description; ask for game whitelist | See below |
| Scott Buckley | Yes, CC BY 4.0 | Legal under CC BY 4.0; author asks for no reupload | Claims if no credit in description | See below |
| Free Music Archive | Per track licence | Per track licence | Per track | See below |
| OpenGameArt | Per asset licence | Per asset licence | Per asset | See below |

Quotes and URLs:

- Pixabay, https://pixabay.com/service/license-summary/ : "You cannot sell
  or distribute Content (either in digital or physical form) on a Standalone
  basis." Terms (https://pixabay.com/service/terms/): Standalone use is
  content in substantially the same form. A raw track in git is standalone.
  FAQ (https://pixabay.com/service/faq/): "Tracks displaying a shield icon
  are registered with Content ID and may trigger claims". Verdict: usable in
  a closed binary only. Not usable in this repo.
- Uppbeat, https://uppbeat.io/uppbeat-basic-license : lists "online games or
  applications" and "Web-based game or application that can be downloaded
  and distributed online, free or paid" (from search snippets; the page
  returned HTTP 429 to direct fetch, so UNVERIFIED in full). Licence is tied
  to a plan and a creator. Verdict: no.
- Epidemic Sound, https://www.epidemicsound.com/game-development/ : game
  studios are pointed to Enterprise plans. Claims help,
  https://help.epidemicsound.com/hc/en-us/articles/26253712691730 : a claim
  means YouTube "couldn't confirm a valid license"; videos must be safelisted.
  Verdict: no. Subscription model breaks for players and streamers.
- Artlist,
  https://artlist.freshdesk.com/support/solutions/articles/43000659339 :
  "Artlist's license was made for video creators. If you are creating a game,
  then you need an Enterprise plan." Verdict: no.
- Soundimage.org, https://soundimage.org/attribution-info/ : "you are
  required to credit me in your actual game"; credit "Music by Eric Matyas"
  and www.soundimage.org; non-attribution licence "$30(USD) per track".
  Author said on OpenGameArt the licence equals CC BY 4.0 plus a ban on
  obscene use (https://opengameart.org/comment/93907, UNVERIFIED wording).
  Repo redistribution is not stated. Verdict: possible, but ask first.
- Kevin MacLeod, https://incompetech.com/music/royalty-free/faq.html : credit
  "Title Kevin MacLeod (incompetech.com) / Licensed under Creative Commons:
  By Attribution 4.0 / https://creativecommons.org/licenses/by/4.0/". For
  games: "credits are placed on a "Credits" screen". "If you expect your game
  to be streamed, email me and I'll try to whitelist the title."
  https://incompetech.com/music/royalty-free/youtube-contentid.html : the
  music is "pre-registered" in Content ID. Verdict: usable; let's plays will
  get claims unless the title is whitelisted.
- Scott Buckley, https://www.scottbuckley.com.au/library/using-this-music/ :
  "made available under the Creative Commons 'Attribution 4.0
  International' (CC BY 4.0) License". Credit "[Track Title] by Scott
  Buckley - released under CC-BY 4.0. www.scottbuckley.com.au". YouTube:
  "Credits must be provided in the video description ... If you don't do
  this, you will likely get a copyright claim". Also: "My music cannot be
  resold in isolation, or redistributed/reuploaded to music streaming
  platforms". A git repo is not a streaming platform, and CC BY 4.0 allows
  redistribution, but ask him as a courtesy. Verdict: usable; let's plays
  without a credit line get claims.
- Free Music Archive, https://freemusicarchive.org/License_Guide : licence
  is per track ("Find the specific CC licence conditions on the song page");
  "FMA cannot alter nor change the terms of a music licence". Track pages
  show an "AI generated?" field. Verdict: CC0 and CC BY tracks are fine;
  check each page.
- OpenGameArt, https://opengameart.org/content/faq : "you can use any of the
  art submitted to this site. Even in commercial projects. Just be sure to
  adhere to the license terms". Suggested credit: "[asset name]" by [author
  name] licensed [license(s)]: [asset url]. Verdict: CC0, CC BY and OGA-BY
  are fine; reject CC BY-SA and GPL-only items.

### 1.3 Commissioned music

Price ranges (vendor and composer blogs, no authoritative survey):

- Entry-level freelancers: about USD 50-150 per finished minute; experienced
  freelancers USD 500-1,000 per minute
  (https://www.twine.net/blog/video-game-music-cost-guide/).
- Indie composers with credits: "within the range of $200-$1000 per minute"
  (https://ninichimusic.com/blog/understanding-how-much-an-indie-game-music-composer-costs).
- A non-exclusive licence often costs about half of a buyout
  (https://www.gamesoundcon.com/post/2016/03/11/becoming-a-game-music-composer-what-should-i-charge-for-indie-games).
- Example: 20 min of music at USD 300/min is about USD 6,000.

Contract points to get in writing:

- Buyout (assignment) or licence. A non-exclusive licence is cheaper; the
  composer keeps the right to sell the tracks elsewhere.
- Explicit right to put the masters and stems in a public git repo under a
  stated licence (for example CC BY 4.0 or MIT-compatible), and to let
  players and modders redistribute them with the game.
- Content ID: the composer must not register the tracks, or must whitelist
  the game, trailers and all let's plays.
- Soundtrack sales on Steam/Bandcamp: who sells, who keeps the revenue.
- Deliverables: OGG plus WAV masters, loop points, stems for layers,
  revision count, deadline.
- Credit text and placement.
- PRO registration (performance royalties): state if allowed.
- No AI-generated material, or disclosure if used (Steam rule below).

### 1.4 Self-made or generated

- Tracker or DAW, made by the owner: zero licence risk. Free tools include
  Furnace, OpenMPT, MilkyTracker and LMMS. The owner keeps the copyright, so
  it falls under the repo MIT licence like the SFX.
- Seeded procedural generation: the repo already renders all SFX from Python
  scripts (`scripts/gen-ui-sfx.py`, `scripts/gen-world-sfx.py`,
  `scripts/nova_sfx.py`). A music renderer on the same toolkit fits that
  precedent. The v0.14 trailer score came from a seeded generator outside
  the repo with no licence record. Fix: move the generator into `scripts/`,
  record the seed and command, and list it under "Original assets". If its
  origin cannot be shown, treat that score as unlicensed and do not ship it.
- A classic algorithmic generator written by the owner is not generative AI
  in Steam's sense. This is an inference, not a Valve statement.

### 1.5 AI-generated music

- Suno terms (effective 2026-09-03, https://suno.com/terms-of-service): for
  paid tiers, "Suno hereby assigns to you all of its right, title and
  interest in and to any Output owned by Suno". "You may not commercially
  exploit Output that has not been downloaded by you through an approved
  channel." "Suno makes no representation or warranty to you that any
  copyright will vest in any Output." "the Service may generate the same or
  similar output for a third party." Free tier: "lawful, personal and
  non-commercial purposes" only.
- Udio: after the October 2025 UMG settlement, downloads were disabled and
  creations moved into a "walled garden"
  (https://www.digitalmusicnews.com/2025/10/31/udio-downloads-disabled-umg-deal/).
  No current 2026 download terms found. Treat as not usable.
- US Copyright Office, Part 2 report (2025-01-29,
  https://www.copyright.gov/newsnet/2025/1060.html): outputs are protected
  only "where a human author has determined sufficient expressive elements";
  prompts alone are not enough. So pure AI tracks are likely public domain
  in the US, and anyone may copy them.
- Steam content survey (https://partner.steamgames.com/doc/gettingstarted/contentsurvey):
  "Pre-Generated: Any kind of content that ships with your game and is
  consumed by players that is created with the help of AI tools during
  development." Scope "includes content such as artwork, sound, narrative,
  localization". AI music must be disclosed on the store page.
- Verdict: reject. It adds a store-page disclosure, gives no copyright, and
  has unclear training-data risk.

## 2. Candidate tracks

Method: licence read from each source page on 2026-11-21. Length probed from
the downloadable file with ffprobe. Mood is judged from the author's
description only. Nobody listened to these tracks. The owner must audition
them.

### 2.1 Calm: menu, flight, station

| # | Title | Artist | URL | Licence | Length | Formats | Attribution |
|---|---|---|---|---|---|---|---|
| 1 | Pondering the Cosmos | Ruskerdax | https://opengameart.org/content/pondering-the-cosmos | CC0 | 5:08 | MP3 320k | Not required; author asks for credit and notice |
| 2 | Space Graveyard | TinyWorlds | https://opengameart.org/content/space-graveyard-ambient-track | CC0 | 5:13 | MP3, WAV | Not required |
| 3 | Observing The Star ("Another space background track") | yd | https://opengameart.org/content/another-space-background-track | CC0 | 2:14 | ZIP: OGG + LMMS project source | Not required |
| 4 | Deep Space Array | Tozan | https://opengameart.org/content/deep-space-array | CC0 | 1:44 | OGG | Not required |
| 5 | Exploration Theme | Cleyton Kauffman | https://opengameart.org/content/exploration-theme | CC0 | 2:14, loops | ZIP: WAV, OGG, FLAC | Optional: "Music by Cleyton Kauffman" |
| 6 | Stellar Cruise | Type 42 | https://opengameart.org/content/distant-sci-fielectronic-music-pack | CC BY 4.0 | 5:42 | WAV | "Music by Type 42 - type42.bandcamp.com" |
| 7 | Distant | Type 42 | same pack | CC BY 4.0 | 3:45 | WAV, MP3 | as above |
| 8 | Hidden Nebula | Type 42 | same pack | CC BY 4.0 | 4:21 | WAV | as above |
| 9 | Futuristic ambient 1 (of 4) | Alexandr Zhelanov | https://opengameart.org/content/futuristic-ambient | CC BY 4.0 | 4:21 (others 3:50, 3:27, 2:45) | OGG | "Futuristic ambient 1" by Alexandr Zhelanov licensed CC-BY 4.0: (url) |
| 10 | Beyond the Stars (Ambient) | HitCtrl | https://opengameart.org/content/beyond-the-stars-ambient | CC BY 3.0 | 3:44 | OGG, MP3 | "HitCtrl" |
| 11 | Sci-Fi Ambient - Crashed Ship | Ted Kerr | https://opengameart.org/content/sci-fi-ambient-crashed-ship | CC BY 3.0 | 4:37 | OGG | "Ted Kerr" |
| 12 | Machina | Scott Buckley | https://www.scottbuckley.com.au/library/machina/ | CC BY 4.0 | 4:51 | MP3 320k | 'Machina' by Scott Buckley - released under CC-BY 4.0. www.scottbuckley.com.au |
| 13 | Computations in a Snowstorm (No Cello mix) | Scott Buckley | https://www.scottbuckley.com.au/library/computations/ | CC BY 4.0 | 6:02 | MP3 320k (two mixes) | same pattern |
| 14 | Floating Cities | Kevin MacLeod | https://incompetech.com/music/royalty-free/mp3-royaltyfree/Floating%20Cities.mp3 | CC BY 4.0 (site FAQ) | 3:04 | MP3 256k | Floating Cities Kevin MacLeod (incompetech.com) / Licensed under Creative Commons: By Attribution 4.0 / https://creativecommons.org/licenses/by/4.0/ |

Descriptions (author's words, shortened):

1. "A dark sci-fi track composed with old-school synthesizers. Inspired by
   ... Mass Effect, Deus Ex, and FTL".
2. "An ambient sci-fi like track that can be used to build tension ...
   Loops seamlessly".
3. LMMS source included, so it can be re-arranged into layers.
4. "errie ... Energy Array".
5. "Progressive synth/piano ambience ... Loops seamlessly: Yes". Uploaded
   2026-02; confirm it is human-made before Steam.
6-8. Pack of about 40 min "Sci-Fi inspired, electronic/synth background
   music". Author: "credit Type 42 and let me know".
9. "A few themes in Tangerine Dream style ... loopable".
10. "deep space exploration ambient ... calm yet alone".
11. "A sci-fi ambient track."
12. "A glitchy, electronic ambient track evoking the atmospheric scores of
    classic 80's science fiction."
13. "Ambient electronics, strings and solo cello"; the No Cello mix is the
    atmospheric version.
14. "It is the future? It certainly sounds like it".

Also seen, CC0, short: "Outer Space Loop" by wipics (1:10, MP3,
https://opengameart.org/content/outer-space-loop), "Near and Far" by Joth
(0:55 80s synth loop, OGG/MP3, https://opengameart.org/content/near-and-far).
FMA: HoliznaCC0 has CC0 space albums ("An Ocean In Outer Space",
https://freemusicarchive.org/music/holiznacc0/an-ocean-in-outer-space/), but
they are tagged lo-fi folk, a weak fit.

### 2.2 Tense: combat

| # | Title | Artist | URL | Licence | Length | Formats | Attribution |
|---|---|---|---|---|---|---|---|
| T1 | Claimed by the Void | Vitalezzz | https://opengameart.org/content/claimed-by-the-void | CC0 | 3:15, loop version included | MP3, WAV, FLAC | Optional: "Vitalezzz - Claimed by the Void" |
| T2 | Space Fighter Loop | Kevin MacLeod | https://incompetech.com/music/royalty-free/mp3-royaltyfree/Space%20Fighter%20Loop.mp3 | CC BY 4.0 (site FAQ) | 1:41 | MP3 320k | Space Fighter Loop Kevin MacLeod (incompetech.com) / Licensed under Creative Commons: By Attribution 4.0 / https://creativecommons.org/licenses/by/4.0/ |
| T3 | Resonance | Scott Buckley | https://www.scottbuckley.com.au/library/resonance/ | CC BY 4.0 | 4:26 (alt mix 4:57) | MP3 320k | 'Resonance' by Scott Buckley - released under CC-BY 4.0. www.scottbuckley.com.au |
| T4 | Action tracks (4 tracks) | Alexandr Zhelanov | https://opengameart.org/content/action-tracks | CC BY 3.0 | 1:02, 1:31, 1:05, 1:17 | MP3 | "Action tracks" by Alexandr Zhelanov licensed CC-BY 3.0: (url) |

Notes: T1 is described only as "Electronic music"; uploaded 2025-03,
confirm it is human-made. T2: "Written as a background loop for a space
fighting video game." T3: "A dark electronica track ... drops into a
driving, synth-heavy backend." Backups: Kevin MacLeod "Chase Pulse" (1:57,
CC BY 4.0), Scott Buckley "Red" (3:33, CC BY 4.0).

### 2.3 Verification status

- Verified on the page: licence and author for all OpenGameArt items (licence
  field), all Scott Buckley items (licence line and credit text), HoliznaCC0
  (CC0 badge).
- Partly verified: Kevin MacLeod per-track licence. The site FAQ states CC BY
  4.0 for the library. The track pages and filmmusic.io did not render for
  the fetch tool.
- Not verified: Content ID status of Type 42, Zhelanov, HitCtrl, Ted Kerr and
  the CC0 authors. Ask before ship.
- Rejected on licence (CC BY-SA or GPL): maxstack "Through Space", "Nebula",
  "Freelance"; remaxim "Deep Space Flight"; Trevor Lentz "The Void";
  Sindwiller "Escape of the Dreadnought" and "Space-ish Suspense".

## 3. How comparable space games structure music

| Game | Contexts | Method | Source |
|---|---|---|---|
| FTL | Per sector: explore and battle pair | Horizontal switch between two versions of one piece. Both versions must be the same length, so the game jumps to the matching point. | Modding notes, https://subsetgames.com/forum/viewtopic.php?f=4&t=3044 ; track list https://benprunty.bandcamp.com/album/ftl |
| Into the Breach (same composer) | Menu, battle | Menu music fades to battle music when a fight starts, and back | https://www.pcgamer.com/au/how-i-made-into-the-breachs-soundtrack/ |
| Everspace 2 | Exploration, alert, combat | Vertical layers: "alert phases introduce tension through added layers, and combat builds into a high-energy mix". Custom system in Unreal, no middleware. | https://everspace-game.com/rockfish-games-senior-sound-designer-lukas-deuschel-interview/ |
| Space Engineers | Space, planet, building, fighting | Category playlists picked "according to the current game situation" (Update 01.133, May 2016). SE2 design: combat first, faction tracks, location ambient loops. | https://www.spaceengineerswiki.com/Update_01.133 ; https://blog.marekrosa.org/2025/03/mareks-dev-diary-march-13-2025.html |
| Starsector | Market per faction, encounter, battle | Faction playlists; a battle playlist per faction in the faction file | https://fractalsoftworks.com/2015/08 ; https://github.hscsec.cn/Laroustine/mod-starsector-battle-music |
| Elite Dangerous | Exploration, combat ("High Intensity Engagement") | A dynamic music system by Jim Croft and Erasmus Talbot. Details not found. | https://forums.frontier.co.uk/goto/post?id=283956 |
| Avorion | Ambient, combat (Combat 0-4, Boss) | Separate ambient and combat tracks. Switch logic not found. | https://steamdb.info/app/1069730 |
| Rogue System | n/a | No source found. | n/a |
| Minecraft (reference for gaps) | Ambient only | Random silence between tracks, reported as up to 10-20 min. Mod pages only, not a primary source. | https://modrinth.com/mod/mixtape |

Patterns:

- All of these split calm and combat. Most add a station or market context.
- Indie games mostly use horizontal switching (playlist per context).
  Layered (vertical) systems need stems; CC tracks do not have stems (one
  exception: the LMMS source of candidate 3).
- Calm ambient games use silence gaps; combat music plays without gaps.
- No source gives a standard crossfade length. Audio middleware guidance
  ties fades to the musical bar, and warns against chains of transitions
  when combat state flickers
  (https://www.audiokinetic.com/blog/music-system-for-avatar-frontiers-of-pandora).

Proposed Nova timings (a recommendation, not sourced; tune by ear):

- Same context, track to track: let the track end, then silence. Flight gap
  random 30-120 s. Station gap 10-30 s. Menu loops with no gap.
- Flight to combat: 1.5 s fade-in of combat over a 1.5 s fade-out of flight.
- Combat to flight: hold combat 8 s after the last hostile event, then a 5 s
  crossfade. This stops flicker.
- Dock or undock: 3 s crossfade.
- Menu to game load: 2 s fade-out, then silence until the first flight track.

## 4. Recommendation

Licence class: CC0 first, CC BY 4.0 second. No BY-SA, NC, ND, stock
subscriptions, Pixabay or AI. Prefer artists who are not in Content ID, so
let's plays and trailers stay clean. Long term, commission or self-make a
set for identity; the CC set fills the slot now.

Context rule: plain playlist per context (menu, flight, station, combat),
horizontal switch on context change, silence gaps in flight. No layers in
the first slice.

First tracks (audition before commit; mood is not verified by ear):

| Context | Track | Licence | Why |
|---|---|---|---|
| Menu | Pondering the Cosmos - Ruskerdax | CC0 | Old-school synth, dark sci-fi, FTL/Deus Ex reference; fits the terminal look |
| Flight 1 | Stellar Cruise - Type 42 | CC BY 4.0 | Long (5:42), sci-fi synth pack gives one coherent voice |
| Flight 2 | Distant - Type 42 | CC BY 4.0 | Same pack |
| Flight 3 | Space Graveyard - TinyWorlds | CC0 | Sparse, tense ambient, loops seamlessly |
| Station | Exploration Theme - Cleyton Kauffman | CC0 | Warmer synth/piano, loops seamlessly |
| Combat | Claimed by the Void - Vitalezzz | CC0 | Has a loop version and lossless masters |

Combat fallback: Space Fighter Loop (Kevin MacLeod, CC BY 4.0). If used,
email incompetech to whitelist the game title, as the FAQ asks.

Attribution plan:

1. `credits/CREDITS.md`: add a "Music" list under "Third-party assets". Each
   entry: title, artist, source URL, licence and version, and changes made
   (for example "transcoded to OGG Vorbis; trimmed to loop point"). Add CC0
   entries too, as a courtesy and as the provenance record.
2. `credits/licenses/`: add the CC BY 4.0 and CC BY 3.0 legal code and the CC0
   deed.
3. Recorded licence per track: make the licence an explicit content field of
   each music entry, so lint fails on a track with no licence (matches the
   task "Done when" line).
4. Store pages (Steam, itch.io): one "Music by ..." line with links.
5. Trailers and devlogs on YouTube: put the credit lines in the video
   description. Scott Buckley and incompetech require this to clear claims.
6. Wiki: a short music credits page.
7. Optional: a small credits panel in the main menu. Soundimage and
   incompetech ask for in-game credits; CC BY does not require it.
8. Contact Type 42 (he asks to be told) and ask about Content ID. Ask the two
   recent CC0 authors to confirm the tracks are human-made.
9. Transcode WAV/FLAC sources to OGG Vorbis before commit. The Type 42 WAVs
   are 30-60 MB each.

## 5. Open risks

- Content ID: Kevin MacLeod music is pre-registered and Scott Buckley claims
  uncredited videos. Players' let's plays and streams can get claims.
- CC0 provenance: recent uploads (2025, 2026) may contain AI or reused
  samples. A third party can also register a CC0 track in Content ID.
- CC BY 3.0 anti-DRM clause: do not wrap the Steam build in DRM.
- Mood fit is unverified. Nobody listened.
- Scott Buckley asks that his music not be reuploaded. CC BY 4.0 allows the
  repo copy, but ask him before you commit his MP3s.
- The v0.14 trailer score has no licence record. Do not reuse it until its
  generator and seed are in the repo.
- Uppbeat pages returned HTTP 429; its game terms are from search snippets.
- Commission prices come from vendor blogs, not a survey.
