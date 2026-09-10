const ID = /^[a-z0-9][a-z0-9-]*$/;
function fields(value, keys, owner) {
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw new Error(`${owner} must be an object`);
    for (const key of keys)
        if (!(key in value)) throw new Error(`${owner} has no ${key}`);
    for (const key of Object.keys(value))
        if (!keys.includes(key))
            throw new Error(`${owner} has unknown field ${key}`);
}
function text(value) {
    if (typeof value !== "string" || !value.trim() || value.length > 16384)
        throw new Error("Invalid story text");
}
function id(value) {
    if (typeof value !== "string" || !ID.test(value))
        throw new Error("Invalid story id");
}
function list(value, max = 64) {
    if (!Array.isArray(value) || value.length > max)
        throw new Error("Invalid story list");
}
function point(value) {
    if (
        !Array.isArray(value) ||
        value.length !== 2 ||
        !value.every((n) => Number.isFinite(n) && Math.abs(n) <= 10000)
    )
        throw new Error("Invalid drawing coordinates");
}
function size(value) {
    point(value);
    if (value.some((n) => n <= 0)) throw new Error("Invalid drawing size");
}
function unique(items) {
    const ids = new Set();
    for (const item of items) {
        id(item.id);
        if (ids.has(item.id)) throw new Error("Duplicate story id");
        ids.add(item.id);
    }
}
function validatePanel(panel, illustrated, cast) {
    fields(
        panel,
        illustrated
            ? [
                  "id",
                  "action",
                  "dialogue",
                  "art",
                  "lettering",
                  "labels",
                  "cards",
              ]
            : ["id", "action", "dialogue"],
        "panel"
    );
    id(panel.id);
    text(panel.action);
    list(panel.dialogue);
    unique(panel.dialogue);
    for (const line of panel.dialogue) {
        fields(line, ["id", "speaker", "text"], "dialogue");
        text(line.speaker);
        text(line.text);
        // The nameplate the reader sees and the transcript line they read are
        // this string, so a misspelling ships silently unless it is checked
        // against the episode's own cast.
        const [name, ...rest] = line.speaker.split(" / ");
        if (!cast.has(name))
            throw new Error(`Speaker is not in the episode cast: ${name}`);
        if (rest.length > 1)
            throw new Error(`Speaker carries two deliveries: ${line.speaker}`);
        if (rest.length && !DELIVERIES.includes(rest[0]))
            throw new Error(`Unknown delivery: ${rest[0]}`);
    }
    if (!illustrated) return;
    id(panel.art);
    fields(
        panel.lettering,
        panel.dialogue.map((d) => d.id),
        "lettering"
    );
    for (const line of panel.dialogue) {
        const b = panel.lettering[line.id];
        fields(b, ["at", "width", "tail", "side", "breakAfter"], "balloon");
        point(b.at);
        point(b.tail);
        if (
            !Number.isFinite(b.width) ||
            b.width <= 36 ||
            b.width > 10000 ||
            !["top", "bottom", "left", "right"].includes(b.side)
        )
            throw new Error("Invalid balloon geometry");
        list(b.breakAfter);
        let previous = 0;
        for (const end of b.breakAfter) {
            if (
                !Number.isSafeInteger(end) ||
                end <= previous ||
                end >= line.text.trim().split(/\s+/).length
            )
                throw new Error("Invalid dialogue line break");
            previous = end;
        }
    }
    list(panel.labels);
    list(panel.cards);
    unique([...panel.labels, ...panel.cards]);
    for (const label of panel.labels) {
        fields(
            label,
            [
                "id",
                "text",
                "slot",
                "at",
                "size",
                "color",
                "weight",
                "spacing",
                "anchor",
            ],
            "label"
        );
        text(label.text);
        id(label.slot);
        point(label.at);
        id(label.color);
        if (
            !Number.isFinite(label.size) ||
            label.size <= 0 ||
            label.size > 10000 ||
            !Number.isFinite(label.spacing) ||
            !["normal", "bold"].includes(label.weight) ||
            !["start", "middle", "end"].includes(label.anchor)
        )
            throw new Error("Invalid label styling");
    }
    for (const card of panel.cards) {
        fields(
            card,
            ["id", "name", "place", "date", "note", "at", "width"],
            "location card"
        );
        for (const key of ["name", "place", "date", "note"]) text(card[key]);
        point(card.at);
        if (
            !Number.isFinite(card.width) ||
            card.width <= 0 ||
            card.width > 10000
        )
            throw new Error("Invalid location card width");
    }
}
/**
 * How a line reaches the reader when it is not simply spoken in frame.
 *
 * Mirrors `DELIVERIES` in `comic-script.ts`; this file is plain JS and reads
 * the compiled script, so the closed set is stated on both sides the way the
 * palette roles below already are.
 */
