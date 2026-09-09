import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-10",
    title: "The decision",
    purpose: "Refusal changes who pays, not whether preparation proceeds.",
    panels: [
        panel("10a", {
            art: scene("asking-clearwell"),
            action: "Elena on Kaveri's comms display. Behind Jonah, Tomas continues the route work.",
            dialogue: [
                say(
                    "jonah-1",
                    "Jonah",
                    "Three people. We can reach them. It will put the replacement in late."
                ),
                say(
                    "elena-2",
                    "Elena / comms",
                    "Understood. I'll ask EarthWorks to cover the diversion. Keep preparing."
                ),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 570,
                    tail: [208, 169],
                    side: "bottom",
                }),
                "elena-2": balloon({
                    at: [763, 18],
                    width: 635,
                    tail: [1305, 214],
                    side: "bottom",
                }),
            },
            labels: [
                label("contact", "ELENA WARD", {
                    slot: "contact",
                    at: [1016, 180],
                    size: 19,
                    color: "ink",
                }),
                label("location", "BAIKAL", {
                    slot: "location",
                    at: [1016, 208],
                    size: 16,
                    color: "ink",
                }),
            ],
        }),
        panel("10b", {
            art: scene("cost-refusal"),
            action: "Aboard Kaveri, Samir monitors Elena's call with Daniel Reed at EarthWorks Operations. Clear speaker labels identify the two ends of the call. Keep the panel on that exchange, rather than crowding the other preparations behind him.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena / comms",
                    "Kaveri can reach your crew in time. Will EarthWorks cover the diversion?"
                ),
                say(
                    "daniel-2",
                    "Daniel / comms",
                    "No. We have a recovery scheduled."
                ),
                say(
                    "elena-3",
                    "Elena / comms",
                    "Your people won't last that long."
                ),
                say(
                    "daniel-4",
                    "Daniel / comms",
                    "I'm not approving the cost."
                ),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [18, 18],
                    width: 520,
                    tail: [600, 320],
                    side: "right",
                }),
                "daniel-2": balloon({
                    at: [18, 158],
                    width: 520,
                    tail: [790, 461],
                    side: "right",
                }),
                "elena-3": balloon({
                    at: [18, 272],
                    width: 520,
                    tail: [600, 324],
                    side: "right",
                }),
                "daniel-4": balloon({
                    at: [18, 386],
                    width: 520,
                    tail: [790, 461],
                    side: "right",
                }),
            },
            labels: [
                label("baikal", "BAIKAL", {
                    slot: "baikal",
                    at: [593, 305],
                    size: 19,
                    color: "mint",
                }),
                label("elena", "ELENA WARD", {
                    slot: "elena",
                    at: [593, 335],
                    size: 16,
                    color: "work-card-text",
                }),
                label("earthworks", "EARTHWORKS", {
                    slot: "earthworks",
                    at: [593, 385],
                    size: 19,
                    color: "mint",
                }),
                label("operations", "OPERATIONS", {
                    slot: "operations",
                    at: [593, 412],
                    size: 16,
                    color: "work-card-text",
                }),
                label("daniel", "DANIEL REED", {
                    slot: "daniel",
                    at: [593, 442],
                    size: 16,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("10c", {
            art: scene("taking-intercept"),
            action: "Elena speaks to Jonah again. He looks to Tomas, not toward an imaginary approval indicator. Leave room around the final instruction.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena / comms",
                    "Clearwell will cover it. Bring them home."
                ),
                say("jonah-2", "Jonah", "Tomas. Take the intercept."),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [18, 18],
                    width: 520,
                    tail: [121, 201],
                    side: "bottom",
                }),
                "jonah-2": balloon({
                    at: [18, 399],
                    width: 310,
                    tail: [407, 370],
                    side: "right",
                }),
            },
            labels: [
                label("contact", "ELENA / BAIKAL", {
                    slot: "contact",
                    at: [36, 349],
                    size: 16,
                    color: "work-card-text",
                }),
            ],
        }),
    ],
    layout: {
        "10a": { at: [42, 86], size: [1416, 305] },
        "10b": { at: [42, 411], size: [840, 532] },
        "10c": { at: [902, 411], size: [556, 532] },
    },
});
