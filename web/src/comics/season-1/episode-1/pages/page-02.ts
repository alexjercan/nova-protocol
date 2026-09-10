import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-2",
    title: "One line down",
    purpose:
        "A failed circulation pump, an isolated line, and the choice to fit a replacement.",
    panels: [
        panel("2a", {
            art: scene("pump-inspection"),
            action: "Leila examines the isolated pump housing with her hands while Jonah listens beside the machinery.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila",
                    "Circulation pump's failed. We've isolated the line. The others are still running."
                ),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [30, 24],
                    width: 513,
                    tail: [420, 170],
                    side: "bottom",
                    breakAfter: [3, 7],
                }),
            },
        }),
        panel("2b", {
            art: scene("residential-console"),
            action: "Samir checks residential supplies at the processing console while Leila examines the isolated line.",
            dialogue: [
                say("samir-1", "Samir", "Residential supply is normal."),
            ],
            lettering: {
                "samir-1": balloon({
                    at: [16, 18],
                    width: 307,
                    tail: [373, 167],
                    side: "bottom",
                    breakAfter: [2],
                }),
            },
            labels: [
                label("label-1", "RESIDENTIAL SUPPLIES", {
                    slot: "label-1",
                    at: [43, 294],
                    size: 15,
                    color: "work-safe",
                    weight: "normal",
                    spacing: 0.5,
                    anchor: "start",
                }),
                label("label-2", "NORMAL", {
                    slot: "label-2",
                    at: [43, 331],
                    size: 26,
                    color: "work-safe",
                    weight: "bold",
                    spacing: 0,
                    anchor: "start",
                }),
            ],
        }),
        panel("2c", {
            art: scene("repair-choice"),
            action: "Leila answers from the machinery wall, the isolated pump housing beside her. Jonah's question comes from off-panel.",
            dialogue: [
                say("jonah-1", "Jonah / off-panel", "Can you rebuild it here?"),
                say(
                    "leila-2",
                    "Leila",
                    "Yes. But we'd lose more production waiting on the overhaul. A replacement gets us running sooner."
                ),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 314,
                    tail: [3, 158],
                    side: "bottom",
                    breakAfter: [],
                }),
                "leila-2": balloon({
                    at: [18, 144],
                    width: 314,
                    tail: [403, 317],
                    side: "bottom",
                    breakAfter: [4, 7, 10, 14],
                }),
            },
        }),
    ],
    layout: {
        "2a": { at: [42, 86], size: [836, 857] },
        "2b": { at: [898, 86], size: [560, 364] },
        "2c": { at: [898, 470], size: [560, 473] },
    },
});
