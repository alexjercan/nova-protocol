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

// Sidebar group order.
export const WIKI_CATEGORIES: string[] = [
    "Ships & building",
    "Flying",
    "Combat",
    "Interface",
    "World",
    "Modding",
];

export const WIKI_PAGES: WikiPage[] = [
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
            "The PD attitude controller that steers the ship; required to fly.",
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
            "What the heads-up display shows: visibility tiers, the diegetic flight readouts, lock brackets and reticles, and the target viewfinder.",
        related: ["targeting-radar", "flight-autopilot", "keybinds"],
        headings: [
            "Visibility tiers",
            "Flight readouts",
            "Locks and reticles",
            "Target viewfinder",
        ],
    },
    {
        slug: "flight-autopilot",
        title: "Flight & autopilot",
        category: "Flying",
        tags: ["flight"],
        summary:
            "How ships move: Newtonian manual flight, center-of-mass thrust balancing, mass-legible handling, and the GOTO / ORBIT / STOP autopilot verbs that fly the real hull.",
        related: ["gravity-wells", "sections", "keybinds"],
        headings: [
            "Flight assist",
            "Newtonian mode",
            "Center of mass",
            "GOTO",
            "ORBIT",
            "STOP",
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
        comingSoon: true,
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
        comingSoon: true,
    },
    {
        slug: "modding",
        title: "Modding",
        category: "Modding",
        tags: ["modding"],
        summary:
            "The data-driven scenario language for authoring your own missions - documented here once it lands.",
        related: ["scenarios"],
        headings: [],
        comingSoon: true,
    },
];
