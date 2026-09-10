import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-9",
    title: "What it will cost",
    purpose:
        "Tomas, Rina, and Leila each set out what the diversion would take and what it would cost.",
    panels: [
        panel("9a", {
            art: scene("arrival-order"),
            action: "Tomas brings the revised route to Jonah. His console shows an ordering of arrivals: Kaveri, then Gantry's remaining reserves, then the scheduled recovery.",
            dialogue: [
                say(
                    "tomas-1",
                    "Tomas",
                    "We can reach them before their reserves run out. The scheduled recovery can't."
                ),
                say("jonah-2", "Jonah", "And Baikal?"),
                say(
                    "tomas-3",
                    "Tomas",
                    "Later. Enough that we miss the repair plan. Ebro's load won't be ready."
                ),
            ],
            lettering: {
                "tomas-1": balloon({
                    at: [18, 18],
                    width: 624,
                    tail: [163, 234],
                    side: "bottom",
                }),
                "jonah-2": balloon({
                    at: [361, 158],
                    width: 280,
                    tail: [517, 316],
                    side: "bottom",
                }),
                "tomas-3": balloon({
                    at: [18, 451],
                    width: 345,
                    tail: [171, 390],
                    side: "top",
                }),
            },
            labels: [
                label("ordering", "ARRIVAL ORDER / NOT TO SCALE", {
                    slot: "ordering",
                    at: [47, 691],
                    size: 18,
                    color: "mint",
                }),
                label("kaveri", "KAVERI", {
                    slot: "kaveri",
                    at: [47, 794],
                    size: 17,
                    color: "work-card-text",
                }),
                label("reserves", "RESERVES END", {
                    slot: "reserves",
                    at: [211, 794],
                    size: 17,
                    color: "work-card-text",
                }),
                label("recovery", "RECOVERY", {
                    slot: "recovery",
                    at: [435, 794],
                    size: 17,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("9b", {
            art: scene("transfer-assessment"),
            action: "Rina and Leila assess the transfer route beside the secured assembly. Through the window, the replacement stays clamped in its cradle outside.",
            dialogue: [
                say(
                    "rina-1",
                    "Rina",
                    "We can bring three people through here. The assembly stays tied down."
                ),
                say(
                    "leila-2",
                    "Leila",
                    "If their port is sound, we can dock. We don't need to restart their main bus."
                ),
                say("rina-3", "Rina", "Then let's get them off it."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [18, 18],
                    width: 330,
                    tail: [142, 217],
                    side: "bottom",
                }),
                "leila-2": balloon({
                    at: [374, 18],
                    width: 344,
                    tail: [628, 235],
                    side: "bottom",
                }),
                "rina-3": balloon({
                    at: [18, 392],
                    width: 350,
                    tail: [142, 305],
                    side: "top",
                }),
            },
            labels: [
                label("cradle", "EXTERNAL CRADLE / SECURED", {
                    slot: "cradle",
                    at: [226, 384],
                    size: 13,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("9c", {
            art: scene("honest-warning"),
            action: "Tomas stays at the shared work station with the route in front of him. Jonah listens from across the cabin and takes the warning seriously.",
            dialogue: [
                say(
                    "tomas-1",
                    "Tomas",
                    "I want them out too. I want Elena to know what we're spending."
                ),
                say(
                    "jonah-2",
                    "Jonah",
                    "She will. Keep working the intercept."
                ),
            ],
            lettering: {
                "tomas-1": balloon({
                    at: [18, 18],
                    width: 406,
                    tail: [142, 202],
                    side: "bottom",
                }),
                "jonah-2": balloon({
                    at: [444, 18],
                    width: 274,
                    tail: [603, 179],
                    side: "bottom",
                }),
            },
            labels: [
                label("work", "ROUTE WORK", {
                    slot: "work",
                    at: [284, 300],
                    size: 17,
                    color: "mint",
                }),
            ],
        }),
    ],
    layout: {
        "9a": { at: [42, 86], size: [660, 857] },
        "9b": { at: [722, 86], size: [736, 500] },
        "9c": { at: [722, 606], size: [736, 337] },
    },
});
