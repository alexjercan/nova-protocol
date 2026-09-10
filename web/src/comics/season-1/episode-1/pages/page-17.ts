import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-17",
    title: "Back to work",
    purpose:
        "Deliver both the people and the intact part; the community can continue.",
    panels: [
        panel("17a", {
            art: scene("welcome-to-baikal"),
            action: "Kaveri back at Baikal. Elena meets Jonah's crew and Gantry's people at arrival. Owen still receives assistance. Keep the welcome practical and relieved rather than staging a civic ceremony.",
            dialogue: [
                say(
                    "elena-1",
                    "Elena",
                    "Welcome to Baikal. Let's get you settled."
                ),
            ],
            lettering: {
                "elena-1": balloon({
                    at: [18, 18],
                    width: 620,
                    tail: [130, 218],
                    side: "bottom",
                }),
            },
            labels: [
                label("arrival", "BAIKAL / ARRIVALS", {
                    slot: "arrival",
                    at: [950, 54],
                    size: 24,
                    color: "interior-edge",
                }),
            ],
        }),
        panel("17b", {
            art: scene("covers-still-on"),
            action: "Leila and Rina bring the replacement into processing. Return to the visual anchors of page 2: isolated line, surrounding plant still safely operating.",
            dialogue: [
                say("rina-1", "Rina", "Every cover still on it."),
                say("leila-2", "Leila", "Now we can take them off."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [18, 18],
                    width: 420,
                    tail: [160, 230],
                    side: "bottom",
                }),
                "leila-2": balloon({
                    at: [458, 18],
                    width: 420,
                    tail: [700, 234],
                    side: "bottom",
                }),
            },
            labels: [
                label("isolated", "LINE / ISOLATED", {
                    slot: "isolated",
                    at: [327, 190],
                    size: 18,
                    color: "work-warning",
                }),
            ],
        }),
        panel("17c", {
            art: scene("line-back-in-service"),
            action: "Later, with the assembly installed and checks complete, the isolated line returns to service. This is a time cut, not an instant repair. Ebro is still waiting for the delayed load; restarting a pump does not create the missing production immediately.",
            dialogue: [
                say(
                    "leila-1",
                    "Leila / comms",
                    "Line's back. We'll have the load, just not on the old schedule."
                ),
            ],
            lettering: {
                "leila-1": balloon({
                    at: [18, 18],
                    width: 464,
                    tail: [456, 171],
                    side: "bottom",
                }),
            },
            labels: [
                label("later", "LATER", {
                    slot: "later",
                    at: [20, 178],
                    size: 21,
                    color: "work-card-title",
                }),
                label("line", "LINE / IN SERVICE", {
                    slot: "line",
                    at: [38, 245],
                    size: 22,
                    color: "work-safe",
                }),
                label("checks", "CHECKS COMPLETE", {
                    slot: "checks",
                    at: [38, 279],
                    size: 17,
                    color: "work-card-text",
                }),
                label("load", "EBRO / LOAD PENDING", {
                    slot: "load",
                    at: [24, 327],
                    size: 17,
                    color: "work-card-text",
                }),
            ],
        }),
    ],
    layout: {
        "17a": { at: [42, 86], size: [1416, 360] },
        "17b": { at: [42, 466], size: [896, 477] },
        "17c": { at: [958, 466], size: [500, 477] },
    },
});
