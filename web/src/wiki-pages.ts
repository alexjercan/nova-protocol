// The wiki manifest: the single source of truth for the whole wiki. Every
// piece of chrome (the sidebar, search, tag chips, "see also", and the wiki
// index) is a view of this array, so adding or renaming a page is a one-line
// edit here (plus authoring its HTML and the one-line wikiPage() registration
// in webpack.config.js). See tasks/20260713-225157/SPIKE.md.

export interface WikiPage {
    // URL segment under /wiki/, e.g. "sections" -> /wiki/sections/.
    slug: string;
    title: string;
    // Sidebar group; must be one of WIKI_CATEGORIES.
    category: string;
    // Small controlled taxonomy - drives tag chips and search, and the auto
    // "shares a tag" half of See also.
    tags: string[];
    // One line, shown on the index cards and in search results.
    summary: string;
    // Explicit cross-links (slugs), shown first under See also.
    related: string[];
    // Section headings, so search matches on in-page topics too.
    headings: string[];
    // Not yet written - rendered as a muted, non-navigable "coming soon" entry.
    comingSoon?: boolean;
    // Slug of the parent page, for two-level pages (e.g. each ship section is a
    // child of "sections"). Children nest under their parent in the sidebar and
    // appear as an icon+title grid on the parent's overview page.
    parent?: string;
    // Icon asset for the parent's child grid (placeholder until captured).
    icon?: string;
}

// The wiki nav is segmented into three audience BANDS - "For players" (the game
// manual), "For creators" (authoring scenarios and mods, no Rust), and "For
// developers" (the codebase and engine) - each holding an ordered list of
// category groups. Every page's `category` must be one of the categories listed
// here. WIKI_CATEGORIES is the flattened order, derived from the bands.
export interface WikiSection {
    name: string;
    categories: string[];
}

export const WIKI_SECTIONS: WikiSection[] = [
    {
        name: "For players",
        categories: [
            "Start here",
            "Ships & building",
            "Flying",
            "Combat",
            "Interface",
            "World",
        ],
    },
    {
        name: "For creators",
        categories: ["Scenarios & mods"],
    },
    {
        name: "For developers",
        categories: ["Get started", "Architecture", "Extending"],
    },
];

export const WIKI_CATEGORIES: string[] = WIKI_SECTIONS.flatMap(
    (s) => s.categories
);

