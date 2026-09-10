import { page, panel, scene, say, balloon, label } from "../../../comic-script";

export default page({
    id: "page-15",
    title: "All three",
    purpose:
        "Nadia crosses last, the passage is closed, and Kaveri backs away from Gantry.",
    panels: [
        panel("15a", {
            art: scene("last-across"),
            action: "From Kaveri's side of the passage, Jonah sees Nadia cross last. A secured case of the crew's working records comes across with her.",
            dialogue: [
                say(
                    "nadia-1",
                    "Nadia",
                    "That's everyone. Three aboard Kaveri."
                ),
                say("jonah-2", "Jonah", "All accounted for."),
            ],
            lettering: {
                "nadia-1": balloon({
                    at: [18, 18],
                    width: 412,
                    tail: [144, 200],
                    side: "bottom",
                }),
                "jonah-2": balloon({
                    at: [448, 18],
                    width: 390,
                    tail: [694, 200],
                    side: "bottom",
                }),
            },
        }),
        panel("15b", {
            art: scene("transfer-side-clear"),
            action: "Rina checks that the passage is clear while Leila works the closing checks at her console. The port is shut behind them.",
            dialogue: [
                say("rina-1", "Rina", "Transfer side clear."),
                say("leila-2", "Leila", "Closing and checking."),
            ],
            lettering: {
                "rina-1": balloon({
                    at: [18, 18],
                    width: 224,
                    tail: [104, 230],
                    side: "bottom",
                }),
                "leila-2": balloon({
                    at: [242, 18],
                    width: 280,
                    tail: [430, 220],
                    side: "bottom",
                }),
            },
            labels: [
                label("closing", "PORT CHECK", {
                    slot: "closing",
                    at: [353, 367],
                    size: 16,
                    color: "work-card-text",
                }),
            ],
        }),
        panel("15c", {
            art: scene("leaving-gantry"),
            action: "Kaveri withdraws along the same connection axis. Gantry is left stranded and whole, with space around it.",
            dialogue: [
                say(
                    "jonah-1",
                    "Jonah / comms",
                    "Baikal, Kaveri. We have all three. Coming home."
                ),
            ],
            lettering: {
                "jonah-1": balloon({
                    at: [18, 18],
                    width: 460,
                    tail: [700, 150],
                    side: "bottom",
                }),
            },
        }),
    ],
    layout: {
        "15a": { at: [42, 86], size: [856, 407] },
        "15b": { at: [918, 86], size: [540, 407] },
        "15c": { at: [42, 513], size: [1416, 430] },
    },
});
