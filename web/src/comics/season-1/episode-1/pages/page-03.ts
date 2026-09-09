import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-3",
    title: "A useful job",
    purpose:
        "The same pickup, delivery commitment, and borrowed-mug exchange, framed closer.",
    panels: [
        panel("3a", {
            art: scene("assignment-window"),
            action: "Elena and Jonah arrange the Aquila pickup, with the waiting Ebro framed by the window.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena",
                    "Aquila has a replacement assembly. Can Kaveri collect it?"
                ),
                say(
                    "jonah-2",
                    "Jonah",
                    "Yes. We can leave as soon as everyone's aboard."
                ),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [25, 20],
                    width: 430,
                    tail: [270, 245],
                    side: "bottom",
                    breakAfter: [4, 7],
                }),
                "jonah-2": balloon({
                    at: [958, 20],
                    width: 433,
                    tail: [1237, 210],
                    side: "bottom",
                    breakAfter: [6],
                }),
            },
        }),
        panel("3b", {
            art: scene("delivery-record"),
            action: "A delivery record connects Ebro and Foundation while Elena explains the commitment.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena",
                    "EarthWorks is expecting Ebro's load aboard Foundation. We need that line back to have it ready on time."
                ),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [20, 20],
                    width: 322,
                    tail: [463, 300],
                    side: "bottom",
                    breakAfter: [3, 6, 10, 15],
                }),
            },
            labels: [
                label("label-1", "EBRO", {
                    slot: "label-1",
                    at: [-98, -25],
                    size: 29,
                    color: "mint",
                    weight: "bold",
                    spacing: 0,
                    anchor: "start",
                }),
                label("label-2", "WATER DELIVERY", {
                    slot: "label-2",
                    at: [-98, 11],
                    size: 17,
                    color: "interior-edge",
                    weight: "normal",
                    spacing: 0,
                    anchor: "start",
                }),
                label("label-3", "FOUNDATION", {
                    slot: "label-3",
                    at: [-98, 52],
                    size: 21,
                    color: "work-warning",
                    weight: "normal",
                    spacing: 0,
                    anchor: "start",
                }),
            ],
        }),
        panel("3c", {
            art: scene("borrowed-mug"),
            action: "Elena closes the exchange with the borrowed-mug joke; Jonah answers from the foreground.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena",
                    "And bring the mug back. We haven't budgeted for a replacement."
                ),
                say("jonah-2", "Jonah", "I'll put it on the cargo list."),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [22, 20],
                    width: 352,
                    tail: [463, 253],
                    side: "bottom",
                    breakAfter: [5, 8],
                }),
                "jonah-2": balloon({
                    at: [24, 177],
                    width: 247,
                    tail: [0, 340],
                    side: "bottom",
                    breakAfter: [4],
                }),
            },
        }),
    ],
    layout: {
        "3a": { at: [42, 86], size: [1416, 415] },
        "3b": { at: [42, 521], size: [680, 422] },
        "3c": { at: [742, 521], size: [716, 422] },
    },
});