const DELIVERIES = ["off-panel", "comms", "recording"];

/** Validate the whole maintained script before drawing any art. */
function validateScript(script, pageCount) {
    fields(script, ["heading", "footer", "cast", "pages"], "episode script");
    text(script.heading);
    text(script.footer);
    list(script.cast, 64);
    const cast = new Set();
    for (const name of script.cast) {
        text(name);
        if (name.includes("/"))
            throw new Error(`Cast name carries a delivery: ${name}`);
        if (cast.has(name)) throw new Error(`Repeated cast name: ${name}`);
        cast.add(name);
    }
    script.cast = [...cast];
    list(script.pages, 256);
    if (script.pages.length !== pageCount)
        throw new Error("Script page count differs from episode pageCount");
    unique(script.pages);
    const panelIds = [];
    for (const page of script.pages) {
        const illustrated = page.kind === "illustrated";
        if (!illustrated && page.kind !== "script")
            throw new Error("Unknown script page kind");
        fields(
            page,
            illustrated
                ? ["kind", "id", "title", "purpose", "panels", "layout"]
                : ["kind", "id", "title", "purpose", "panels"],
            "script page"
        );
        text(page.title);
        text(page.purpose);
        list(page.panels);
        if (!page.panels.length) throw new Error("Page needs panels");
        if (illustrated)
            fields(
                page.layout,
                page.panels.map((p) => p.id),
                "page layout"
            );
        for (const panel of page.panels) {
            // Named, because these checks fire on an author's edit and the
            // message is the whole diagnostic: "Invalid balloon geometry" with
            // eighteen pages of balloons in the tree is a search, not a report.
            try {
                validatePanel(panel, illustrated, cast);
            } catch (cause) {
                throw new Error(
                    `${page.id}/${panel && panel.id}: ${cause.message}`,
                    { cause }
                );
            }
            panelIds.push(panel);
            if (illustrated) {
                const p = page.layout[panel.id];
                fields(p, ["at", "size"], "panel placement");
                point(p.at);
                size(p.size);
                if (
                    p.at.some((n) => n < 0) ||
                    p.at[0] + p.size[0] > 1500 ||
                    p.at[1] + p.size[1] > 1000
                )
                    throw new Error("Panel outside page");
            }
        }
    }
    unique(panelIds);
}
/** Derive a literal transcript from the same story data that is lettered. */
function transcript(page) {
    return page.panels
        .map((p) =>
            [
                `Panel ${p.id.toUpperCase()}: ${p.action}`,
                ...(p.cards || []).map(
                    (c) =>
                        `Location: ${c.name} / ${c.place} / ${c.date} / ${c.note}`
                ),
                ...(p.labels || []).map((l) => `Display: ${l.text}`),
                ...p.dialogue.map((d) => `${d.speaker}: ${d.text}`),
                ...(p.dialogue.length ? [] : ["No dialogue."]),
            ].join("\n")
        )
        .join("\n\n");
}
/** A derived reading copy, never an independently edited script. */
function scriptMarkdown(script, title) {
    return (
        `<!-- Generated from episode.ts and pages/*.ts. Do not edit. -->\n# ${title}\n\n` +
        script.pages
            .map(
                (p, i) =>
                    `## Page ${i + 1}: ${p.title}\n\n**Purpose:** ${p.purpose}\n\n` +
                    p.panels
                        .map(
                            (panel) =>
                                `### Panel ${panel.id}\n\n${panel.action}\n\n` +
                                [
                                    ...(panel.cards || []).map(
                                        (c) =>
                                            `**Location:** ${c.name} / ${c.place} / ${c.date} / ${c.note}`
                                    ),
                                    ...(panel.labels || []).map(
                                        (l) => `**Display:** ${l.text}`
                                    ),
                                    ...panel.dialogue.map(
                                        (d) => `**${d.speaker}:** ${d.text}`
                                    ),
                                ].join("\n\n")
                        )
                        .join("\n\n")
            )
            .join("\n\n") +
        "\n"
    );
}
function validatePalette(palette) {
    if (!palette || typeof palette !== "object" || Array.isArray(palette))
        throw new Error("Invalid comic palette");
    for (const color of Object.values(palette))
        if (typeof color !== "string" || !/^#[0-9a-f]{6}$/i.test(color))
            throw new Error("Invalid comic palette");
    for (const role of [
        "ink",
        "paper",
        "balloon",
        "speaker",
        "mint",
        "work-screen",
        "work-card-border",
        "work-card-title",
        "work-warning",
        "work-card-text",
        "folio-muted",
        "folio-title",
    ])
        if (!Object.hasOwn(palette, role))
            throw new Error(`Missing comic color: ${role}`);
}
/**
 * The drawn height of a location card, from `comic-panels.ts`.
 *
 * Fixed, because the card's four rows are at fixed offsets; stated here so the
 * escape check below is arithmetic and not a guess.
 */
const CARD_HEIGHT = 137;

/** The height of the shortest possible balloon: `42 + 27 * lines`, one line. */
const BALLOON_MIN_HEIGHT = 69;

/**
 * Every lettered thing on a panel has to fit the panel's OWN scene.
 *
 * Neither failure is loud on its own. A card is drawn inside the panel's clip
 * path, so one that runs off the scene is silently cropped away while the
 * transcript still reads it out. A balloon is drawn OUTSIDE that clip - the
 * panel layer is `overflow: visible`, deliberately, so a tail may cross the
 * frame - so one that runs off the scene overdraws the NEIGHBOURING panel.
 *
 * A balloon's height is the one thing not knowable here: it is
 * `42 + 27 * lines` and the wrap needs the browser's font metrics. So the
 * check is the box origin, its right edge, its tail tip, and the single line
 * every balloon has at minimum. A balloon that escapes only once it wraps to
 * three lines is not caught, and is caught by eye in the rendered page.
 *
 * LABELS are not checked here and cannot be: `comic-panels.ts` appends a label
 * inside the scene's own `data-story-slot` element, so `label.at` is in that
 * SLOT's coordinate space, under whatever transform the generated art gave it -
 * not the panel's. A slot that does not exist is already a loud error there.
 */
function fitsScene(panel, [width, height], where) {
    const escaped = (x, y, w = 0, h = 0) =>
        x < 0 || y < 0 || x + w > width || y + h > height;
    for (const card of panel.cards)
        if (escaped(card.at[0], card.at[1], card.width, CARD_HEIGHT))
            throw new Error(
                `Location card ${card.id} runs off panel ${where}: ` +
                    `${card.width}x${CARD_HEIGHT} at ${card.at} in ${width}x${height}`
            );
    for (const line of panel.dialogue) {
        const b = panel.lettering[line.id];
        if (escaped(b.at[0], b.at[1], b.width, BALLOON_MIN_HEIGHT))
            throw new Error(
                `Balloon ${line.id} runs off panel ${where}: ` +
                    `${b.width}x${BALLOON_MIN_HEIGHT}+ at ${b.at} in ${width}x${height}`
            );
        if (escaped(b.tail[0], b.tail[1]))
            throw new Error(
                `Balloon ${line.id}'s tail points off panel ${where}: ` +
                    `${b.tail} in ${width}x${height}`
            );
    }
}

/** Resolve scene ids only for illustrated pages in the selected episode. */
function compileScript(script, scenes, palette) {
    validatePalette(palette);
    return script.pages
        .filter((p) => p.kind === "illustrated")
        .map((page) => ({
            id: page.id,
            title: page.title,
            transcript: transcript(page),
            definition: {
                layout: "panels",
                title: page.title,
                alt: `${page.title}. ${page.purpose}`,
                number: script.pages.indexOf(page) + 1,
                width: 1500,
                height: 1000,
                heading: script.heading,
                footer: script.footer,
                palette,
                panels: page.panels.map((panel) => {
                    const art = scenes[panel.art];
                    if (!art) throw new Error(`Unknown scene: ${panel.art}`);
                    fields(art, ["file", "size", "slots"], "generated scene");
                    size(art.size);
                    fitsScene(panel, art.size, `${page.id}/${panel.id}`);
                    list(art.slots);
                    const slots = new Set(art.slots);
                    if (slots.size !== art.slots.length)
                        throw new Error("Duplicate scene text slot");
                    const used = new Set();
                    for (const label of panel.labels) {
                        if (!slots.has(label.slot) || used.has(label.slot))
                            throw new Error(
                                `Unknown or repeated text slot: ${label.slot}`
                            );
                        used.add(label.slot);
                        if (!Object.hasOwn(palette, label.color))
                            throw new Error(
                                `Unknown label color: ${label.color}`
                            );
                    }
                    if (used.size !== slots.size)
                        throw new Error(`Unlettered scene slot: ${panel.art}`);
                    const { art: unused, ...story } = panel;
                    return {
                        ...story,
                        image: art.file,
                        sceneSize: art.size,
                        placement: page.layout[panel.id],
                    };
                }),
            },
        }));
}
/** Validate the selected, serialized panel definition at the manifest boundary. */
function validatePanelDefinition(page, assertAsset) {
    fields(
        page,
        [
            "layout",
            "title",
            "alt",
            "number",
            "width",
            "height",
            "heading",
            "footer",
            "palette",
            "panels",
        ],
        "panel page"
    );
    text(page.alt);
    validatePalette(page.palette);
    list(page.panels);
    if (
        page.width !== 1500 ||
        page.height !== 1000 ||
        !Number.isSafeInteger(page.number) ||
        page.number < 1
    )
        throw new Error("Invalid panel page dimensions or number");
    const layout = {};
    const panels = page.panels.map((p) => {
        fields(
            p,
            [
                "id",
                "action",
                "dialogue",
                "lettering",
                "labels",
                "cards",
                "image",
                "sceneSize",
                "placement",
            ],
            "compiled panel"
        );
        const { image, sceneSize, placement, ...story } = p;
        assertAsset(image);
        size(sceneSize);
        layout[p.id] = placement;
        for (const label of p.labels)
            if (!Object.hasOwn(page.palette, label.color))
                throw new Error("Unknown label color");
        return { ...story, art: "generated-scene" };
    });
    // The cast is an AUTHORING registry: it catches a misspelled speaker in the
    // source script, before this page was ever compiled. By the boundary the
    // names are fixed, so re-deriving membership here would assert nothing -
    // the synthetic cast is every name the page actually uses. What still
    // bites at the boundary is the SHAPE of each speaker, which is checkable
    // from the string alone: at most one delivery, and only a known one.
    validateScript(
        {
            heading: page.heading,
            footer: page.footer,
            cast: [
                ...new Set(
                    panels.flatMap((p) =>
                        p.dialogue.map((d) => d.speaker.split(" / ")[0])
                    )
                ),
            ],
            pages: [
                {
                    kind: "illustrated",
                    id: "generated-page",
                    title: page.title,
                    purpose: page.alt,
                    panels,
                    layout,
                },
            ],
        },
        1
    );
}
module.exports = {
    validateScript,
    compileScript,
    transcript,
    scriptMarkdown,
    validatePanelDefinition,
};
