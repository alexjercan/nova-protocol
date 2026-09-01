// The docs manifest: the single source of truth for BOTH doc sections of the
// site - /wiki/ (players) and /create/ (mod authors). webpack.config.js
// requires this at config time to generate every page, its crumbs, TOC and
// dev-server rewrite; docs.ts imports it in the browser to render the sidebar,
// search, tags, see-also and index cards. One entry here IS the page - there is
// no second list to keep in sync.
//
// Plain CommonJS on purpose: node (webpack config) consumes it directly and
// the browser bundle picks it up through docs-manifest.d.ts for types.
//
// Page fields:
// - slug: URL segment under the section root ("actions" -> /create/actions/).
//   Children use nested slugs only in the wiki ("sections/hull").
// - md: markdown source, relative to the section's mdDir.
// - category: sidebar group; must be one of the section's categories.
// - tags: small controlled taxonomy - tag chips, search, and the auto
//   "shares a tag" half of See also.
// - summary: one line - index cards, search results, and the meta description.
// - related: explicit cross-links (same-section slugs), first under See also.
// - headings: section headings, so search matches in-page topics too.
// - parent: slug of the parent page. Drives the sidebar nesting, the crumb
//   ("Create / <parent> / <title>") and the parent's #wiki-children grid.
// - toc: render the build-time contents box (the reference pages opt in).
// - icon: asset for the parent's child grid (placeholder until captured).
// - comingSoon: not yet written - muted, non-navigable entry.

