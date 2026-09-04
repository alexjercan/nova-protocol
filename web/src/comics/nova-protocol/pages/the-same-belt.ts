import {
    caption,
    chapterHeader,
    circle,
    comicPage,
    grid,
    panel,
    readout,
    speech,
    svg,
    svgAsset,
    svgText,
} from "../../comic-page";

const contacts = svg(
    { viewBox: [0, 0, 300, 180], label: "Five cleanup ships enter the belt" },
    ...[42, 96, 150, 204, 258].flatMap((x, index) => [
        circle({ center: [x, 78], radius: 16, tone: "danger" }),
        svgText(String(index + 1).padStart(2, "0"), {
            at: [x - 9, 83],
            tone: "danger",
        }),
    ])
);

export default comicPage(
    chapterHeader({
        number: "02",
        title: "Second Shift",
        subtitle: "Fragments // testimony // pursuit",
    }),
    grid(
        panel(
            { size: "wide" },
            svgAsset("second-shift.svg", {
                alt: "Cutter One moves through Meridian fragments as five search lanes enter the field.",
                focus: "wreck",
            }),
            speech(
                "control",
                "Meridian, Cutter One. I have your beacon. I am coming in."
            )
        ),
        panel(
            { variant: "recorders" },
            readout(
                ["DISTRESS RELAY", "ENGINEERING LOG", "BRIDGE RECORDER"],
                "amber"
            )
        ),
        panel({ variant: "contacts" }, contacts),
        caption(
            "The same geography now means something else. Rocks become cover, work lights become recorder signals, and the route home becomes an escape line."
        )
    )
);
