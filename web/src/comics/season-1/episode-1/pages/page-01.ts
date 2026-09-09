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
    id: "page-1",
    title: "A place to come back to",
    purpose:
        "An inhabited workplace, a coffee break, and an ordinary call from processing.",
    panels: [
        panel("1a", {
            art: scene("baikal-work"),
            action: "Baikal at work, with Ebro and Kaveri berthed against Saturn.",
            dialogue: [],
            lettering: {},
            cards: [
                location("location", {
                    name: "BAIKAL",
                    place: "Saturn system",
                    date: "2078",
                    note: "Clearwell Waterworks' main base",
                    at: [25, 24],
                    width: 365,
                }),
            ],
        }),
        panel("1b", {
            art: scene("coffee-break"),
            action: "Rina and Jonah share a coffee break in Baikal before Leila calls from processing.",
            dialogue: [
                say(
                    "rina-1",
                    "Rina",
                    "Coffee's still hot. Take a minute before somebody finds you another job."
                ),
                say("jonah-2", "Jonah", "I'll settle for half a minute."),
                say(
                    "leila-3",
                    "Leila / comms",
                    "Jonah? Processing line's stopped. Can you come down?"
                ),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [426, 20],
                    width: 500,
                    tail: [620, 215],
                    side: "bottom",
                    breakAfter: [6, 10],
                }),
                "jonah-2": balloon({
                    at: [982, 20],
                    width: 409,
                    tail: [1128, 214],
                    side: "bottom",
                    breakAfter: [],
                }),
                "leila-3": balloon({
                    at: [25, 320],
                    width: 365,
                    tail: [12, 472],
                    side: "bottom",
                    breakAfter: [3, 6],
                }),
            },
            labels: [
                label("label-1", "GAME NIGHT", {
                    slot: "label-1",
                    at: [72, 249],
                    size: 19,
                    color: "mint",
                    weight: "normal",
                    spacing: 1,
                    anchor: "start",
                }),
            ],
        }),
    ],
    layout: {
        "1a": { at: [42, 86], size: [1416, 360] },
        "1b": { at: [42, 466], size: [1416, 477] },
    },
});
