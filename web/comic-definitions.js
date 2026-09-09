const { validatePanelDefinition } = require("./comic-script-build");

function fields(value, names, owner) {
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw new Error(`${owner} must be an object`);
    for (const name of names)
        if (!(name in value)) throw new Error(`${owner} has no ${name}`);
    for (const name of Object.keys(value))
        if (!names.includes(name))
            throw new Error(`${owner} has unknown field ${name}`);
}

/** Validate generated illustration data before it enters a reader manifest. */
function validatePageDefinition(page, assertAsset) {
    if (page?.layout === "panels")
        return validatePanelDefinition(page, assertAsset);
    const keys =
        page?.layout === "lettered"
            ? [
                  "layout",
                  "image",
                  "alt",
                  "width",
                  "height",
                  "palette",
                  "balloons",
              ]
            : ["layout", "image", "alt"];
    fields(page, keys, "page definition");
    if (!["art", "lettered"].includes(page.layout))
        throw new Error("Unknown data page layout");
    if (typeof page.alt !== "string" || !page.alt.trim())
        throw new Error("Page needs alt text");
    assertAsset(page.image);
    if (page.layout === "art") return;
    if (
        ![page.width, page.height].every(
            (n) => Number.isFinite(n) && n > 0 && n <= 10000
        )
    )
        throw new Error("Invalid page dimensions");
    fields(page.palette, ["ink", "balloon", "speaker"], "page palette");
    if (
        !Object.values(page.palette).every(
            (c) => typeof c === "string" && /^#[0-9a-f]{6}$/i.test(c)
        )
    )
        throw new Error("Invalid page palette");
    if (!Array.isArray(page.balloons) || page.balloons.length > 64)
        throw new Error("Invalid balloons");
    for (const balloon of page.balloons) {
        fields(
            balloon,
            ["x", "y", "width", "speaker", "text", "tip", "side"],
            "balloon"
        );
        if (
            ![balloon.x, balloon.y, balloon.width].every(Number.isFinite) ||
            balloon.x < 0 ||
            balloon.y < 0 ||
            balloon.width <= 36 ||
            balloon.x + balloon.width > page.width ||
            balloon.y >= page.height
        )
            throw new Error("Invalid balloon bounds");
        if (
            !Array.isArray(balloon.tip) ||
            balloon.tip.length !== 2 ||
            !balloon.tip.every(Number.isFinite)
        )
            throw new Error("Invalid balloon tip");
        if (!["top", "bottom", "left", "right"].includes(balloon.side))
            throw new Error("Invalid balloon side");
        if (
            ![balloon.text, balloon.speaker].every(
                (t) => typeof t === "string" && t.trim() && t.length <= 16384
            )
        )
            throw new Error("Invalid balloon text");
    }
}

module.exports = { validatePageDefinition };
