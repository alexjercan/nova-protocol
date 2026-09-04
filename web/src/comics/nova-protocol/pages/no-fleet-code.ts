import {
    caption,
    chapterHeader,
    comicPage,
    grid,
    line,
    panel,
    readout,
    speech,
    svg,
    svgAsset,
} from "../../comic-page";

const railStrike = svg(
    { viewBox: [0, 0, 300, 180], label: "Two railgun strikes cross the dark" },
    line({ from: [-20, 65], to: [320, 100], tone: "default", width: 7 }),
    line({ from: [-20, 95], to: [320, 130], tone: "default", width: 7 })
);

export default comicPage(
    chapterHeader({
        number: "01",
        title: "No fleet code",
        subtitle: "Plume // alignment // silence",
    }),
    grid(
        panel(
            { size: "wide" },
            svgAsset("first-shift.svg", {
                alt: "An unidentified naval warship fires across Cutter One's view toward Meridian.",
                focus: "warship",
            }),
            speech(
                "captain",
                "That is an Earth Navy hull. It is turning toward you.",
                "danger"
            )
        ),
        panel({ variant: "impact" }, railStrike),
        panel(
            { variant: "silent" },
            readout(
                ["MERIDIAN CONTROL", "NO CARRIER", "AUTOMATED SIGNAL / WEAK"],
                "danger"
            )
        ),
        caption(
            "Cutter One has no weapon and no place in the attacker's plan. That is why the crew survives.",
            "danger"
        )
    )
);
