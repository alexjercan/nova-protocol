# Nova Protocol: open-world progression research

Date: 2026-10-11. Online design research for the seeded open-world mode of Nova Protocol (Newtonian 3D, modular ship built from sections in a graph, sections have health and can be destroyed, armour plating can be knocked off; today: mine, pick up and throw, trade, fight, dock, basic economy). Owner's complaint: no sense of progression and nothing pulls the player back to a saved world. Owner's plan: space stations as service hubs (upgrades, rebuild destroyed sections, re-attach lost plating).

Hard constraints respected throughout: no reputation, no factions (relations are allies / enemies / neutral only), no crew simulation, no skill trees or character levels, focus on controlling a physical ship, seeded procedural world, mods can add stations, items and sections.

Method: three research lanes (about 120 searches and 150 page reads in total). Every claim carries a URL. "[snippet]" or "snippet" marks a claim taken from a search summary where the page refused the fetch (fractalsoftworks.com blog and forum, all fandom.com wikis, frontier forums, barotraumagame wiki and mature-gated Steam threads return 402/403/429). Numbers from snippets are listed for re-verification in the appendix. No repository file was touched.

Structure: A progression models, B stations and service points, C structural damage and repair, D return hooks and the endgame problem, E economy without factions, F ternary relations and nemesis, G step-by-step mechanics (G.1-G.13), H pitfalls and heuristics, then a Nova synthesis and the mechanism catalogue.

---
## A. Survey: progression models that do not rest on factions or reputation

### A.1 Ship ladder with net-worth gating