const WIKI_PAGES = [
    {
        slug: "getting-started",
        md: "getting-started.md",
        title: "Your first flight",
        category: "Start here",
        tags: ["ui", "flight"],
        summary:
            "The full first flight: launch into the Shakedown Run, learn every gesture beat by beat (burn, lock, GOTO, ORBIT, weapons), then the sandbox and where to go next.",
        related: [
            "keybinds",
            "flight-autopilot",
            "targeting-radar",
            "glossary",
        ],
        headings: [
            "Launch and start",
            "The first two minutes",
            "The Shakedown Run, beat by beat",
            "The sandbox",
            "Where to go next",
        ],
    },
    {
        slug: "glossary",
        md: "glossary.md",
        title: "Glossary",
        category: "Start here",
        tags: ["ui"],
        summary:
            "Short definitions for the recurring terms and units - prograde/retrograde, standoff, sphere of influence, hysteresis, fine-lock, hot weapons, neutralized, point defense, Kinetic/Pierce, the torpedo types, diegetic, and the m / km / m-per-s units.",
        related: ["getting-started", "flight-autopilot", "targeting-radar"],
        headings: ["Units", "Terms"],
    },
    {
        slug: "sections",
        md: "sections.md",
        title: "Ship sections",
        category: "Ships & building",
        tags: ["ships"],
        summary:
            "The modular parts a ship is built from - hull, controller, thruster, turret and torpedo bay - each with its own mass, health and one behavior.",
        related: ["combat-weapons", "flight-autopilot", "hud"],
        headings: [
            "Hull",
            "Controller",
            "Thruster",
            "Turret",
            "Torpedo bay",
            "Variants",
        ],
    },
    {
        slug: "sections/hull",
        md: "sections/hull.md",
        title: "Hull",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-hull.png",
        tags: ["ships"],
        summary:
            "Passive structure and armor - the backbone the other sections mount to.",
        related: ["sections", "combat-weapons"],
        headings: ["Variants"],
    },
    {
        slug: "sections/controller",
        md: "sections/controller.md",
        title: "Controller",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-controller.png",
        tags: ["ships"],
        summary:
            "The steering system that rotates the ship toward a heading; required to fly.",
        related: ["sections", "flight-autopilot"],
        headings: [
            "What sets how hard a ship turns",
            "Stacking controllers",
            "Variants",
        ],
    },
    {
        slug: "sections/thruster",
        md: "sections/thruster.md",
        title: "Thruster",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-thruster.png",
        tags: ["ships"],
        summary:
            "Produces forward thrust and drives the exhaust plume; analog throttle.",
        related: ["sections", "flight-autopilot"],
        headings: ["Variants"],
    },
    {
        slug: "sections/turret",
        md: "sections/turret.md",
        title: "Turret",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-turret.png",
        tags: ["ships", "combat"],
        summary:
            "An articulated mount that aims with intercept lead and fires bullets.",
        related: ["sections", "combat-weapons", "targeting-radar"],
        headings: ["Variants"],
    },
    {
        slug: "sections/torpedo-bay",
        md: "sections/torpedo-bay.md",
        title: "Torpedo bay",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-torpedo-bay.png",
        tags: ["ships", "combat"],
        summary:
            "Fires guided, proportional-navigation torpedoes that deal blast damage.",
        related: ["sections", "combat-weapons"],
        headings: ["Variants"],
    },
    {
        slug: "keybinds",
        md: "keybinds.md",
        title: "Keybinds",
        category: "Interface",
        tags: ["ui"],
        summary:
            "The full control reference: flight, autopilot verbs, radar locking, weapons, camera and interface, for keyboard and gamepad - and all of it rebindable under Settings > Controls.",
        related: [
            "flight-autopilot",
            "targeting-radar",
            "combat-weapons",
            "hud",
            "nova-os",
        ],
        headings: ["Flight", "Targeting and camera", "Weapons", "Interface"],
    },
    {
        slug: "hud",
        md: "hud.md",
        title: "HUD",
        category: "Interface",
        tags: ["ui"],
        summary:
            "What the heads-up display shows: visibility tiers, the diegetic flight readouts, lock brackets and reticles, the target viewfinder, and the story comms panel.",
        related: ["targeting-radar", "flight-autopilot", "keybinds", "nova-os"],
        headings: [
            "Visibility tiers",
            "Flight readouts",
            "Locks and reticles",
            "Target viewfinder",
            "Comms and objectives",
        ],
    },
    {
        slug: "nova-os",
        md: "nova-os.md",
        title: "NOVA OS",
        category: "Interface",
        tags: ["ui"],
        summary:
            "The Tab ship computer: a CRT terminal that freezes the game while you use it, the full command reference, the MAP and SHIP apps, section rebinding, and the monitor's own knobs and sounds.",
        related: ["hud", "keybinds", "flight-autopilot"],
        headings: [
            "Opening and closing",
            "The terminal",
            "Command reference",
            "Apps",
            "Rebinding a section",
            "The monitor",
        ],
    },
    {
        slug: "settings",
        md: "settings.md",
        title: "Settings",
        category: "Interface",
        tags: ["ui"],
        summary:
            "The Settings menu: a master audio volume slider, the Low/Medium/High graphics-quality preset (juice plus low-end visual gating and render scale), and the Controls tab that rebinds every action - reachable from the main menu and the pause menu, remembered across restarts.",
        related: ["keybinds", "hud", "flight-autopilot"],
        headings: ["Audio", "Graphics quality", "Controls"],
    },
    {
        slug: "flight-autopilot",
        md: "flight-autopilot.md",
        title: "Flight & autopilot",
        category: "Flying",
        tags: ["flight"],
        summary:
            "How ships move: Newtonian manual flight, center-of-mass thrust balancing, mass-legible handling, the GOTO / ORBIT / STOP autopilot verbs that fly the real hull, and RCS fine docking thrusters.",
        related: ["gravity-wells", "sections", "keybinds", "settings"],
        headings: [
            "Flight assist",
            "Newtonian mode",
            "Center of mass",
            "GOTO",
            "ORBIT",
            "STOP",
            "RCS",
        ],
    },
    {
        slug: "targeting-radar",
        md: "targeting-radar.md",
        title: "Targeting & radar",
        category: "Combat",
        tags: ["combat", "ui"],
        summary:
            "Deliberate radar locking: hold CTRL to sweep, stance picks the slot (white nav vs red combat), per-section fine-lock, and staged clearing.",
        related: ["combat-weapons", "hud", "factions"],
        headings: [
            "Radar locking",
            "Stances and slots",
            "Fine-lock",
            "Clearing locks",
        ],
    },
    {
        slug: "combat-weapons",
        md: "combat-weapons.md",
        title: "Combat & weapons",
        category: "Combat",
        tags: ["combat"],
        summary:
            "Turrets and torpedoes, speed-driven damage types (Kinetic punches, Pierce rakes, Explosive blasts), how far a round travels, and point-defense fire.",
        related: ["targeting-radar", "sections", "factions"],
        headings: [
            "Turrets",
            "Torpedoes",
            "Damage types",
            "Closing speed",
            "How far a round travels",
            "Blast and armour",
            "Point defense",
        ],
    },
    {
        slug: "gravity-wells",
        md: "gravity-wells.md",
        title: "Gravity wells",
        category: "Flying",
        tags: ["flight", "world"],
        summary:
            "Large asteroids pull ships and torpedoes with real inverse-square gravity; the dominant well is what the ORBIT autopilot flies around.",
        related: ["flight-autopilot", "scenarios"],
        headings: [
            "Inverse-square pull",
            "Sphere of influence",
            "Dominant well",
        ],
    },
    {
        slug: "factions",
        md: "factions.md",
        title: "Factions",
        category: "World",
        tags: ["world", "combat"],
        summary:
            "The player / enemy / neutral relation model that drives acquisition, projectile allegiance and reticle tint.",
        related: ["targeting-radar", "combat-weapons"],
        headings: ["Relations", "Allegiance", "Reticle tint"],
    },
    {
        slug: "scenarios",
        md: "scenarios.md",
        title: "Scenarios",
        category: "World",
        tags: ["world", "modding"],
        summary:
            "What a scenario places into the world and how objectives are wired through events, filters and actions; the scenarios that ship today.",
        related: ["gravity-wells", "sections"],
        headings: [
            "Shipped scenarios",
            "Objectives and events",
            "Beacons and salvage",
        ],
    },
];

