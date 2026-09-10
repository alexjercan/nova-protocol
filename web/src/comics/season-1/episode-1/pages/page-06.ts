import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-6",
    title: "Another crew's work",
    purpose:
        "Two crews share the transfer hall at Aquila, clear each other's loads, and introduce themselves.",
    panels: [
        panel("6a", {
            art: scene("shared-workspace"),
            action: "Nadia and Rina share the pressurized transfer hall. Through its windows we see Gantry intact outside. Nadia waits while Rina guides Clearwell's restrained assembly toward the freight lock. Gantry's name is visible on its hull.",
            dialogue: [
                say(
                    "nadia-1",
                    "Nadia",
                    "Gantry here. Are you finished with that space?"
                ),
                say("rina-2", "Rina", "Nearly. It's awkward to turn."),
                say("nadia-3", "Nadia", "I'll keep our load clear."),
            ],
            lettering: {
                "nadia-1": balloon({
                    at: [25, 18],
                    width: 402,
                    tail: [356, 193],
                    side: "bottom",
                    breakAfter: [5],
                }),
                "rina-2": balloon({
                    at: [487, 18],
                    width: 395,
                    tail: [714, 190],
                    side: "bottom",
                    breakAfter: [],
                }),
                "nadia-3": balloon({
                    at: [25, 212],
                    width: 275,
                    tail: [365, 243],
                    side: "right",
                    breakAfter: [4],
                }),
            },
        }),
        panel("6b", {
            art: scene("introductions"),
            action: "Jonah joins Nadia at the hall's handholds after the load clears their shared access. The freight lock has closed behind them, with the assembly restrained inside. Both keep a grip while they talk.",
            dialogue: [
                say("jonah-1", "Jonah", "Thanks for waiting."),
                say(
                    "nadia-2",
                    "Nadia",
                    "Better than chasing both loads. Sen. Gantry."
                ),
                say("jonah-3", "Jonah", "Mercer. Kaveri."),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 267,
                    tail: [129, 191],
                    side: "bottom",
                    breakAfter: [],
                }),
                "nadia-2": balloon({
                    at: [321, 18],
                    width: 339,
                    tail: [543, 194],
                    side: "bottom",
                    breakAfter: [4],
                }),
                "jonah-3": balloon({
                    at: [222, 307],
                    width: 251,
                    tail: [168, 252],
                    side: "left",
                    breakAfter: [],
                }),
            },
            labels: [
                label("freight-lock", "FREIGHT LOCK", {
                    slot: "freight-lock",
                    at: [135, 68],
                    size: 13,
                    color: "mint",
                    weight: "normal",
                    spacing: 1,
                    anchor: "middle",
                }),
            ],
        }),
        panel("6c", {
            art: scene("safe-trip"),
            action: "Rina signals that the assembly is secure for the freight-lock cycle. Nadia lets go of her handhold and reaches away, turning back to her own job. Both crews have somewhere to be.",
            dialogue: [
                say("nadia-1", "Nadia", "Replacement?"),
                say("jonah-2", "Jonah", "Baikal's lost a processing line."),
                say("nadia-3", "Nadia", "Glad you found one."),
                say("jonah-4", "Jonah", "So are we. Safe trip."),
            ],
            lettering: {
                "nadia-1": balloon({
                    at: [18, 18],
                    width: 241,
                    tail: [133, 188],
                    side: "bottom",
                    breakAfter: [],
                }),
                "jonah-2": balloon({
                    at: [310, 18],
                    width: 385,
                    tail: [549, 196],
                    side: "bottom",
                    breakAfter: [4],
                }),
                "nadia-3": balloon({
                    at: [18, 308],
                    width: 291,
                    tail: [132, 276],
                    side: "top",
                    breakAfter: [],
                }),
                "jonah-4": balloon({
                    at: [383, 320],
                    width: 313,
                    tail: [558, 284],
                    side: "top",
                    breakAfter: [],
                }),
            },
            labels: [
                label("freight-lock", "FREIGHT LOCK", {
                    slot: "freight-lock",
                    at: [135, 68],
                    size: 13,
                    color: "mint",
                    weight: "normal",
                    spacing: 1,
                    anchor: "middle",
                }),
            ],
        }),
    ],
    layout: {
        "6a": { at: [42, 86], size: [1416, 400] },
        "6b": { at: [42, 506], size: [680, 437] },
        "6c": { at: [742, 506], size: [716, 437] },
    },
});