**Freelancer (2003).** Levels are derived from total net worth, and both story stages
and ship / equipment purchases are level-locked. Wikipedia: "it is not possible to gain
sufficient levels just through the story itself, so players must spend at least some
time on side missions" and "some stages would be too tough with only the money and items
collected in the story" (https://en.wikipedia.org/wiki/Freelancer_(video_game)).
Equipment classes must match hardpoint classes [snippet]
(https://freelancer.fandom.com/wiki/Vessel_Types).
- Worked: a single scalar (net worth) drives both access and difficulty; no faction
  needed to gate hulls.
- Criticised: reviewers found the filler missions "so repetitive that they become chores"
  (same Wikipedia page). Forced grind between story beats.

**Escape Velocity Nova.** Start in a shuttle, trade in hulls at 75% value, outfits
consume mass and space; some hulls need licences or storylines [snippet]
(https://tvtropes.org/pmwiki/pmwiki.php/VideoGame/EscapeVelocity,
https://themagnuskit.squarespace.com/reviews/2016/5/13/lets-look-at-ev-nova).
- Worked: mass / space budget makes outfit choice a real trade; 70+ pages of mods.
- Criticised: "there's no way to reach the end of anything without killing everything";
  obtuse mission gating (magnuskit review above).

**Rebel Galaxy Outlaw.** 12 hulls, half of them end-game; four hidden behind an easy-to-miss
quest line; power budget forces gun-vs-shield choices
(https://wolfsgamingblog.com/2019/08/16/rebel-galaxy-outlaw-review/).
- Worked: fast travel and fast fights make the credit grind tolerable [snippet]
  (https://www.geeksundergrace.com/gaming/review-rebel-galaxy-outlaw/).
- Criticised: "hefty grind at its core, stemming from its sharp difficulty spikes";
  "repair bills can be a bit hefty"; "space is almost entirely free of things to seek
  out, discover and explore" (wolfsgamingblog). Hull-limited vs non-hull-limited upgrades
  make the mid tier lopsided [snippet] (steam guide 2239224505).

**Elite Dangerous (ship ladder, rebuy, engineering, carrier).** Rebuy = 10% hull +
5% modules; if you cannot pay you lose the ship and modules, only the Sidewinder is free
(https://steamcommunity.com/app/359320/discussions/0/485623406955626685/). "Rule #1:
don't fly without a rebuy" [snippet]
(https://steamcommunity.com/app/359320/discussions/0/3827541651932083655/).
- Worked: rebuy gives loss a real but bounded sting; "never risk what you cannot afford
  to lose" is a learned skill.
- Criticised: endgame is "when you no longer need to do the busywork"; "people who have
  balances in the tens of billions and have nothing to spend that money on"
  (https://forums.frontier.co.uk/threads/endgame-what.539852/). A 350 h player: "I have no
  goal left. Nothing to achieve", wants housing / colonies; veterans answer "your goals
  are your own" (https://forums.frontier.co.uk/threads/elite-dangerous-lacks-goal.603134/).
  Small ships: "engineering helps bigger ship a lot more than small ships", small weapons
  penalised vs large hulls
  (https://steamcommunity.com/app/359320/discussions/0/1652171126128080925/).

### A.2 Tiered materials and capability-gated regions

**Avorion.** Seven materials (Iron, Titanium, Naonite, Trinium, Xanion, Ogonite, Avorion)
appear in concentric rings toward the galactic core; each unlocks block types (Titanium:
shields; Naonite: hyperspace cores; Trinium: hangars; Xanion: transporters, computer
cores; Ogonite: armour only; Avorion: rift-piercing hyperspace cores)
(https://shapes.inc/fandom/avorion/materials-and-progression). Subsystem sockets cap at
5/6/8/10/12/15 by Building Knowledge tier
(https://steamcommunity.com/sharedfiles/filedetails/?id=2862685514). Building Knowledge
(2.0) comes from a boss, a hard pirate encounter, or a shipyard purchase [snippet]
(https://steamcommunity.com/games/445220/announcements/detail/5736998873836173612);
Titanium knowledge arrives by mail as soon as you mine Titanium
(https://steamcommunity.com/app/445220/discussions/0/4841941418351191389/).
- Worked: spatial tiering (distance = tier) is legible; the rift barrier is a clear
  capability gate; "Classic Avorion" scenario kept the old sandbox for people who hated
  gates [snippet] (same announcement).
- Criticised: "The problem is that there is no more challenge after 0,0. The galaxy is not
  evolving"; "There is no motivation for continued growth. In a procedural game, that is a
  fatal flaw"; unlimited ship scaling turns combat into raw firepower
  (https://steamcommunity.com/app/445220/discussions/0/1319962514593203785/). Reviews:
  "artificially slow", "designed to pad hours" [snippet]
  (https://steamcommunity.com/app/445220/negativereviews/?l=english&p=1&browsefilter=toprated).

**Starbound.** Six tiers tie planet threat level, ore (Iron, Tungsten, Titanium,
Durasteel, Aegisalt/Violium/Ferozium, Solarium), armour, merchant stock and loot; tier 4-6
planets require Radiation / Heating / Cooling EPP suits; "no armor is stronger than tier
6" so gear progression ends there (https://starbounder.org/Tier). Early builds had 10
linear tiers with sector boss gates that players bypassed by warping straight to
high-tier planets (same page). Tier 5 split into three ores with different play styles
rather than flat stat increases [snippet] (same page).

**Terraria.** Pickaxe power gates ore; "the block simply won't break, you need the next
tier up"; Wall of Flesh flips the world into Hardmode with new ore chain Cobalt ->
Mythril -> Adamantite, then boss drills (https://terrariadb.com/guides/pickaxe-and-ore-progression).
- Worked: gate is physical and self-explanatory (tool vs block hardness).
- Lesson: the world-state flip (Hardmode) refreshes old regions instead of adding only
  new ones.

**Minecraft.** Wood -> Stone -> Iron -> Diamond -> Netherite, damage 4/5/6/7/8 linear
[snippet] (https://craftdex.net/articles/the-sword-tier-progression). Criticised:
informed players skip wood entirely; "finding iron or diamonds now feels like simple
tasks" [snippet] (https://www.minecraftforum.net/forums/minecraft-java-edition/suggestions/80135-a-slight-alteration-to-the-minecraft-tier-system).

**Valheim.** Biome -> boss -> boss drop unlocks the station / tool for the next biome
(Eikthyr -> antler pickaxe, Moder -> Artisan Table) [snippet]
(https://www.pcgamer.com/games/survival-crafting/valheim-biome-order/). Nothing stops you
entering a biome early; you get "absolutely destroyed"
(https://steamcommunity.com/app/892970/discussions/2/4142816460195186059/).
- Criticised (same thread): "New biome releases are placed at the end of progression,
  meaning only late game players see the new content"; weapon classes locked to late
  biomes; exploration becomes "finding required items rather than discovering unknowns".
  Proposed fix: tiers with a choice of 2-3 biomes per tier.

**Subnautica.** See G.3. Analysis: "Progression is all about finding the resources you
need to build an ever expanding number of options"; criticised for blueprints "hidden
like needles in haystacks" and "a lot of dead time" ferrying resources late
(https://game-wisdom.com/analysis/subnautica).

### A.3 Module slots, sockets and class upgrades

**No Man's Sky.** Ships and freighters have class C/B/A/S; class sets slot count and
bonuses. Class can be upgraded C->B->A->S at a station for nanites: "Yes you can but it
cost a ton nanites"; "there's rarely any point to buying a C or B class starship"; exotics
are always S and cheaper than upgrading
(https://steamcommunity.com/app/275850/discussions/0/3464976518706618346/). Freighter: S
has 48 general + 21 tech slots; Cargo Bulkheads from derelict freighters add slots;
Salvaged Frigate Modules / Fleet Units feed freighter tech; frigates run expeditions
(https://www.gamerguides.com/no-mans-sky/guide/walkthrough/technology-upgrades/how-to-upgrade-the-freighter).
- Criticised: "around 30 hours" is when the main quest ends and "they have no other
  objective besides explore"; "wide as an ocean, deep as a puddle"; "after Artemis
  everything does feel a bit pointless"
  (https://steamcommunity.com/app/275850/discussions/0/3874841497810499196).
  Freighter S-class adds little over B (adjacency only needs ~30-40 slots) [snippet]
  (https://gamefaqs.gamespot.com/boards/304536-no-mans-sky/80595577).

**Avorion sockets.** Socket count = f(processing power, knowledge tier); subsystems are
loot items of rarity tiers (see A.2).

**Everspace 2.** Gear rarity tiers, item level, player level, enemy scaling. See G.7.
- Criticised: "as you level, you're actually becoming weaker due to outgrowing your
  equipment while enemies are growing stronger"; crafted items "instantly start becoming
  less valuable as you gain XP"; "You can only increase rarity of same weapons you already
  found" (https://steamcommunity.com/app/1128920/discussions/0/3035976310909368101/).
  "it completely obsoletes the idea of progression"; dev: "you can't come back to early
  regions and kill capital ships with just one shot", "Enemies in low level locations will
  adapt their level to yours but will not reach it", "There will be no option to turn that
  on or off" (https://steamcommunity.com/app/1128920/discussions/5/5879990700813147728/).

**Deep Rock Galactic.** Each weapon has 3-5 upgrade slots with 1-3 mutually exclusive
options; promotions reset level; 16 perks, 5 slots; "actual progression for maybe
60-80ish hours" then 200 more hours for the loop itself
(https://steamcommunity.com/app/548430/discussions/1/1679190184058714412/). Horizontal by
design so a new player can join a veteran [snippet] (same thread).

### A.4 Blueprints, salvage and knowledge found in the world

**Starsector.** See G.1. Blueprints are found, not bought: research stations, caches,
ruins after survey, tech-mining, raids on Heavy Industry, bar contacts; packages
(Low/Mid/High tech) vs singles; known blueprints unlock Custom Production at a colony with
Heavy Industry; blueprints sold on the black market leak to pirates in 60-120 days
(https://starsector.wiki.gg/wiki/Blueprint).

**Factorio (trigger technologies).** Problem: "you could (and most typically would)
already be researching ways to process the new unique resource that you have not even
seen yet". Fix: unlock by action (mine an entity, craft an item, launch cargo) so
"discovery precedes advancement"; starting low "makes it all feel much more deserved";
"acceptable weirdness threshold" (https://www.factorio.com/blog/post/fff-376).
Earlier: 0.15 moved from linear packs to branching choices, and big steps were split into
"smaller manageable sub-parts which yield usefulness and reward on their own" [snippet]
(https://www.factorio.com/blog/post/fff-275, https://www.factorio.com/blog/post/fff-381).

**Space Engineers progression tree.** See G.5. Weld a prerequisite block by hand to
unlock dependants; "You do not have to keep or use the welded preliminary block"
(https://spaceengineers.wiki.gg/wiki/Progression).
- Criticised: "Why do block weapons depend on landing gear, what if I'm starting in a
  base?"; cannot repair / hack blocks you have not unlocked, which blocks salvage play
  (same page). "There is no help. There is no tutorial. There is no status screen";
  "hindrance system" rather than progression
  (https://steamcommunity.com/app/244850/discussions/8/3247565033774252813/).
  SE2 proposal thread: "start with the smallest/weakest version of every block" instead of
  locking types; "Progression should be driven by player intent"; Keen: "we're exploring
  ways to better support different playstyles without forcing everyone into a single
  approach" (https://support.keenswh.com/spaceengineers2/pc/topic/53661-balanced-progression-how-we-unlock-blocks).

**Empyrion.** XP-fed tech tree. "Blueprints always break down to the basecomponents and
therefore not really require any techtree unlocks"; defences: onboarding, server settings
(https://steamcommunity.com/app/383120/discussions/0/1326718197224110326/). "locking
things behind levels just comes off as silly, since either your avatar knows how to make
something or not" [snippet]
(https://steamcommunity.com/app/383120/discussions/0/490124466466471348/).

**Outer Wilds (knowledge is the only upgrade).** "knowledge to become your upgrades",
"the player feel it's them who is improving, not their avatar"; the ship log is a
narrative web so "the things you do matter in relation to other things you do"; Brittle
Hollow: learning an alternate route removes a time-pressure gate without a new verb
(https://www.gamedeveloper.com/design/outer-wilds-critical-analysis). Limits: opening is
"bafflingly ordinary", zero replay once known (same page).

**Dredge.** Research Parts (62 total in game) found by dredging or pursuits; four trees
(rods, engines, pots, trawl); research only makes an item purchasable at a shipwright,
you still pay and fit it into the hull grid; "going fast trumps everything" [snippet]
(https://www.neoseeker.com/dredge/guides/Research,
https://steamcommunity.com/sharedfiles/filedetails/?id=2956824687).

### A.5 Physical loss, repair economy, insurance

**Kenshi.** Seven body parts, each with its own HP; a limb below -100% can come off;
prosthetics give stat bonuses plus maintenance and social cost [snippet]
(https://kenshi.zone/en/limbs-prosthetics). Players: "since you rarely die anyways,
there's got to be a punishment"; veterans deliberately lose limbs: "I only ever start as
an armless beep now"; frequency is a slider
(https://steamcommunity.com/app/233860/discussions/0/2564160288795667430/).
- Lesson: loss that converts into an upgrade path (prosthetic) is accepted, loss that is
  only subtraction is resented.

**Starsector D-mods.** Recovered ships gain d-mods; restoration = hull cost x 1.2 x
1.2^dmods (1 M ship, 2 d-mods = 1.728 M); each d-mod cuts deployment supply cost 20%
multiplicatively; Derelict Operations skill rewards running junk fleets
(https://starsector.wiki.gg/wiki/D-mod). Loss is partial and re-enters the economy.

**Hardspace Shipbreaker.** Debt 1,252,594,441.92 cr; Lynx Tokens from salvage goals buy
upgrades gated by Certification Rank 1-30 [snippet]
(https://hardspaceshipbreaker.fandom.com/wiki/Upgrades); tethers, cutter heat, charges,
suit O2 / fuel / scanner (https://shapes.inc/fandom/hardspace-shipbreaker/tools-and-upgrades).
- Criticised: "why would I ever pick hard or medium ships, with same listed reward as easy
  ones?"; 15-minute shifts push rushing easy ships
  (https://steamcommunity.com/app/1161580/discussions/0/2527030866873624102/). Paying the
  debt: "It makes absolutely no difference in game"; "52 is just a bystander to the plot"
  (https://steamcommunity.com/app/1161580/discussions/0/5595176692465862885/).

**Highfleet.** Fuel and repair drain money; players advise parking the flagship, stripping
armour, running efficient strike groups at ~1100 fuel / 1000 km; framed as strategy, not
as bad design (https://steamcommunity.com/app/1434950/discussions/0/2950411088479374116/).

**Barotrauma.** "the state of the submarine/crew is near identical from start to finish";
upgrades are "massively expensive" and do not transfer between subs, so nobody buys a new
sub; missing vertical, horizontal and narrative progression
(https://github.com/FakeFishGames/Barotrauma/discussions/5229). Talents give
"imperceptible, passive" bonuses [snippet]
(https://github.com/FakeFishGames/Barotrauma/discussions/9625).

**FTL.** Scrap is the single currency for system levels (any time) and stores (random).
"I die sometimes because I have +200 scrap but no place to repair near-by"; "repairing
hull damage and getting ship upgrades is unpredictable and based almost entirely on luck"
(https://steamcommunity.com/app/212680/discussions/0/864973577897403615/). Community
rule: keep 65-80 scrap in reserve for a store [snippet]
(https://steamcommunity.com/sharedfiles/filedetails/?id=216052651).

### A.6 Debt, contract and upkeep loops

**Elite fleet carrier.** Upkeep 7.5-32 M / week by services; "feel like ball and chain
mechanic"; cannot enter permit systems; worth it only for heavy exploration or
colonisation; casuals ride community carriers
(https://steamcommunity.com/app/359320/discussions/0/846244042087864441/).
- Lesson: a sink that charges real time, not play, becomes an obligation.

**Cosmoteer career.** Fame never decreases, earned only by missions / kills with higher
fame than yours, caps crew (24 at 0 fame, 1,500 at 10,000); 10,000 = Legendary = win
(https://cosmoteer.wiki.gg/wiki/Fame). Missing materials are "automatically purchased from
the station"; dev admits Bounty Hunter "becomes repetitive pretty quickly"
(https://blog.cosmoteer.net/2022/08/cosmoteer-feature-preview-career-mode.html).
- Criticised: "getting blueprints is much to easy", "we need much more to do"; defence:
  "crew is the real bottleneck", progression is "designing effective ships adapted for the
  amount of crew you have"
  (https://steamcommunity.com/app/799600/discussions/0/3593338530534029923/). Death
  spiral: lose crew, cannot rehire until fame 215 at 210; "the progression curve of tech
  and fame seem completely out of step with each other"
  (https://steamcommunity.com/app/799600/discussions/0/3471738893387185118/?ctp=3).
  Economy: "all the salvaging/mining/production is ultimately just a meaningless early game
  time sink" vs "Being put on the clock adds stress"
  (https://steamcommunity.com/app/799600/discussions/0/3830913643997218452/).

**Sunless Sea.** Lights cut terror but burn fuel; overstoke for speed at fuel + hull cost;
crew size buffers terror (https://community.failbettergames.com/t/have-we-lost-the-micro-economy/17831).
- Criticised: "the amount of money you'll spend on fuel and supplies will quickly exceed
  the amount of money you make on the cargo"; "astonishingly stingy"; ten hours for one
  ship upgrade; permadeath "very, very unsuitable" at that pace
  (https://scientificgamer.com/thoughts-sunless-sea/). Light-flashing exploit "rewards
  tedium" [snippet] (https://community.failbettergames.com/t/interaction-between-fuel-terror-and-ships-light/15968).
- Skies removed those choices; players: "Some actual gameplay has been removed", "I was
  becoming a better Zee captain ... learning all the tricks" (micro-economy thread).

**Star Control 2.** RUs from lander mining or kills; flagship modules; lander upgrades
bought outright; "The first couple hours of the game consist of nothing but doing this,
over and over"; Melnorme info sold in fixed order
(https://www.hardcoregaming101.net/star-control-ii/). Mosolov on what he carried into
Starsector: "to plan an expedition, to try to make it come out with enough profit in the
end to keep going"; "every single star on the map was somewhere I could go"; quasi-space
"had been there all along"
(https://pistolshrimpgames.com/2022/11/reflections-on-uqm-from-alex-mosolov-creator-of-starsector/).

### A.7 Ownership, base and claim building

**X4.** Mid game: "At this point you no longer need credits. They're basically useless";
the goal becomes closed-loop production feeding your own shipyard, then fleets
(https://steamcommunity.com/app/392160/discussions/0/6580353067864293442/).

**Starsector colonies.** Need a fully surveyed planet, 1,000 crew, 200 supplies, 100
heavy machinery; size 3 start; growth from accessibility, stability, hazard; Heavy
Industry turns known blueprints into ships; threats scale with size
(https://starsector.wiki.gg/wiki/Colony). (Faction expeditions are a threat source Nova
cannot copy; pirate raids and hostile activity can be.)

**Star Valor.** Weight / power budget: a big hull without engines "cannot turn or defend";
"Big ships are as viable as small ships, merely in a different way"; "The reason is because
you don't yet have the gear to do it justice"
(https://steamcommunity.com/app/833360/discussions/0/3761103203680949263/). Tier 4 to 5
hull jumps 15x in price while income does not
(https://steamcommunity.com/app/833360/discussions/0/694248493352586719/).

**Void Destroyer 2.** Trade-in loses 25%; fleet UI "long-winded"; late game advanced
fleets lose to Outsider corvette swarms [snippet]
(https://steamcommunity.com/app/369530/discussions/0/3183345176707736058,
https://steamcommunity.com/app/369530/discussions/0/1700541698702011389/).

### A.8 Cosmetic-only / prestige

**Sea of Thieves.** All progression is cosmetic; for: "skill alone rules"; no smurfing;
against: "a fancy way of saying no progression", "most people like seeing their numbers
go up" (https://www.resetera.com/threads/sea-of-thieves-%E2%80%9Clateral-progression%E2%80%9D-subverts-20-years-of-online-game-design-brilliant-or-foolish.31405/).
Paid cosmetics became the de-facto progression [snippet]
(https://www.seaofthieves.com/community/forums/topic/160368/cosmetics-are-your-game-progression-but-they-became-pay-to-win).

### A.9 Mechanism extraction (what each mechanism needs to work without factions)

| Mechanism | Best example | Precondition that made it work | Failure mode seen |
|---|---|---|---|
| Tiered materials by region | Avorion, Starbound | spatial legibility, each tier unlocks a verb (shields, jump, hangar) | ends at the centre; nothing evolves after |
| Capability gate (depth / range / env) | Subnautica, Starbound EPP | gate is physical, upgrade is crafted from what the gate hides | needle-in-haystack fragments |
| Module slots / sockets | Avorion, NMS | slots tied to hull size and tier | class upgrade cost makes lower classes pointless |
| Service-point upgrade | EV Nova, Dredge, Hardspace | mass / grid / rank budget | hull-limited vs unlimited upgrades lopsided |
| Physical loss + repair | Starsector D-mods, Kenshi | loss re-enters economy, has an upside path | repair too cheap or too dear |
| Knowledge / discovery | Outer Wilds, Factorio triggers | discovery precedes unlock | no replay, player stuck |
| Blueprints in world | Starsector | rarity + leak to enemies | "too easy" (Cosmoteer) |
| Debt / contract | Hardspace, Elite carrier | debt visible, payoff meaningful | payoff "makes no difference" |
| Base / claim | X4, Starsector colony | base consumes the economy | money useless once loop closes |
| Collection / catalogue | NMS scanning, SC2 Melnorme | catalogue feeds something | fixed-order info sale |
| Net-worth ship ladder | Freelancer, Elite | one scalar gates hull + mission tier | small ships obsolete |
| Insurance / rebuy | Elite | bounded loss, learnable rule | permanent loss drives players away |
| Prestige / cosmetic | SoT, DRG promotions | strong core loop | "no progression" |

---

## B. Stations and service points as progression hubs

### B1. Service catalogue by game

**Elite Dangerous (station).** Refuel, Repair, Restock, Outfitting (install, remove, replace modules), Shipyard (buy, sell, store, swap, transfer ships), Universal Cartographics (sell exploration data), Technology Broker, Refinery Contact, Commodities Market, Mission Board, Passenger Lounge, Contacts. Most stations stock only a subset of modules and ships, which forces travel. Commodities, missions, cartographics, refuel and repair are available on the pad; Outfitting, Shipyard and Livery need the sealed hangar (https://elite-dangerous.fandom.com/wiki/Station, snippet). Mission board types: Courier, Supply, Assassination, Massacre, Theft/Liberation, Recovery, Salvage (black boxes), Surface Operations (https://elite-dangerous.fandom.com/wiki/Mission_Board/Types, snippet).

**Elite Dangerous (fleet carrier, the owned station).** Default services are Bridge Crew, Commodities Market, Tritium Depot. Paid modules: Refuel, Repair ("hull and subsystem repairs ... very useful in far reach-of-out places"), Armoury, Redemption Office, Shipyard ("most expensive module to install and keep running"), Outfitting, Secure Warehouse (https://roguey.co.uk/elite-dangerous/fleet-carriers/services/). Price: 5 billion credits; Refuel 40 M (1.5 M/week), Repair 50 M (1.5 M/week), Armoury 95 M (1.5 M/week), Outfitting 250 M (5 M/week), Shipyard 250 M (6.5 M/week) (https://elite-dangerous.fandom.com/wiki/Drake-Class_Carrier, snippet). Player thread: 5 to 30 M/week upkeep, tritium ~50 k/t at ~0.25 t per ly plus 100 k per jump; traders must load cargo twice; "just an expensive vanity piece" if you already run missions from a home station (https://steamcommunity.com/app/359320/discussions/0/4353365620733038584/).

**Starsector (market).** Trade, refit (weapons, hullmods, vents, fighters within Ordnance Points), Restore (removes all d-mods at once, only docked), storage, ship purchase, dockside bar (missions, loans, intel). Refitting in deep space costs combat readiness; docked refits are free (https://starsector.wiki.gg/wiki/Refit_Screen). Bars: generic mission pool plus bar-only encounters (spacer sells pirate base location, veteran marines) (https://starsector.wiki.gg/wiki/Bar). Colonies: founded with 1000 crew, 200 supplies, 100 heavy machinery; provide income, remote storage and market, and custom production from blueprints via the Command screen; upkeep per structure; threats are expeditions, pirate bases, Pather cells (https://starsector.wiki.gg/wiki/Colony).

**X4 (player HQ).** Obtained in the intro plot by sacrificing 3 antimatter cells, 1 ship and 1 resource probe; grants research (teleportation, module hacks, ship mods), personal office with trophies and vault, embassy, teleport, and it gates most later plots (https://wiki.egosoft.com/X4%20Foundations%20Wiki/Manual%20and%20Guides/Objects%20in%20the%20Game%20Universe/Plots/Player%20Headquarters%20Plot/, https://x4-foundations-wiki.fandom.com/wiki/Player_Headquarters, snippet).

**Avorion.** Equipment Dock: buy/sell system upgrades, turrets; the only place a permanently installed upgrade can be removed intact. Repair Dock: repair cheaper than build menu, hire crew, set as Reconstruction Zone (respawn point, needs top standing, ~5 M credits). Shipyard: build ships by style, size, material. Player stations: found with a Station Founder ship, 8 to 920 M founding fee plus ~5 M physical plant, 4 to 15 % hourly ROI (https://avorion.fandom.com/wiki/Repair_Dock, https://avorion.fandom.com/wiki/Equipment_Dock, https://avorion.fandom.com/wiki/Shipyard, snippets; https://steamcommunity.com/sharedfiles/filedetails/?id=2981872758).

**Cosmoteer (career).** Hail a station to accept missions, buy/sell resources, hire crew, buy blueprints (1 k to 45 k credits, prerequisites chain, every station sells the same set), auto-buy missing materials for a build (https://cosmoteer.wiki.gg/wiki/Game_Modes, https://cosmoteer.wiki.gg/wiki/Blueprints). NPC ships restock stations and can be hailed for trade and crew. 0.28.0 scaled rare resources with station size and guarantees a bounty near the start (https://cosmoteer.wiki.gg/wiki/0.28.0).

**Freelancer.** Base = launchpad, bar (NPC rumours, job board), Equipment Dealer (weapons, munitions, Ship Repair sub-button only when damaged), Commodity Trader (green/yellow/red price lights), Ship Dealer (https://freelancer.fandom.com/wiki/Base_Features, snippet). Reviewers called the job-board missions "so repetitive that they become chores" (https://en.wikipedia.org/wiki/Freelancer_(video_game)).

**Rebel Galaxy.** Buy/sell goods, upgrade weapons/defence/components, buy ships (pay or receive the difference), repair and restock missiles, missions, hire mercenary wingmen, bartender sells bounty intel and route intel; not every station has every service, so players travel for specific items (https://rebelgalaxy.fandom.com/wiki/Space_Stations, snippet; https://gamerant.com/rebel-galaxy-beginner-tips/).

**Everspace 2 (home base).** Storage (grows with companion perks), crafting from found blueprints, fleet and inventory management only there. Players call it dormant after the intro despite story weight and ask for a lab/workshop (https://steamcommunity.com/app/1128920/discussions/5/5440953210431353182/; https://everspace.fandom.com/wiki/Rhodia_II, snippet).

**No Man's Sky (freighter).** Mobile warehouse (21x21 buildable floors, Matter Beam remote inventory), fleet command for up to 30 frigates on timed expeditions, longer warp range, summonable, trade room, scanner room, extractor room. First one is given free after ~5 warps via a scripted rescue (https://nomanssky.wiki/starships/freighter-guide/).

**Barotrauma (outpost).** Shipyard with Maintenance bay (buy subs, sell back for partial upgrade value) and Reactor sector (upgrades). Chief Mechanic sells upgrades, repairs hull and items, recovers lost shuttles. Prices scale with outpost reputation (not usable for Nova; note the mechanism: 1 RP = 0.5 %). Upgrade levels persist when switching subs. Marks from missions buy items and upgrades (https://barotraumagame.com/wiki/Shipyard, snippet; https://barotraumagame.com/gameplay-features/campaign-progression-submarines-some-other-things/).

**Highfleet (city).** Shipworks (repair, remodel), unlimited fuel (~1.30/unit, ~0.80 at fuel depots), ammunition depots with city-specific stock, rare-parts cities, hidden allies, faster-repair cities with landing-spot bonuses, salvage money from crash sites, Fleet HQ cities as checkpoints (https://steamcommunity.com/sharedfiles/filedetails/?id=2580804883).

**Dredge (port).** Shipwright sells hull and equipment upgrades; any dock can spend Research Parts to unlock upgrades; fishmongers buy catch (https://gamerant.com/dredge-best-boat-upgrades/).

**Hardspace Shipbreaker (hab).** Equipment screen opens at rank 4, buys tool and suit upgrades with LYNX tokens gated by certification rank; the hab also picks the next job (https://hardspaceshipbreaker.fandom.com/wiki/Upgrades, snippet; https://en.wikipedia.org/wiki/Hardspace:_Shipbreaker).

**Star Valor.** Dock to trade, take a mission, install/remove equipment, repair, hire crew, store items, craft; stored equipment is shared across all stations, bulk goods only at the depositing station; stations differ in stock (https://star-valor.fandom.com/wiki/Stations, snippet).

**FTL (store).** "Hit a store about once per sector to repair and see what's available"; scrap is both gold and XP so store visits are the tension point (https://sullla.com/FTL/ftladvancedstrategy.html).

### B2. Which services create the "come back here" loop

Observed across the games, the strongest return drivers are:

1. **Repair that only a dock can do.** Starsector Restore (d-mod removal) is docked-only (https://starsector.wiki.gg/wiki/Refit_Screen). Avorion repair dock is cheaper than build mode and the only cheap way to replace destroyed blocks; mechanics on board only heal damaged, not lost, blocks (https://steamcommunity.com/app/445220/discussions/0/135510669606528475/). Highfleet repairs are planned in a city and executed after leaving, with city speed bonuses (https://www.magicgameworld.com/highfleet-shipworks/).
2. **Selling what you bring back.** Starsector exploration brings commodities, weapons, hulls, blueprints and AI cores to market (https://starsector.wiki.gg/wiki/Exploration). Elite Universal Cartographics turns scan data into credits (station snippet above). Cosmoteer salvage turns into construction materials used at or near a station (https://blog.cosmoteer.net/2022/08/cosmoteer-feature-preview-career-mode.html).
3. **Stock that differs by station.** Elite, Rebel Galaxy, Star Valor and Highfleet all make specific stations the only source of specific parts, ammo or rare parts (sources above). This is a faction-free lever.
4. **A job board that refreshes.** Starsector bar missions rotate and include non-faction types (procurement, derelict recovery, spysat, cheap commodity) (https://starsector.wiki.gg/wiki/Generic_missions). Cosmoteer stations post bounties, deliveries, exploration (https://cosmoteer.wiki.gg/wiki/Game_Modes).
5. **Storage.** Starsector market storage, Star Valor shared equipment storage, NMS freighter, Everspace 2 base storage. Storage is the quiet anchor: the player returns because the spare hull or the rare part sits there.
6. **Blueprint / capability purchase** (Cosmoteer blueprints, X4 research, Dredge research at a dock). These are one-time unlocks; they drive early returns and stop mattering later.

Weak drivers: pure refuel (Highfleet fuel is "unlimited" everywhere; only price differs), pure restock. Everspace 2 shows a base with story weight but few services goes "dormant" (thread above).

### B3. How stations feel different from docking with a ship

- **Scope of work.** Only docks do structural work (Starsector Restore, Avorion destroyed blocks, Highfleet destroyed parts need stock or shop, Barotrauma hull repair at outpost). Ship-to-ship contact is trade or crew only (Cosmoteer NPC ships: trade and crew, not blueprints, https://cosmoteer.wiki.gg/wiki/Game_Modes).
- **Persistence.** A station holds storage and a queue (Highfleet repair schedule runs after you leave; Battletech mech bays queue sequentially, https://steamcommunity.com/app/637090/discussions/0/1696048786954524472/).
- **Physical entry.** Elite gates outfitting behind the sealed hangar; Freelancer shows the launchpad with walkable dealers; Highfleet makes landing optional but rewards it with speed (sources above).
- **Fleet carrier lesson.** An owned station that duplicates a public station becomes vanity unless it is placed where no station is (remote repair, remote refuel) (https://steamcommunity.com/app/359320/discussions/0/4353365620733038584/).

### B4. Player home base / owned station

- **Starsector colony**: income, storage, custom production, but it pulls threats to it (expeditions, pirate bases) and has monthly upkeep (https://starsector.wiki.gg/wiki/Colony). Endgame thread: without conquest "war is out of the question", players invent their own goals or run Nexerelin (https://fractalsoftworks.com/forum/index.php?topic=27472.0, snippet).
- **X4 HQ**: research hub plus plot gate (egosoft wiki above).
- **Avorion player stations**: economic, 8 to 920 M to found, 4 to 15 %/h ROI; Reconstruction Zone at a repair dock is the respawn anchor (sources above).
- **NMS freighter**: free mid-game gift, becomes mobile storage and fleet HQ (https://nomanssky.wiki/starships/freighter-guide/).
- **Elite fleet carrier**: 5 B credits, weekly upkeep, worth it only remote (sources above).
- **Valheim base**: hub for sleep, cook, craft; the loop is "go out, come back" because ore cannot pass portals, forcing carts and ships; late-game boss search called tedious filler (https://bottomfeeder.substack.com/p/design-deep-dive-valheim). Players site bases by boss stones, coast, ore and aesthetics; many rebuild per tier (https://steamcommunity.com/app/892970/discussions/0/563659924718076284/).
- **Kenshi base**: "provide resources for free" but "attract attention"; advice is 10 trained characters first; many play the whole game living out of town houses and pack animals (https://steamcommunity.com/app/233860/discussions/0/3041607712756286352/, snippet).

Takeaway for Nova: a home station earns its keep by (a) being where lost sections and plating get rebuilt, (b) holding storage of recovered sections, (c) sitting where no public station is. Income and raids are optional and pull in faction-like systems; skip them.

### B5. Fuel and supplies as pacing loops, and whether players found them tedious

- **Starsector supplies/fuel/CR.** Supplies are consumed constantly for maintenance, faster for repair and CR recovery; fuel is burned per hyperspace distance only; zero fuel means drifting to a gravity well with distress call (https://starsector.wiki.gg/wiki/Campaign_guide). CR was designed so that "recovering CR costs time and supplies, thus rewarding the player for winning with fewer ships" (https://fractalsoftworks.com/2013/10/17/combat-readiness-update/, snippet). Mitigation: mothball recovered hulls, Efficiency Overhaul (https://starsector.fandom.com/wiki/Efficiency_Overhaul, snippet). A "whats the deal with supplies" thread exists on the official forum; new-player confusion is the recurring complaint (https://fractalsoftworks.com/forum/index.php?topic=24431.0, snippet).
- **Highfleet fuel.** Range circle ignores take-off fuel; the tutorial puts the first target outside fuel range, called "miserable" (https://steamcommunity.com/app/1434950/discussions/0/3033725780693715402/, snippet). A late-game event makes landed ships burn fuel forever with no penalty for refusing; players call it a trap: "before I'd be finished, I would need to refuel another half of it" (https://steamcommunity.com/app/1434950/discussions/0/3185738755289984991/).
- **Sunless Sea fuel/supplies/terror.** "you need money to buy fuel and supplies so you can travel to make money, so not having any money means I can't do anything"; others: "once I got the hang of the process ... too much fuel and supplies" (https://forums.anandtech.com/threads/sunless-sea.2420119/post-37154908). Steam threads call it a "masochism simulator" early and, in Sunless Skies, "tedium" and "extra padding" between story beats (https://steamcommunity.com/app/304650/discussions/0/3832043873385198800, https://steamcommunity.com/app/596970/discussions/0/1777136225009521801, snippets). Bycer cites Sunless Sea as the example of aimless design where players "cannot distinguish between meaningful actions and wasted effort" (https://game-wisdom.com/critical/problems-aimless-game-design).
- **FTL fuel.** Each jump burns 1 fuel and advances the rebel fleet; players keep 14 to 15 fuel leaving a store; "there is no situation where it's actually good to run out of fuel"; stress is real but accepted because runs are short (https://steamcommunity.com/app/212680/discussions/0/840610807207302397/; https://ftl.fandom.com/wiki/No_fuel:_Rebel_fleet_delay, snippet).
- **Barotrauma fuel rods.** Rods are cheap and stocked at most outposts; the friction is the AI engineer refusing a second rod and outposts not generating spent rods (https://steamcommunity.com/app/602960/discussions/0/3802776070753023313/, https://github.com/FakeFishGames/Barotrauma/issues/18022, snippets).

Pattern: fuel works as pacing when it is a route-planning choice (FTL, Highfleet depots at 0.80 vs 1.30) and reads as tedium when it is a flat tax with no decision (Highfleet landed burn, Sunless Sea early poverty loop, Starsector constant maintenance for newcomers).

---

## C. Physical damage, repair and refit as progression

### C1. Mechanism per game

**Highfleet.** Right-click a part or auto-repair with a slider; destroyed parts need stock or shop purchase; the "repair cost" shown is the full price without stock and drives time, so "you only pay for the parts, not for the job" (https://steamcommunity.com/app/1434950/discussions/0/3051737007088540886/). Removing parts is free and instant; adding or repairing costs time and money; work runs only after leaving the city; landed ships get a dock speed bonus; the queue can be reordered by dragging ships (https://www.magicgameworld.com/highfleet-shipworks/). A "1000 hours of repairing" complaint got the answer: land ships, prefer wrench cities, and the real fix is not taking artillery duels (https://steamcommunity.com/app/1434950/discussions/0/3409804177174531191/).

**Starsector.** Recovered ships gain at least one d-mod, destroyed ones gain Structural Damage; d-mods cut stats but reduce deployment supply cost 20 % each; Restore cost = baseHullCost x 1.2 x 1.2^dMods, so a heavily damaged ship costs more than a new one, "only worthwhile for ships that cannot be purchased easily"; Hull Restoration skill removes one d-mod per month by chance (https://starsector.wiki.gg/wiki/Damage_hullmod). A forum thread proposes changing restoration because of this (https://fractalsoftworks.com/forum/index.php?topic=26363.0, snippet). The design blog on recovery exists but refused fetch (https://fractalsoftworks.com/2017/01/20/ship-recovery/).

**Cosmoteer.** Destroyed player parts drop 85 % of construction resources (up from 50 %), independent of economic difficulty; salvage marks whole ships and queues targets (https://cosmoteer.wiki.gg/wiki/0.28.0). Enemy wreckage is "valuable materials that you'll need to repair and upgrade your own ship" (https://blog.cosmoteer.net/2022/08/cosmoteer-feature-preview-career-mode.html). Player advice: capture and dismantle beats salvage beats blowing up; strip weapons and engines but do not hit reactors because they explode neighbours (https://steamcommunity.com/app/799600/discussions/0/3875969933567232297, snippet).

**Avorion.** Damaged blocks heal for free via mechanics over time; destroyed blocks must be rebuilt in build mode (full price) or at a repair dock (cheaper); "you just pay for the destroyed blocks like you place them anew" (https://steamcommunity.com/app/445220/discussions/0/135510669606528475/). New-player shock: one broken block on a high-tier hull showed an astronomical repair bill; defenders argue mass production makes building cheaper than repair and expensive repair keeps PvP skill-based (https://steamcommunity.com/app/445220/discussions/0/2143092024472431574/). Whole-ship loss: Reconstruction Tokens at repair docks, 20 to 30 % of ship value, first token free below Hard; reconstructed ships cap at 80 % HP until repaired; replaced an insurance system that "caused beginners to rage quit" (https://steamdb.info/patchnotes/3821935/, snippet). Criticism: tokens can be exploited by founding, loading, destroying and reconstructing (https://steamcommunity.com/app/445220/discussions/6/1649918058732026688/). Integrity Field Generators multiply block HP x10 locally; joints between big sections have deeper HP pools (thread above).

**Space Engineers.** Blocks are layers of components; unwelded frames have almost no HP; grinding returns components, damaged blocks return scrap instead (https://spaceengineers.wiki.gg/wiki/How_to_build; https://spaceengineers.wiki.gg/wiki/Scrap_Metal, snippet). Ship welders will not repair a block that still holds scrap, so combat repair is "often faster to print out an entire new ship" (https://support.keenswh.com/spaceengineers/pc/topic/ship-welders-dont-repair-weapons-damage-on-blocks, snippet). The SE2 repair service only welds existing frames, so players must place frames via projector first; "makes bounty missions economically unattractive compared to simply reprinting ships" (https://steamcommunity.com/app/244850/discussions/0/833871657871478060/). SE2 added Automatic Block Recovery so a rebuilt block reconnects to toolbars (https://blog.marekrosa.org/2026/03/space-engineers-2-vs2-2/, snippet).

**From the Depths.** Repair bots rebuild missing blocks at exactly the block's material cost from local or global storage; slowest of three repair options, useful out of combat (https://fromthedepths.wiki.gg/wiki/Repair, snippet).

**Barotrauma.** Hull repaired at outposts by the Chief Mechanic; Hull Reinforcements: 6 levels, +20 % to +120 % wall health, 2 000 to 8 666 mk plus 5 steel / titanium-aluminium / physicorium bars per pair of levels; Engine Power 4 levels +8 % to +32 %, 2 500 to 6 250 mk; 13 categories, 20 upgrades, each level costs more than the last (https://barotraumagame.com/wiki/Shipyard, snippet). Experienced players: "you can pass everything without upgrades"; upgrades mainly "compensate for design flaws" (https://steamcommunity.com/app/602960/discussions/0/2969524117986888413/).

**Battletech (HBS).** Salvage parts: 1 part for centre-torso kill, 2 for both legs, 3 for headshot or pilot kill; 3 parts assemble a mech; display "2/3" = priority picks vs maximum (https://steamcommunity.com/app/637090/discussions/0/1752358461522535673/). Sell price is 10 % of base and parts cannot be sold individually; advice: take cash early, salvage once solvent (https://steamcommunity.com/app/637090/discussions/0/1696044439570659247/). Repair needs parts in the warehouse and the destroyed location must be salvaged then scrapped before replacement; "It's not obvious that there are still parts to salvage before you can replace" (https://battletech.com/forums/index.php?topic=77274.0). Complaint: a refit can take 30 days early, 10 structure points take 20+ days, single bay queues sequentially; defenders: Argo upgrades cut it, long repairs reward avoiding damage, and "7-day full assault refit" is reachable (https://steamcommunity.com/app/637090/discussions/0/1696048786954524472/).

**MechWarrior 5.** Negotiation points (max 15) split between pay, salvage shares and damage insurance; shares ~175 k C-bills each; damaged mechs cost fewer shares (https://steamcommunity.com/app/784080/discussions/0/591810592556302321/). A headshot Centurion cost 36 shares, the same mech shot to pieces cost 16, so "the salvage system penalises you for better mission performance" (https://steamcommunity.com/app/784080/discussions/0/4411920671413611943/). Repairs in conflict zones cost up to 300 % more; cold storage stops upkeep; full active hangar upkeep ~3 M (https://gamerant.com/mechwarrior-5-mercenaries-pro-tips-making-money/; https://steamcommunity.com/app/784080/discussions/0/3768985793294516731/?ctp=2, snippet).

**Hardspace Shipbreaker.** Tools have durability; Durability Drain upgrades slow it; suit has 4 bars, more from hab upgrades; tokens, not credits, buy gear (https://hardspaceshipbreaker.fandom.com/wiki/Upgrades, snippet; https://steamcommunity.com/app/1161580/discussions/0/3395163747096925301/).

**Kerbal.** No repair in stock; revert to launch or VAB is the loop (common knowledge; no page fetched, treat as unverified here).

### C2. How losing a section hurts without frustrating

Patterns that worked:

- **Loss becomes a discount, not only a penalty.** Starsector d-mods cut supply cost 20 % each so a scarred hull is cheaper to run; players "accept d-mods on expendable ships" and restore only rare hulls (https://starsector.wiki.gg/wiki/Damage_hullmod).
- **Most of the material comes back.** Cosmoteer 85 % drop on destroyed parts (https://cosmoteer.wiki.gg/wiki/0.28.0). Avorion mechanics heal damage free; only lost blocks cost (thread above).
- **Time, not money, is the main cost, and the player can trade one for the other.** Highfleet parts in stock cost zero money but still time; land for speed (https://www.magicgameworld.com/highfleet-shipworks/). Battletech bays reduce time with investment (thread above).
- **Local HP pools for joints.** Avorion gives connection points deeper HP and integrity fields so a hit does not split the ship at random (thread above). Relevant to Nova's section graph: protect the graph edges more than leaves.
- **Repair location matters.** MW5 industrial vs conflict zone pricing and Highfleet wrench cities make "where to repair" a decision (sources above).

Patterns that were criticised:

- **Opaque cost.** Highfleet repair cost shows a hypothetical full price (thread above). Avorion one-block repair bill shocked a new player (thread above). MW5 salvage cost does not track component value (thread above).
- **Restore more expensive than replace.** Starsector restore formula makes the loop "buy a new one" for common hulls (wiki above).
- **Repair that needs the player to redo the build.** Space Engineers frame placement before repair, welders blocked by scrap (threads above).
- **Serial queues early.** Battletech single mech bay (thread above).
- **Better play punished.** MW5 headshot salvage cost (thread above).
- **Hidden prerequisites.** Battletech salvage-then-scrap before replace (forum above).

### C3. Salvage of enemy parts as reward

- Starsector: post-battle recovery of own and enemy hulls with d-mods; derelict ships, research stations with blueprints, motherships drop a Catalytic Core, Synchrotron Core or Pristine Nanoforge (https://starsector.wiki.gg/wiki/Exploration; https://starsector.wiki.gg/wiki/Damage_hullmod).
- Cosmoteer: wreck to materials; capture intact beats scrap (sources above).
- Avorion: salvage lasers cut generators and shields out of wrecks; boss ships have blocks worth tens of thousands of scrap (https://steamcommunity.com/app/445220/discussions/0/135507780430781573/, snippet).
- Battletech: 3-part jigsaw per chassis; MW5: shares priced by remaining functionality (sources above).
- Hardspace: the whole game is salvage; reactor extraction before meltdown is the set piece (https://en.wikipedia.org/wiki/Hardspace:_Shipbreaker).

Nova fit: an enemy section with its plating still on is worth more intact than shot off; price salvage by remaining section health, but show the price before the fight ends (MW5 lesson) and do not make a clean kill cost more.

---

## D. Hooks that pull players back to a saved world

### D1. Persistent personal projects

- **Ship under construction.** Battletech 3-part mech assembly across many missions (thread above). Everspace 2 Mainframe Components combined into mainframe expansions (https://everspace.fandom.com/wiki/Rhodia_II, snippet). Dredge hull tiers each need wood, metal, cloth and research parts plus money (https://gamerant.com/dredge-best-boat-upgrades/).
- **Base.** Valheim base rebuilt per material tier (thread above). NMS freighter build-out (guide above).
- **Collection / catalogue.** Dredge "Recording Rarities": two research parts per fish discovery, eight total (https://indiegameculture.com/guides/dredge-side-quests-guide/). Subnautica PDA creates a databank entry per scanned unknown species with synthesized theories; abandoned PDAs download logs (https://wiki.subnautica.com/sn/PDA, snippet).
- **Discovery map.** Outer Wilds ship log: entries hold facts; rumor facts (heard) vs explore facts (seen); a "more to explore" marker shows when facts remain; rumor mode draws connections and layout differs per player (https://nh.outerwildsmods.com/guides/ship-log/; https://steamcommunity.com/sharedfiles/filedetails/?id=2143085586, snippet). Starsector derelicts "stay discovered with predictable orbits" once found (https://fractalsoftworks.com/2016/07/17/exploration-salvage/, snippet).

### D2. Recurring job boards without factions

Mission types that need no faction or reputation:

- Starsector generic pool: Cheap Commodity, Procurement (deliver N of X), Hand-me-down Freighter (buy a used hull at a third of price), Spysat Deployment (reach a point undetected), Extraction, Derelict bounty, Station bounty (https://starsector.wiki.gg/wiki/Generic_missions; https://starsector.wiki.gg/wiki/Bounty).
- Starsector personal bounties: Intel shows the target's ship and approximate location; hidden level 0 to 10 rises with each completion; targets are named with a competence tag ("unremarkable" to "exceptionally capable") (https://starsector.wiki.gg/wiki/Bounty).
- Cosmoteer: bounty, delivery, exploration (and faction conflict, skip) (https://cosmoteer.wiki.gg/wiki/Game_Modes).
- Elite: courier, supply, assassination, salvage (black boxes with a scanner) (https://elite-dangerous.fandom.com/wiki/Mission_Board/Types, snippet).
- Barotrauma: Salvage (item marked on sonar, counts only inside the hull at a safe location), Monster, Cargo (easiest, least paid), Escort, Beacon, Abandoned Outpost (https://barotrauma.fandom.com/wiki/Missions, snippet).
- Dredge pursuits: corkboard journal; most untimed; four Hooded Figure pursuits fail permanently if not done in two days ("they have died"); rewards are research parts and books that give passive bonuses (https://indiegameculture.com/guides/dredge-side-quests-guide/).
- Rebel Galaxy: bartender bounty intel and route intel (gamerant above).

Criticism to design around: Freelancer's "fly there, kill them" boards became "chores" (Wikipedia above); Hardspace needs 140 work orders at rank 8 which "incentivizes grinding easy, low-grade ships" (https://steamcommunity.com/app/1161580/discussions/0/2527030866866302664/).

### D3. World events

- Starsector: hyperstorms with warning periods, derelict fleets, research stations, pirate bases that disrupt trade (https://starsector.fandom.com/wiki/Hyperspace_Topography, snippet; https://starsector.wiki.gg/wiki/Colony).
- NMS expeditions: fresh save, same start planet, phases of milestones, rendezvous points, 4 to 6 weeks, save converts to Normal after (https://www.nomanssky.com/expeditions-update/; https://steamcommunity.com/app/275850/discussions/0/601913886224142738/, snippet). Needs a live service; a single-player analogue is a seeded time-boxed scenario with unique cosmetic rewards.
- Elite community goals: weekly tiered telethon targets, rewards by contribution percentile (https://elite-dangerous.fandom.com/wiki/Community_Goals, snippet). Multiplayer only.
- FTL: the rebel fleet as a moving front that converts beacons to elite fights (https://ftl.fandom.com/wiki/No_fuel:_Rebel_fleet_delay, snippet).

### D4. Unfinished business

- Nemesis system: the game "would 'remember' your actions, and point them out to you"; player death is "an opportunity to avenge oneself"; enemies gain scars and promotions; "never shut [players] off. Build upon [what they did]" (https://www.gamedeveloper.com/design/designing-i-shadow-of-mordor-i-s-nemesis-system; GDC: https://www.gdcvault.com/play/1025150/Helping-Players-Hate-(or-Love)).
- Starsector personal bounties already name the target and the ship; a faction-free version is a named hull with its own section loss history (wiki above).
- Highfleet Fleet HQ checkpoints keep a lost campaign alive (guide above).

### D5. Long-term goals and the sandbox endgame problem

- **Minecraft**: the End poem "dumps players back into their world because Minecraft has no real ending"; dragon fight is player-paced (https://parryeverything.com/2022/01/28/boss-breakdown-the-ender-dragon/, snippet; https://minecraft.wiki/w/The_End, snippet).
- **Valheim**: each boss gates a crafting tier; ore cannot teleport, forcing the go-out-come-back rhythm; late boss hunting is "hours of searching" (https://bottomfeeder.substack.com/p/design-deep-dive-valheim).
- **Terraria**: Wall of Flesh flips the world into hardmode, "start over at a higher weight class"; fixed gates with flexible order inside (https://xgamingserver.com/blog/terraria-progression-guide-boss-order/, snippet).
- **Subnautica**: goal is leave or stay; the rocket plus disabling the quarantine platform are the meta goals; a dev answered "Yes OP, there is a story & plot" (https://steamcommunity.com/app/264710/discussions/0/1693785669848895896/; https://subnautica.fandom.com/wiki/Neptune_Escape_Rocket, snippet).
- **NMS centre of galaxy**: warps you to an identical new galaxy with a broken ship; "I don't think I've ever been more disappointed in the ending of a game" (https://mein-mmo.de/en/no-mans-sky-the-center-of-the-galaxy-the-biggest-disappointment-in-the-game,107435/; https://steamcommunity.com/app/275850/discussions/4/361798516945845411, snippet).
- **Starsector**: no end; players restart "with a different gimmick" or mod in 4X conquest (forum snippet above).
- **Bycer**: aimless design fails when "the goals or the rules of the world don't present themselves"; fix is "lay things out so that the basic building blocks are there" (https://game-wisdom.com/critical/problems-aimless-game-design).
- **Hardspace**: a numeric goal (the debt) plus rank unlocks; IGN found "intentionally slow progression made shipbreaking start to feel like ... hard labor" (https://en.wikipedia.org/wiki/Hardspace:_Shipbreaker).

Pattern: a sandbox needs at least one visible, finite, world-changing goal (dragon, Wall of Flesh, rocket) and the goal must change the world or let the player keep playing afterward. A long march to an unchanged place (NMS centre) is the failure case. For Nova, a candidate is a hull-class ladder with a world-changing final build, not a location.

### D6. The endgame / long-tail problem: additional sources

- Minecraft's Ender Dragon and End Poem were added about a month before 1.0 as a last-minute "silly over-the-top out-of-nowhere text for when you win" (https://en.wikipedia.org/wiki/End_Poem). The sandbox did fine without a goal, but the goal gave speedrunners and first-timers a frame.
- Valheim: bosses gate progression physically: Eikthyr drops antlers which "is the only way players can craft their first pickaxe", the Elder's drop opens iron (search snippet of https://www.pcgamer.com/valheim-developer-says-zelda-and-skyrim-were-bigger-influences-than-survival-games/; page body not fetched, UNVERIFIED wording). The CEO cites Zelda and Skyrim rather than survival games as the model: a campaign skeleton inside a sandbox.
- Terraria: boss kills permanently alter the world (Hardmode spawns new biomes/ores) and pickaxe power gates each ore tier in sequence (https://terraria.wiki.gg/wiki/Guide:Game_progression). Progression is "before/after states" of the world, not a level number.
- Subnautica: the community debated whether an ending would "completely DESTROY the point of spending hours on building big impressive seabases"; proposed compromises were half-endings (contact Earth but stay) and multiple endings (https://forums.unknownworlds.com/discussion/137737/thinking-on-the-endgame). Shipped answer: a rocket whose parts are revealed step-by-step so the build itself is the tour of the map.
- Starsector: no ending; developer Alex: "The game will have proper a endgame at some point, so effort making this stage 'fun' would be pretty much wasted until it's clear exactly what the real endgame looks like"; players: "It's a sandbox. The endgame is whatever you decide it to be" (https://fractalsoftworks.com/forum/index.php?topic=14127.0) versus "the game is no longer fun, its just an endless series of wack-a-mole" (https://fractalsoftworks.com/forum/index.php?topic=30309.0).
- Game Wisdom on aimless design: "When your game is aimless it means that there is no sense of progression or learning curve that the player can use to figure out how to play"; fix: "Give them the immediate basic items or crafting materials right at the beginning" and put advanced content farther away (https://game-wisdom.com/critical/problems-aimless-game-design).
- Breslin, "The History and Theory of Sandbox Gameplay": "Their basic problem was that the air was empty, so there was literally everywhere to go and nothing to do" (on early flight sims); on Freelancer: "After the story finishes, the game becomes boring very quickly. There are only a few randomly-generated missions, and by the end of the game, the player has probably already played all of them"; prescription: sandbox games need MORE top-level design, a reward system with progression, custom scenarios not only random ones, and "a typical game must respond to correct input, while a sandbox game must reward all input" (https://www.gamedeveloper.com/design/the-history-and-theory-of-sandbox-gameplay).

---

## E. Economy without factions

### E.1 How the surveyed games price goods

**Starsector (per-market supply/demand with explicit deficit/excess numbers)**
- Each market has supply (production) and demand (consumption) per commodity; a global pass matches producers to consumers and keeps a "for sale" quantity separate from stockpiles (Economy Revamp blog, https://fractalsoftworks.com/2016/04/20/economy-revamp/; design follow-ups: https://fractalsoftworks.com/2018/01/03/revisiting-the-economy/ and "Economy & Outposts" https://fractalsoftworks.com/2017/09/19/economy-outposts/).
- Player impact is explicit and bounded: "the per-unit cost rises when buying items in larger batches and falls when selling, particularly when sales exceed the demanded quantity"; the F1 tooltip shows a **Deficit** and **Excess** number which is "the optimum amount of goods to buy and sell before the commodity returns to a normal price" (https://starsector.wiki.gg/wiki/Trade).
- Recovery is time-based and slow: "goods sold by the player on a market will fill its demands for 120 days and the items available on a market are refreshed every 30 days" (same page). A deficit "may be resolved while the player is attempting to exploit it" because NPC trade fleets also fill it.
- Degeneracy guard #1: a flat **30% tariff** on open-market buys and sells makes generic buy-low/sell-high "generally unprofitable" (effective 130% buy / 70% sell). Profit only exists in shortages, excesses, **trade disruptions** (random events that spike a price for a period), the tariff-free black market, and delivery/procurement missions (https://starsector.wiki.gg/wiki/Trade).
- What worked: players report that the tooltip numbers turn trading into a readable puzzle; event-driven spikes give a reason to be in the right place. What was criticised: Mosolov has himself noted the colony/endgame economy becomes "wack-a-mole" (https://fractalsoftworks.com/forum/index.php?topic=30309.0) and "the moment i colonize the fun ends" (https://fractalsoftworks.com/forum/index.php?topic=25742.0).

**Elite Dangerous (station-level stock with a linear sell penalty)**
- Each station has supply/demand per commodity; price rises as stock falls and the top price occurs with high demand and zero stock (https://elitedangerous-archive.fandom.com/wiki/Trading_Guide, https://elite-dangerous.fandom.com/wiki/Market_Economy).
- Sell penalty rule (post June 2020): if the cargo you carry exceeds **25% of the station's current demand**, the paid price drops linearly until 80% of demand, bottoming at 30-55% of full price; the trigger moved from 4-5% to 25% to reduce tedium (https://elitepve.com/viewtopic.php?t=10275; player restatement: "if a station has 1000 demand for Platinum, and you show up with less than 250 tons, you'll get the full displayed price", https://steamcommunity.com/app/359320/discussions/0/3272436038142318100/).
- Recovery: demand "will revert back to the original number when you leave and return (unless changed by a server tick)" (same elitepve thread) and "over time other traders will come in and replenish the goods" (Steam thread).
- Criticism: "newer stations won't develop from demand fulfillment, limiting long-term progression incentives beyond accumulating credits" (Steam thread above). I.e. a trade economy that does not change the world gets boring once the credit goal is met.

**Rebel Galaxy (price moves after you leave; events as spikes)**
- "The prices fluctuate based on events, but also in relation with in stock / out of stock, and the amount you sell / buy (this effect is not instant, but takes effect after you leave the station and dock again)"; famine raises food, mining rush drops ore; each system has a specialty (https://steamcommunity.com/app/290300/discussions/0/3377008022038789440/).
- Criticism in the same thread: "this games trade economy is so shallow it's just not that hard to make a lot of money and not really worth it"; trading matters early, then loses relevance.

**Endless Sky (route flattening)**
- "Commodity prices will vary from day to day, and will drop if you sell a particularly large load (tens of thousands of tons) on one planet" (Players Manual, https://github.com/endless-sky/endless-sky/wiki/PlayersManual).
- Player view: "When you run one trade route for too long it flattens out. No such thing as the best route anymore"; some disliked losing a permanent best route, others felt it rewarded rotating routes and mixing in missions (https://steamcommunity.com/app/404410/discussions/0/143388408894599823).

**Freelancer (static prices)**
- Prices are fixed per base; players publish routes with fixed per-hour profit (1.1-2.5 M/hour), e.g. 1,980/unit on Liberty to Tau (https://freelancer.fandom.com/wiki/Trade_Routes, https://lancersreactor.com/viewtopic.php_f=16_t=35806). The ladder is still bounded because the best routes need large freighters and safe standing, and because ship/equipment prices are high relative to run income. Lesson: a static economy works when the SINKS are steep and the route itself carries risk, not because the market is clever.

**Everspace 2 (static by design)**
- Prices are "static based on location rather than dynamic", described by devs as a credit faucet, "not intended as a simulator feature" (https://steamcommunity.com/app/1128920/discussions/0/3823034248725561092/).

**X4 (full simulation, broken by missing sinks)**
- "The economy's imbalance comes from the fact that it's all fountains and only the most irregular of sinks, so naturally, the world will be flooded eventually" (https://steamcommunity.com/app/392160/discussions/0/571540929904725056/); in long saves shipyards clog, "station storage full ... then the station will fail to put up buy orders for the things that the station actually need" (https://steamcommunity.com/app/392160/discussions/0/1744479698798947277/). The only sink is war; when war stalls, the galaxy stalls.

**Space Engineers Economy 2 (contracts and services, no price depth)**
- Trade stations now offer Services Terminal (store grid, repair grid for credits, sell grid as scrap) and contract types: hauling (container size/value/risk set reward; damage or tampering reduces payout), courier, search (find missing grids), PvE bounty, salvage, item recovery, grid recovery (https://support.keenswh.com/spaceengineers/pc/announcement/update-1-209-economy-2).
- Player reaction: "all I gained were more contracts" without "meaningful price variations between stations to incentivize strategic trading"; "without substantial distance between stations or compelling reason to visit specific locations, the economy becomes superficial" (https://steamcommunity.com/app/244850/discussions/0/833872326144657878/). Note that SE *does* have faction reputation and Economy 2 added a "Reputation Decay Over Time" option; the complaints are about price depth, not reputation.

### E.2 Anti-degeneracy mechanisms observed

| Guard | Where | Effect |
|---|---|---|
| Price impact proportional to (sold / local demand) | Starsector, Elite (25%->80% linear), Endless Sky, Rebel Galaxy | Big hauls self-limit; small ships keep full margin |
| Slow recovery (days of world time) | Starsector 120 days / 30-day restock; Elite server tick | A route is a resource that depletes and regrows |
| Flat transaction tax | Starsector 30% tariff | Kills generic arbitrage; only events/shortages pay |
| Event spikes | Starsector trade disruptions; Rebel Galaxy famine/rush | Gives a reason to move NOW, time-limited |
| Limited stock per visit | Starsector 30-day refresh; Elite station stock | Caps per-trip profit |
| Hard sinks | Elite rebuy/outfitting; Freelancer ship prices; SE repair fees | Money leaves the world |
| Cargo as mass with physics cost | Space Engineers: "drill ships may fly just fine while empty but when full of ore they fall out of the sky" (https://steamcommunity.com/app/244850/discussions/0/1291817837639304052/, https://spaceengineers.wiki.gg/wiki/Thruster_mechanics) | Bigger hauls need more thrust, more fuel, worse handling: a natural, physical cap |
| Risk routes | Endless Sky escort missions spawn pirates scaled to cargo value (https://steamcommunity.com/app/404410/discussions/0/3374780959386760311/); Elite pirates interdict cargo carriers | Profit correlates with exposure |

### E.3 Trading as ship progression
- Elite's trade ladder is purely cargo volume: Hauler ~22-26t -> Type-6 ~114t (~1M cr) -> Type-7 ~432t -> Type-9 ~752t unshielded (https://www.thegamer.com/elite-dangerous-best-trading-ships/, https://elite-dangerous.fandom.com/wiki/Type-9_Heavy). Each step is "buy a bigger box"; a Frontier forum thread titled "Let's talk ship size, cargo capacity and how incredibly silly this is in Elite: Dangerous" exists but its body could not be fetched (https://forums.frontier.co.uk/threads/lets-talk-ship-size-cargo-capacity-and-how-incredibly-silly-this-is-in-elite-dangerous.570637/, title only).
- In a section-graph ship the same ladder is more interesting: cargo is a section with mass, so hauling more means refitting thrusters, and the trade-off is on the hull not the wallet.

### E.4 Fit for Nova
- Keep prices per station, one scalar per commodity: `price = base * f(stock / target_stock)`, with stock drifting toward target over world-time. The Elite "25% of demand" rule and Starsector's "fill demand for N days" are both one line of code and both documented as understandable by players.
- Add a flat transaction fee at stations (Starsector 30%) so money comes from events, shortages, and contracts, not from a spreadsheet.
- Make events the thing that breaks the rule: a sector-level "shortage of X at station Y for N days" is the simplest procedural hook and is what Rebel Galaxy and Starsector players actually chase.
- Cargo capacity progression comes free from physics: more cargo sections, more mass, more thrusters, more fuel.

---

## F. Allies / enemies / neutral only

### F.1 Games that run on a ternary (or smaller) relation

- **Endless Sky**: radar colours are the whole model: "green: owned by you ... blue: friendly ... yellow: unfriendly ... red: ships that are attacking you". Attacking a ship makes you hostile to its government; planets repair and refuel on landing; disabled ships can be boarded and captured (https://github.com/endless-sky/endless-sky/wiki/PlayersManual). The per-government standing exists, but moment-to-moment it collapses to four colours.
- **FTL**: every encounter is scripted as hostile, neutral, or an event choice; scrap is the single currency; the Rebel Fleet is the only persistent "enemy" and it is a pacing wall, not a relationship (https://en.wikipedia.org/wiki/FTL:_Faster_Than_Light).
- **Subnautica**: no NPCs at all; creatures are the only antagonists and a leviathan's territory is a region gate: "It is difficult to judge where the edge of a Reaper's territory is, so the player gets a general sense that if they can see the creature, then they're probably too close" (https://forums.unknownworlds.com/discussion/154770/reaper-leviathan-phobia). Design lead Anthony Gallegos: it "bothered him that in the first game, players would immediately focus on trying to figure out how to fight leviathans ... so that they would no longer pose a threat" (https://gamerant.com/subnautica-2-no-weapons-why/). Lesson: an enemy class that you cannot grind away keeps a region dangerous permanently.
- **No Man's Sky sentinels**: a consistent enemy class with a per-planet aggression level (Passive -> Ferocious) and a 1-5 wanted level that escalates in waves while you keep fighting and decays after "20 + (wanted level x 10) seconds" of evasion (https://www.carlsguides.com/nomanssky/sentinels.php, https://nomanssky.fandom.com/wiki/Sentinel). No reputation, only an escalation timer.
- **Starsector pirates**: "not technically a unified faction but more a loose category of bandits ... and free captains"; always hostile, run bases that spawn raids, are the default loot/salvage source, and get named bounties posted on their flagship captains (https://starsector.wiki.gg/wiki/Pirates). Even in a faction game, the pirate class is the thing that gives the open world a baseline antagonist that nobody has to be diplomatic with.
- **Space Engineers**: pirate ("SPRT") grids are permanently hostile; Economy 2 adds PvE bounty contracts to destroy enemy grids and "Search" contracts to locate missing grids (https://support.keenswh.com/spaceengineers/pc/announcement/update-1-209-economy-2).

### F.2 Dynamics that a ternary model still supports

1. **Neutral turns hostile when attacked, with a decay.** NMS wanted level and Elite notoriety both decay by time ("1 real-time hour ... to go down 1 notoriety level", https://steamcommunity.com/app/359320/discussions/0/595146546178167663/; players found 2-hour-per-level too punitive for newcomers). For Nova: a per-ship "provoked" flag with a sector-local timer, no global number.
2. **Pirates as a consistent enemy class** (Starsector, SE, Freelancer, Elite). Gives: a target for bounties, a source of salvage, a threat to convoys, a reason to carry turrets. No standing needed.
3. **Hired wingmen as a purchasable ally.** Rebel Galaxy: hire one mercenary at a station bar; "they stay with you until they die"; six fixed mercs whose price scales with system difficulty; the player gives targeting priority orders; the merc ejects instead of dying and can be re-hired (https://rebelgalaxy.fandom.com/wiki/Mercenary_(Hireable), https://steamcommunity.com/sharedfiles/filedetails/?id=565326813). Endless Sky: buy extra ships and they follow as escorts; "having many small escorts is more cost-effective than one large ship, but small escorts are more easily destroyed" (https://steamcommunity.com/app/404410/discussions/0/3183345176707089524/).
4. **Escort / convoy contracts.** Endless Sky escort missions spawn pirates scaled to the escorted ship's cargo value and route (https://steamcommunity.com/app/404410/discussions/0/3374780959386760311/). SE hauling contracts set reward by container size, value and risk (link above).
5. **Distress calls as procedural points of interest.** Elite's Distress Call USS "has randomly generated content": a damaged trader under pirate attack, a trader needing hull repair limpets, a debris field with search-and-rescue ships, or "several heavily armed ships ... who have faked a distress signal in order to attack any pilot who responds" (https://elite-dangerous.fandom.com/wiki/Distress_Call, guide: https://forums.frontier.co.uk/threads/unidentified-signal-sources-uss-a-complete-guide.326047/). The fake-call variant is the important one: it makes "neutral" calls a gamble without any reputation.
6. **Named target bounties.** Starsector person bounties: the intel entry shows "which ship the target are piloting", rough location (constellation / system type / planet), fleet size; reward = 30,000 + 10,000 x level + random extra; difficulty is a hidden 0-10 level that rises with completed bounties; above level 3 targets may be "deserters" with better gear (https://starsector.wiki.gg/wiki/Bounty). Reputation is a side-reward there, but the loop (poster -> target ship -> location -> kill flagship) needs none of it.

### F.3 Nemesis without reputation

- **Shadow of Mordor/War**: "each orc captain will remember the player whether from a previous battle or killing their ally"; a captain "has a chance of coming back to life if killed by the player (unless decapitated) and will visibly show wounds"; "if a regular orc kills the player they automatically become a captain"; each captain has random role, clan, traits, weaknesses (https://www.gamedeveloper.com/design/core-system-analysis-of-middle-earth-shadow-of-war). Design intent: "How can we make narrative out of the gameplay?" and "never shut [players] off. Build upon [what they did]" (https://www.gamedeveloper.com/design/designing-i-shadow-of-mordor-i-s-nemesis-system). GDC 2018 talk: "Helping Players Hate (or Love) Their Nemesis" (https://gdconf.com/news/attend-gdc-2018-behind-scenes-look-shadow-wars-nemesis-system).
- Critique: Shadow of War's version was called "at once both brilliant and frustrating" because returns could feel cheap (https://www.techradar.com/news/shadow-of-wars-nemesis-system-is-at-once-brilliant-and-yet-incredibly-frustrating).
- **AC Odyssey mercenaries** are a nemesis-lite with no standing: a bounty meter rises with crimes, tiers 1-5 send more and stronger procedurally generated hunters with names and traits; killing one above you climbs a 9-tier ladder with passive perks; you clear the bounty by paying it off or killing the sponsor (https://www.pushsquare.com/news/2018/10/guide_assassins_creed_odyssey_mercenaries_-_how_to_beat_them_bounties_tiers_and_rewards, https://vulkk.com/2018/10/05/assassins-creed-odyssey-mercenaries-and-bounties-system/).
- Minimal space version for Nova: a pirate ship that survives an encounter (fled with <X% sections) is promoted to a *named* persistent entity: keep its section graph (damage included), its seed, a name, and the sector it escaped toward. It returns repaired-with-scars (missing plating, mismatched sections), can hold the player's lost cargo, and is the target of a posted bounty. No standing variable. This is "a place of unfinished business" and reuses the modular-ship data the game already has.

---

## G. Step-by-step mechanics


### G.1 Starsector: salvage, derelicts, survey, blueprints, story points
1. Detection: sensor strength vs object detection range (derelict ship 1000 su, probe 1700,
   survey ship 2700, mothership 3500); found objects stay on the map
   (https://starsector.wiki.gg/wiki/Exploration).
2. Salvage cost: crew + heavy machinery per target (derelict ship 30 crew / 10 machinery;
   mothership 300 / 100); Salvaging skill up to +50%, Salvage Rig +25%
   (https://starsector.wiki.gg/wiki/Salvaging).
3. Derelict defence drones escalate as you destroy more derelicts [snippet]
   (https://starsector.wiki.gg/wiki/Domain_Exploration_Derelict).
4. Survey: costs supplies + heavy machinery; Surveying Equipment hullmod cuts cost 5-40 by
   hull size; orbital junk hints ruin size (scattered .. vast)
   (https://starsector.wiki.gg/wiki/Exploration).
5. Loot: supplies, fuel, metals, weapons, ships, blueprints, AI cores, colony items;
   outer sector is not richer, only further (fuel / supply strain) (same page).
6. Blueprints: package vs single; known list is persistent; Heavy Industry -> Custom
   Production; black-market sale leaks to pirates (https://starsector.wiki.gg/wiki/Blueprint).
7. Recovered ships carry D-mods; restoration formula above; D-mods reduce supply use
   (https://starsector.wiki.gg/wiki/D-mod).
8. Story points: 4 per level, keep accruing past level 15; s-mod a hullmod (frigate refunds
   75% bonus XP, capital 0%); "The less long-term or impactful the use, the more bonus
   experience it grants" (https://starsector.wiki.gg/wiki/Story_Points). Blog post text
   itself is behind CAPTCHA; design summary from the X post: SP "encourage players to
   create their own story", max 2 s-mods per ship at the time [snippet]
   (https://fractalsoftworks.com/2019/07/08/skills-and-story-points/).
Nova relevance: everything in 1-7 is faction-free. Only skills / levels (8) collide.

### G.2 Avorion: block materials and subsystem sockets
1. Material = ring distance; mine it, get it; first use needs Building Knowledge.
2. Knowledge: Titanium mailed on first mine; later tiers from a boss, a hard pirate fight,
   or purchase at an allied shipyard [snippet] (2.0 announcement, thread 4841941418351191389).
3. Socket cap by knowledge: 5/6/8/10/12/15; sockets also need processing power from
   functional blocks (steam guide 2862685514).
4. Material also sets block stats and unlocks block types (shapes.inc materials page).
5. Rift at the core needs Avorion hyperspace cores (same).
Note: step 2 purchase path uses faction relations; the boss / pirate path does not.

### G.3 Subnautica: depth-module gating
1. Crush depths: Seamoth 300 / 500 / 900 m (MK1-3); Cyclops 900 / 1300 / 1700; Prawn
   1300 / 1700 [snippet] (https://subnautica.fandom.com/wiki/Upgrade_Modules).
2. MK1 modules at the Vehicle Upgrade Console (Moonpool); MK2 / MK3 at the Modification
   Station; Cyclops modules in its engine room [snippet] (same).
3. Recipes need materials from the next depth band (e.g. Prawn MK1: plasteel, nickel x3,
   ruby x2) so each upgrade is built from the frontier it opens [snippet] (same).
4. Players name the Moonpool and the Cyclops as the true gates; a 10 h fragment hunt is
   "more of a WALL than a gate"; "the biggest gate is mental.. it's my fear"
   (https://steamcommunity.com/app/264710/discussions/0/1473095331495255292/).
5. GDC 2019 talk: pillars, "adding structure to a sandbox through radio signals",
   "mysterious tooltips", "embracing player phobias"
   (https://gdcvault.com/play/1025745/The-Design-of-Subnautica,
   https://www.gamedeveloper.com/design/video-inside-the-design-of-i-subnautica-i-).

### G.4 Elite Dangerous: engineering grades and fleet carrier
1. Each module has grades 1-5; each roll consumes materials; rolls per grade vary ("as
   little as one roll at grade one ... about 8 at grade 5"); materials: raw, manufactured,
   data; storage 300 at grade 1 down to 100 at grade 5; experimental effects are bought,
   not rolled; pinned blueprints can be applied remotely; material traders convert
   (https://tothebreach.wordpress.com/2018/03/16/elite-dangerous-3-0-engineers/).
2. Pre-3.0 rolls were RNG; 3.0 principle: every roll improves the module [snippet]
   (https://steamcommunity.com/app/359320/discussions/0/1692659769940711680/).
3. Complaints: "GrindSim 9000"; engineers scattered so "you have to spend hours jumping";
   trader rates make corrections a net loss; defence: "you chose that path, then complain
   about the path" (https://steamcommunity.com/app/359320/discussions/0/3826411948444487050/).
4. Carrier: 5 bn buy, 7.5-32 M / week upkeep, services, jump all ships at once
   (thread 846244042087864441). Odyssey re-used material grind as suit progression
   (51 manufacturing instructions for a G5 suit) [snippet]
   (https://steamcommunity.com/app/359320/discussions/0/3145133215257268689).

### G.5 Space Engineers: survival progression tree
1. Optional, on by default; blocks locked until a prerequisite is hand-welded.
2. Tree from ResearchBlocks.sbc / ResearchGroups.sbc; first tier free (armour, basic
   assembler, O2/H2 gen, landing gear, cargo, solar); O2/H2 gen -> H2 tanks -> H2 thrusters.
3. The welded block can be ground back down; ship welders do not count.
4. Locked blocks cannot be repaired or hacked, so salvage of found wrecks is blocked.
(https://spaceengineers.wiki.gg/wiki/Progression)

### G.6 No Man's Sky: ship class / slots and freighter
1. Class C/B/A/S sets slot count ceiling and bonus; nanite upgrade C->S at a station;
   cosmetics never change; "there's rarely any point to buying a C or B class starship"
   (thread 3464976518706618346).
2. Freighter: class fixed at purchase; S = 48 + 21 slots; bulkheads from derelicts; 9
   storage units x 20 slots; frigate modules; fleet expeditions; base rooms
   (gamerguides freighter page).
3. Endgame complaint: main quest ends ~30 h; "wide as an ocean, deep as a puddle"
   (thread 3874841497810499196).

### G.7 Everspace 2: gear and crafting
1. Dismantle items for rarity-tier parts; dismantling 3 of a type + rarity grants its
   blueprint; blueprints cap at Superior, no crafted Legendaries.
2. Modify found items: +1 rarity, raise item level, remove level lock; catalysts are
   permanent; crafted items cannot be modified
   (https://itemlevel.net/everspace-2-ultimate-crafting-guide/).
3. Regions have level bands; enemies scale toward but not up to the player (dev quotes in
   thread 5879990700813147728).


### G.8 Highfleet city repair/refit (Shipworks)

1. Enter a city; open Shipworks. Ships listed at the top; drag to set repair priority (https://www.magicgameworld.com/highfleet-shipworks/).
2. Click a ship. Damaged parts flash yellow or red. Right-click a part to repair, or press auto-repair and set a slider for overall repair level.
3. Destroyed parts need the part in stock or the city shop must sell it; the game pulls from inventory first, then buys (same source).
4. The repair schedule shows time and cost. Cost shown is the full no-stock price and also sets install time; money only leaves for parts you lack (https://steamcommunity.com/app/1434950/discussions/0/3051737007088540886/).
5. Removing parts is free and instant; adding is treated like repair, so dock bonuses apply to refits too (same source).
6. Optional: "Put in docks" lands the ship on a numbered landing spot for a repair speed bonus by spot difficulty; "Faster Repairs" cities give larger bonuses, only for the docked ship (https://steamcommunity.com/sharedfiles/filedetails/?id=2580804883).
7. Leave the city. Work runs on the map clock, hours to days; staying long makes cities "belligerent or warn the enemy"; leaving early returns uninstalled modules to inventory (magicgameworld above).
8. Criticised: unclear cost display, 1000-hour queues after bad fights, fuel burn while landed after a trap event (threads above).

### G.9 Hardspace Shipbreaker certification and tool loop

1. Pick a work order at the hab; ships by grade (fuel hazards at grade 2, electrical at grade 3) (https://hardspaceshipbreaker.fandom.com/wiki/Certification_Rank, snippet).
2. Run a timed shift (Open Shift removes the timer); cut, tether, sort into furnace, processor, barge; extract the reactor before meltdown (https://en.wikipedia.org/wiki/Hardspace:_Shipbreaker).
3. Salvage goals pay LYNX tokens; credits pay the debt; tools rent per shift (Wikipedia and reviews above).
4. Rank up by work orders (140 at rank 8, complaint above) and Mastery Points (snippet above); ranks unlock ship types (Mackerel at 2, Javelin at 5, Gecko at 14) (Certification Rank snippet above).
5. At rank 4 the hab Equipment tab opens; buy cutter range, grapple/tether charges, durability drain, suit bars with tokens; each upgrade needs a rank (https://hardspaceshipbreaker.fandom.com/wiki/Upgrades, snippet).
6. Repair tools and suit at the hab workstation with a repair kit (https://steamcommunity.com/app/1161580/discussions/0/3395163747096925301/).
7. What worked: tactile disassembly ("derelicts are simply a delight to break apart", PC Gamer via Wikipedia). Criticised: repetition, rank grind on easy ships (threads above).

### G.10 Dredge shipwright grid and research

1. Boat is a cargo grid; equipment occupies grid cells; hull tiers add cells (Tier 5 adds 11) and raise Resistance (durability against creatures) (https://strategywiki.org/wiki/DREDGE/Boat, snippet; https://gamerant.com/dredge-best-boat-upgrades/).
2. Regular upgrades reassign cargo cells to a slot type (rod, net, engine, light); hull upgrades add cells (StrategyWiki snippet).
3. Research Parts come from dredging wrecks and pursuit rewards; spend them at any dock in four research trees (rods, engines, pots, nets); a researched item then appears for purchase at the shipwright (https://www.neoseeker.com/dredge/guides/Research, snippet; https://steamcommunity.com/sharedfiles/filedetails/?id=2956824687, snippet).
4. Buy the upgrade with money plus lumber, metal scrap, cloth, refined metal; Tier 4 hull: 4 wood, 3 metal scraps, 4 cloth, 3 research parts, 1 500 (gamerant above).
5. Installation takes in-game time; bigger upgrades take longer (gamerant).
6. Damaged boat sections are repaired at the shipwright for money (gamerant, durability note). Pursuit books give passive bonuses such as lower upgrade costs (indiegameculture above).

### G.11 Barotrauma outpost upgrade system

1. Dock at an outpost; talk to the Chief Mechanic (Upgrade icon) in the Reactor sector; Maintenance bay mechanic sells subs (https://barotraumagame.com/wiki/Shipyard, snippet).
2. Pick a category (13 categories, 20 upgrades); each level costs more than the previous (snippet above).
3. Hull Reinforcements 6 levels +20 % to +120 % wall HP, 2 000 to 8 666 mk, plus 5 bars of steel, then titanium-aluminium, then physicorium per two levels; Engine Power 4 levels +8 % to +32 %, 2 500 to 6 250 mk (snippet above).
4. Pay in marks earned from missions (snippet above).
5. Upgrades persist when switching subs; selling a sub returns part of the upgrade value (dev post above).
6. Same NPC repairs hull and items and recovers lost shuttles (snippet above).
7. Criticised: upgrades "compensate for design flaws", optional, prices tied to reputation (threads above).

### G.12 Cosmoteer career bounty and part purchase

1. Start with a small scout and few crew; a bounty is guaranteed near spawn (https://cosmoteer.wiki.gg/wiki/0.28.0).
2. Hail a station: missions tab (bounty, delivery, exploration), trade tab, crew tab, blueprints tab (https://cosmoteer.wiki.gg/wiki/Game_Modes).
3. Accept a bounty, fly, destroy the target; send crew to salvage the wreck into materials; mine asteroids for copper (https://blog.cosmoteer.net/2022/08/cosmoteer-feature-preview-career-mode.html).
4. Return: cash reward plus fame; fame unlocks crew limit in batches and the LEGENDARY title at 10 000 (0.28.0 and blog above).
5. Buy a blueprint (1 k to 45 k, prerequisite chain) to unlock a part; build in the editor; missing materials auto-buy from the station (https://cosmoteer.wiki.gg/wiki/Blueprints).
6. Repair: destroyed parts drop 85 % of materials, so rebuilding is mostly a time cost near a station (0.28.0 above).
7. Criticised: every station sells the same blueprint list (Blueprints wiki).

### G.13 Battletech and MW5 repair plus salvage

Battletech (HBS):
1. Negotiate contract: slider between cash and salvage priority picks (thread above).
2. Fight; damage location determines parts: 1 (centre torso), 2 (both legs), 3 (head or pilot) (thread above).
3. Post-battle screen: pick priority salvage, rest is random; "2/3" display (thread above).
4. Three parts of one chassis assemble a mech (thread above).
5. Repair: salvage the remaining components off the destroyed location, scrap the location, then replace from warehouse stock (forum above).
6. Mech bays queue; refits early take up to 30 days; Argo upgrades reduce to ~7 days for an assault refit (thread above).
7. Sell at 10 % of base; parts cannot be sold individually (thread above).

MechWarrior 5:
1. Negotiation points (up to 15) split pay, salvage shares, damage insurance (thread above).
2. Shares ~175 k each; a mech caps at 50 shares; damaged mechs cost fewer shares (threads above).
3. Choose salvage from the post-mission list; cleaner kills cost more shares (36 vs 16 Centurion) (thread above).
4. Repair at the hangar; price multiplies in conflict zones up to 300 %; cold storage removes upkeep; full hangar upkeep ~3 M (gamerant and thread above).
5. Criticised: opaque share value, gambling feel, better play punished (threads above). Mods Purchase Salvage and YAML fix it (threads above).

---

## H. Pitfalls and heuristics

### H.1 Catalogue of criticisms with sources
- Grind for its own sake: Elite engineering "GrindSim 9000" (thread 3826411948444487050);
  Avorion "artificially slow ... designed to pad hours" (negative reviews); SC2 "over and
  over and over again" (hardcoregaming101); Freelancer filler "chores" (Wikipedia).
- Number inflation / treadmill: Everspace 2 "every item rarity goes up too ... at level
  25 every item is rare" [snippet] (thread 3035976310909368101); Larian forum: numbers
  "change so quickly that they dictate success" [snippet]
  (https://forums.larian.com/ubbthreads.php?ubb=showflat&Number=620531).
- Bigger-ship-only progression: Elite small weapons penalised, engineering favours big
  hulls (thread 1652171126128080925); Avorion "unlimited ship scaling" removes tactics
  (thread 1319962514593203785); counter-example Star Valor "Big ships are as viable as
  small ships, merely in a different way" (thread 3761103203680949263).
- Fuel / supply micromanagement: Starsector "why is there a mechanic that literally
  punishes you for doing any kind of exploration?" vs "really easy to manage" with tankers
  [snippet] (https://fractalsoftworks.com/forum/index.php?topic=13106.0,
  https://fractalsoftworks.com/forum/index.php?topic=10591.0); Sunless Sea fuel outruns
  cargo profit (scientificgamer); bgdf designer dropped per-turn fuel as "clunky" [snippet]
  (http://www.bgdf.com/forum/game-creation/mechanics/space-game-fuel-mechanic).
- Repairs trivial or brutal: FTL repair "based almost entirely on luck" (stores thread);
  RGO "repair bills can be a bit hefty"; Endless Space 2 "1000 dust to repair ... daft"
  [snippet] (https://steamcommunity.com/app/392110/discussions/0/350544272216509660).
- Nothing to spend money on: Elite tens of billions (endgame thread); X4 "credits ...
  basically useless" (mid-game thread); Cosmoteer resource surplus (thread 3593338530534029923).
- Nothing to do after X hours: NMS ~30 h; Avorion after 0,0; Elite 350 h; Hardspace debt
  "makes absolutely no difference".
- Progression kills skill play: Everspace 2 scaling "obsoletes the idea of progression";
  Sea of Thieves critics vs "skill alone rules".
- Gate that is a wall: Subnautica 10 h fragment hunt; Cosmoteer crew death spiral; SE
  locked blocks block salvage.
- Gate that is arbitrary: SE "weapons depend on landing gear"; Empyrion "either your
  avatar knows how to make something or not".
- Unlock before discovery: Factorio FFF-376 chicken-and-egg.
- Upgrades that do not transfer: Barotrauma sub upgrades trap you in one hull; NMS class
  upgrade cost makes low classes pointless.
- Upkeep that charges real time: Elite carrier "ball and chain".
- Loss that is only subtraction: Elite permanent ship loss "driven them away" [snippet]
  (thread 3827541651932083655); contrast Kenshi prosthetics, Starsector D-mods.
- Linear biome order starves new content: Valheim thread.

### H.2 Heuristics extracted (with the source that backs each)
1. Gate with physics the player can read (crush depth, pickaxe power, rift, radiation),
   not with ranks. Subnautica, Terraria, Starbound, Avorion.
2. Build the key from what lies behind the lock: Subnautica MK recipes, Valheim boss drops,
   Avorion knowledge from the next ring. Factorio: discovery precedes research.
3. Never lock a verb that salvage or repair needs (SE lesson). Let tiers scale a block's
   stats; "start with the smallest/weakest version of every block" (SE2 thread).
4. Make loss partial and re-usable: D-mods cut upkeep; Kenshi limbs become prosthetics;
   rebuy is a percentage, not a wipe. Permanent total loss drives players out.
5. Price repair so it is a decision, not a tax and not a lottery: FTL store randomness
   and RGO repair bills both read as unfair; Highfleet fuel reads as strategy because the
   player can change the design to fix it.
6. Keep a money sink that is a goal, not an obligation: carrier upkeep vs Hardspace
   tokens. If the sink ends (debt paid), something must change in the world.
7. Scale enemies by place, not by player: Everspace 2 dev quotes show the compromise;
   Starbound ties threat to planet tier; Star Valor ties XP to sector level.
8. Refresh old space when the frontier moves (Terraria Hardmode, Starsector derelict
   drones escalate, Kohr-Ah in SC2) so the centre is not the end (Avorion 0,0 complaint).
9. Let small hulls keep a role: speed, signature, cost; small weapons must not be
   penalised against big hulls (Elite); Star Valor mass / power budget.
10. Give horizontal choice per slot (DRG 1-3 options per slot, Factorio 0.15 branches,
    Starbound tier-5 style ores) so late game is build variety, not stat inflation.
11. Split big steps into useful sub-steps (FFF-381 satellites then platforms); Freelancer
    and SC2 show what unrewarded intermediate grind feels like.
12. Blueprints must be scarce and leak (Starsector black market); Cosmoteer "too easy".
13. Knowledge counts as progression only if the world has hidden routes (Outer Wilds,
    quasi-space); mark discoveries persistently (Starsector found objects stay found).
14. Structure the sandbox with signals, not orders (Subnautica radio); Elite veterans'
    "your goals are your own" is the failure case for players who need a thread.
15. Tie the win / end state to a world change, not a counter (Cosmoteer fame 10,000 and
    Hardspace debt both land flat).

### H.3 Talks and blogs located
- GDC 2019 The Design of Subnautica (Charlie Cleveland): vault
  https://gdcvault.com/play/1025745/The-Design-of-Subnautica, free video
  https://www.youtube.com/watch?v=7R-x9NSBS2Y, abstract
  https://gdconf.com/news/dive-deep-design-subnautica-gdc-2019.
- Factorio FFF-376 (trigger tech), FFF-275 (0.17 science), FFF-381 (sub-steps).
- Starsector blog posts (CAPTCHA-blocked at fetch time; listed for manual reading):
  https://fractalsoftworks.com/2016/07/17/exploration-salvage/,
  https://fractalsoftworks.com/2019/07/08/skills-and-story-points/.
- Cosmoteer career preview: https://blog.cosmoteer.net/2022/08/cosmoteer-feature-preview-career-mode.html.
- Avorion 2.0 newsflash: https://steamcommunity.com/games/445220/announcements/detail/5736998873836173612 (body not fetched; snippet only).
- SE2 progression discussion: https://support.keenswh.com/spaceengineers2/pc/topic/53661-balanced-progression-how-we-unlock-blocks.
- Elite "what now" threads: endgame-what.539852, elite-dangerous-lacks-goal.603134,
  gameplay-loops-in-elite-dangerous.543692 (forums.frontier.co.uk).
- Mosolov on UQM: https://pistolshrimpgames.com/2022/11/reflections-on-uqm-from-alex-mosolov-creator-of-starsector/.
- NMS GDC talks are about procedural tech and post-launch recovery, not progression
  [snippet] (https://mcvuk.com/development-news/sean-murray-discusses-no-mans-sky-development-at-gdc/).
- No Valheim GDC talk on gating was found; design intent is only documented in guides.

### H.4 Direct notes for Nova (no factions, no rep, no crew, no levels)
- Every faction-free mechanism above survives the constraints: material rings, depth /
  range / radiation gates, sockets by hull size, service-point refit with a mass budget,
  blueprints found and leaked, D-mod-style partial loss, debt or contract sinks, claims,
  catalogue, discovery map.
- Mechanisms that need replacement: Avorion shipyard knowledge purchase (faction),
  Cosmoteer fame -> crew cap (crew sim), Starsector colony expeditions (faction), Elite
  engineer unlock quests (reputation). Replace with boss / wreck / cache sources and with
  hostile-activity pressure keyed to ally / enemy / neutral only.
- Biggest documented risks for a physical-ship game: repair economics (H.2 #5), small
  hull obsolescence (#9), and the "centre is the end" problem (#8).

---

## Synthesis for Nova

What the evidence supports, phrased as a progression spine that uses only things Nova already simulates (sections, plating, mass, thrust, fuel, cargo, docking, seeded sectors) and stays inside the constraints.

1. **Loss is the progression currency, the station is where it is redeemed.** The games where damage is structural and players still enjoy repair share four rules: most material comes back (Cosmoteer 85 %), a scarred hull stays usable and is cheaper to run (Starsector d-mods), time is the main cost and docking trades money for time (Highfleet), and the bill is visible before commit (every failure case in C2 is an opaque bill). Nova: a destroyed section leaves a debris item with most of its value; a damaged section can be flown as a "patched" variant with worse mass/performance; full rebuild and plating re-attachment are docked-only station services with a price list shown before the player accepts.

2. **Gate regions with physics the player can read, and build the key from what the gate hides.** Subnautica crush depth, Avorion rings and rift, Starbound EPPs, Terraria pickaxe power all work because the gate is physical and the upgrade recipe uses materials from behind it (H.2 #1, #2). Nova's Newtonian sim offers delta-v (fuel/thrust-to-mass), radiation and heat (plating type), debris density (hull strength), and sensor range for free. Tier materials by distance from the start sector, but avoid "the centre is the end" (Avorion complaint) by refreshing old sectors (Terraria hardmode, Starsector escalating derelict drones).

3. **Stations differ from ships by scope of work, stock, storage and a board.** Across B2 the services that create a return loop are docked-only structural repair, selling what you bring back, station-specific stock, a refreshing job board and storage. Pure refuel/restock does not pull. A ship you dock with trades and maybe sells one thing; a station rebuilds, stores, stocks and posts work.

4. **Job board without factions.** Shipped faction-free mission types: procurement/delivery with cargo mass (hauling reward by size, value, risk; damage reduces payout, SE Economy 2), derelict recovery, salvage request (black box / marked item), escort/convoy with pirates scaled to cargo value (Endless Sky), named-hull bounty with hidden difficulty ladder (Starsector), timed offers that expire (Dredge hooded figures). Keep the generator small and physical; Freelancer's "fly there, kill them" boards became chores.

5. **Nemesis = a named hull with a saved section graph.** A pirate that escapes with sections missing is promoted to a persistent named entity carrying its damage, its seed and the player's lost cargo; it returns repaired-with-scars and appears on the board as a bounty. Shadow of Mordor's memory/scars/promotion loop and AC Odyssey's bounty tiers both work without any standing variable (F.3). Nova already persists ship graphs, so this is data retention plus one promotion rule.

6. **Ternary relations are enough.** Pirates as a permanent enemy class (Starsector, SE SPRT), neutrals that turn hostile when attacked with a sector-local decay timer (NMS sentinels), hired wingmen as the purchasable ally (Rebel Galaxy), distress calls that may be fakes (Elite USS). None of these needs a ledger.

7. **Economy: per-station stock, sale price falls with (sold / demand), slow recovery, a flat fee, and shortage events as the real pull.** Starsector (30 % tariff, 120-day fill, explicit deficit/excess numbers), Elite (25 % of demand threshold), Endless Sky (route flattening) and Rebel Galaxy (post-visit price move, famine/rush events) all agree; X4 and Elite show what happens with no sinks. Cargo mass already caps hauling physically (Space Engineers).

8. **Long tail: a discovery log and one finite world-changing goal.** Outer Wilds' ship log (rumor vs explore facts, "more to explore" marker), Subnautica's PDA and Dredge's rarities give a visible completion map; Starsector keeps found objects found. For an end: Minecraft, Valheim, Terraria and Subnautica all give one finite goal that changes the world and let play continue; NMS's galaxy centre (far place, nothing there) is the failure case. Candidate for Nova: a specific large build (hull class or a station of your own) whose parts come from across the tier rings.

9. **Avoid:** fame/crew caps (Cosmoteer death spiral), enemy level scaling (Everspace 2), class upgrades that make lower hulls pointless (NMS), repair that costs more than replacement (Starsector restore formula for common hulls), upkeep that charges real time (Elite carrier), numeric goals that change nothing when reached (Hardspace debt), blueprints sold identically everywhere (Cosmoteer), gates that are 10-hour fragment hunts (Subnautica), locked verbs that block salvage (Space Engineers tree).

---

## Mechanism catalogue

Fit = fit with a Newtonian modular-ship game with allies/enemies/neutral only, no reputation, no crew, no levels. Cost = rough implementation cost given Nova's existing section graph, docking, sector streaming and economy.

| Mechanism | Example games | What it gives the player | Cost to build | Fit and why |
|---|---|---|---|---|
| Docked-only structural repair: rebuild destroyed sections, re-attach plating | Starsector Restore, Avorion repair dock, Highfleet Shipworks, Barotrauma outpost | A reason to return; loss has a remedy and a place | Low-Med: station service UI, section price list, optional time cost | High. Owner's plan; sections and plating are already physical |
| Loss as discount: damaged section stays flyable with worse stats, cheaper upkeep | Starsector d-mods, Kenshi prosthetics | Scarred ship remains playable; restore is a choice | Low: "patched" section variant flag | High. Health per section exists; restore only rare sections |
| Material return on destroyed parts | Cosmoteer 85 %, SE grinding | Softens loss; salvage your own wreck | Low: debris items with value | High. Pick-up/throw of objects exists |
| Time-vs-money repair with dock bonus | Highfleet, Battletech bays | "Where and when to repair" is a decision | Med: wait/clock while docked, queue | Med. Nova is real-time; needs a skip-time-at-dock rule |
| Tiered materials by distance ring | Avorion, Starbound, Terraria ores | Legible spatial progression; reason to push out | Med: tier tables keyed to sector distance; mod-extensible | High for seeded sectors; design the "after the centre" refresh early |
| Capability gates by physics (delta-v, radiation, heat, debris density, sensor range) | Subnautica depth, Avorion rift, Starbound EPP | An upgrade opens a region; recipe uses what the region hides | Med: hazard fields + section stats | High. Newtonian sim gives these gates for free |
| Mass / power budget instead of slots | EV Nova, Star Valor, Avorion sockets | Trade-offs, not stat inflation; small hulls keep a role | Low: already implicit | High. Formalise and show it in the refit UI |
| Blueprints found in the world; station catalogue grows | Starsector, Subnautica fragments, Dredge research | Reason to board derelicts; station visit has new stock | Med: unlock flags, derelict loot tables, blueprint items for mods | High. Avoid needle-in-haystack: show counts and hints |
| Station-specific stock | Elite, Rebel Galaxy, Star Valor, Highfleet rare-part cities | Reason to travel to one station | Low: seeded per-station inventory | High |
| Station storage / section hangar | Starsector storage, Star Valor shared storage, NMS freighter | Anchor: spare sections and ships live somewhere | Low-Med: persistent per-station inventory | High |
| Rotating faction-free job board (delivery by mass, procurement, derelict recovery, salvage request, escort, named-hull bounty) | Starsector generic missions, SE Economy 2, Cosmoteer, Barotrauma | Short-term goals with risk-priced money | Med: contract generator and tracking | High. Few types, all physical; expire some (Dredge) |
| Named-hull nemesis: escaped ship persists with scars, holds your cargo, posted as bounty | Starsector person bounty, Shadow of Mordor, AC Odyssey mercenaries | Unfinished business; personal story | Med: persist ship graph + seed + sector, one promotion rule | High. Reuses section-graph persistence; no standing variable |
| Pirates as permanent enemy class + provoked-neutral timer | Starsector pirates, NMS sentinels, SE SPRT | Baseline antagonist; neutral turns hostile without a ledger | Low | High. Exactly the ternary model |
| Hired wingman / escort ally | Rebel Galaxy mercenaries, Endless Sky escorts | A purchasable ally slot; force multiplier | Med-High: wingman AI | Med. Fits "allies", AI cost is real |
| Distress calls and signal sources, some fake | Elite USS, FTL beacons | Procedural points of interest; a gamble | Med: sector event spawner | High |
| Per-station price with sale impact, slow recovery, flat fee, shortage events | Starsector, Elite 25 % rule, Endless Sky, Rebel Galaxy | Trade that cannot be farmed; events as a pull | Low-Med | High. Events are the hook; the formula is one line |
| Cargo as physical mass | Space Engineers | Natural cap on hauling; refits needed to haul more | Exists | High |
| Big-ticket goal (next hull, a station) | Elite ladder, Star Valor tiers, Hardspace debt | Long-term money goal | Low: price tags | Med. Pair with a world change or it lands flat (Hardspace) |
| Owned remote home station | Elite carrier, Avorion stations, NMS freighter | A place of your own; repair where none exists | High: ownership, placement, storage | Med. Only worth it where public stations are absent; skip income and raids |
| Discovery log / catalogue with "more to explore" | Outer Wilds ship log, Subnautica PDA, Dredge rarities, Starsector found objects | Visible completion map; long tail | Med: log UI and data; mods add entries | High for a seeded world |
| Frontier refresh / world-state flip | Terraria hardmode, Starsector escalating drones | Old sectors become new | Med: sector re-seed rules | Med. Must be designed with sector streaming |
| Finite world-changing end goal with long tail | Minecraft dragon, Valheim bosses, Subnautica rocket | Closure and a reason to keep playing after | Med-High: content | Med. A specific build, never a far empty place (NMS centre) |
| Bounded loss on ship death (rebuy / token / tow) | Elite rebuy, Avorion reconstruction tokens | Death is not a wipe | Low-Med | Med. Nova loses sections physically; a wreck-tow or token at a station fits |
| Knowledge as progression | Outer Wilds, Factorio trigger tech | The player improves, not the avatar | Med | Med. Works for mechanics (docking, hazards), weak for seeded secrets |
| Fuel / supplies as pacing | Starsector, Highfleet, FTL, Sunless Sea | Route decisions | Low | Med. Delta-v already is fuel; keep it a route choice with depot price variance, never a flat tax |
| Timed / expiring offers and events | Dredge hooded figures, Starsector trade disruptions | Urgency; "come back now" | Low | High |
| Cosmetic-only progression (liveries) | Sea of Thieves | Identity without power | Low-Med | Low-Med. Nice at stations, not a spine |
| Fame / crew caps, enemy level scaling, character levels | Cosmoteer, Everspace 2 | (excluded) | - | Low. Outside constraints; evidence shows death spirals and "obsoletes progression" anyway |


---

## Appendix: part-2 compact takeaways, method and primary sources

### Nova takeaways (compact)

- Station services that pull players back without factions: docked-only section rebuild and plating re-attachment, storage of recovered sections, station-specific stock, a rotating board of named-hull bounties, delivery, salvage-request and derelict-recovery jobs, and a sell point for scan data.
- Make loss a discount as well as a cost (Starsector d-mods), return most of the material (Cosmoteer 85 %), charge time more than money, let landing or a home station trade money for time (Highfleet docks), and show the real bill before the player commits (Highfleet, Avorion, MW5 all failed this).
- Price enemy salvage by remaining section health and never charge more for a clean kill (MW5 lesson).
- One finite world-changing goal plus a long tail (Valheim, Terraria), never a far location with nothing at it (NMS centre).
- A home station earns its place only where public stations are absent (Elite fleet carrier lesson); skip income and raids, which drag in factions.

### Method and confidence

- Searches run: about 45. Pages fetched and read: about 60 (Steam threads, wiki.gg, egosoft wiki, roguey, gamerant, substack, Wikipedia, gamedeveloper.com, nomanssky.com, nh.outerwildsmods, anandtech, battletech.com forum).
- Fetch refused (402/403/429), so cited as "snippet" from the search summary only: all fandom.com wikis, fractalsoftworks.com blog and forum, barotraumagame.com wiki, neoseeker, strategywiki, techraptor, the Cosmoteer "Bullet Point Basics" Steam guide, Steam threads behind the mature-content gate (Sunless Sea, Sunless Skies), and web.archive.org (blocked by the tool).
- Snippet claims to re-verify before quoting in durable docs: Elite carrier module prices, Barotrauma upgrade level prices, Avorion reconstruction token percentages, Cosmoteer fame numbers, Hardspace rank unlock list, Highfleet fuel prices (the fuel prices come from a fetched Steam guide and are fine).
- Not researched for lack of time or sources: Mechwarrior 2 Mercenaries repair, From the Depths beyond repair bots, Kerbal (no page fetched), Subnautica Cyclops as base, Space Engineers base, Star Valor base-building DLC detail.

### Most useful primary pages (fetched in full)

- Highfleet Shipworks mechanics: https://www.magicgameworld.com/highfleet-shipworks/
- Highfleet city services and loop: https://steamcommunity.com/sharedfiles/filedetails/?id=2580804883
- Starsector d-mods and restore formula: https://starsector.wiki.gg/wiki/Damage_hullmod
- Starsector refit screen (space vs dock): https://starsector.wiki.gg/wiki/Refit_Screen
- Starsector generic (faction-light) missions: https://starsector.wiki.gg/wiki/Generic_missions
- Starsector named bounties and level scaling: https://starsector.wiki.gg/wiki/Bounty
- Starsector exploration and salvage rewards: https://starsector.wiki.gg/wiki/Exploration
- Starsector colonies: https://starsector.wiki.gg/wiki/Colony
- Cosmoteer career design preview: https://blog.cosmoteer.net/2022/08/cosmoteer-feature-preview-career-mode.html
- Cosmoteer 85 % material return and salvage UX: https://cosmoteer.wiki.gg/wiki/0.28.0
- Cosmoteer blueprint prices: https://cosmoteer.wiki.gg/wiki/Blueprints
- Avorion repair pricing threads: https://steamcommunity.com/app/445220/discussions/0/2143092024472431574/ and https://steamcommunity.com/app/445220/discussions/0/135510669606528475/
- Avorion reconstruction token critique: https://steamcommunity.com/app/445220/discussions/6/1649918058732026688/
- Avorion station economics: https://steamcommunity.com/sharedfiles/filedetails/?id=2981872758
- Space Engineers repair-service critique: https://steamcommunity.com/app/244850/discussions/0/833871657871478060/
- Battletech repair prerequisite: https://battletech.com/forums/index.php?topic=77274.0
- Battletech refit time complaint and defence: https://steamcommunity.com/app/637090/discussions/0/1696048786954524472/
- MW5 salvage penalises clean kills: https://steamcommunity.com/app/784080/discussions/0/4411920671413611943/
- Barotrauma upgrade design intent: https://barotraumagame.com/gameplay-features/campaign-progression-submarines-some-other-things/
- Elite fleet carrier owner considerations: https://steamcommunity.com/app/359320/discussions/0/4353365620733038584/
- Elite grind debate: https://steamcommunity.com/app/359320/discussions/0/1740013932904973885/?ctp=4
- NMS freighter as mid-game hook: https://nomanssky.wiki/starships/freighter-guide/
- NMS expeditions design: https://www.nomanssky.com/expeditions-update/
- NMS galaxy centre anticlimax: https://mein-mmo.de/en/no-mans-sky-the-center-of-the-galaxy-the-biggest-disappointment-in-the-game,107435/
- Outer Wilds ship log data model: https://nh.outerwildsmods.com/guides/ship-log/
- Nemesis system rationale: https://www.gamedeveloper.com/design/designing-i-shadow-of-mordor-i-s-nemesis-system
- Valheim loop analysis: https://bottomfeeder.substack.com/p/design-deep-dive-valheim
- Aimless sandbox design: https://game-wisdom.com/critical/problems-aimless-game-design
- Dredge pursuits and timed Hooded Figures: https://indiegameculture.com/guides/dredge-side-quests-guide/
- Hardspace work-order grind: https://steamcommunity.com/app/1161580/discussions/0/2527030866866302664/
- Sunless Sea economy opinions: https://forums.anandtech.com/threads/sunless-sea.2420119/post-37154908
- FTL fuel thresholds: https://steamcommunity.com/app/212680/discussions/0/840610807207302397/