// The creator section: a short learning path plus a hierarchical reference.
// Mod files owns the content items; the scenario vocabulary pages (events,
// filters, actions, objects, expressions) sit directly under the reference
// root so the tree stays shallow. The sidebar supports hierarchy recursively.
const CREATE_PAGES = [
    {
        slug: "author-a-scenario",
        md: "author-a-scenario.md",
        title: "Create your first scenario",
        category: "Learn to mod",
        tags: ["guide", "modding"],
        summary:
            "Build and play a small shooting-range scenario in RON by following one objective from setup through events, filters, actions, variables, and victory.",
        related: ["scenarios", "publish-a-mod", "actions"],
        headings: [
            "Start with the working example",
            "Understand the scenario file",
            "Plan one short story",
            "Events, filters, and actions",
            "Start the objective",
            "Finish the objective",
            "Make it yours",
            "Load and play it",
            "Common mistakes",
        ],
    },
    {
        slug: "publish-a-mod",
        md: "publish-a-mod.md",
        title: "Publish a mod",
        category: "Learn to mod",
        tags: ["guide", "modding"],
        summary:
            "Prepare a finished mod release, lint it, generate and preview the static portal, publish it, and ship updates.",
        related: ["mod-files", "author-a-scenario", "reference"],
        headings: [
            "Prepare the release folder",
            "Set the version",
            "Lint the mod",
            "Generate the portal",
            "Publish",
            "Publish an update",
            "Preview the repository portal",
            "What the portal verifies",
            "Publishing failures",
        ],
    },
    {
        slug: "reference",
        md: "reference.md",
        title: "Modding reference",
        category: "Modding reference",
        tags: ["modding", "reference"],
        summary:
            "The complete mod file hierarchy and catalog of campaigns, scenarios, sections, events, filters, actions, objects, expressions, base ids, and assets.",
        related: ["mod-files", "base-content"],
        headings: [
            "How RON content is written",
            "The reference pages",
            "The vocabulary by family",
            "Every construct, A to Z",
        ],
    },
    {
        slug: "mod-files",
        md: "mod-files.md",
        title: "Mod files",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "The mod folder and bundle manifest, content and resource lists, asset schemes, overlay rules, and the content items: campaigns, scenarios, sections, ships, styles, and impacts.",
        related: [
            "campaigns",
            "scenarios",
            "sections",
            "ships",
            "styles",
            "impacts",
        ],
        headings: [
            "Folder structure",
            "The bundle manifest",
            "Balance acknowledgments",
            "Content files",
            "The five content chapters",
            "Paths and dependencies",
            "Overlay behavior",
        ],
    },
    {
        slug: "campaigns",
        md: "campaigns.md",
        title: "Campaign files",
        category: "Modding reference",
        parent: "mod-files",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "Define a campaign id, menu name, and ordered scenario list; connect chapters and add or replace a campaign from a mod.",
        related: ["scenarios", "mod-files"],
        headings: [
            "File shape",
            "Fields",
            "How campaign order works",
            "Adding or replacing a campaign",
            "Check it",
        ],
    },
    {
        slug: "scenarios",
        md: "scenarios.md",
        title: "Scenario files",
        category: "Modding reference",
        parent: "mod-files",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "Define a playable mission or backdrop, its picker metadata and handlers, then script it with events, filters, actions, objects, and expressions.",
        related: ["author-a-scenario", "events", "actions"],
        headings: [
            "File shape",
            "Fields",
            "Handler shape",
            "Scenario scripting chapters",
            "Lifecycle",
            "Add or replace a scenario",
            "Check it",
            "Common mistakes",
        ],
    },
    {
        slug: "sections",
        md: "sections.md",
        title: "Ship sections for mods",
        category: "Modding reference",
        parent: "mod-files",
        toc: true,
        tags: ["modding", "reference", "ships"],
        summary:
            "Define reusable ship parts field by field: shared identity and physics, then hull, thruster, controller, turret, and torpedo behavior and mod overlays.",
        related: ["base-content", "objects", "publish-a-mod"],
        headings: [
            "The Section item",
            "Common fields",
            "Structural link points",
            "Assets and visual transforms",
            "Hull",
            "Thruster",
            "Controller",
            "Turret",
            "Torpedo",
            "Railgun",
            "A section in a mod",
        ],
    },
    {
        slug: "ships",
        md: "ships.md",
        title: "Ships for mods",
        category: "Modding reference",
        parent: "mod-files",
        toc: true,
        tags: ["modding", "reference", "ships"],
        summary:
            "Author a whole hull once as content and spawn it by id: the section layout, cladding and collapse threshold, spawn-time overrides, and one-off hulls.",
        related: ["styles", "objects", "base-content"],
        headings: [
            "Why a ship is content",
            "The Ship item",
            "Spawning one",
            "Changing one hull for one spawn",
            "One-off hulls",
            "Overlay and lint",
            "Base ships",
        ],
    },
    {
        slug: "styles",
        md: "styles.md",
        title: "Ship skin styles for mods",
        category: "Modding reference",
        parent: "mod-files",
        toc: true,
        tags: ["modding", "reference", "ships"],
        summary:
            "Author the look a ship's derived cladding wears: plate material, scattered decoration, the deterministic scatter rule, and how a ship names a style by id.",
        related: ["ships", "sections", "base-content"],
        headings: [
            "What a style can and cannot change",
            "The Style item",
            "The scatter rule",
            "The scatter is deterministic",
            "The frame a greeble is authored in",
            "Using a style",
        ],
    },
    {
        slug: "impacts",
        md: "impacts.md",
        title: "The impact table for mods",
        category: "Modding reference",
        parent: "mod-files",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "Pair a damage type with a material to voice a hit: the Impact item, the single fallback, naming your own materials, and the four base rows.",
        related: ["sections", "objects", "base-content"],
        headings: [
            "The Impact item",
            "How a hit finds its row",
            "Naming a material",
            "Base rows",
            "Destruction is not here",
        ],
    },
    {
        slug: "events",
        md: "events.md",
        title: "Events",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "Everything that can fire a scenario handler: sixteen event kinds, payloads, lifecycle edges, and dispatch order.",
        related: ["filters", "actions", "scenarios"],
        headings: [
            "OnStart",
            "OnUpdate",
            "OnTimerEnd",
            "OnDefeated",
            "OnDestroyed",
            "OnNeutralized",
            "OnEnter",
            "OnExit",
            "Orbit lifecycle",
            "Lock lifecycle",
            "Dispatch order",
        ],
    },
    {
        slug: "filters",
        md: "filters.md",
        title: "Filters",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "The four scenario filter kinds: Entity, Timer, Expression, and Conditional, with payload and fail-closed rules.",
        related: ["events", "expressions", "author-a-scenario"],
        headings: [
            "Entity",
            "Timer",
            "Expression",
            "Conditional",
            "Traps for the unwary",
        ],
    },
    {
        slug: "actions",
        md: "actions.md",
        title: "Actions",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "All 25 actions a scenario handler can run, grouped by spawning, mission, flow, ship state, timers, and camera.",
        related: ["objects", "events", "expressions"],
        headings: [
            "SpawnScenarioObject",
            "ScatterObjects",
            "DespawnScenarioObject",
            "CreateScenarioArea",
            "Objective",
            "ObjectiveComplete",
            "ObjectiveMarkerAttach",
            "ObjectiveMarkerDetach",
            "StoryMessage",
            "HudReadout",
            "HintEmphasisSet",
            "HintEmphasisClear",
            "Outcome",
            "NextScenario",
            "SetSpeedCap",
            "SetControllerVerb",
            "SetAllegiance",
            "ForceTorpedoLaunch",
            "TimerStart",
            "TimerCancel",
            "VariableSet",
            "DebugMessage",
            "SetCamera",
            "Screenshot",
            "SetSkybox",
        ],
    },
    {
        slug: "objects",
        md: "objects.md",
        title: "Scenario objects",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "The six spawnable scenario object kinds: Anchor, Asteroid, Spaceship, Beacon, SalvageCrate, and Light.",
        related: ["actions", "base-content", "sections"],
        headings: [
            "Anchor",
            "Asteroid",
            "Spaceship",
            "The controller",
            "The sections list",
            "Beacon",
            "SalvageCrate",
            "Light",
            "Traps for the unwary",
        ],
    },
    {
        slug: "expressions",
        md: "expressions.md",
        title: "Variables & expressions",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference"],
        summary:
            "The complete scenario expression grammar: literals, factors, terms, conditions, reserved variables, and common recipes.",
        related: ["filters", "actions", "scenarios"],
        headings: [
            "Values: the literal types",
            "Factors: the atoms",
            "Terms: multiply / divide",
            "Expressions: add / subtract",
            "Conditions: the boolean root",
            "Queries and watched variables",
            "Recipes",
            "Traps for the unwary",
        ],
    },
    {
        slug: "base-content",
        md: "base-content.md",
        title: "Base content catalog",
        category: "Modding reference",
        parent: "reference",
        toc: true,
        tags: ["modding", "reference", "ships"],
        summary:
            "Every id and asset the base game ships: section prototype ids, ship and style ids, scenario ids, the campaign, dep://base asset paths, and overlay rules.",
        related: ["sections", "mod-files", "objects", "impacts"],
        headings: [
            "Section prototypes",
            "Impact rows",
            "Scenario ids",
            "Assets: what dep://base/ can reach",
            "The overlay rule",
        ],
    },
];