export const WIKI_PAGES: WikiPage[] = [
    {
        slug: "getting-started",
        title: "Your first flight",
        category: "Start here",
        tags: ["ui", "flight"],
        summary:
            "The shortest path from launch to flying: New Game into the Shakedown Run, the first two minutes (burn, lock, GOTO, raise weapons and fire), and where to go next.",
        related: [
            "keybinds",
            "flight-autopilot",
            "targeting-radar",
            "glossary",
        ],
        headings: [
            "Launch and start",
            "The first two minutes",
            "Where to go next",
        ],
    },
    {
        slug: "glossary",
        title: "Glossary",
        category: "Start here",
        tags: ["ui"],
        summary:
            "Short definitions for the recurring terms and units - prograde/retrograde, standoff, sphere of influence, hysteresis, fine-lock, hot weapons, diegetic, and the u / u-per-s units.",
        related: ["getting-started", "flight-autopilot", "targeting-radar"],
        headings: ["Units", "Terms"],
    },
    {
        slug: "sections",
        title: "Ship sections",
        category: "Ships & building",
        tags: ["ships"],
        summary:
            "The modular parts a ship is built from - hull, controller, thruster, turret and torpedo bay - each with its own mass, health and one behavior.",
        related: ["combat-weapons", "flight-autopilot", "hud"],
        headings: ["Hull", "Controller", "Thruster", "Turret", "Torpedo bay"],
    },
    {
        slug: "sections/hull",
        title: "Hull",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-hull.png",
        tags: ["ships"],
        summary:
            "Passive structure and armor - the backbone the other sections mount to.",
        related: ["sections", "combat-weapons"],
        headings: [],
    },
    {
        slug: "sections/controller",
        title: "Controller",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-controller.png",
        tags: ["ships"],
        summary:
            "The steering system that rotates the ship toward a heading; required to fly.",
        related: ["sections", "flight-autopilot"],
        headings: [],
    },
    {
        slug: "sections/thruster",
        title: "Thruster",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-thruster.png",
        tags: ["ships"],
        summary:
            "Produces forward thrust and drives the exhaust plume; analog throttle.",
        related: ["sections", "flight-autopilot"],
        headings: [],
    },
    {
        slug: "sections/turret",
        title: "Turret",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-turret.png",
        tags: ["ships", "combat"],
        summary:
            "An articulated mount that aims with intercept lead and fires bullets.",
        related: ["sections", "combat-weapons", "targeting-radar"],
        headings: [],
    },
    {
        slug: "sections/torpedo-bay",
        title: "Torpedo bay",
        category: "Ships & building",
        parent: "sections",
        icon: "assets/icon-torpedo-bay.png",
        tags: ["ships", "combat"],
        summary:
            "Fires guided, proportional-navigation torpedoes that deal blast damage.",
        related: ["sections", "combat-weapons"],
        headings: [],
    },
    {
        slug: "keybinds",
        title: "Keybinds",
        category: "Interface",
        tags: ["ui"],
        summary:
            "The full control reference: flight, autopilot verbs, radar locking, weapons, camera and interface, for keyboard and gamepad.",
        related: [
            "flight-autopilot",
            "targeting-radar",
            "combat-weapons",
            "hud",
        ],
        headings: ["Flight", "Targeting and camera", "Weapons", "Interface"],
    },
    {
        slug: "hud",
        title: "HUD",
        category: "Interface",
        tags: ["ui"],
        summary:
            "What the heads-up display shows: visibility tiers, the diegetic flight readouts, lock brackets and reticles, the target viewfinder, and the story comms panel.",
        related: ["targeting-radar", "flight-autopilot", "keybinds"],
        headings: [
            "Visibility tiers",
            "Flight readouts",
            "Locks and reticles",
            "Target viewfinder",
            "Comms and objectives",
        ],
    },
    {
        slug: "settings",
        title: "Settings",
        category: "Interface",
        tags: ["ui"],
        summary:
            "The Settings menu: a master audio volume slider, the Low/Medium/High graphics-quality preset (juice plus low-end visual gating and render scale), and the read-only keybind reference - reachable from the main menu and the pause menu, remembered across restarts.",
        related: ["keybinds", "hud", "flight-autopilot"],
        headings: ["Audio", "Graphics quality", "Controls reference"],
    },
    {
        slug: "flight-autopilot",
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
        title: "Combat & weapons",
        category: "Combat",
        tags: ["combat"],
        summary:
            "Turrets and torpedoes, typed damage (Kinetic / AP / EMP / Explosive) against per-section resistances, and point-defense fire.",
        related: ["targeting-radar", "sections", "factions"],
        headings: [
            "Turrets",
            "Torpedoes",
            "Damage types",
            "Resistances",
            "Point defense",
        ],
    },
    {
        slug: "gravity-wells",
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
        title: "Scenarios",
        category: "World",
        tags: ["world", "modding"],
        summary:
            "What a scenario places into the world and how objectives are wired through events, filters and actions; the scenarios that ship today.",
        related: ["modding", "gravity-wells", "sections"],
        headings: [
            "Shipped scenarios",
            "Objectives and events",
            "Beacons and salvage",
        ],
    },
    // === For creators: authoring scenarios and mods (RON, no Rust). The
    // "modding" overview is the band's front door; the two guides come before the
    // deeper data-format / portal reference. ===
    {
        slug: "modding",
        title: "Modding",
        category: "Scenarios & mods",
        tags: ["modding"],
        summary:
            "Content creation, top to bottom: author scenarios and mods as RON data, test them in the game, and publish them to the portal. Start here, then follow the guides.",
        related: [
            "dev/guide-author-scenario",
            "dev/guide-make-a-mod",
            "scenarios",
        ],
        headings: [
            "Author a scenario",
            "Package and share a mod",
            "Extend the engine",
        ],
    },
    {
        slug: "dev/guide-author-scenario",
        title: "Author a scenario (RON)",
        category: "Scenarios & mods",
        tags: ["dev", "guide", "modding"],
        summary:
            "Write a scenario in RON end to end with existing primitives - the file shape, the event/filter/action structure, variables and expressions, and a worked objective loop built up from the shipped scenarios.",
        related: [
            "dev/scenario-system",
            "dev/guide-make-a-mod",
            "dev/guide-extend-scenarios",
        ],
        headings: [
            "The scenario file",
            "Events, filters, actions",
            "Variables and expressions",
            "A worked objective loop",
            "Loading and testing it",
        ],
    },
    {
        slug: "dev/guide-author-section",
        title: "Author a section (RON)",
        category: "Scenarios & mods",
        tags: ["dev", "guide", "modding", "ships"],
        summary:
            "Author a ship part in RON - the Section content item and its BaseSectionConfig, then each SectionKind (hull, thruster, controller, turret, torpedo) with every field grounded in the shipped catalog, plus overlaying a base section in a mod.",
        related: [
            "dev/guide-make-a-mod",
            "dev/guide-author-scenario",
            "dev/sections",
        ],
        headings: [
            "The Section item",
            "Hull",
            "Thruster",
            "Controller",
            "Turret",
            "Torpedo",
            "A section in a mod",
        ],
    },
    {
        slug: "dev/guide-make-a-mod",
        title: "Make and publish a mod",
        category: "Scenarios & mods",
        tags: ["dev", "guide", "modding"],
        summary:
            "The mod-author lifecycle end to end - bundle anatomy and the stemmed-extension rule, overlay semantics, local testing, publishing to the portal with nova_portal_gen, what the player sees, and the honest sharp edges.",
        related: [
            "dev/modding-ron",
            "dev/mod-portal",
            "dev/guide-author-scenario",
            "dev/guide-author-section",
        ],
        headings: [
            "Bundle anatomy",
            "Overlay semantics",
            "Testing locally",
            "Publishing to the portal",
            "What the player sees",
            "Sharp edges",
        ],
    },
    {
        slug: "dev/modding-ron",
        title: "Modding data format (RON)",
        category: "Scenarios & mods",
        tags: ["dev", "modding"],
        summary:
            "The RON data format for scenarios and mods: the catalog, bundles and enabled set, the local download cache and the mods:// source, file naming, and RON syntax gotchas.",
        related: ["dev/scenario-system", "dev/mod-portal", "modding"],
        headings: [
            "Architecture decisions",
            "RON syntax notes (gotchas)",
            "Mods: catalog + bundles + enabled set",
            "File naming",
        ],
    },
    {
        slug: "dev/mod-portal",
        title: "Mod portal",
        category: "Scenarios & mods",
        tags: ["dev", "modding"],
        summary:
            "The static mod portal: its layout, the catalog generator, the catalog.json wire schema, how to publish a mod today, local development, and how installed mods are stored game-side.",
        related: ["dev/modding-ron", "dev/development", "dev/guide-make-a-mod"],
        headings: [
            "Layout",
            "The generator",
            "The wire schema (catalog.json)",
            "Publishing a mod",
            "How installed mods are stored",
        ],
    },

    // === For developers: the codebase and engine. Rendered from markdown under
    // src/wiki/dev/ (see WIKI_DOC_PAGES in webpack.config.js); slugs are
    // `dev/`-prefixed and must match that list. ===
    {
        slug: "dev/development",
        title: "Building & running",
        category: "Get started",
        tags: ["dev", "build"],
        summary:
            "The developer's getting-started: toolchain, everyday cargo commands, features, examples, the web build, and the versioning/release checklist.",
        related: [
            "dev/architecture",
            "dev/mod-portal",
            "dev/keeping-docs-in-sync",
        ],
        headings: [
            "Toolchain",
            "Everyday commands",
            "Features",
            "Examples",
            "Web build",
            "Versioning and release",
        ],
    },
    {
        slug: "dev/keeping-docs-in-sync",
        title: "Keeping docs in sync",
        category: "Get started",
        tags: ["dev", "docs"],
        summary:
            "The map of documentation surfaces (CHANGELOG, News, wiki, tutorial) and what to update when you change code or cut a release - so nothing drifts.",
        related: ["dev/development", "dev/architecture", "dev/mod-portal"],
        headings: [
            "The documentation surfaces",
            "When you change code",
            "The dependency map",
            "When you cut a release",
            "Adding or renaming a page",
        ],
    },
    {
        slug: "dev/project-tour",
        title: "Project tour",
        category: "Get started",
        tags: ["dev", "onboarding"],
        summary:
            "The 20-minute front door: the crate map at a glance, where each kind of thing lives, the app boot path, and a 'want to change X? start here' table.",
        related: [
            "dev/architecture",
            "dev/development",
            "dev/guide-add-section",
            "dev/guide-make-a-mod",
        ],
        headings: [
            "Crate map at a glance",
            "The boot path",
            "Want to change X? Start here",
            "Where to go next",
        ],
    },
    {
        slug: "dev/architecture",
        title: "Architecture",
        category: "Architecture",
        tags: ["dev", "architecture"],
        summary:
            "How the codebase fits together: the crate map and dependency graph, app assembly and plugin order, the state machines, and the Update vs FixedUpdate frame flow.",
        related: ["dev/development", "dev/scenario-system", "dev/sections"],
        headings: [
            "Crate map",
            "App assembly",
            "States",
            "Frame flow",
            "Assets",
        ],
    },
    {
        slug: "dev/sections",
        title: "Ship sections (internals)",
        category: "Architecture",
        tags: ["dev", "architecture", "combat"],
        summary:
            "The section components and how a ship is built from them, the integrity pipeline (damage -> disable -> destroy), typed damage against resistances, and ammo slots.",
        related: ["sections", "dev/architecture", "combat-weapons"],
        headings: [
            "Sections",
            "Building a ship",
            "Integrity: damage -> disable -> destroy",
            "Typed damage",
            "Ammo",
        ],
    },
    {
        slug: "dev/scenario-system",
        title: "Scenario engine",
        category: "Architecture",
        tags: ["dev", "architecture", "modding"],
        summary:
            "The event-driven scenario/modding engine: scenario structure, loading, the event/filter/action pipeline, variables and the event world, scenario objects, and where to add new pieces.",
        related: ["scenarios", "dev/modding-ron", "dev/architecture"],
        headings: [
            "Scenario structure",
            "Loading / unloading",
            "Events",
            "Filters",
            "Actions",
            "Scenario patterns",
            "The gate-counter ordering pattern",
            "The act-gating pattern",
            "The Gauntlet worked example",
            "Scenario objects",
            "Adding new pieces",
        ],
    },
    {
        slug: "dev/guide-add-section",
        title: "Add a ship section",
        category: "Extending",
        tags: ["dev", "guide", "ships"],
        summary:
            "The ordered checklist to add a new ship-section kind - section module, the SectionKind enum, damage class and resistances, the section plugin, spawn and editor wiring, an asset prototype, and a runnable example.",
        related: ["dev/sections", "dev/architecture", "dev/project-tour"],
        headings: [
            "Closed by design",
            "The checklist",
            "The config module",
            "Wiring the plugin",
            "Damage and resistances",
            "Editor and prototype",
        ],
    },
    {
        slug: "dev/guide-extend-scenarios",
        title: "Extend the scenario engine",
        category: "Extending",
        tags: ["dev", "guide", "modding"],
        summary:
            "Add a new scenario primitive in Rust - an event kind, filter, action, or scenario-object kind - via the enum-variant + trait-impl + prelude recipe, one worked example each, plus the NovaEventWorld state/command seam.",
        related: ["dev/scenario-system", "dev/guide-author-scenario"],
        headings: [
            "Add an event kind",
            "Add a filter",
            "Add an action",
            "Add a scenario object kind",
            "The NovaEventWorld seam",
        ],
    },
];
