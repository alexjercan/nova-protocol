import {
    page,
    panel,
    scene,
    say,
    balloon,
    label,
    location,
} from "../../../comic-script";

export default page({
    id: "page-5",
    title: "Arrivals",
    purpose:
        "Kaveri reaches Aquila, and the crew check and accept the replacement assembly.",
    panels: [
        panel("5a", {
            art: scene("approach-aquila"),
            action: "Aquila from Kaveri's approach. The freight town spreads across the distance, with Kaveri holding the foreground.",
            dialogue: [
                say(
                    "tomas-1",
                    "Tomas / comms",
                    "Aquila, Kaveri. Inbound for the Clearwell collection."
                ),
            ],
            lettering: {
                "tomas-1": balloon({
                    at: [25, 209],
                    width: 455,
                    tail: [5, 333],
                    side: "left",
                    breakAfter: [5],
                }),
            },
            cards: [
                location("location", {
                    name: "AQUILA",
                    place: "Saturn system",
                    date: "Later",
                    note: "Farspan's freight hub",
                    at: [25, 24],
                    width: 365,
                }),
            ],
        }),
        panel("5b", {
            art: scene("collection-check"),
            action: "In Aquila's pressurized transfer hall, Rina controls the assembly's motion with the handling frame and restraints while Samir compares its identification with the order and delivery record. Crew and equipment hold to handholds and tethers in freefall. Kaveri waits at its berth beyond the hall window.",
            dialogue: [
                say("rina-1", "Rina", "This is ours?"),
                say("samir-2", "Samir", "Matches the order. Leila?"),
                say("rina-3", "Rina", "Before I strap it down."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [18, 18],
                    width: 247,
                    tail: [152, 191],
                    side: "bottom",
                    breakAfter: [],
                }),
                "samir-2": balloon({
                    at: [346, 18],
                    width: 335,
                    tail: [555, 210],
                    side: "bottom",
                    breakAfter: [],
                }),
                "rina-3": balloon({
                    at: [220, 253],
                    width: 262,
                    tail: [181, 250],
                    side: "left",
                    breakAfter: [4],
                }),
            },
            labels: [
                label("label-1", "CLEARWELL", {
                    slot: "label-1",
                    at: [489, 349],
                    size: 14,
                    color: "mint",
                    weight: "normal",
                    spacing: 1,
                    anchor: "start",
                }),
                label("label-2", "ORDER MATCH", {
                    slot: "label-2",
                    at: [489, 370],
                    size: 12,
                    color: "work-card-text",
                    weight: "normal",
                    spacing: 0,
                    anchor: "start",
                }),
            ],
        }),
        panel("5c", {
            art: scene("assembly-inspection"),
            action: "Leila finishes checking the connection faces in person. Samir is beside her, ready to complete the receipt; Rina waits for her check before securing the load.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila",
                    "That's the right one. Keep the protective covers on."
                ),
                say("samir-2", "Samir", "Even the awkward one?"),
                say("leila-3", "Leila", "Especially the awkward one."),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 374,
                    tail: [488, 211],
                    side: "right",
                    breakAfter: [5],
                }),
                "samir-2": balloon({
                    at: [18, 174],
                    width: 315,
                    tail: [0, 240],
                    side: "left",
                    breakAfter: [],
                }),
                "leila-3": balloon({
                    at: [18, 318],
                    width: 341,
                    tail: [503, 293],
                    side: "right",
                    breakAfter: [3],
                }),
            },
        }),
    ],
    layout: {
        "5a": { at: [42, 86], size: [1416, 350] },
        "5b": { at: [42, 456], size: [700, 487] },
        "5c": { at: [762, 456], size: [696, 487] },
    },
});