// The two doc sections. Each owns its URL root, sidebar categories, markdown
// directory and page list; the chrome and page generation both walk this array.
// - `landing` (optional): a markdown page rendered at the section root itself
//   (/create/). The wiki instead keeps a hand-authored index shell
//   (src/wiki.html), registered directly in webpack.config.js.
const DOC_SECTIONS = [
    {
        root: "wiki",
        title: "Wiki",
        homeLabel: "Wiki index",
        searchPlaceholder: "Search the wiki...",
        titleSuffix: "Nova Protocol Wiki",
        mdDir: "src/wiki",
        categories: [
            "Start here",
            "Ships & building",
            "Flying",
            "Combat",
            "Interface",
            "World",
        ],
        pages: WIKI_PAGES,
    },
    {
        root: "create",
        title: "Create",
        homeLabel: "Create home",
        searchPlaceholder: "Search the creator docs...",
        titleSuffix: "Nova Protocol Creator Docs",
        mdDir: "src/create",
        landing: {
            md: "index.md",
            title: "Create",
            description:
                "Make content for Nova Protocol: author scenarios and campaigns in RON, add ship sections, ships and skins, and publish mods to the portal.",
        },
        categories: ["Learn to mod", "Modding reference"],
        pages: CREATE_PAGES,
    },
];

module.exports = { DOC_SECTIONS };
